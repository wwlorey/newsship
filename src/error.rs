use thiserror::Error;

#[derive(Error, Debug)]
pub enum NewsshipError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Feed '{0}' not found in configuration")]
    FeedNotFound(String),

    #[error("API key not set: {0}")]
    ApiKeyMissing(String),

    #[error("AI provider error: {0}")]
    ProviderError(String),

    #[error("Rate limited by API provider: retry after {0} seconds")]
    RateLimited(u64),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("XML generation error: {0}")]
    XmlError(String),

    #[error("Cache error: {0}")]
    CacheError(String),
}

pub type Result<T> = std::result::Result<T, NewsshipError>;
