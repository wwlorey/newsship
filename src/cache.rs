use crate::config::{Config, FeedConfig};
use crate::error::{NewsshipError, Result};
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct CacheMeta {
    generated_at: DateTime<Utc>,
    ttl_seconds: u64,
    expires_at: DateTime<Utc>,
    article_count: usize,
    provider: String,
    model: String,
}

fn get_cache_paths(feed_name: &str, config: &Config) -> (PathBuf, PathBuf) {
    let cache_dir = &config.cache_dir;
    let xml_path = cache_dir.join(format!("{}.xml", feed_name));
    let meta_path = cache_dir.join(format!("{}.meta", feed_name));

    (xml_path, meta_path)
}

fn ensure_cache_dir(config: &Config) -> Result<()> {
    if !config.cache_dir.exists() {
        debug!("Creating cache directory: {}", config.cache_dir.display());
        fs::create_dir_all(&config.cache_dir).map_err(|e| {
            NewsshipError::CacheError(format!("Failed to create cache directory: {}", e))
        })?;
    }
    Ok(())
}

pub fn get_cached_feed(feed_name: &str, config: &Config) -> Result<Option<String>> {
    let (xml_path, meta_path) = get_cache_paths(feed_name, config);

    // Check if cache files exist
    if !xml_path.exists() || !meta_path.exists() {
        debug!("Cache miss: files don't exist for feed '{}'", feed_name);
        return Ok(None);
    }

    // Read and parse metadata
    let meta_content = fs::read_to_string(&meta_path).map_err(|e| {
        NewsshipError::CacheError(format!("Failed to read cache metadata: {}", e))
    })?;

    let meta: CacheMeta = serde_json::from_str(&meta_content).map_err(|e| {
        warn!("Failed to parse cache metadata, ignoring cache: {}", e);
        return NewsshipError::CacheError(format!("Invalid cache metadata: {}", e));
    })?;

    // Check if cache is expired
    let now = Utc::now();
    if now > meta.expires_at {
        debug!(
            "Cache expired for feed '{}' (expired at {})",
            feed_name, meta.expires_at
        );
        return Ok(None);
    }

    // Read cached RSS
    let rss_content = fs::read_to_string(&xml_path).map_err(|e| {
        NewsshipError::CacheError(format!("Failed to read cached RSS: {}", e))
    })?;

    info!(
        "Cache hit for feed '{}' ({} articles, expires in {} seconds)",
        feed_name,
        meta.article_count,
        (meta.expires_at - now).num_seconds()
    );

    Ok(Some(rss_content))
}

pub fn write_cache(
    feed_name: &str,
    rss_xml: &str,
    feed_config: &FeedConfig,
    config: &Config,
) -> Result<()> {
    ensure_cache_dir(config)?;

    let (xml_path, meta_path) = get_cache_paths(feed_name, config);

    // Count articles (simple heuristic: count <item> tags)
    let article_count = rss_xml.matches("<item>").count();

    let ttl_seconds = config.get_refresh_interval(feed_config);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(ttl_seconds as i64);

    // Get provider and model info
    let provider = config.get_provider(feed_config);
    let model = config.get_model(feed_config, &provider);

    let meta = CacheMeta {
        generated_at: now,
        ttl_seconds,
        expires_at,
        article_count,
        provider: provider.as_str().to_string(),
        model,
    };

    // Write RSS XML
    fs::write(&xml_path, rss_xml).map_err(|e| {
        NewsshipError::CacheError(format!("Failed to write RSS cache: {}", e))
    })?;

    // Write metadata
    let meta_json = serde_json::to_string_pretty(&meta)?;
    fs::write(&meta_path, meta_json).map_err(|e| {
        NewsshipError::CacheError(format!("Failed to write cache metadata: {}", e))
    })?;

    info!(
        "Cached feed '{}' ({} articles, expires at {})",
        feed_name, article_count, expires_at
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Provider;
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        Config {
            default_provider: Provider::OpenAI,
            cache_dir: std::env::temp_dir().join("newsship_test_cache"),
            log_level: "info".to_string(),
            global_prompt: None,
            feeds: HashMap::new(),
        }
    }

    #[test]
    fn test_cache_roundtrip() {
        let config = create_test_config();
        let feed_config = FeedConfig {
            name: "test-feed".to_string(),
            prompt: "test prompt".to_string(),
            provider: None,
            model: None,
            refresh: Some(3600),
            max_articles: None,
            temperature: None,
        };

        let test_rss = r#"<?xml version="1.0"?><rss><channel><item><title>Test</title></item></channel></rss>"#;

        // Write cache
        write_cache("test-feed", test_rss, &feed_config, &config).unwrap();

        // Read cache
        let cached = get_cached_feed("test-feed", &config).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), test_rss);

        // Cleanup
        fs::remove_dir_all(&config.cache_dir).ok();
    }
}
