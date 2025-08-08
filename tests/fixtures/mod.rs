use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub database_url: String,
    pub redis_url: String,
    pub test_timeout_ms: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://test:test@localhost/test_sam".to_string(),
            redis_url: "redis://127.0.0.1:6379/1".to_string(),
            test_timeout_ms: 5000,
        }
    }
}

pub fn sample_crawl_job() -> crate::sam::services::crawler::CrawlJob {
    crate::sam::services::crawler::CrawlJob {
        id: uuid::Uuid::new_v4(),
        url: "https://example.com".to_string(),
        status: crate::sam::services::crawler::job::CrawlStatus::Pending,
        retry_count: 0,
        max_retries: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        error: None,
        metadata: HashMap::new(),
    }
}

pub fn sample_crawled_page() -> crate::sam::services::crawler::CrawledPage {
    crate::sam::services::crawler::CrawledPage {
        url: "https://example.com".to_string(),
        title: Some("Example Domain".to_string()),
        content: "This is an example domain for use in illustrative examples.".to_string(),
        links: vec!["https://www.iana.org/domains/example".to_string()],
        status_code: 200,
        headers: HashMap::new(),
        crawled_at: chrono::Utc::now(),
        tokens: vec!["example".to_string(), "domain".to_string()],
        error: None,
    }
}

pub fn mock_html_content() -> String {
    r#"<!DOCTYPE html>
    <html>
    <head>
        <title>Test Page</title>
    </head>
    <body>
        <h1>Test Header</h1>
        <p>Test content with <a href="https://example.com">link</a></p>
        <p>Another <a href="/relative/path">relative link</a></p>
    </body>
    </html>"#.to_string()
}

pub fn mock_dns_responses() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("example.com".to_string(), true);
    map.insert("nonexistent.invalid".to_string(), false);
    map.insert("google.com".to_string(), true);
    map.insert("localhost".to_string(), true);
    map
}