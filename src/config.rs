use crate::error::{NattyLangFeederError, Result};
use log::{debug, warn};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    OpenAI,
    Claude,
}

impl Provider {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Provider::OpenAI),
            "anthropic" => Ok(Provider::Claude),
            _ => Err(NattyLangFeederError::Config(format!("Unknown provider: {}", s))),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Claude => "anthropic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub default_provider: Provider,
    pub cache_dir: PathBuf,
    pub log_level: String,
    pub global_prompt: Option<String>,
    pub feeds: HashMap<String, FeedConfig>,
}

#[derive(Debug, Clone)]
pub struct FeedConfig {
    pub name: String,
    pub prompt: String,
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub refresh: Option<u64>,
    pub max_articles: Option<u8>,
    pub temperature: Option<f32>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(NattyLangFeederError::Config(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            NattyLangFeederError::Config(format!("Failed to read config file: {}", e))
        })?;

        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self> {
        let mut default_provider = Self::detect_default_provider();
        let mut cache_dir = dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".natty-lang-feeder")
            .join("cache");
        let mut log_level = "info".to_string();
        let mut global_prompt = None;
        let mut feeds = HashMap::new();

        let mut current_feed: Option<(String, FeedConfig)> = None;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check if this is a feed definition
            if trimmed.starts_with("feed ") {
                // Save previous feed if exists
                if let Some((name, feed)) = current_feed.take() {
                    feeds.insert(name, feed);
                }

                // Start new feed
                let feed_name = trimmed.strip_prefix("feed ").unwrap().trim().to_string();
                current_feed = Some((
                    feed_name.clone(),
                    FeedConfig {
                        name: feed_name,
                        prompt: String::new(),
                        provider: None,
                        model: None,
                        refresh: None,
                        max_articles: None,
                        temperature: None,
                    },
                ));
                continue;
            }

            // Check if this is a feed property (indented)
            if line.starts_with("  ") || line.starts_with('\t') {
                if let Some((_, ref mut feed)) = current_feed {
                    if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"');

                        match key {
                            "prompt" => feed.prompt = value.to_string(),
                            "provider" => feed.provider = Some(Provider::from_str(value)?),
                            "model" => feed.model = Some(value.to_string()),
                            "refresh" => {
                                feed.refresh = Some(value.parse().map_err(|_| {
                                    NattyLangFeederError::Config(format!(
                                        "Invalid refresh value: {}",
                                        value
                                    ))
                                })?)
                            }
                            "max-articles" => {
                                feed.max_articles = Some(value.parse().map_err(|_| {
                                    NattyLangFeederError::Config(format!(
                                        "Invalid max-articles value: {}",
                                        value
                                    ))
                                })?)
                            }
                            "temperature" => {
                                feed.temperature = Some(value.parse().map_err(|_| {
                                    NattyLangFeederError::Config(format!(
                                        "Invalid temperature value: {}",
                                        value
                                    ))
                                })?)
                            }
                            _ => {
                                warn!("Unknown feed property at line {}: {}", line_num + 1, key)
                            }
                        }
                    }
                }
                continue;
            }

            // Global settings
            if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "default-provider" => {
                        default_provider = Provider::from_str(value)?;
                    }
                    "cache-dir" => {
                        cache_dir = PathBuf::from(value.replace("~/", &format!("{}/",
                            dirs::home_dir().unwrap().display())));
                    }
                    "log-level" => {
                        log_level = value.to_string();
                    }
                    "global-prompt" => {
                        global_prompt = Some(value.to_string());
                    }
                    _ => warn!("Unknown global setting at line {}: {}", line_num + 1, key),
                }
            }
        }

        // Save last feed if exists
        if let Some((name, feed)) = current_feed {
            feeds.insert(name, feed);
        }

        debug!("Loaded {} feeds from config", feeds.len());

        Ok(Config {
            default_provider,
            cache_dir,
            log_level,
            global_prompt,
            feeds,
        })
    }

    pub fn get_feed(&self, name: &str) -> Result<FeedConfig> {
        self.feeds
            .get(name)
            .cloned()
            .ok_or_else(|| NattyLangFeederError::FeedNotFound(name.to_string()))
    }

    pub fn list_feeds(&self) -> Vec<String> {
        let mut feed_names: Vec<String> = self.feeds.keys().cloned().collect();
        feed_names.sort();
        feed_names
    }

    fn detect_default_provider() -> Provider {
        // Check environment variables to determine default provider
        if std::env::var("OPENAI_API_KEY").is_ok() {
            Provider::OpenAI
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            Provider::Claude
        } else {
            // Default to OpenAI if no keys are set (will error later if needed)
            Provider::OpenAI
        }
    }

    pub fn get_provider(&self, feed: &FeedConfig) -> Provider {
        feed.provider
            .clone()
            .unwrap_or_else(|| self.default_provider.clone())
    }

    pub fn get_model(&self, feed: &FeedConfig, provider: &Provider) -> String {
        if let Some(ref model) = feed.model {
            return model.clone();
        }

        match provider {
            Provider::OpenAI => "gpt-4o".to_string(),
            Provider::Claude => "claude-sonnet-4-5-20250929".to_string(),
        }
    }

    pub fn get_refresh_interval(&self, feed: &FeedConfig) -> u64 {
        feed.refresh.unwrap_or(3600)
    }

    pub fn get_max_articles(&self, feed: &FeedConfig) -> u8 {
        feed.max_articles.unwrap_or(10)
    }

    pub fn get_temperature(&self, feed: &FeedConfig) -> f32 {
        feed.temperature.unwrap_or(0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config_str = r#"
# Global settings
default-provider openai
cache-dir ~/.natty-lang-feeder/cache
log-level info

feed tech-news
  prompt "Find 10 recent AI articles"
  model gpt-4o
  refresh 3600

feed security-news
  prompt "Latest CVEs"
  provider anthropic
  temperature 0.2
"#;

        let config = Config::parse(config_str).unwrap();
        assert_eq!(config.default_provider, Provider::OpenAI);
        assert_eq!(config.feeds.len(), 2);

        let tech_feed = config.get_feed("tech-news").unwrap();
        assert_eq!(tech_feed.prompt, "Find 10 recent AI articles");
        assert_eq!(tech_feed.model, Some("gpt-4o".to_string()));
    }
}
