pub mod openai;
pub mod claude;

use crate::config::{Config, FeedConfig, Provider};
use crate::error::{NattyLangFeederError, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub title: String,
    pub summary: String,
    pub sources: Vec<Source>,
    pub date: DateTime<Utc>,
    pub guid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub title: String,
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn generate_articles(&self, feed_config: &FeedConfig) -> Result<Vec<Article>>;
}

pub fn create_provider(feed_config: &FeedConfig, config: &Config) -> Result<Box<dyn AIProvider>> {
    let provider = config.get_provider(feed_config);
    let model = config.get_model(feed_config, &provider);
    let temperature = config.get_temperature(feed_config);
    let max_articles = config.get_max_articles(feed_config);

    match provider {
        Provider::OpenAI => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| NattyLangFeederError::ApiKeyMissing("OPENAI_API_KEY".to_string()))?;

            Ok(Box::new(openai::OpenAIProvider::new(
                api_key,
                model,
                temperature,
                max_articles,
                config.global_prompt.clone(),
            )))
        }
        Provider::Claude => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| NattyLangFeederError::ApiKeyMissing("ANTHROPIC_API_KEY".to_string()))?;

            Ok(Box::new(claude::ClaudeProvider::new(
                api_key,
                model,
                temperature,
                max_articles,
                config.global_prompt.clone(),
            )))
        }
    }
}
