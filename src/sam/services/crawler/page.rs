use serde::{Serialize, Deserialize};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::sam::memory::{Config, PostgresQueries};
use tokio_postgres::Row;
use serde_json;
use redis::{AsyncCommands, aio::MultiplexedConnection, Client as RedisClient};
use once_cell::sync::OnceCell;
use log;
use reqwest::Url;
use regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tokio::io::{AsyncReadExt,AsyncWriteExt};
use base64::{engine::general_purpose, Engine as _};

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_MANAGER: OnceCell<RedisClient> = OnceCell::new();

async fn redis_client() -> redis::RedisResult<MultiplexedConnection> {
    let client = match REDIS_MANAGER.get_or_try_init(|| RedisClient::open(REDIS_URL)) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to initialize Redis client: {}", e);
            return Err(redis::RedisError::from((redis::ErrorKind::IoError, "Redis client init failed")));
        }
    };
    client.get_multiplexed_async_connection().await
}

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
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
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
    pub fn sql_table_name() -> String { "crawled_pages".to_string() }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawled_pages (
            id serial PRIMARY KEY,
            url varchar NOT NULL,
            tokens text,
            timestamp BIGINT
        );"
    }
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_url ON crawled_pages (url);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_timestamp ON crawled_pages (timestamp);",
            // For tokens, a GIN index is best if using Postgres full-text search, but here we use a normal index for the text column:
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_tokens ON crawled_pages (tokens);",
        ]
    }
    pub fn migrations() -> Vec<&'static str> { vec![] }
    pub fn from_row(row: &Row) -> crate::sam::memory::Result<Self> {
        // let links_str: Option<String> = row.get("links");
        // let links = links_str.map(|s| s.split('\n').map(|s| s.to_string()).collect()).unwrap_or_default();
        let tokens_str: Option<String> = row.get("tokens");
        let tokens = tokens_str.map(|s| s.split('\n').map(|s| s.to_string()).collect()).unwrap_or_default();
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
            tokens,
            crawl_job_oid: String::new(),
            links: Vec::new(), 
            timestamp: row.get("timestamp"),
        })
    }
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> crate::sam::memory::Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;
        for j in jsons {
            let object: Self = match serde_json::from_str(&j) {
                Ok(obj) => obj,
                Err(e) => {
                    log::error!("Failed to deserialize CrawledPage: {}", e);
                    return Err(crate::sam::memory::Error::with_chain(e, "Deserialization error"));
                }
            };
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }
    pub async fn select_async(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> crate::sam::memory::Result<Vec<Self>> {
        // For simple queries (by url), try Redis first
        if let Some(q) = &query {
            if q.queries.len() == 1 {
                if let crate::sam::memory::PGCol::String(ref url) = q.queries[0] {
                    if let Some(obj) = Self::get_redis(url).await {
                        return Ok(vec![obj]);
                    }
                }
            }
        }
        match tokio::task::spawn_blocking(move || Self::select(limit, offset, order, query)).await {
            Ok(res) => res,
            Err(e) => {
                log::error!("Failed to join select_async task: {}", e);
                Err(crate::sam::memory::Error::with_chain(e, "JoinError in select_async"))
            }
        }
    }
    pub fn save(&self) -> crate::sam::memory::Result<Self> {
        let mut client = Config::client()?;
        // Check for existing by url
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.url.clone()));
        pg_query.query_columns.push("url =".to_string());
        let rows = Self::select(None, None, None, Some(pg_query.clone()))?;
        let links_str = self.links.join("\n");
        let tokens_str = self.tokens.join("\n");
        if rows.is_empty() {
            client.execute(
                "INSERT INTO crawled_pages (url, tokens, timestamp) VALUES ($1, $2, $3)",
                &[&self.url, &tokens_str, &self.timestamp]
            )?;
        } else {
            client.execute(
                "UPDATE crawled_pages SET tokens = $1, timestamp = $2 WHERE url = $3",
                &[&tokens_str, &self.timestamp, &self.url]
            )?;
        }
        Ok(self.clone())
    }
    pub async fn save_async(&self) -> crate::sam::memory::Result<Self> {
        let this = self.clone();
        // Save to Redis first for fast access
        let _ = this.save_redis().await;
        match tokio::task::spawn_blocking(move || this.save()).await {
            Ok(res) => res,
            Err(e) => {
                log::error!("Failed to join save_async task: {}", e);
                Err(crate::sam::memory::Error::with_chain(e, "JoinError in save_async"))
            }
        }
    }
    pub fn destroy(url: String) -> crate::sam::memory::Result<bool> {
        Config::destroy_row(url, Self::sql_table_name())
    }

    async fn redis_key(&self) -> String {
        format!("crawledpage:{}", encode_url_hash(&self.url))
    }
    pub async fn save_redis(&self) -> redis::RedisResult<()> {
        log::info!("Saving CrawledPage to Redis: {}", self.url);
        let mut con = redis_client().await?;
        let key = self.redis_key().await;
        let val = match serde_json::to_string(self) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to serialize CrawledPage for Redis: {}", e);
                return Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Serialization error")));
            }
        };
        con.set(key, val).await
    }
    pub async fn get_redis(url: &str) -> Option<Self> {
        let mut con = match redis_client().await {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("crawledpage:{}", encode_url_hash(url));
        let val: Option<String> = con.get(key).await.ok();
        val.and_then(|v| {
            let obj: Result<CrawledPage, _> = serde_json::from_str(&v);
            obj.ok()
        })
    }

    /// Query crawled pages for the most probable results for a given query string.
    /// Returns a vector of (CrawledPage, score), sorted by descending score.
    pub fn query_by_relevance(query: &str, limit: usize) -> crate::sam::memory::Result<Vec<(CrawledPage, usize)>> {
        // Catch panics to avoid crashing the CLI/TUI
        let result = std::panic::catch_unwind(|| {
            // Tokenize the query (lowercase, split on whitespace, remove punctuation)
            let query_tokens: Vec<String> = query
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| !w.is_empty())
                .collect();

            // Early return if no tokens
            if query_tokens.is_empty() {
                return Ok::<Vec<(CrawledPage, usize)>, crate::sam::memory::Error>(vec![]);
            }

            // Fetch all crawled pages (could optimize with DB full-text search)
            let pages = tokio::runtime::Handle::current()
                .block_on(CrawledPage::select_async(None, None, Some("timestamp DESC".to_string()), None))?;

            // Score each page by number of query tokens present in its tokens
            let mut scored: Vec<(CrawledPage, usize)> = pages
                .into_iter()
                .map(|page| {
                    let page_tokens: std::collections::HashSet<_> = page.tokens.iter().map(|t| t.as_str()).collect();
                    let mut score = 0;
                    for token in &query_tokens {
                        if page_tokens.contains(token.as_str()) {
                            score += 1;
                        }
                    }
                    // Bonus: if query is substring of URL, add to score
                    if page.url.to_lowercase().contains(&query.to_lowercase()) {
                        score += 2;
                    }
                    (page, score)
                })
                .filter(|(_, score)| *score > 0)
                .collect();

            // Additional scoring heuristics:
            for (page, score) in &mut scored {
                // Bonus: if query tokens appear in the URL path or domain, add to score
                let url_lower = page.url.to_lowercase();
                for token in &query_tokens {
                if url_lower.contains(token) {
                    *score += 1;
                }
                }
                // Bonus: if query tokens appear in the links, add to score
                // for link in &page.links {
                // let link_lower = link.to_lowercase();
                // for token in &query_tokens {
                //     if link_lower.contains(token) {
                //     *score += 1;
                //     }
                // }
                // }
                // Bonus: if the page is more recent (timestamp within last 30 days), add to score
                let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => {
                        log::error!("SystemTime error: {}", e);
                        0
                    }
                };
                if page.timestamp > now - 30 * 24 * 60 * 60 {
                *score += 1;
                }
                // Bonus: if the query tokens appear in the page's domain name, add to score
                if let Ok(parsed_url) = Url::parse(&page.url) {
                if let Some(domain) = parsed_url.domain() {
                    let domain_lower = domain.to_lowercase();
                    for token in &query_tokens {
                    if domain_lower.contains(token) {
                        *score += 1;
                    }
                    }
                }
                }
                // Bonus: if the page has more tokens (longer content), add to score
                if page.tokens.len() > 100 {
                *score += 1;
                }
                // Bonus: if the page has many links (potentially a hub), add to score
                if page.links.len() > 20 {
                *score += 1;
                }
                // Penalty: if the page is very old (older than 1 year), subtract from score
                if page.timestamp < now - 365 * 24 * 60 * 60 {
                *score = score.saturating_sub(1);
                }
                // Bonus: if the query matches the start of the URL, add to score
                if page.url.to_lowercase().starts_with(&query.to_lowercase()) {
                *score += 1;
                }
                // Bonus: if the query matches the end of the URL, add to score
                if page.url.to_lowercase().ends_with(&query.to_lowercase()) {
                *score += 1;
                }
            }

            // Sort by descending score
            scored.sort_by(|a, b| b.1.cmp(&a.1));

            // Limit results
            scored.truncate(limit);

            Ok(scored)
        });

        match result {
            Ok(Ok(scored)) => Ok(scored),
            Ok(Err(e)) => {
                // Underlying DB or logic error
                log::error!("query_by_relevance error: {}", e);
                Ok(vec![])
            }
            Err(_) => {
                // Panic occurred
                log::error!("query_by_relevance panicked");
                Ok(vec![])
            }
        }
    }

    pub async fn query_by_relevance_async(query: &str, limit: usize) -> crate::sam::memory::Result<Vec<(CrawledPage, usize)>> {
        // Use spawn_blocking to move the CPU-intensive search to a separate thread
        // without trying to create a new runtime
        let query_string = query.to_string(); // Clone the query for move
        tokio::task::spawn_blocking(move || {
            Self::query_by_relevance(&query_string, limit)
        })
        .await
        .unwrap_or_else(|e| {
            log::error!("Search task panicked: {}", e);
            Ok(vec![])
        })
    }

    /// Collect all tokens from crawled pages, rank by frequency, and write top X to a file.
    /// The file will be written to /opt/sam/tmp/common.tokens, one token per line.
    pub async fn write_most_common_tokens_async(limit: usize) -> std::io::Result<()> {
        // Collect all tokens from all crawled pages asynchronously
        let pages = match Self::select_async(None, None, None, None).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to select crawled pages: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
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
                writeln!(file, "{}", token)?;
            }
            Ok(())
        }).await?
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
    pub async fn send_p2p<W: tokio::io::AsyncWrite + Unpin>(&self, mut writer: W) -> std::io::Result<()> {
        let json = self.to_p2p_json().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
        let json = String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::from_p2p_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}


/// Encodes a URL into a predictable, reversible hash string.
/// The encoding is URL-safe base64 of the UTF-8 bytes of the URL.
pub fn encode_url_hash(url: &str) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(url.as_bytes())
}

/// Decodes a hash string back into the original URL.
/// Returns None if the hash is invalid or not decodable.
pub fn decode_url_hash(hash: &str) -> Option<String> {
    general_purpose::URL_SAFE_NO_PAD
        .decode(hash)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}