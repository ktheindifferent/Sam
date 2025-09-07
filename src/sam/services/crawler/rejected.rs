//! Storage for URLs rejected by robots.txt or other policies
//! 
//! This module provides persistent storage for URLs that were rejected during crawling,
//! primarily due to robots.txt restrictions. This helps avoid repeated checks and
//! provides data for analysis of crawling patterns.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use log::{debug, info, warn};

/// Reasons why a URL was rejected from crawling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// Blocked by robots.txt rules
    RobotsTxt,
    /// Domain blocked by circuit breaker due to errors
    CircuitBreaker,
    /// Rate limit exceeded
    RateLimit,
    /// Invalid or malformed URL
    InvalidUrl,
    /// SSRF protection (private IP, localhost, etc.)
    SsrfProtection,
    /// Content type not supported
    UnsupportedContentType,
    /// Domain explicitly blacklisted
    Blacklisted,
    /// Other reason with description
    Other(String),
}

impl RejectionReason {
    /// Convert to string for database storage
    pub fn to_string(&self) -> String {
        match self {
            Self::RobotsTxt => "robots_txt".to_string(),
            Self::CircuitBreaker => "circuit_breaker".to_string(),
            Self::RateLimit => "rate_limit".to_string(),
            Self::InvalidUrl => "invalid_url".to_string(),
            Self::SsrfProtection => "ssrf_protection".to_string(),
            Self::UnsupportedContentType => "unsupported_content_type".to_string(),
            Self::Blacklisted => "blacklisted".to_string(),
            Self::Other(desc) => format!("other:{}", desc),
        }
    }
    
    /// Parse from database string
    pub fn from_string(s: &str) -> Self {
        match s {
            "robots_txt" => Self::RobotsTxt,
            "circuit_breaker" => Self::CircuitBreaker,
            "rate_limit" => Self::RateLimit,
            "invalid_url" => Self::InvalidUrl,
            "ssrf_protection" => Self::SsrfProtection,
            "unsupported_content_type" => Self::UnsupportedContentType,
            "blacklisted" => Self::Blacklisted,
            other => {
                if let Some(desc) = other.strip_prefix("other:") {
                    Self::Other(desc.to_string())
                } else {
                    Self::Other(other.to_string())
                }
            }
        }
    }
}

/// Represents a URL that was rejected from crawling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRejected {
    pub id: i64,
    pub url: String,
    pub domain: String,
    pub path: String,
    pub reason: RejectionReason,
    pub robots_rule: Option<String>, // The specific robots.txt rule that blocked it
    pub user_agent: String,
    pub crawl_job_oid: Option<String>,
    pub rejected_at: i64,
    pub retry_after: Option<i64>, // Timestamp when retry might be allowed
    pub rejection_count: i32, // How many times this URL has been rejected
}

impl CrawlRejected {
    /// Create a new rejection record
    pub fn new(
        url: String,
        reason: RejectionReason,
        user_agent: String,
        crawl_job_oid: Option<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        // Parse URL to extract domain and path
        let (domain, path) = if let Ok(parsed) = url::Url::parse(&url) {
            (
                parsed.host_str().unwrap_or("").to_string(),
                parsed.path().to_string(),
            )
        } else {
            ("".to_string(), "".to_string())
        };
        
        Self {
            id: 0,
            url,
            domain,
            path,
            reason,
            robots_rule: None,
            user_agent,
            crawl_job_oid,
            rejected_at: now,
            retry_after: None,
            rejection_count: 1,
        }
    }
    
    /// Create a rejection specifically for robots.txt
    pub fn robots_blocked(
        url: String,
        rule: Option<String>,
        user_agent: String,
        crawl_job_oid: Option<String>,
    ) -> Self {
        let mut rejection = Self::new(url, RejectionReason::RobotsTxt, user_agent, crawl_job_oid);
        rejection.robots_rule = rule;
        rejection
    }
    
    /// SQL table name
    pub fn sql_table_name() -> String {
        "crawl_rejected".to_string()
    }
    
    /// SQL table creation statement
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawl_rejected (
            id BIGSERIAL PRIMARY KEY,
            url TEXT NOT NULL,
            domain TEXT NOT NULL,
            path TEXT NOT NULL,
            reason TEXT NOT NULL,
            robots_rule TEXT,
            user_agent TEXT NOT NULL,
            crawl_job_oid TEXT,
            rejected_at BIGINT NOT NULL,
            retry_after BIGINT,
            rejection_count INTEGER NOT NULL DEFAULT 1,
            UNIQUE(url, user_agent)
        );"
    }
    
    /// SQL migrations
    pub fn migrations() -> Vec<&'static str> {
        vec![]
    }
    
    /// SQL indexes for efficient lookups
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            // Index on URL for fast lookups
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_url ON crawl_rejected(url);",
            // Index on domain for domain-level analysis
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_domain ON crawl_rejected(domain);",
            // Index on reason for filtering by rejection type
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_reason ON crawl_rejected(reason);",
            // Index on rejected_at for time-based queries
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_at ON crawl_rejected(rejected_at DESC);",
            // Composite index for checking if URL was rejected for specific user agent
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_url_agent ON crawl_rejected(url, user_agent);",
            // Index for finding rejections by job
            "CREATE INDEX IF NOT EXISTS idx_crawl_rejected_job ON crawl_rejected(crawl_job_oid);",
        ]
    }
    
    /// Build from database row
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
            domain: row.get("domain"),
            path: row.get("path"),
            reason: RejectionReason::from_string(row.get("reason")),
            robots_rule: row.get("robots_rule"),
            user_agent: row.get("user_agent"),
            crawl_job_oid: row.get("crawl_job_oid"),
            rejected_at: row.get("rejected_at"),
            retry_after: row.get("retry_after"),
            rejection_count: row.get("rejection_count"),
        })
    }
    
    /// Save or update rejection to database
    pub async fn save(&mut self) -> Result<()> {
        // Get database connection from crawler pool
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        // Try to insert, on conflict update the count and timestamp
        let result = client.query_one(
            "INSERT INTO crawl_rejected (
                url, domain, path, reason, robots_rule, user_agent,
                crawl_job_oid, rejected_at, retry_after, rejection_count
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (url, user_agent) DO UPDATE SET
                rejection_count = crawl_rejected.rejection_count + 1,
                rejected_at = EXCLUDED.rejected_at,
                retry_after = EXCLUDED.retry_after,
                robots_rule = EXCLUDED.robots_rule,
                reason = EXCLUDED.reason
            RETURNING id, rejection_count",
            &[
                &self.url,
                &self.domain,
                &self.path,
                &self.reason.to_string(),
                &self.robots_rule,
                &self.user_agent,
                &self.crawl_job_oid,
                &self.rejected_at,
                &self.retry_after,
                &self.rejection_count,
            ]
        ).await?;
        
        self.id = result.get("id");
        self.rejection_count = result.get("rejection_count");
        
        if self.rejection_count > 1 {
            debug!("URL {} rejected {} times for user agent {}", 
                   self.url, self.rejection_count, self.user_agent);
        }
        
        Ok(())
    }
    
    /// Check if a URL has been rejected before
    pub async fn is_rejected(url: &str, user_agent: &str) -> Result<Option<Self>> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        let rows = client.query(
            "SELECT * FROM crawl_rejected WHERE url = $1 AND user_agent = $2",
            &[&url, &user_agent]
        ).await?;
        
        if let Some(row) = rows.first() {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// Get all rejections for a domain
    pub async fn get_domain_rejections(domain: &str) -> Result<Vec<Self>> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        let rows = client.query(
            "SELECT * FROM crawl_rejected WHERE domain = $1 ORDER BY rejected_at DESC",
            &[&domain]
        ).await?;
        
        rows.iter()
            .map(Self::from_row)
            .collect()
    }
    
    /// Get rejection statistics
    pub async fn get_stats() -> Result<serde_json::Value> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        // Get counts by reason
        let reason_counts = client.query(
            "SELECT reason, COUNT(*) as count FROM crawl_rejected GROUP BY reason",
            &[]
        ).await?;
        
        // Get top rejected domains
        let top_domains = client.query(
            "SELECT domain, COUNT(*) as count FROM crawl_rejected 
             GROUP BY domain ORDER BY count DESC LIMIT 10",
            &[]
        ).await?;
        
        // Get recent rejections
        let recent_count: i64 = client.query_one(
            "SELECT COUNT(*) FROM crawl_rejected 
             WHERE rejected_at > $1",
            &[&(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - 3600)]
        ).await?.get(0);
        
        Ok(serde_json::json!({
            "total_rejections": client.query_one("SELECT COUNT(*) FROM crawl_rejected", &[]).await?.get::<_, i64>(0),
            "unique_urls": client.query_one("SELECT COUNT(DISTINCT url) FROM crawl_rejected", &[]).await?.get::<_, i64>(0),
            "rejections_last_hour": recent_count,
            "by_reason": reason_counts.iter().map(|row| {
                serde_json::json!({
                    "reason": row.get::<_, String>("reason"),
                    "count": row.get::<_, i64>("count")
                })
            }).collect::<Vec<_>>(),
            "top_rejected_domains": top_domains.iter().map(|row| {
                serde_json::json!({
                    "domain": row.get::<_, String>("domain"),
                    "count": row.get::<_, i64>("count")
                })
            }).collect::<Vec<_>>(),
        }))
    }
    
    /// Clean up old rejection records (optional maintenance)
    pub async fn cleanup_old_records(days_to_keep: i64) -> Result<i64> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - (days_to_keep * 86400);
        
        let deleted = client.execute(
            "DELETE FROM crawl_rejected WHERE rejected_at < $1",
            &[&cutoff]
        ).await?;
        
        info!("Cleaned up {} old rejection records", deleted);
        Ok(deleted as i64)
    }
}