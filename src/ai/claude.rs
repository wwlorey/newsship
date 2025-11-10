use super::{AIProvider, Article, Source};
use crate::config::FeedConfig;
use crate::error::{NewsshipError, Result};
use async_trait::async_trait;
use chrono::Utc;
use log::{debug, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

pub struct ClaudeProvider {
    api_key: String,
    model: String,
    temperature: f32,
    max_articles: u8,
    global_prompt: Option<String>,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArticleData {
    title: String,
    summary: String,
    url: String,
    source_title: Option<String>,
}

impl ClaudeProvider {
    pub fn new(
        api_key: String,
        model: String,
        temperature: f32,
        max_articles: u8,
        global_prompt: Option<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            temperature,
            max_articles,
            global_prompt,
            client,
        }
    }

    fn build_prompt(&self, feed_prompt: &str) -> String {
        let system_prompt = self.global_prompt.as_deref().unwrap_or(
            "You are an expert news curator. Provide accurate, concise summaries with source URLs."
        );

        format!(
            r#"{}

Find exactly {} recent news articles based on this request: "{}"

Return ONLY a valid JSON array with no additional text. Each object must have:
- "title": Article headline (max 100 characters)
- "summary": Brief summary (max 500 characters)
- "url": Direct link to the article
- "source_title": Name of the publication/website

Example format:
[
  {{
    "title": "Major AI Breakthrough Announced",
    "summary": "Researchers unveil new model with unprecedented capabilities...",
    "url": "https://example.com/article",
    "source_title": "Tech News Daily"
  }}
]

Return ONLY the JSON array, nothing else."#,
            system_prompt, self.max_articles, feed_prompt
        )
    }

    async fn call_api(&self, prompt: &str) -> Result<String> {
        debug!("Calling Anthropic API with model: {}", self.model);

        let request_body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "temperature": self.temperature,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;

            if status.as_u16() == 429 {
                return Err(NewsshipError::RateLimited(60));
            } else if status.as_u16() == 401 {
                return Err(NewsshipError::AuthenticationFailed(
                    "Invalid Anthropic API key".to_string(),
                ));
            } else {
                return Err(NewsshipError::ProviderError(format!(
                    "Anthropic API error {}: {}",
                    status, error_text
                )));
            }
        }

        let claude_response: ClaudeResponse = response.json().await?;

        if claude_response.content.is_empty() {
            return Err(NewsshipError::InvalidResponse(
                "No content in Anthropic response".to_string(),
            ));
        }

        Ok(claude_response.content[0].text.clone())
    }

    fn parse_articles(&self, content: &str) -> Result<Vec<Article>> {
        debug!("Parsing Anthropic response");

        // Try to extract JSON array from the response
        let json_str = if let Some(start) = content.find('[') {
            if let Some(end) = content.rfind(']') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        let articles_data: Vec<ArticleData> = serde_json::from_str(json_str).map_err(|e| {
            warn!("Failed to parse Anthropic response as JSON: {}", e);
            warn!("Response content: {}", content);
            NewsshipError::InvalidResponse(format!("Failed to parse article JSON: {}", e))
        })?;

        let now = Utc::now();
        let mut articles = Vec::new();

        for data in articles_data {
            let guid = super::super::rss::generate_guid(&data.title, &data.summary);

            articles.push(Article {
                title: data.title,
                summary: data.summary,
                sources: vec![Source {
                    url: data.url,
                    title: data.source_title.unwrap_or_else(|| "Source".to_string()),
                }],
                date: now,
                guid,
            });
        }

        Ok(articles)
    }

    async fn generate_with_retry(&self, prompt: &str, max_retries: u32) -> Result<String> {
        let mut delay = 1000;

        for attempt in 0..max_retries {
            match self.call_api(prompt).await {
                Ok(content) => return Ok(content),
                Err(NewsshipError::RateLimited(retry_after)) => {
                    if attempt < max_retries - 1 {
                        warn!(
                            "Rate limited, waiting {} seconds before retry",
                            retry_after
                        );
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                    } else {
                        return Err(NewsshipError::RateLimited(retry_after));
                    }
                }
                Err(e) => {
                    if attempt < max_retries - 1 {
                        warn!("API call failed (attempt {}): {}", attempt + 1, e);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        delay *= 2;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(NewsshipError::ProviderError(
            "Max retries exceeded".to_string(),
        ))
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    async fn generate_articles(&self, feed_config: &FeedConfig) -> Result<Vec<Article>> {
        info!("Generating articles using Anthropic ({})", self.model);

        let prompt = self.build_prompt(&feed_config.prompt);
        let content = self.generate_with_retry(&prompt, 3).await?;

        let articles = self.parse_articles(&content)?;

        info!("Successfully generated {} articles", articles.len());

        Ok(articles)
    }

    fn name(&self) -> &str {
        "Anthropic"
    }
}
