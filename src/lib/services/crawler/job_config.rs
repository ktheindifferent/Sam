//! Enhanced job configuration for flexible crawling
//!
//! This module provides configurable parameters for crawl jobs including
//! max depth, domain filters, and other crawling policies.

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for a crawl job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlJobConfig {
    /// Maximum crawl depth (default: 10)
    pub max_depth: usize,

    /// Maximum number of pages to crawl (0 = unlimited)
    pub max_pages: usize,

    /// Domain whitelist (if empty, all domains allowed)
    pub domain_whitelist: HashSet<String>,

    /// Domain blacklist (domains to exclude)
    pub domain_blacklist: HashSet<String>,

    /// URL patterns to include (regex)
    pub include_patterns: Vec<String>,

    /// URL patterns to exclude (regex)
    pub exclude_patterns: Vec<String>,

    /// Follow external links (links to different domains)
    pub follow_external: bool,

    /// Follow redirects
    pub follow_redirects: bool,

    /// Maximum redirects to follow
    pub max_redirects: usize,

    /// Respect robots.txt
    pub respect_robots: bool,

    /// User agent string (if different from default)
    pub user_agent: Option<String>,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Minimum delay between requests to same domain (milliseconds)
    pub min_delay_ms: u64,

    /// Maximum concurrent requests
    pub max_concurrent: usize,

    /// Store page content (not just metadata)
    pub store_content: bool,

    /// Store page screenshots (requires headless browser)
    pub store_screenshots: bool,

    /// Extract and follow links from JavaScript
    pub parse_javascript: bool,

    /// Language filter (ISO 639-1 codes, e.g., ["en", "es"])
    pub languages: Vec<String>,

    /// Content type filters (e.g., ["text/html", "application/pdf"])
    pub content_types: Vec<String>,

    /// Maximum content size in bytes (0 = unlimited)
    pub max_content_size: usize,

    /// Custom headers to send with requests
    pub custom_headers: Vec<(String, String)>,

    /// Crawl priority (higher = more important)
    pub priority: i32,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Notification webhook URL for completion
    pub webhook_url: Option<String>,

    /// Schedule expression (cron-like) for recurring crawls
    pub schedule: Option<String>,
}

impl Default for CrawlJobConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_pages: 0,
            domain_whitelist: HashSet::new(),
            domain_blacklist: HashSet::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            follow_external: false,
            follow_redirects: true,
            max_redirects: 5,
            respect_robots: true,
            user_agent: None,
            timeout_seconds: 30,
            min_delay_ms: 1000,
            max_concurrent: 10,
            store_content: true,
            store_screenshots: false,
            parse_javascript: false,
            languages: Vec::new(),
            content_types: vec!["text/html".to_string()],
            max_content_size: 10 * 1024 * 1024, // 10MB
            custom_headers: Vec::new(),
            priority: 0,
            tags: Vec::new(),
            webhook_url: None,
            schedule: None,
        }
    }
}

impl CrawlJobConfig {
    /// Create a config for shallow crawling (low depth, same domain only)
    pub fn shallow() -> Self {
        Self {
            max_depth: 2,
            follow_external: false,
            max_concurrent: 5,
            ..Default::default()
        }
    }

    /// Create a config for deep crawling (high depth, follow external)
    pub fn deep() -> Self {
        Self {
            max_depth: 20,
            follow_external: true,
            max_concurrent: 20,
            ..Default::default()
        }
    }

    /// Create a config for focused crawling (specific domains only)
    pub fn focused(domains: Vec<String>) -> Self {
        let whitelist: HashSet<String> = domains.into_iter().collect();
        Self {
            domain_whitelist: whitelist,
            follow_external: false,
            ..Default::default()
        }
    }

    /// Create a config for archival crawling (store everything)
    pub fn archival() -> Self {
        Self {
            store_content: true,
            store_screenshots: true,
            parse_javascript: true,
            max_content_size: 100 * 1024 * 1024, // 100MB
            content_types: vec![
                "text/html".to_string(),
                "text/plain".to_string(),
                "application/pdf".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Check if a URL should be crawled based on configuration
    pub fn should_crawl(&self, url: &str) -> Result<bool> {
        // Extract domain from URL
        let parsed = url::Url::parse(url).context("Failed to parse URL")?;
        let domain = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in URL"))?
            .to_string();

        // Check domain whitelist
        if !self.domain_whitelist.is_empty() && !self.domain_whitelist.contains(&domain) {
            return Ok(false);
        }

        // Check domain blacklist
        if self.domain_blacklist.contains(&domain) {
            return Ok(false);
        }

        // Check include patterns
        if !self.include_patterns.is_empty() {
            let mut included = false;
            for pattern in &self.include_patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    if regex.is_match(url) {
                        included = true;
                        break;
                    }
                }
            }
            if !included {
                return Ok(false);
            }
        }

        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(url) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate regex patterns
        for pattern in &self.include_patterns {
            Regex::new(pattern).context(format!("Invalid include pattern: {}", pattern))?;
        }

        for pattern in &self.exclude_patterns {
            Regex::new(pattern).context(format!("Invalid exclude pattern: {}", pattern))?;
        }

        // Validate schedule if present
        if let Some(schedule) = &self.schedule {
            validate_cron_expression(schedule)?;
        }

        // Validate other parameters
        if self.max_depth == 0 {
            return Err(anyhow::anyhow!("max_depth must be greater than 0"));
        }

        if self.timeout_seconds == 0 {
            return Err(anyhow::anyhow!("timeout_seconds must be greater than 0"));
        }

        if self.max_concurrent == 0 {
            return Err(anyhow::anyhow!("max_concurrent must be greater than 0"));
        }

        Ok(())
    }

    /// Merge with another config (other takes precedence)
    pub fn merge(&mut self, other: &CrawlJobConfig) {
        // Only override non-default values
        if other.max_depth != 10 {
            self.max_depth = other.max_depth;
        }
        if other.max_pages != 0 {
            self.max_pages = other.max_pages;
        }
        if !other.domain_whitelist.is_empty() {
            self.domain_whitelist = other.domain_whitelist.clone();
        }
        if !other.domain_blacklist.is_empty() {
            self.domain_blacklist = other.domain_blacklist.clone();
        }
        if !other.include_patterns.is_empty() {
            self.include_patterns = other.include_patterns.clone();
        }
        if !other.exclude_patterns.is_empty() {
            self.exclude_patterns = other.exclude_patterns.clone();
        }
        if other.follow_external != self.follow_external {
            self.follow_external = other.follow_external;
        }
        if other.user_agent.is_some() {
            self.user_agent = other.user_agent.clone();
        }
        if other.timeout_seconds != 30 {
            self.timeout_seconds = other.timeout_seconds;
        }
        if other.min_delay_ms != 1000 {
            self.min_delay_ms = other.min_delay_ms;
        }
        if other.max_concurrent != 10 {
            self.max_concurrent = other.max_concurrent;
        }
        if !other.languages.is_empty() {
            self.languages = other.languages.clone();
        }
        if !other.content_types.is_empty() {
            self.content_types = other.content_types.clone();
        }
        if other.max_content_size != 10 * 1024 * 1024 {
            self.max_content_size = other.max_content_size;
        }
        if !other.custom_headers.is_empty() {
            self.custom_headers = other.custom_headers.clone();
        }
        if other.priority != 0 {
            self.priority = other.priority;
        }
        if !other.tags.is_empty() {
            self.tags = other.tags.clone();
        }
        if other.webhook_url.is_some() {
            self.webhook_url = other.webhook_url.clone();
        }
        if other.schedule.is_some() {
            self.schedule = other.schedule.clone();
        }
    }
}

/// Enhanced crawl job with configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurableCrawlJob {
    pub id: i64,
    pub oid: String,
    pub start_url: String,
    pub config: CrawlJobConfig,
    pub status: JobStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub pages_crawled: usize,
    pub pages_failed: usize,
    pub bytes_downloaded: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Scheduled,
}

impl ConfigurableCrawlJob {
    /// Create a new configurable crawl job
    pub fn new(start_url: String, config: CrawlJobConfig) -> Self {
        let oid = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            id: 0,
            oid,
            start_url,
            config,
            status: JobStatus::Pending,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            pages_crawled: 0,
            pages_failed: 0,
            bytes_downloaded: 0,
            error_message: None,
        }
    }

    /// SQL table name
    pub fn sql_table_name() -> &'static str {
        "configurable_crawl_jobs"
    }

    /// SQL table creation
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS configurable_crawl_jobs (
            id BIGSERIAL PRIMARY KEY,
            oid VARCHAR(36) NOT NULL UNIQUE,
            start_url TEXT NOT NULL,
            config JSONB NOT NULL,
            status VARCHAR(20) NOT NULL,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            started_at BIGINT,
            completed_at BIGINT,
            pages_crawled INTEGER DEFAULT 0,
            pages_failed INTEGER DEFAULT 0,
            bytes_downloaded BIGINT DEFAULT 0,
            error_message TEXT
        );"
    }

    /// SQL indexes
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_configurable_crawl_jobs_oid ON configurable_crawl_jobs(oid);",
            "CREATE INDEX IF NOT EXISTS idx_configurable_crawl_jobs_status ON configurable_crawl_jobs(status);",
            "CREATE INDEX IF NOT EXISTS idx_configurable_crawl_jobs_created_at ON configurable_crawl_jobs(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_configurable_crawl_jobs_priority ON configurable_crawl_jobs((config->>'priority')::int DESC);",
            "CREATE INDEX IF NOT EXISTS idx_configurable_crawl_jobs_start_url ON configurable_crawl_jobs(start_url);",
        ]
    }

    /// Check if a URL has been crawled recently (within the past month) for configurable jobs.
    /// Returns true if the URL was crawled recently and shouldn't be crawled again.
    pub async fn is_recently_crawled(start_url: &str) -> crate::memory::Result<bool> {
        let config = crate::memory::Config::new();
        let client = config.connect_pool().await?;

        // Calculate timestamp for one month ago (30 days)
        let one_month_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64 - (30 * 24 * 60 * 60)) // 30 days in seconds
            .unwrap_or(0);

        let query =
            "SELECT COUNT(*) FROM configurable_crawl_jobs WHERE start_url = $1 AND created_at > $2";
        let row = client
            .query_one(query, &[&start_url, &one_month_ago])
            .await?;
        let count: i64 = row.get(0);

        Ok(count > 0)
    }
}

/// Validate a cron expression
fn validate_cron_expression(expr: &str) -> Result<()> {
    // Simple validation - just check format
    // In production, use a proper cron parser library
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 && parts.len() != 6 {
        return Err(anyhow::anyhow!(
            "Invalid cron expression: expected 5 or 6 fields"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_crawl() {
        let mut config = CrawlJobConfig::default();
        config.domain_whitelist.insert("example.com".to_string());

        assert!(config.should_crawl("https://example.com/page").unwrap());
        assert!(!config.should_crawl("https://other.com/page").unwrap());

        config.domain_whitelist.clear();
        config.domain_blacklist.insert("bad.com".to_string());

        assert!(config.should_crawl("https://example.com/page").unwrap());
        assert!(!config.should_crawl("https://bad.com/page").unwrap());
    }

    #[test]
    fn test_pattern_matching() {
        let mut config = CrawlJobConfig::default();
        config.exclude_patterns.push(r".*\.pdf$".to_string());

        assert!(config
            .should_crawl("https://example.com/page.html")
            .unwrap());
        assert!(!config
            .should_crawl("https://example.com/document.pdf")
            .unwrap());
    }

    #[test]
    fn test_config_validation() {
        let mut config = CrawlJobConfig::default();
        assert!(config.validate().is_ok());

        config.max_depth = 0;
        assert!(config.validate().is_err());

        config.max_depth = 10;
        config.include_patterns.push("[invalid regex".to_string());
        assert!(config.validate().is_err());
    }
}
