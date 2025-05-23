//! Crawler page definition and persistence layer.
//!
//! Provides the CrawledPage struct and async/sync DB/Redis persistence for crawled web pages.

use crate::sam::memory::{Config, PostgresQueries};
use log;
// use rand::distributions::Alphanumeric;
// use rand::{thread_rng, Rng};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_postgres::Row;

/// Represents a crawled web page (tokens, links, timestamp, etc).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawledPage {
    pub id: i32,
    pub crawl_job_oid: String,
    pub url: String,
    pub tokens: Vec<String>,
    pub links: Vec<String>,
    pub timestamp: i64,
}

impl Default for CrawledPage {
    fn default() -> Self {
        Self::new()
    }
}
impl CrawledPage {
    pub fn new() -> CrawledPage {
        CrawledPage {
            id: 0,
            crawl_job_oid: String::new(),
            url: String::new(),
            tokens: vec![],
            links: vec![],
            timestamp: match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    log::error!("SystemTime error in CrawledPage::new(): {}", e);
                    0
                }
            },
        }
    }
    pub fn sql_table_name() -> String {
        "crawled_pages".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawled_pages (
            id serial PRIMARY KEY,
            url varchar NOT NULL UNIQUE,
            tokens text,
            timestamp BIGINT
        );"
    }
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_url ON crawled_pages (url);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_timestamp ON crawled_pages (timestamp);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_tokens ON crawled_pages (tokens);",
        ]
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "DROP INDEX IF EXISTS idx_crawled_pages_tokens;",
            "CREATE INDEX idx_crawled_pages_tokens_gin ON crawled_pages USING GIN (tokens);",
        ]
    }

    pub fn from_row(row: &Row) -> crate::sam::memory::Result<Self> {
        let tokens_str: Option<String> = row.get("tokens");
        let tokens = tokens_str
            .map(|s| s.split('\n').map(|s| s.to_string()).collect())
            .unwrap_or_default();
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
            tokens,
            crawl_job_oid: String::new(),
            links: Vec::new(),
            timestamp: row.get("timestamp"),
        })
    }

    pub async fn from_row_async(row: &Row) -> crate::sam::memory::Result<Self> {
        let tokens_str: Option<String> = row.get("tokens");
        let tokens = tokens_str
            .map(|s| s.split('\n').map(|s| s.to_string()).collect())
            .unwrap_or_default();
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
            tokens,
            crawl_job_oid: String::new(),
            links: Vec::new(),
            timestamp: row.get("timestamp"),
        })
    }

    pub async fn select_async(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> crate::sam::memory::Result<Vec<Self>> {
        let config = crate::sam::memory::Config::new();
        let client = config.connect_pool().await?;
        let jsons = crate::sam::memory::Config::pg_select_async(
            Self::sql_table_name(),
            None,
            limit,
            offset,
            order,
            query,
            client,
        )
        .await?;
        let mut parsed_rows: Vec<Self> = Vec::new();
        for j in jsons {
            let object: Self = match serde_json::from_str(&j) {
                Ok(obj) => obj,
                Err(e) => {
                    log::error!("Failed to deserialize CrawledPage: {}", e);
                    return Err(crate::sam::memory::Error::Other(
                        format!("Deserialization error: {e}")
                    ).into());
                }
            };
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Save a batch of CrawledPage objects asynchronously.
    /// If a page with the same URL exists, it is updated; otherwise, it is inserted.
    /// Returns the vector of saved pages.
    pub async fn save_async_batch(
        pages: &[CrawledPage],
    ) -> crate::sam::memory::Result<Vec<CrawledPage>> {
        let mut pages_cleaned = pages
            .iter()
            .filter(|p| !p.url.is_empty())
            .collect::<Vec<_>>();
        pages_cleaned.sort_by(|a, b| a.url.cmp(&b.url));
        let mut seen = HashSet::new();
        pages_cleaned.retain(|p| seen.insert(&p.url));

        // Collect all URLs from pages_cleaned
        let urls: Vec<&String> = pages_cleaned.iter().map(|p| &p.url).collect();
        if urls.is_empty() {
            return Ok(vec![]);
        }

        // Build a PostgresQueries to select rows where url matches any of the URLs
        let mut pg_query = PostgresQueries::default();
        let mut i = 0;
        for url in &urls {
            if i > 0 {
                pg_query
                    .queries
                    .push(crate::sam::memory::PGCol::String((*url).clone()));
                pg_query.query_columns.push(" OR url =".to_string());
            } else {
                pg_query
                    .queries
                    .push(crate::sam::memory::PGCol::String((*url).clone()));
                pg_query.query_columns.push("url =".to_string());
            }
            i += 1;
        }

        // Query for existing pages by URL
        let existing_pages = Self::select_async(None, None, None, Some(pg_query)).await?;

        // Remove from pages_cleaned any page whose URL matches an existing page
        let existing_urls: HashSet<&String> = existing_pages.iter().map(|p| &p.url).collect();
        pages_cleaned.retain(|p| !existing_urls.contains(&p.url));

        // Early return if nothing to insert
        if pages_cleaned.is_empty() {
            return Ok(vec![]);
        }

        if pages.is_empty() {
            return Ok(vec![]);
        }

        // Prepare bulk UPSERT (insert or update on conflict)
        // Only url is unique, so use ON CONFLICT(url)
        let mut values = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut tokens_strs: Vec<String> = Vec::new();

        // First, collect all tokens_strs
        for page in pages_cleaned.iter() {
            tokens_strs.push(page.tokens.join("\n"));
        }
        // Then, build values and params
        for (i, page) in pages_cleaned.iter().enumerate() {
            values.push(format!("(${}, ${}, ${})", i * 3 + 1, i * 3 + 2, i * 3 + 3));
            params.push(&page.url);
            params.push(&tokens_strs[i]);
            params.push(&page.timestamp);
        }

        let sql = format!(
            "INSERT INTO crawled_pages (url, tokens, timestamp) VALUES {} \
            ON CONFLICT(url) DO UPDATE SET tokens = EXCLUDED.tokens, timestamp = EXCLUDED.timestamp",
            values.join(", ")
        );

        let config = crate::sam::memory::Config::new();
        let client = config.connect_pool().await?;
        client.execute(sql.as_str(), &params[..]).await?;

        Ok(pages.to_vec())
    }

    pub async fn save_async(&self) -> crate::sam::memory::Result<Self> {
        let tokens_str = self.tokens.join("\n");
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.url.clone()));
        pg_query.query_columns.push("url =".to_string());

        // Check for existing by url
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;

        let config = crate::sam::memory::Config::new();
        let client = config.connect_pool().await?;

        if rows.is_empty() {
            client
                .execute(
                    "INSERT INTO crawled_pages (url, tokens, timestamp) VALUES ($1, $2, $3)",
                    &[&self.url, &tokens_str, &self.timestamp],
                )
                .await?;
        } else {
            client
                .execute(
                    "UPDATE crawled_pages SET tokens = $1, timestamp = $2 WHERE url = $3",
                    &[&tokens_str, &self.timestamp, &self.url],
                )
                .await?;
        }
        Ok(self.clone())
    }
    pub fn destroy(url: String) -> crate::sam::memory::Result<bool> {
        Config::destroy_row(url, Self::sql_table_name())
    }

    /// Query crawled pages for the most probable results for a given query string.
    /// Returns a vector of (CrawledPage, score), sorted by descending score.
    /// Query crawled pages for the most probable results for a given query string.
    /// Returns a vector of (CrawledPage, score), sorted by descending score.
    pub async fn query_by_relevance_async(
        query: &str,
        limit: usize,
    ) -> crate::sam::memory::Result<Vec<(CrawledPage, usize)>> {
        // Tokenize the query (lowercase, split on whitespace, remove punctuation)
        let query_tokens: Vec<String> = query
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect();

        if query_tokens.is_empty() {
            return Ok(vec![]);
        }

        // Try to filter at the DB level if possible (e.g., by LIKE on url or tokens)
        let mut pg_query = PostgresQueries::default();
        let like_pattern_zero = format!("%{}%", query_tokens[0]);
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(like_pattern_zero.clone()));
        pg_query.query_columns.push("url ilike".to_string());
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(like_pattern_zero));
        pg_query.query_columns.push(" OR tokens ilike".to_string());
        for token in &query_tokens {
            let like_pattern = format!("%{token}%");
            pg_query
                .queries
                .push(crate::sam::memory::PGCol::String(like_pattern));
            pg_query.query_columns.push(" OR tokens ilike".to_string());
        }

        // Fetch a subset of pages matching the first token in the URL (as a filter)
        let pages = match Self::select_async(
            Some(500),
            None,
            Some("timestamp DESC".to_string()),
            Some(pg_query.clone()),
        )
        .await
        {
            Ok(p) if !p.is_empty() => p,
            _ => vec![],
        };

        let query_tokens_set: HashSet<&str> = query_tokens.iter().map(|s| s.as_str()).collect();
        let query_lower = query.to_lowercase();

        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(_) => 0,
        };

        let mut scored: Vec<(CrawledPage, usize)> = pages
            .into_iter()
            .map(|page| {
                let page_tokens: HashSet<&str> = page.tokens.iter().map(|t| t.as_str()).collect();
                let mut score: usize = 0;
                for token in &query_tokens_set {
                    if page_tokens.contains(token) {
                        score += 1;
                    }
                }

                if page.url.to_lowercase() == format!("https://www.{query_lower}.com/")
                    || page.url.to_lowercase() == format!("https://{query_lower}.com/")
                    || page.url.to_lowercase() == format!("https://www.{query_lower}.com")
                    || page.url.to_lowercase() == format!("https://{query_lower}.com")
                {
                    score += 1000;
                }

                if page.url.to_lowercase() == format!("http://www.{query_lower}.com/")
                    || page.url.to_lowercase() == format!("http://{query_lower}.com/")
                {
                    score += 700;
                }

                if page.url.to_lowercase().contains(&query_lower) {
                    score += 2;
                }
                // Heuristics
                let url_lower = page.url.to_lowercase();
                for token in &query_tokens_set {
                    if url_lower.contains(token) {
                        score += 1;
                    }
                }
                if page.timestamp > now - 30 * 24 * 60 * 60 {
                    score += 1;
                }
                if let Ok(parsed_url) = Url::parse(&page.url) {
                    if let Some(domain) = parsed_url.domain() {
                        let domain_lower = domain.to_lowercase();
                        for token in &query_tokens_set {
                            if domain_lower.contains(token) {
                                score += 1;
                            }
                        }
                    }
                }
                if page.tokens.len() > 100 {
                    score += 1;
                }
                if page.links.len() > 20 {
                    score += 1;
                }
                if page.timestamp < now - 365 * 24 * 60 * 60 {
                    score = score.saturating_sub(1);
                }
                if url_lower.starts_with(&query_lower) {
                    score += 1;
                }
                if url_lower.ends_with(&query_lower) {
                    score += 1;
                }
                (page, score)
            })
            .filter(|(_, score)| *score > 0)
            .collect();

        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(limit);

        Ok(scored)
    }

    /// Collect all tokens from crawled pages, rank by frequency, and write top X to a file.
    /// The file will be written to /opt/sam/tmp/common.tokens, one token per line.
    pub async fn write_most_common_tokens_async(limit: usize) -> std::io::Result<()> {
        // Collect all tokens from all crawled pages asynchronously

        let pages = match Self::select_async(None, None, None, None).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to select crawled pages: {}", e);
                return Err(std::io::Error::other(e.to_string()));
            }
        };

        let mut freq: HashMap<String, usize> = HashMap::new();
        for page in pages {
            for token in page.tokens {
                *freq.entry(token).or_insert(0) += 1;
            }
        }

        // Sort tokens by frequency, descending
        let mut freq_vec: Vec<(String, usize)> = freq.into_iter().collect();
        freq_vec.sort_by(|a, b| b.1.cmp(&a.1));

        // Take the top `limit` tokens
        let top_tokens = freq_vec.into_iter().take(limit).map(|(token, _)| token);

        // Write to file (use spawn_blocking for file I/O)
        let tokens: Vec<String> = top_tokens.collect();
        tokio::task::spawn_blocking(move || {
            let mut file = File::create("/opt/sam/tmp/common.tokens")?;
            for token in tokens {
                writeln!(file, "{token}")?;
            }
            Ok(())
        })
        .await?
    }

    /// Serialize this CrawledPage to a JSON string for P2P sharing.
    pub fn to_p2p_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize a CrawledPage from a JSON string received via P2P.
    pub fn from_p2p_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Send this CrawledPage to a peer over a TCP stream (async).
    /// The stream must be connected. The message is length-prefixed (u32, big-endian).
    pub async fn send_p2p<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        mut writer: W,
    ) -> std::io::Result<()> {
        let json = self.to_p2p_json().map_err(std::io::Error::other)?;
        let bytes = json.as_bytes();
        let len = bytes.len() as u32;
        writer.write_u32(len).await?;
        writer.write_all(bytes).await?;
        Ok(())
    }

    /// Receive a CrawledPage from a peer over a TCP stream (async).
    /// Expects a length-prefixed (u32, big-endian) JSON message.
    pub async fn recv_p2p<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> std::io::Result<Self> {
        let len = reader.read_u32().await?;
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf).await?;
        let json = String::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::from_p2p_json(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
