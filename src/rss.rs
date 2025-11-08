use crate::ai::Article;
use crate::error::Result;
use chrono::{DateTime, Utc};
use log::debug;
use sha2::{Digest, Sha256};

/// Generate a deterministic GUID for an article based on its content
pub fn generate_guid(title: &str, summary: &str) -> String {
    let content = format!(
        "{}{}",
        title,
        &summary[..summary.len().min(200)]
    );

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    // Use TAG URI scheme as specified in architecture
    format!("tag:newsship.local,2025:{}", hex::encode(&hash[..8]))
}

/// Format a DateTime for RSS pubDate (RFC 822 format)
fn format_rfc822(dt: &DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Escape HTML entities in text
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Build RSS 2.0 XML from articles
pub fn build_rss(feed_name: &str, articles: &[Article]) -> Result<String> {
    debug!("Building RSS 2.0 XML for feed: {}", feed_name);

    let now = Utc::now();
    let build_date = format_rfc822(&now);

    let mut rss = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    rss.push_str("\n<rss version=\"2.0\">\n");
    rss.push_str("  <channel>\n");

    // Channel metadata
    rss.push_str(&format!("    <title>{}</title>\n", escape_html(feed_name)));
    rss.push_str(&format!(
        "    <link>https://newsship.local/{}</link>\n",
        feed_name
    ));
    rss.push_str(&format!(
        "    <description>AI-generated feed: {}</description>\n",
        escape_html(feed_name)
    ));
    rss.push_str("    <language>en-us</language>\n");
    rss.push_str(&format!("    <lastBuildDate>{}</lastBuildDate>\n", build_date));
    rss.push_str("    <ttl>60</ttl>\n");
    rss.push_str("    <generator>newsship/0.1.0</generator>\n");

    // Add articles
    for article in articles {
        rss.push_str("\n    <item>\n");
        rss.push_str(&format!("      <title>{}</title>\n", escape_html(&article.title)));

        // Use primary source URL as link
        if let Some(source) = article.sources.first() {
            rss.push_str(&format!("      <link>{}</link>\n", escape_html(&source.url)));
        }

        rss.push_str(&format!(
            "      <guid isPermaLink=\"false\">{}</guid>\n",
            escape_html(&article.guid)
        ));
        rss.push_str(&format!(
            "      <pubDate>{}</pubDate>\n",
            format_rfc822(&article.date)
        ));

        // Description with summary and sources
        rss.push_str("      <description><![CDATA[\n");
        rss.push_str(&format!("        <p>{}</p>\n", escape_html(&article.summary)));

        if !article.sources.is_empty() {
            rss.push_str("\n        <p><strong>Sources:</strong></p>\n");
            rss.push_str("        <ul>\n");
            for source in &article.sources {
                rss.push_str(&format!(
                    "          <li><a href=\"{}\">{}</a></li>\n",
                    escape_html(&source.url),
                    escape_html(&source.title)
                ));
            }
            rss.push_str("        </ul>\n");
        }

        rss.push_str("      ]]></description>\n");
        rss.push_str("    </item>\n");
    }

    rss.push_str("  </channel>\n");
    rss.push_str("</rss>\n");

    Ok(rss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Source;

    #[test]
    fn test_generate_guid() {
        let guid1 = generate_guid("Test Title", "Test summary content");
        let guid2 = generate_guid("Test Title", "Test summary content");
        let guid3 = generate_guid("Different Title", "Test summary content");

        // Same content should produce same GUID
        assert_eq!(guid1, guid2);

        // Different content should produce different GUID
        assert_ne!(guid1, guid3);

        // Should follow TAG URI format
        assert!(guid1.starts_with("tag:newsship.local,2025:"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("Test & <html> \"quotes\""),
            "Test &amp; &lt;html&gt; &quot;quotes&quot;"
        );
    }

    #[test]
    fn test_build_rss() {
        let articles = vec![Article {
            title: "Test Article".to_string(),
            summary: "This is a test summary".to_string(),
            sources: vec![Source {
                url: "https://example.com".to_string(),
                title: "Example Source".to_string(),
            }],
            date: Utc::now(),
            guid: "test-guid-123".to_string(),
        }];

        let rss = build_rss("test-feed", &articles).unwrap();

        assert!(rss.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(rss.contains("<rss version=\"2.0\">"));
        assert!(rss.contains("<title>test-feed</title>"));
        assert!(rss.contains("<title>Test Article</title>"));
        assert!(rss.contains("This is a test summary"));
        assert!(rss.contains("https://example.com"));
    }
}
