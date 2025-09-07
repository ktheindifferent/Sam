//! Content storage and deduplication for crawled pages
//! 
//! This module provides enhanced storage capabilities for crawled page content
//! with hash-based deduplication and compression.

use anyhow::{Result};
use sha2::{Sha256, Digest};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use log::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};

/// Enhanced crawled page with full content storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledContent {
    pub id: i64,
    pub url: String,
    pub content_hash: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_text: String,
    pub content_html: Option<Vec<u8>>, // Compressed HTML
    pub headers: serde_json::Value,
    pub status_code: i16,
    pub content_type: Option<String>,
    pub content_length: i64,
    pub language: Option<String>,
    pub crawled_at: i64,
    pub updated_at: i64,
}

impl CrawledContent {
    /// Create a new crawled content entry
    pub fn new(url: String, content: &str, html: Option<&str>, status_code: u16) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        // Calculate content hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());
        
        // Compress HTML if provided
        let content_html = html.map(|h| Self::compress_content(h.as_bytes()));
        
        Self {
            id: 0,
            url,
            content_hash,
            title: None,
            description: None,
            content_text: content.to_string(),
            content_html,
            headers: serde_json::json!({}),
            status_code: status_code as i16,
            content_type: None,
            content_length: content.len() as i64,
            language: None,
            crawled_at: now,
            updated_at: now,
        }
    }
    
    /// Compress content using gzip
    fn compress_content(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap_or_else(|e| {
            warn!("Failed to compress content: {}", e);
        });
        encoder.finish().unwrap_or_else(|_| Vec::new())
    }
    
    /// Decompress content
    pub fn decompress_html(&self) -> Option<String> {
        self.content_html.as_ref().and_then(|compressed| {
            let mut decoder = GzDecoder::new(&compressed[..]);
            let mut decompressed = String::new();
            match decoder.read_to_string(&mut decompressed) {
                Ok(_) => Some(decompressed),
                Err(e) => {
                    warn!("Failed to decompress HTML: {}", e);
                    None
                }
            }
        })
    }
    
    /// Extract title from HTML content
    pub fn extract_title(html: &str) -> Option<String> {
        // Simple regex-based title extraction
        let title_regex = regex::Regex::new(r"(?i)<title[^>]*>(.*?)</title>").ok()?;
        title_regex.captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| html_escape::decode_html_entities(m.as_str()).to_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    
    /// Extract meta description from HTML
    pub fn extract_description(html: &str) -> Option<String> {
        let desc_regex = regex::Regex::new(
            r#"(?i)<meta\s+(?:[^>]*\s+)?(?:name|property)\s*=\s*["'](?:description|og:description)["'][^>]*content\s*=\s*["']([^"']+)["']"#
        ).ok()?;
        
        desc_regex.captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| html_escape::decode_html_entities(m.as_str()).to_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    
    /// Detect language from content
    pub fn detect_language(text: &str) -> Option<String> {
        // Simple language detection based on common words
        // In production, use a proper language detection library
        let english_words = ["the", "and", "is", "in", "to", "of", "a", "for"];
        let spanish_words = ["el", "la", "de", "que", "y", "en", "un", "por"];
        let french_words = ["le", "de", "un", "être", "et", "à", "il", "avoir"];
        let german_words = ["der", "die", "und", "in", "den", "von", "zu", "das"];
        
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        
        let mut scores = vec![
            ("en", english_words.iter().filter(|&&w| words.contains(&w)).count()),
            ("es", spanish_words.iter().filter(|&&w| words.contains(&w)).count()),
            ("fr", french_words.iter().filter(|&&w| words.contains(&w)).count()),
            ("de", german_words.iter().filter(|&&w| words.contains(&w)).count()),
        ];
        
        scores.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
        
        if scores[0].1 > 0 {
            Some(scores[0].0.to_string())
        } else {
            None
        }
    }
    
    /// SQL table name
    pub fn sql_table_name() -> String {
        "crawled_content".to_string()
    }
    
    /// SQL migrations
    pub fn migrations() -> Vec<&'static str> {
        vec![]
    }
    
    /// SQL table creation statement
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawled_content (
            id BIGSERIAL PRIMARY KEY,
            url TEXT NOT NULL,
            content_hash VARCHAR(64) NOT NULL,
            title TEXT,
            description TEXT,
            content_text TEXT NOT NULL,
            content_html BYTEA,
            headers JSONB,
            status_code SMALLINT NOT NULL,
            content_type VARCHAR(255),
            content_length BIGINT,
            language VARCHAR(10),
            crawled_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            UNIQUE(url),
            UNIQUE(content_hash)
        );"
    }
    
    /// SQL indexes
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_url ON crawled_content(url);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_hash ON crawled_content(content_hash);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_crawled_at ON crawled_content(crawled_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_language ON crawled_content(language);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_content_type ON crawled_content(content_type);",
            // Full-text search index on content_text
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_fts ON crawled_content USING GIN(to_tsvector('english', content_text));",
            // Full-text search on title
            "CREATE INDEX IF NOT EXISTS idx_crawled_content_title_fts ON crawled_content USING GIN(to_tsvector('english', title));",
        ]
    }
    
    /// Build from database row
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
            content_hash: row.get("content_hash"),
            title: row.get("title"),
            description: row.get("description"),
            content_text: row.get("content_text"),
            content_html: row.get("content_html"),
            headers: row.get("headers"),
            status_code: row.get("status_code"),
            content_type: row.get("content_type"),
            content_length: row.get("content_length"),
            language: row.get("language"),
            crawled_at: row.get("crawled_at"),
            updated_at: row.get("updated_at"),
        })
    }
    
    /// Save content to database (with deduplication check)
    pub async fn save(&self) -> Result<bool> {
        // Get database connection from crawler pool
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        // Check if content with same hash already exists
        let existing = client.query(
            "SELECT id FROM crawled_content WHERE content_hash = $1",
            &[&self.content_hash]
        ).await?;
        
        if !existing.is_empty() {
            debug!("Content already exists with hash {}", self.content_hash);
            
            // Update the URL mapping if it's a different URL
            client.execute(
                "UPDATE crawled_content SET url = $1, updated_at = $2 WHERE content_hash = $3 AND url != $1",
                &[&self.url, &self.updated_at, &self.content_hash]
            ).await?;
            
            return Ok(false); // Content was deduplicated
        }
        
        // Insert new content
        client.execute(
            "INSERT INTO crawled_content (
                url, content_hash, title, description, content_text, content_html,
                headers, status_code, content_type, content_length, language,
                crawled_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (url) DO UPDATE SET
                content_hash = EXCLUDED.content_hash,
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                content_text = EXCLUDED.content_text,
                content_html = EXCLUDED.content_html,
                headers = EXCLUDED.headers,
                status_code = EXCLUDED.status_code,
                content_type = EXCLUDED.content_type,
                content_length = EXCLUDED.content_length,
                language = EXCLUDED.language,
                updated_at = EXCLUDED.updated_at",
            &[
                &self.url, &self.content_hash, &self.title, &self.description,
                &self.content_text, &self.content_html, &self.headers,
                &self.status_code, &self.content_type, &self.content_length,
                &self.language, &self.crawled_at, &self.updated_at
            ]
        ).await?;
        
        Ok(true) // New content was saved
    }
    
    /// Batch save multiple content entries
    pub async fn save_batch(contents: Vec<Self>) -> Result<(usize, usize)> {
        let mut saved = 0;
        let mut deduplicated = 0;
        
        for content in contents {
            match content.save().await {
                Ok(true) => saved += 1,
                Ok(false) => deduplicated += 1,
                Err(e) => warn!("Failed to save content for {}: {}", content.url, e),
            }
        }
        
        info!("Saved {} new pages, deduplicated {} pages", saved, deduplicated);
        Ok((saved, deduplicated))
    }
    
    /// Full-text search
    pub async fn search(query: &str, limit: usize) -> Result<Vec<Self>> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        // Use PostgreSQL full-text search
        let sql = "
            SELECT * FROM crawled_content
            WHERE to_tsvector('english', content_text) @@ plainto_tsquery('english', $1)
               OR to_tsvector('english', COALESCE(title, '')) @@ plainto_tsquery('english', $1)
            ORDER BY 
                ts_rank(to_tsvector('english', content_text), plainto_tsquery('english', $1)) +
                ts_rank(to_tsvector('english', COALESCE(title, '')), plainto_tsquery('english', $1)) DESC
            LIMIT $2
        ";
        
        let rows = client.query(sql, &[&query, &(limit as i64)]).await?;
        
        rows.into_iter()
            .map(|row| Self::from_row(&row))
            .collect()
    }
    
    /// Get deduplication statistics
    pub async fn get_dedup_stats() -> Result<DeduplicationStats> {
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        let total_urls: i64 = client.query_one(
            "SELECT COUNT(DISTINCT url) FROM crawled_content",
            &[]
        ).await?.get(0);
        
        let unique_content: i64 = client.query_one(
            "SELECT COUNT(DISTINCT content_hash) FROM crawled_content",
            &[]
        ).await?.get(0);
        
        let total_size: i64 = client.query_one(
            "SELECT COALESCE(SUM(content_length), 0) FROM crawled_content",
            &[]
        ).await?.get(0);
        
        let compressed_size: i64 = client.query_one(
            "SELECT COALESCE(SUM(LENGTH(content_html)), 0) FROM crawled_content WHERE content_html IS NOT NULL",
            &[]
        ).await?.get(0);
        
        Ok(DeduplicationStats {
            total_urls: total_urls as usize,
            unique_content: unique_content as usize,
            dedup_ratio: if total_urls > 0 {
                1.0 - (unique_content as f64 / total_urls as f64)
            } else {
                0.0
            },
            total_size_bytes: total_size as usize,
            compressed_size_bytes: compressed_size as usize,
            compression_ratio: if total_size > 0 {
                1.0 - (compressed_size as f64 / total_size as f64)
            } else {
                0.0
            },
        })
    }
}

/// Statistics about content deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationStats {
    pub total_urls: usize,
    pub unique_content: usize,
    pub dedup_ratio: f64,
    pub total_size_bytes: usize,
    pub compressed_size_bytes: usize,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_hashing() {
        let content1 = CrawledContent::new(
            "https://example.com/page1".to_string(),
            "This is test content",
            None,
            200
        );
        
        let content2 = CrawledContent::new(
            "https://example.com/page2".to_string(),
            "This is test content", // Same content
            None,
            200
        );
        
        let content3 = CrawledContent::new(
            "https://example.com/page3".to_string(),
            "Different content",
            None,
            200
        );
        
        // Same content should have same hash
        assert_eq!(content1.content_hash, content2.content_hash);
        
        // Different content should have different hash
        assert_ne!(content1.content_hash, content3.content_hash);
    }
    
    #[test]
    fn test_compression() {
        let html = "<html><body>Test content repeated many times. ".repeat(100);
        let content = CrawledContent::new(
            "https://example.com".to_string(),
            "Test",
            Some(&html),
            200
        );
        
        assert!(content.content_html.is_some());
        let compressed = content.content_html.as_ref().unwrap();
        assert!(compressed.len() < html.len());
        
        let decompressed = content.decompress_html();
        assert!(decompressed.is_some());
        assert_eq!(decompressed.unwrap(), html);
    }
    
    #[test]
    fn test_title_extraction() {
        let html = r#"<html><head><title>Test Page Title</title></head><body>Content</body></html>"#;
        let title = CrawledContent::extract_title(html);
        assert_eq!(title, Some("Test Page Title".to_string()));
    }
    
    #[test]
    fn test_language_detection() {
        let english = "The quick brown fox jumps over the lazy dog";
        assert_eq!(CrawledContent::detect_language(english), Some("en".to_string()));
        
        let spanish = "El rápido zorro marrón salta sobre el perro perezoso";
        assert_eq!(CrawledContent::detect_language(spanish), Some("es".to_string()));
    }
}