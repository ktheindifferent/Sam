use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use crate::sam::memory::{Config, PostgresQueries};
use tokio_postgres::Row;
use reqwest::Url;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, Duration};
use trust_dns_resolver::TokioAsyncResolver;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, LevelFilter};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use once_cell::sync::{Lazy, OnceCell};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::path::Path;
use tokio::fs;
use serde_json;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use regex;
use url::ParseError;
use redis::{AsyncCommands, aio::MultiplexedConnection, Client as RedisClient};

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_MANAGER: OnceCell<RedisClient> = OnceCell::new();

async fn redis_client() -> redis::RedisResult<MultiplexedConnection> {
    let client = REDIS_MANAGER.get_or_init(|| RedisClient::open(REDIS_URL).unwrap());
    client.get_multiplexed_async_connection().await
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlJob {
    pub id: i32,
    pub oid: String,
    pub start_url: String,
    pub status: String, // "pending", "running", "done", "error"
    pub created_at: i64,
    pub updated_at: i64,
}
impl Default for CrawlJob {
    fn default() -> Self {
        Self::new()
    }
}
impl CrawlJob {
    pub fn new() -> CrawlJob {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        CrawlJob {
            id: 0,
            oid,
            start_url: String::new(),
            status: "pending".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
    pub fn sql_table_name() -> String { "crawl_jobs".to_string() }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawl_jobs (
            id serial PRIMARY KEY,
            oid varchar NOT NULL UNIQUE,
            start_url varchar NOT NULL,
            status varchar NOT NULL,
            created_at BIGINT,
            updated_at BIGINT
        );"
    }
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_crawl_jobs_oid ON crawl_jobs (oid);",
            "CREATE INDEX IF NOT EXISTS idx_crawl_jobs_start_url ON crawl_jobs (start_url);",
            "CREATE INDEX IF NOT EXISTS idx_crawl_jobs_status ON crawl_jobs (status);",
            "CREATE INDEX IF NOT EXISTS idx_crawl_jobs_created_at ON crawl_jobs (created_at);",
            "CREATE INDEX IF NOT EXISTS idx_crawl_jobs_updated_at ON crawl_jobs (updated_at);",
        ]
    }
    pub fn migrations() -> Vec<&'static str> { vec![] }
    pub fn from_row(row: &Row) -> crate::sam::memory::Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            start_url: row.get("start_url"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> crate::sam::memory::Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
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
        // For simple queries (by oid), try Redis first
        if let Some(q) = &query {
            if q.queries.len() == 1 {
                if let crate::sam::memory::PGCol::String(ref oid) = q.queries[0] {
                    if let Some(obj) = Self::get_redis(oid).await {
                        return Ok(vec![obj]);
                    }
                }
            }
        }
        tokio::task::spawn_blocking(move || Self::select(limit, offset, order, query))
            .await
            .unwrap()
    }
    pub fn save(&self) -> crate::sam::memory::Result<Self> {
        let mut client = Config::client()?;
        // Check for existing by oid
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select(None, None, None, Some(pg_query.clone()))?;
        if rows.is_empty() {
            client.execute(
                "INSERT INTO crawl_jobs (oid, start_url, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                &[&self.oid, &self.start_url, &self.status, &self.created_at, &self.updated_at]
            )?;
        } else {
            client.execute(
                "UPDATE crawl_jobs SET start_url = $1, status = $2, updated_at = $3 WHERE oid = $4",
                &[&self.start_url, &self.status, &self.updated_at, &self.oid]
            )?;
        }
        Ok(self.clone())
    }
    pub async fn save_async(&self) -> crate::sam::memory::Result<Self> {
        let this = self.clone();
        // Save to Redis first for fast access
        let _ = this.save_redis().await;
        tokio::task::spawn_blocking(move || this.save()).await.unwrap()
    }
    pub fn destroy(oid: String) -> crate::sam::memory::Result<bool> {
        Config::destroy_row(oid, Self::sql_table_name())
    }

    async fn redis_key(&self) -> String {
        format!("crawljob:{}", self.oid)
    }
    pub async fn save_redis(&self) -> redis::RedisResult<()> {
        log::info!("Saving CrawlJob to Redis: {}", self.oid);
        let mut con = redis_client().await?;
        let key = self.redis_key().await;
        let val = serde_json::to_string(self).unwrap();
        con.set(key, val).await
    }
    pub async fn get_redis(oid: &str) -> Option<Self> {
        let mut con = match redis_client().await {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("crawljob:{}", oid);
        let val: Option<String> = con.get(key).await.ok();
        val.and_then(|v| {
            let obj: Result<CrawlJob, _> = serde_json::from_str(&v);
            obj.ok()
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawledPage {
    pub id: i32,
    pub oid: String,
    pub crawl_job_oid: String,
    pub url: String,
    pub tokens: Vec<String>,
    pub links: Vec<String>,
    pub status_code: Option<i32>,
    pub error: Option<String>,
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
            oid,
            crawl_job_oid: String::new(),
            url: String::new(),
            tokens: vec![],
            links: vec![],
            status_code: None,
            error: None,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        }
    }
    pub fn sql_table_name() -> String { "crawled_pages".to_string() }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawled_pages (
            id serial PRIMARY KEY,
            oid varchar NOT NULL UNIQUE,
            crawl_job_oid varchar NOT NULL,
            url varchar NOT NULL,
            tokens text,
            links text,
            status_code integer,
            error text,
            timestamp BIGINT
        );"
    }
    pub fn sql_indexes() -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_oid ON crawled_pages (oid);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_url ON crawled_pages (url);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_crawl_job_oid ON crawled_pages (crawl_job_oid);",
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_timestamp ON crawled_pages (timestamp);",
            // For tokens, a GIN index is best if using Postgres full-text search, but here we use a normal index for the text column:
            "CREATE INDEX IF NOT EXISTS idx_crawled_pages_tokens ON crawled_pages (tokens);",
        ]
    }
    pub fn migrations() -> Vec<&'static str> { vec![] }
    pub fn from_row(row: &Row) -> crate::sam::memory::Result<Self> {
        let links_str: Option<String> = row.get("links");
        let links = links_str.map(|s| s.split('\n').map(|s| s.to_string()).collect()).unwrap_or_default();
        let tokens_str: Option<String> = row.get("tokens");
        let tokens = tokens_str.map(|s| s.split('\n').map(|s| s.to_string()).collect()).unwrap_or_default();
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            crawl_job_oid: row.get("crawl_job_oid"),
            url: row.get("url"),
            tokens,
            links,
            status_code: row.get("status_code"),
            error: row.get("error"),
            timestamp: row.get("timestamp"),
        })
    }
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> crate::sam::memory::Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
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
        // For simple queries (by oid), try Redis first
        if let Some(q) = &query {
            if q.queries.len() == 1 {
                if let crate::sam::memory::PGCol::String(ref oid) = q.queries[0] {
                    if let Some(obj) = Self::get_redis(oid).await {
                        return Ok(vec![obj]);
                    }
                }
            }
        }
        tokio::task::spawn_blocking(move || Self::select(limit, offset, order, query))
            .await
            .unwrap()
    }
    pub fn save(&self) -> crate::sam::memory::Result<Self> {
        let mut client = Config::client()?;
        // Check for existing by oid
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select(None, None, None, Some(pg_query.clone()))?;
        let links_str = self.links.join("\n");
        let tokens_str = self.tokens.join("\n");
        if rows.is_empty() {
            client.execute(
                "INSERT INTO crawled_pages (oid, crawl_job_oid, url, tokens, links, status_code, error, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[&self.oid, &self.crawl_job_oid, &self.url, &tokens_str, &links_str, &self.status_code, &self.error, &self.timestamp]
            )?;
        } else {
            client.execute(
                "UPDATE crawled_pages SET crawl_job_oid = $1, url = $2, tokens = $3, links = $4, status_code = $5, error = $6, timestamp = $7 WHERE oid = $8",
                &[&self.crawl_job_oid, &self.url, &tokens_str, &links_str, &self.status_code, &self.error, &self.timestamp, &self.oid]
            )?;
        }
        Ok(self.clone())
    }
    pub async fn save_async(&self) -> crate::sam::memory::Result<Self> {
        let this = self.clone();
        // Save to Redis first for fast access
        let _ = this.save_redis().await;
        tokio::task::spawn_blocking(move || this.save()).await.unwrap()
    }
    pub fn destroy(oid: String) -> crate::sam::memory::Result<bool> {
        Config::destroy_row(oid, Self::sql_table_name())
    }

    async fn redis_key(&self) -> String {
        format!("crawledpage:{}", self.oid)
    }
    pub async fn save_redis(&self) -> redis::RedisResult<()> {
        log::info!("Saving CrawledPage to Redis: {}", self.oid);
        let mut con = redis_client().await?;
        let key = self.redis_key().await;
        let val = serde_json::to_string(self).unwrap();
        con.set(key, val).await
    }
    pub async fn get_redis(oid: &str) -> Option<Self> {
        let mut con = match redis_client().await {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("crawledpage:{}", oid);
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
                for link in &page.links {
                let link_lower = link.to_lowercase();
                for token in &query_tokens {
                    if link_lower.contains(token) {
                    *score += 1;
                    }
                }
                }
                // Bonus: if the page has a status_code of 200, add to score
                if page.status_code == Some(200) {
                *score += 1;
                }
                // Penalty: if the page has an error, subtract from score
                if page.error.is_some() {
                *score = score.saturating_sub(1);
                }
                // Bonus: if the page is more recent (timestamp within last 30 days), add to score
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
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
}

// Internal boxed async fn for recursion
async fn crawl_url_inner(
    job_oid: String,
    url: String,
    depth: usize,
) -> crate::sam::memory::Result<CrawledPage> {
    let max_depth = 2;

    let mut pg_query = PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String(url.clone()));
    pg_query.query_columns.push("url =".to_string());
    let existing = CrawledPage::select_async(None, None, None, Some(pg_query)).await.unwrap_or_default();
    if !existing.is_empty() {
        return Ok(existing[0].clone());
    }

    let mut page = CrawledPage::new();
    page.crawl_job_oid = job_oid.clone();
    page.url = url.clone();
    info!("Fetching URL: {}", url);
    let resp = reqwest::get(&url).await;
    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            page.status_code = Some(status as i32);
            if status == 200 {
                let html = resp.text().await.unwrap_or_default();
                if (!html.is_empty()) {
                    // Move HTML parsing and token extraction to a blocking thread
                    let url_clone = url.clone();
                    let (mut tokens, mut links) = tokio::task::spawn_blocking(move || {
                        let document = scraper::Html::parse_document(&html);
                        // Extract visible text only (ignore script/style/noscript)
                        let mut tokens = Vec::new();
                        let mut links = Vec::new();
                        let body_selector = scraper::Selector::parse("body").unwrap();
                        let skip_tags = ["script", "style", "noscript", "svg", "canvas", "iframe", "template"];
                        let skip_selector = skip_tags
                            .iter()
                            .map(|tag| scraper::Selector::parse(tag).unwrap())
                            .collect::<Vec<_>>();

                        fn extract_text(element: &scraper::ElementRef, skip_selector: &[scraper::Selector], tokens: &mut Vec<String>) {
                            // Skip unwanted tags
                            for sel in skip_selector {
                                if sel.matches(element) {
                                    return;
                                }
                            }
                            // Collect text nodes
                            for child in element.children() {
                                match child.value() {
                                    scraper::node::Node::Text(t) => {
                                        for word in t.text.split_whitespace() {
                                            let w = word.trim_matches(|c: char| !c.is_alphanumeric());
                                            if !w.is_empty() {
                                                tokens.push(w.to_lowercase());
                                            }
                                        }
                                    }
                                    scraper::node::Node::Element(_) => {
                                        if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                                            extract_text(&child_elem, skip_selector, tokens);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // Only extract text if the document is not an image/media file
                        let url_lc = url_clone.to_ascii_lowercase();
                        let mut file_mime: Option<&str> = None;
                        // Map of file extensions to MIME types
                        let mime_map = [
                            // Images
                            (".jpg", "image/jpeg"),
                            (".jpeg", "image/jpeg"),
                            (".png", "image/png"),
                            (".gif", "image/gif"),
                            (".bmp", "image/bmp"),
                            (".webp", "image/webp"),
                            (".svg", "image/svg+xml"),
                            (".ico", "image/x-icon"),
                            (".tiff", "image/tiff"),
                            (".tif", "image/tiff"),
                            (".heic", "image/heic"),
                            (".heif", "image/heif"),
                            (".apng", "image/apng"),
                            (".avif", "image/avif"),
                            // Audio
                            (".mp3", "audio/mpeg"),
                            (".wav", "audio/wav"),
                            (".ogg", "audio/ogg"),
                            (".oga", "audio/ogg"),
                            (".flac", "audio/flac"),
                            (".aac", "audio/aac"),
                            (".m4a", "audio/mp4"),
                            (".opus", "audio/opus"),
                            (".mid", "audio/midi"),
                            (".midi", "audio/midi"),
                            (".amr", "audio/amr"),
                            // Video
                            (".mp4", "video/mp4"),
                            (".webm", "video/webm"),
                            (".mov", "video/quicktime"),
                            (".avi", "video/x-msvideo"),
                            (".mkv", "video/x-matroska"),
                            (".flv", "video/x-flv"),
                            (".mpg", "video/mpeg"),
                            (".mpeg", "video/mpeg"),
                            (".3gp", "video/3gpp"),
                            (".3g2", "video/3gpp2"),
                            (".wmv", "video/x-ms-wmv"),
                            (".m4v", "video/x-m4v"),
                            (".ts", "video/mp2t"),
                            (".ogv", "video/ogg"),
                            // Documents
                            (".pdf", "application/pdf"),
                            (".doc", "application/msword"),
                            (".docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
                            (".ppt", "application/vnd.ms-powerpoint"),
                            (".pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
                            (".xls", "application/vnd.ms-excel"),
                            (".xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
                            (".epub", "application/epub+zip"),
                            (".mobi", "application/x-mobipocket-ebook"),
                            (".azw3", "application/vnd.amazon.ebook"),
                            (".fb2", "application/x-fictionbook+xml"),
                            (".chm", "application/vnd.ms-htmlhelp"),
                            (".xps", "application/vnd.ms-xpsdocument"),
                            (".odt", "application/vnd.oasis.opendocument.text"),
                            (".ods", "application/vnd.oasis.opendocument.spreadsheet"),
                            (".odp", "application/vnd.oasis.opendocument.presentation"),
                            (".odg", "application/vnd.oasis.opendocument.graphics"),
                            (".odf", "application/vnd.oasis.opendocument.formula"),
                            (".odc", "application/vnd.oasis.opendocument.chart"),
                            (".odm", "application/vnd.oasis.opendocument.text-master"),
                            // Archives
                            (".zip", "application/zip"),
                            (".tar", "application/x-tar"),
                            (".gz", "application/gzip"),
                            (".rar", "application/x-rar-compressed"),
                            (".7z", "application/x-7z-compressed"),
                            (".bz2", "application/x-bzip2"),
                            (".xz", "application/x-xz"),
                            // Text/code
                            (".csv", "text/csv"),
                            (".json", "application/json"),
                            (".xml", "application/xml"),
                            (".yaml", "application/x-yaml"),
                            (".yml", "application/x-yaml"),
                            (".md", "text/markdown"),
                            (".rst", "text/x-rst"),
                            // Scripts/styles
                            (".js", "application/javascript"),
                            (".mjs", "application/javascript"),
                            (".cjs", "application/javascript"),
                            (".ts", "application/typescript"),
                            (".tsx", "application/typescript"),
                            (".jsx", "application/javascript"),
                            (".css", "text/css"),
                            (".scss", "text/x-scss"),
                            (".sass", "text/x-sass"),
                            (".less", "text/x-less"),
                            // Fonts
                            (".woff", "font/woff"),
                            (".woff2", "font/woff2"),
                            (".ttf", "font/ttf"),
                            (".otf", "font/otf"),
                            (".eot", "application/vnd.ms-fontobject"),
                            // Others
                            (".swf", "application/x-shockwave-flash"),
                            (".jar", "application/java-archive"),
                            (".exe", "application/vnd.microsoft.portable-executable"),
                            (".apk", "application/vnd.android.package-archive"),
                            (".dmg", "application/x-apple-diskimage"),
                            (".iso", "application/x-iso9660-image"),
                        ];

                        // Extract file extension from path (before query/fragment)
                        let file_ext = {
                            let url_no_query = url_lc.split(['?', '#'].as_ref()).next().unwrap_or("");
                            std::path::Path::new(url_no_query)
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| format!(".{}", ext))
                        };

                        if let Some(ref ext) = file_ext {
                            for (map_ext, mime) in mime_map.iter() {
                                if ext.eq_ignore_ascii_case(map_ext) {
                                    file_mime = Some(*mime);
                                    break;
                                }
                            }
                        }

                        if let Some(mime) = file_mime {
                            tokens.push(mime.to_string());
                        } else {
                            for body in document.select(&body_selector) {
                                extract_text(&body, &skip_selector, &mut tokens);
                            }
                        }

                        // Extract <a href>
                        let a_selector = scraper::Selector::parse("a[href]").unwrap();
                        for element in document.select(&a_selector) {
                            if let Some(link) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(link)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(link)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Extract <img src>
                        let img_selector = scraper::Selector::parse("img[src]").unwrap();
                        for element in document.select(&img_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Extract <audio src> and <audio><source src>
                        let audio_selector = scraper::Selector::parse("audio[src]").unwrap();
                        for element in document.select(&audio_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }
                        let source_selector = scraper::Selector::parse("audio source[src], video source[src]").unwrap();
                        for element in document.select(&source_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Extract <video src>
                        let video_selector = scraper::Selector::parse("video[src]").unwrap();
                        for element in document.select(&video_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Extract <link rel="stylesheet" href="">
                        let link_selector = scraper::Selector::parse("link[rel=\"stylesheet\"]").unwrap();
                        for element in document.select(&link_selector) {
                            if let Some(href) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(href)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(href)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Extract <script src="">
                        let script_selector = scraper::Selector::parse("script[src]").unwrap();
                        for element in document.select(&script_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        // Add MIME type tokens for media links
                        let mut mime_tokens = Vec::new();
                        for link in &links {
                            if let Ok(parsed) = Url::parse(link) {
                                if let Some(path) = parsed.path_segments().and_then(|s| s.last()) {
                                    // Remove query and fragment
                                    let fname = path.split(['?', '#'].as_ref()).next().unwrap_or("");
                                    // Get extension after last dot, if any
                                    let ext = fname.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                                    let mime = match ext.as_str() {
                                        "jpg" | "jpeg" => Some("image/jpeg"),
                                        "png" => Some("image/png"),
                                        "gif" => Some("image/gif"),
                                        "mp3" => Some("audio/mp3"),
                                        "wav" => Some("audio/wav"),
                                        "ogg" => Some("audio/ogg"),
                                        "mp4" => Some("video/mp4"),
                                        "webm" => Some("video/webm"),
                                        "mov" => Some("video/quicktime"),
                                        "avi" => Some("video/x-msvideo"),
                                        "bpm" => Some("image/bmp"),
                                        "webp" => Some("image/webp"),
                                        "svg" => Some("image/svg+xml"),
                                        "ico" => Some("image/x-icon"),
                                        "tiff" => Some("image/tiff"),
                                        "flv" => Some("video/x-flv"),
                                        "css" => Some("text/css"),
                                        "js" => Some("application/javascript"),
                                        _ => None,
                                    };
                                    if let Some(m) = mime {
                                        // tokens = Vec::new();
                                        mime_tokens.push(m.to_string());
                                    }
                                }
                            }
                        }
                        tokens.extend(mime_tokens);

                        (tokens, links)
                    }).await.unwrap();

                    tokens.sort();
                    tokens.dedup();
                    links.sort();
                    links.dedup();

                    // Remove tokens that are too common to be usefull for querying
                    // Only include the most common English stopwords and trivial tokens.
                    // Common tokens (stopwords) in multiple languages to conserve DB space.
                    // English, Spanish, French, German, Italian, Portuguese, Dutch, Russian, Chinese (pinyin), Japanese (romaji), etc.
                    let common_tokens = vec![
                        // English
                        "the", "is", "in", "and", "to", "a", "of", "for", "on", "that", "this", "it", "with",
                        "as", "at", "by", "an", "be", "are", "was", "were", "from", "or", "but", "not", "have",
                        "has", "had", "will", "would", "can", "could", "should", "do", "does", "did", "so",
                        "if", "then", "than", "which", "who", "whom", "whose", "what", "when", "where", "why",
                        "how", "about", "all", "any", "each", "few", "more", "most", "other", "some", "such",
                        "no", "nor", "only", "own", "same", "too", "very", "just", "over", "under", "again",
                        "once", "also", "into", "out", "up", "down", "off", "above", "below", "between", "after",
                        "before", "during", "through", "because", "while", "both", "either", "neither", "may",
                        "might", "must", "our", "your", "their", "his", "her", "its", "them", "they", "he", "she",
                        "we", "you", "i", "me", "my", "mine", "yours", "theirs", "ours", "us",
                        "him", "hers", "himself", "herself", "itself", "themselves", "ourselves", "yourself",
                        "yourselves", "am", "shall",
                        // Numbers
                        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
                        "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40",
                        "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "100", "1000",

                        // Spanish
                        "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "del", "al", "y", "o", "u", "en", "con", "por", "para",
                        "es", "que", "se", "no", "sí", "su", "sus", "le", "lo", "como", "más", "pero", "ya", "o", "muy", "sin", "sobre",
                        "entre", "también", "hasta", "desde", "todo", "todos", "todas", "toda", "mi", "mis", "tu", "tus", "su", "sus",
                        "este", "esta", "estos", "estas", "ese", "esa", "esos", "esas", "aquel", "aquella", "aquellos", "aquellas",
                        "yo", "tú", "él", "ella", "nosotros", "vosotros", "ellos", "ellas", "me", "te", "se", "nos", "os", "les",

                        // French
                        "le", "la", "les", "un", "une", "des", "du", "de", "en", "et", "à", "au", "aux", "pour", "par", "sur", "dans",
                        "est", "ce", "cette", "ces", "il", "elle", "ils", "elles", "nous", "vous", "tu", "je", "me", "te", "se", "leur",
                        "lui", "son", "sa", "ses", "mon", "ma", "mes", "ton", "ta", "tes", "notre", "nos", "votre", "vos", "leur", "leurs",
                        "qui", "que", "quoi", "dont", "où", "quand", "comment", "pourquoi", "avec", "sans", "sous", "entre", "aussi",
                        "plus", "moins", "très", "bien", "mal", "comme", "mais", "ou", "donc", "or", "ni", "car",

                        // German
                        "der", "die", "das", "ein", "eine", "einer", "eines", "einem", "einen", "und", "oder", "aber", "den", "dem", "des",
                        "zu", "mit", "auf", "für", "von", "an", "im", "in", "am", "aus", "bei", "nach", "über", "unter", "vor", "zwischen",
                        "ist", "war", "sind", "sein", "hat", "haben", "wird", "werden", "nicht", "kein", "keine", "mehr", "weniger", "auch",
                        "nur", "schon", "noch", "immer", "man", "wir", "ihr", "sie", "er", "es", "ich", "du", "mein", "dein", "sein", "ihr",
                        "unser", "euer", "dies", "diese", "dieser", "dieses", "jener", "jene", "jenes",

                        // Italian
                        "il", "lo", "la", "i", "gli", "le", "un", "una", "uno", "dei", "delle", "degli", "del", "della", "dello", "dei",
                        "e", "o", "ma", "per", "con", "su", "tra", "fra", "di", "da", "a", "al", "ai", "agli", "alla", "alle", "allo",
                        "che", "chi", "cui", "come", "quando", "dove", "perché", "quale", "quali", "questo", "questa", "questi", "queste",
                        "quello", "quella", "quelli", "quelle", "io", "tu", "lui", "lei", "noi", "voi", "mi", "ti", "si", "ci", "vi",

                        // Portuguese
                        "o", "a", "os", "as", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
                        "por", "para", "com", "sem", "sobre", "entre", "e", "ou", "mas", "também", "como", "mais", "menos", "muito", "pouco",
                        "já", "ainda", "só", "todo", "toda", "todos", "todas", "meu", "minha", "meus", "minhas", "teu", "tua", "teus", "tuas",
                        "seu", "sua", "seus", "suas", "nosso", "nossa", "nossos", "nossas", "vosso", "vossa", "vossos", "vossas", "ele", "ela",
                        "eles", "elas", "nós", "vós", "eu", "tu", "você", "vocês", "lhe", "lhes", "me", "te", "se", "nos", "vos",

                        // Dutch
                        "de", "het", "een", "en", "of", "maar", "want", "dus", "voor", "na", "met", "zonder", "over", "onder", "tussen",
                        "in", "op", "aan", "bij", "tot", "van", "uit", "door", "om", "tot", "als", "dan", "dat", "die", "dit", "deze",
                        "die", "wie", "wat", "waar", "wanneer", "hoe", "waarom", "welke", "wij", "jij", "hij", "zij", "het", "ik", "je",
                        "mijn", "jouw", "zijn", "haar", "ons", "onze", "hun", "uw", "hun", "ze", "u", "men", "er", "hier", "daar",

                        // Russian (transliterated)
                        "i", "a", "no", "da", "net", "on", "ona", "ono", "oni", "my", "vy", "ty", "ya", "moy", "tvoy", "ego", "ee", "nas",
                        "vas", "ikh", "kto", "chto", "gde", "kogda", "pochemu", "kak", "eto", "v", "na", "s", "k", "o", "po", "za", "ot",
                        "do", "iz", "u", "nad", "pod", "pervyy", "vtoroy", "odin", "dva", "tri", "chetyre", "pyat", "shest", "sem", "vosem",
                        "devyat", "desyat", "bolshe", "menshe", "vse", "vsyo", "vsego", "eto", "tak", "zdes", "tam", "tut", "to", "eto",

                        // Chinese (pinyin, most common stopwords)
                        "de", "shi", "bu", "le", "zai", "ren", "wo", "ni", "ta", "men", "zhe", "na", "yi", "ge", "you", "he", "ye", "ma",
                        "ba", "ne", "li", "dui", "dao", "zai", "shang", "xia",

                        // Japanese (romaji, common particles and pronouns)
                        "no", "ni", "wa", "ga", "wo", "de", "to", "mo", "kara", "made", "yori", "e", "ka", "ne", "yo", "kore", "sore", "are",
                        "dore", "kono", "sono", "ano", "dono", "watashi", "anata", "kare", "kanojo", "watashitachi", "anatatachi", "karera",
                        "kanojotachi", "koko", "soko", "asoko", "doko", "itsu", "dare", "nani", "nan", "ikutsu", "ikura", "doushite", "dou",

                        // Turkish
                        "ve", "bir", "bu", "da", "de", "için", "ile", "ama", "veya", "çok", "az", "daha", "en", "gibi", "mi",
                        "mu", "mü", "ben", "sen", "o", "biz", "siz", "şu", "bu", "şey", "her", "hiç", "bazı", "bazı", "bazı",

                        // Arabic (transliterated)
                        "wa", "fi", "min", "ila", "an", "ala", "ma", "la", "huwa", "hiya", "anta", "anti", "nahnu", "antum", "antunna",
                        "hum", "hunna", "hadha", "hadhi", "dhalika", "tilka", "huna", "hunaka", "ayna", "mata", "kayfa", "limadha",

                        // Hindi (transliterated)
                        "hai", "ka", "ki", "ke", "mein", "par", "aur", "ya", "lekin", "bhi", "ko", "se", "tak", "ko", "mein", "tum", "main",
                        "vah", "yeh", "ham", "aap", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka",

                        // Polish
                        "i", "w", "na", "z", "do", "o", "za", "po", "przez", "dla", "od", "bez", "pod", "nad", "przy", "między",
                        "jest", "być", "był", "była", "było", "byli", "były", "ten", "ta", "to", "ci", "te", "tam", "tu", "kto",
                        "co", "gdzie", "kiedy", "jak", "dlaczego", "który", "która", "które", "którzy",

                        // Scandinavian (Danish, Norwegian, Swedish)
                        "och", "att", "det", "som", "en", "ett", "den", "de", "på", "av", "med", "till", "för", "från", "är", "var", "har",
                        "hade", "inte", "men", "om", "eller", "så", "vi", "ni", "han", "hon", "de", "vi", "ni", "jag", "du", "mig", "dig",

                        // Greek (transliterated)
                        "kai", "se", "apo", "me", "gia", "os", "stin", "sto", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin",
                        "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin",

                        // Add more as needed for other languages...
                    ];
                    // Retain tokens not in common_tokens, or that look like dates (e.g., 05/20/1996)
                    let date_regex = regex::Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}$").unwrap();
                    tokens.retain(|token| {
                        !common_tokens.contains(&token.as_str())
                            || date_regex.is_match(token)
                            // Match YYYY-MM-DD, YYYY/MM/DD, DD-MM-YYYY, DD/MM/YYYY, MM-DD-YYYY, MM/DD/YYYY
                            || regex::Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$").unwrap().is_match(token)
                            || regex::Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$").unwrap().is_match(token)
                            // Match YYYYMMDD or DDMMYYYY or MMDDYYYY
                            || regex::Regex::new(r"^\d{8}$").unwrap().is_match(token)
                            // Match YYYY.MM.DD or DD.MM.YYYY
                            || regex::Regex::new(r"^\d{4}\.\d{1,2}\.\d{1,2}$").unwrap().is_match(token)
                            || regex::Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$").unwrap().is_match(token)
                            // Match ISO 8601 date with optional time (e.g., 2023-05-20T15:30:00Z)
                            || regex::Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?(Z|([+-]\d{2}:\d{2}))?)?$").unwrap().is_match(token)
                    });
                    // Remove tokens that are too long or too short
                    tokens.retain(|token| token.len() > 2 && token.len() < 20);
                    // Remove tokens that are too similar to the URL
                    let url_tokens: HashSet<_> = url.split('/').map(|s| s.to_lowercase()).collect();
                    tokens.retain(|token| !url_tokens.contains(&token.to_lowercase()));
                    // Remove tokens that are too similar to the domain
                    if let Ok(domain) = Url::parse(&url).and_then(|u| {
                        u.domain()
                            .map(|d| d.to_string())
                            .ok_or_else(|| ParseError::EmptyHost)
                    }) {
                        let domain_tokens: HashSet<_> = domain.split('.').map(|s| s.to_lowercase()).collect();
                        tokens.retain(|token| !domain_tokens.contains(&token.to_lowercase()));
                    }

                    

                    page.tokens = tokens;
                    page.links = links;
                    info!("Fetched URL: {} ({} links, {} tokens)", url, page.links.len(), page.tokens.len());
                    // Only save if we have tokens (i.e., HTML was present)
                    if !page.tokens.is_empty() {
                        page.save_async().await?;
                    }
                }
            }
        }
        Err(e) => {
            info!("Error fetching URL {}: {}", url, e);
            page.error = Some(e.to_string());
        }
    }

    // Crawl links sequentially (not in parallel), but only if depth < max_depth
    if depth < max_depth && !page.links.is_empty() {
        let job_oid = page.crawl_job_oid.clone();
        let links = page.links.clone();
        for link in links {
            let _ = crawl_url_boxed(job_oid.clone(), link, depth + 1).await;
        }
    }
    Ok(page)
}

// Boxed async fn for recursion
fn crawl_url_boxed(job_oid: String, url: String, depth: usize) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::sam::memory::Result<CrawledPage>> + Send>> {
    Box::pin(crawl_url_inner(job_oid, url, depth))
}

// Public entry point (non-recursive, just calls boxed version)
pub async fn crawl_url(job_oid: String, url: String) -> crate::sam::memory::Result<CrawledPage> {
    crawl_url_boxed(job_oid, url, 0).await
}

static CRAWLER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Start the crawler service in the background (call from main or CLI)
pub fn start_service() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);

        // Only create a runtime if not already inside one
        if tokio::runtime::Handle::try_current().is_ok() {
            // Already inside a runtime: spawn the service directly
            tokio::spawn(async {
                run_crawler_service().await;
            });
        } else {
            // Not inside a runtime: spawn a thread and create a runtime
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    run_crawler_service().await;
                });
            });
        }
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
    log::info!("Crawler service started.");
}

/// Async-friendly version for use from async contexts (e.g., ratatui CLI)
pub async fn start_service_async() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);
        tokio::spawn(async {
            run_crawler_service().await;
        });
        log::info!("Crawler service started.");
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
}

pub fn stop_service() {
    info!("Crawler service stopping...");
    CRAWLER_RUNNING.store(false, Ordering::SeqCst);
    info!("Crawler service stopped.");
}

pub fn service_status() -> &'static str {
    if CRAWLER_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

// Add a static DNS cache (domain -> Option<bool> for found/not found)
static DNS_CACHE_PATH: &str = "/opt/sam/dns.cache";
static DNS_LOOKUP_CACHE: Lazy<TokioMutex<HashMap<String, bool>>> = Lazy::new(|| TokioMutex::new(HashMap::new())) ;

// Load DNS cache from disk at startup
async fn load_dns_cache() {
    if !Path::new(DNS_CACHE_PATH).exists() {
        // Create an empty cache file if it doesn't exist
        let _ = fs::write(DNS_CACHE_PATH, b"{}").await;
    }
    let path = Path::new(DNS_CACHE_PATH);
    if let Ok(data) = fs::read(path).await {
        if let Ok(map) = serde_json::from_slice::<HashMap<String, bool>>(&data) {
            let mut cache = DNS_LOOKUP_CACHE.lock().await;
            *cache = map;
            log::info!("Loaded DNS cache with {} entries", cache.len());
        }
    }
}

// Save DNS cache to disk
async fn save_dns_cache() {
    let cache = DNS_LOOKUP_CACHE.lock().await;
    if let Ok(data) = serde_json::to_vec(&*cache) {
        let _ = fs::write(DNS_CACHE_PATH, data).await;
        log::info!("Saved DNS cache with {} entries", cache.len());
    }
}

// Cache all CrawlJob and CrawledPage entries from Postgres into Redis
async fn cache_all_to_redis() {
    log::info!("Caching all CrawlJob and CrawledPage entries to Redis...");
    // Limit DB select to 100 at a time to avoid freezing with huge tables
    let mut offset = 0;
    let batch_size = 100;
    loop {
        match CrawlJob::select_async(Some(batch_size), Some(offset), None, None).await {
            Ok(jobs) if jobs.is_empty() => break,
            Ok(jobs) => {
                let mut handles = Vec::new();
                for job in &jobs {
                    handles.push(job.save_redis());
                }
                for handle in handles {
                    let _ = handle.await;
                }
                offset += jobs.len();
                log::info!("Cached {}/? CrawlJob entries into Redis", offset);
                if jobs.len() < batch_size { break; }
            }
            Err(e) => {
                log::warn!("Failed to cache CrawlJob entries to Redis: {}", e);
                break;
            }
        }
    }
    offset = 0;
    loop {
        match CrawledPage::select_async(Some(batch_size), Some(offset), None, None).await {
            Ok(pages) if pages.is_empty() => break,
            Ok(pages) => {
                let mut handles = Vec::new();
                for page in &pages {
                    handles.push(page.save_redis());
                }
                for handle in handles {
                    let _ = handle.await;
                }
                offset += pages.len();
                log::info!("Cached {}/? CrawledPage entries into Redis", offset);
                if pages.len() < batch_size { break; }
            }
            Err(e) => {
                log::warn!("Failed to cache CrawledPage entries to Redis: {}", e);
                break;
            }
        }
    }
}

/// Main crawler loop: finds pending jobs, crawls, updates status
async fn run_crawler_service() {
    use trust_dns_resolver::config::*;
    // Suppress trust_dns_proto noisy logs
    log::set_max_level(LevelFilter::Info);
    // if let Some(logger) = log::logger().downcast_ref::<tui_logger::Logger>() {
    //     logger.filter_module("trust_dns_proto", LevelFilter::Warn);
    // }
    let crawling = Arc::new(TokioMutex::new(()));
    let common_urls = vec![
        "https://www.youtube.com/",
        "https://www.rust-lang.org/",
        "https://www.wikipedia.org/",
        "https://www.example.com/",
        "https://www.mozilla.org/",
        "https://www.github.com/",
        "https://www.google.com/",
        "https://www.facebook.com/",
        "https://www.twitter.com/",
        "https://www.instagram.com/",
        "https://www.linkedin.com/",
        "https://www.reddit.com/",
        "https://www.amazon.com/",
        "https://www.apple.com/",
        "https://www.microsoft.com/",
        "https://www.netflix.com/",
        "https://www.stackoverflow.com/",
        "https://www.bbc.com/",
        "https://www.cnn.com/",
        "https://www.nytimes.com/",
        "https://www.quora.com/",
        "https://www.paypal.com/",
        "https://www.dropbox.com/",
        "https://www.adobe.com/",
        "https://www.slack.com/",
        "https://www.twitch.tv/",
        "https://www.spotify.com/",
        "https://www.medium.com/",
        "https://www.booking.com/",
        "https://www.airbnb.com/",
        "https://www.uber.com/",
        "https://www.lyft.com/",
        "https://www.soundcloud.com/",
        "https://www.vimeo.com/",
        "https://www.flickr.com/",
        "https://www.imdb.com/",
        "https://www.pinterest.com/",
        "https://www.wordpress.com/",
        "https://www.tumblr.com/",
        "https://www.ebay.com/",
        "https://www.bing.com/",
        "https://www.duckduckgo.com/",
        "https://www.yandex.com/",
        "https://www.yahoo.com/",
        "https://www.weather.com/",
        "https://www.office.com/",
        "https://www.salesforce.com/",
        "https://www.shopify.com/",
        "https://www.tesla.com/",
        "https://www.walmart.com/",
        "https://www.target.com/",
        "https://www.nasa.gov/",
        "https://www.nationalgeographic.com/",
        "https://www.forbes.com/",
        "https://www.wsj.com/",
        "https://www.bloomberg.com/",
        "https://www.cnbc.com/",
        "https://www.foxnews.com/",
        "https://www.usatoday.com/",
        "https://www.time.com/",
        "https://www.theguardian.com/",
        "https://www.huffpost.com/",
        "https://www.latimes.com/",
        "https://www.chicagotribune.com/",
        "https://www.nbcnews.com/",
        "https://www.cbsnews.com/",
        "https://www.abcnews.go.com/",
        "https://www.npr.org/",
        "https://www.smh.com.au/",
        "https://www.lemonde.fr/",
        "https://www.spiegel.de/",
        "https://www.elpais.com/",
        "https://www.corriere.it/",
        "https://www.asahi.com/",
        "https://www.sina.com.cn/",
        "https://www.qq.com/",
        "https://www.taobao.com/",
        "https://www.tmall.com/",
        "https://www.baidu.com/",
        "https://www.sohu.com/",
        "https://www.weibo.com/",
        "https://www.163.com/",
        "https://www.jd.com/",
        "https://www.aliexpress.com/",
        "https://www.alibaba.com/",
        "https://www.booking.com/",
        "https://www.expedia.com/",
        "https://www.tripadvisor.com/",
        "https://www.skyscanner.net/",
        "https://www.kayak.com/",
        "https://www.zillow.com/",
        "https://www.trulia.com/",
        "https://www.rightmove.co.uk/",
        "https://www.autotrader.com/",
        "https://www.cars.com/",
        "https://www.carmax.com/",
        "https://www.indeed.com/",
        "https://www.glassdoor.com/",
        "https://www.monster.com/",
        "https://www.simplyhired.com/",
        "https://www.craigslist.org/",
        "https://www.meetup.com/",
        "https://www.eventbrite.com/",
        "https://www.change.org/",
        "https://www.whitehouse.gov/",
        "https://www.usa.gov/",
        "https://www.loc.gov/",
        "https://www.nih.gov/",
        "https://www.cdc.gov/",
        "https://www.fbi.gov/",
        "https://www.cia.gov/",
        "https://www.nsa.gov/",
        "https://www.un.org/",
        "https://www.europa.eu/",
        "https://www.who.int/",
        "https://www.imf.org/",
        "https://www.worldbank.org/",
        "https://www.oecd.org/",
        "https://www.wto.org/",
        "https://www.icann.org/",
        "https://www.iso.org/",
        "https://www.ietf.org/",
        "https://www.w3.org/",
        "https://www.gnu.org/",
        "https://www.linuxfoundation.org/",
        "https://www.apache.org/",
        "https://www.python.org/",
        "https://www.nodejs.org/",
        "https://www.npmjs.com/",
        "https://www.ruby-lang.org/",
        "https://www.php.net/",
        "https://www.mysql.com/",
        "https://www.postgresql.org/",
        "https://www.mongodb.com/",
        "https://www.redis.io/",
        "https://www.heroku.com/",
        "https://www.digitalocean.com/",
        "https://www.linode.com/",
        "https://www.cloudflare.com/",
        "https://www.vercel.com/",
        "https://www.netlify.com/",
        "https://www.gitlab.com/",
        "https://www.bitbucket.org/",
        "https://www.atlassian.com/",
        "https://www.trello.com/",
        "https://www.notion.so/",
        "https://www.zoho.com/",
        "https://www.mailchimp.com/",
        "https://www.hubspot.com/",
        "https://www.squarespace.com/",
        "https://www.wix.com/",
        "https://www.weebly.com/",
        "https://www.medium.com/",
        "https://www.substack.com/",
        "https://www.patreon.com/",
        "https://www.kickstarter.com/",
        "https://www.indiegogo.com/",
        "https://www.gofundme.com/",
        "https://www.ted.com/",
        "https://www.coursera.org/",
        "https://www.edx.org/",
        "https://www.udemy.com/",
        "https://www.khanacademy.org/",
        "https://www.codecademy.com/",
        "https://www.pluralsight.com/",
        "https://www.udacity.com/",
        "https://www.duolingo.com/",
        "https://www.memrise.com/",
        "https://www.rosettastone.com/",
        "https://www.babbel.com/",
        "https://www.openai.com/",
        "https://www.deepmind.com/",
        "https://www.anthropic.com/",
        "https://www.stability.ai/",
        "https://www.midjourney.com/",
        "https://www.perplexity.ai/",
        "https://www.runwayml.com/",
        "https://www.huggingface.co/",
        "https://www.replit.com/",
        "https://www.jsfiddle.net/",
        "https://www.codepen.io/",
        "https://www.codesandbox.io/",
        "https://www.stackexchange.com/",
        "https://www.superuser.com/",
        "https://www.serverfault.com/",
        "https://www.askubuntu.com/",
        "https://www.mathoverflow.net/",
        "https://www.acm.org/",
        "https://www.ieee.org/",
        "https://www.nature.com/",
        "https://www.sciencemag.org/",
        "https://www.cell.com/",
        "https://www.thelancet.com/",
        "https://www.jstor.org/",
        "https://www.arxiv.org/",
        "https://www.biorxiv.org/",
        "https://www.medrxiv.org/",
        "https://www.springer.com/",
        "https://www.elsevier.com/",
        "https://www.taylorandfrancis.com/",
        "https://www.cambridge.org/",
        "https://www.oxfordjournals.org/",
        "https://www.ssrn.com/",
        "https://www.researchgate.net/",
        "https://www.academia.edu/",
        "https://www.mit.edu/",
        "https://www.harvard.edu/",
        "https://www.stanford.edu/",
        "https://www.berkeley.edu/",
        "https://www.ox.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.ethz.ch/",
        "https://www.tum.de/",
        "https://www.tokyo-u.ac.jp/",
        "https://www.kyoto-u.ac.jp/",
        "https://www.sydney.edu.au/",
        "https://www.unimelb.edu.au/",
        "https://www.tsinghua.edu.cn/",
        "https://www.pku.edu.cn/",
        "https://www.iitb.ac.in/",
        "https://www.iisc.ac.in/",
        "https://www.nus.edu.sg/",
        "https://www.ntu.edu.sg/",
        "https://www.kaist.ac.kr/",
        "https://www.snu.ac.kr/",
        "https://www.technion.ac.il/",
        "https://www.weizmann.ac.il/",
        "https://www.utoronto.ca/",
        "https://www.mcgill.ca/",
        "https://www.ubc.ca/",
        "https://www.uq.edu.au/",
        "https://www.unsw.edu.au/",
        "https://www.monash.edu/",
        "https://www.ucl.ac.uk/",
        "https://www.imperial.ac.uk/",
        "https://www.lse.ac.uk/",
        "https://www.kcl.ac.uk/",
        "https://www.ed.ac.uk/",
        "https://www.manchester.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.sheffield.ac.uk/",
        "https://www.southampton.ac.uk/",
        "https://www.nottingham.ac.uk/",
        "https://www.birmingham.ac.uk/",
        "https://www.leeds.ac.uk/",
        "https://www.liverpool.ac.uk/",
        "https://www.cardiff.ac.uk/",
        "https://www.gla.ac.uk/",
        "https://www.strath.ac.uk/",
        "https://www.abdn.ac.uk/",
        "https://www.dundee.ac.uk/",
        "https://www.st-andrews.ac.uk/",
        "https://www.hw.ac.uk/",
        "https://www.rgu.ac.uk/",
        "https://www.qmul.ac.uk/",
        "https://www.gold.ac.uk/",
        "https://www.soas.ac.uk/",
        "https://www.bbk.ac.uk/",
        "https://www.city.ac.uk/",
        "https://www.lshtm.ac.uk/",
        "https://www.open.ac.uk/",
        "https://www.roehampton.ac.uk/",
        "https://www.westminster.ac.uk/",
        "https://www.gre.ac.uk/",
        "https://www.kingston.ac.uk/",
        "https://www.mdx.ac.uk/",
        "https://www.uel.ac.uk/",
        "https://www.londonmet.ac.uk/",
        "https://www.sunderland.ac.uk/",
        "https://www.northumbria.ac.uk/",
        "https://www.newcastle.ac.uk/",
        "https://www.durham.ac.uk/",
        "https://www.york.ac.uk/",
        "https://www.hull.ac.uk/",
        "https://www.lincoln.ac.uk/",
        "https://www.derby.ac.uk/",
        "https://www.staffs.ac.uk/",
        "https://www.keele.ac.uk/",
        "https://www.wlv.ac.uk/",
        "https://www.coventry.ac.uk/",
        "https://www.warwick.ac.uk/",
        "https://www.le.ac.uk/",
        "https://www.lboro.ac.uk/",
        "https://www.nottstrent.ac.uk/",
        "https://www.shef.ac.uk/",
        "https://www.hud.ac.uk/",
        "https://www.bradford.ac.uk/",
        "https://www.salford.ac.uk/",
        "https://www.mmu.ac.uk/",
        "https://www.ljmu.ac.uk/",
        "https://www.edgehill.ac.uk/",
        "https://www.uclan.ac.uk/",
        "https://www.lancaster.ac.uk/",
        "https://www.bangor.ac.uk/",
        "https://www.swansea.ac.uk/",
        "https://www.aber.ac.uk/",
        "https://www.glyndwr.ac.uk/",
        "https://www.cardiffmet.ac.uk/",
        "https://www.southwales.ac.uk/",
        "https://www.wrexham.ac.uk/",
        "https://www.uwtsd.ac.uk/",
        "https://www.oxfordbrookes.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.bucks.ac.uk/",
        "https://www.chi.ac.uk/",
        "https://www.canterbury.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.sussex.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.plymouth.ac.uk/",
        "https://www.exeter.ac.uk/",
        "https://www.bath.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.glos.ac.uk/",
        "https://www.uwe.ac.uk/",
        "https://www.westofengland.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.reading.ac.uk/",
        "https://www.ox.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.bucks.ac.uk/",
        "https://www.chi.ac.uk/",
        "https://www.canterbury.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.sussex.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.plymouth.ac.uk/",
        "https://www.exeter.ac.uk/",
        "https://www.bath.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.glos.ac.uk/",
        "https://www.uwe.ac.uk/",
        "https://www.westofengland.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.reading.ac.uk/",
    ];
    // DNS resolver setup
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
        .expect("Failed to create DNS resolver");

    // Helper function to perform concurrent DNS lookups with cache
    async fn lookup_domains<I: IntoIterator<Item = String>>(
        resolver: &TokioAsyncResolver,
        domains: I,
    ) -> Vec<String> {
        let mut futures = FuturesUnordered::new();
        for domain in domains {
            let resolver = resolver.clone();
            let domain_clone = domain.clone();
            futures.push(async move {
                // Check cache first
                {
                    let cache = DNS_LOOKUP_CACHE.lock().await;
                    if let Some(found) = cache.get(&domain_clone) {
                        if *found {
                            return Some(domain_clone);
                        } else {
                            return None;
                        }
                    }
                }
                // Not in cache, do DNS lookup
                let found = match resolver.lookup_ip(domain_clone.clone()).await {
                    Ok(lookup) => lookup.iter().next().is_some(),
                    Err(_) => false,
                };
                // Update cache (but don't save to disk here)
                {
                    let mut cache = DNS_LOOKUP_CACHE.lock().await;
                    cache.insert(domain_clone.clone(), found);
                }
                if found {
                    Some(domain_clone)
                } else {
                    None
                }
            });
        }
        let mut found = Vec::new();
        while let Some(result) = futures.next().await {
            if let Some(domain) = result {
                found.push(domain);
            }
        }
        // Save cache after each batch
        save_dns_cache().await;
        found
    }

    load_dns_cache().await;

    // Cache all Postgres crawler data into Redis on service start
    cache_all_to_redis().await;

    loop {
        if (!CRAWLER_RUNNING.load(Ordering::SeqCst)) {
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        // Only one crawl at a time
        let _guard = crawling.lock().await;

        // Find a pending job
        let jobs = match CrawlJob::select_async(Some(1), None, None, None).await {
            Ok(jobs) => jobs.into_iter().filter(|j| j.status == "pending").collect::<Vec<_>>(),
            Err(_) => vec![],
        };

        if let Some(mut job) = jobs.into_iter().next() {
            info!("Starting crawl job: oid={} url={}", job.oid, job.start_url);
            // Mark as running
            job.status = "running".to_string();
            job.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
            let _ = job.save_async().await;

            // Crawl start_url and discovered links (BFS, depth 2)
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back((job.start_url.clone(), 0));
            let max_depth = 2;
            while let Some((url, depth)) = queue.pop_front() {
                if visited.contains(&url) || depth > max_depth {
                    continue;
                }
                visited.insert(url.clone());
                info!("Crawling url={} depth={}", url, depth);
                match crawl_url(job.oid.clone(), url.clone()).await {
                    Ok(page) => {
                        // Already saved in crawl_url
                        for link in &page.links {
                            if !visited.contains(link) {
                                queue.push_back((link.clone(), depth + 1));
                            }
                        }
                    }
                    Err(e) => {
                        info!("Crawler error: {}", e);
                        log::error!("Crawler error: {}", e);
                    }
                }
            }
            // Mark job as done
            job.status = "done".to_string();
            job.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
            let _ = job.save_async().await;
            info!("Finished crawl job: oid={}", job.oid);
        } else {
            // No jobs: scan common URLs and/or use DNS queries to find domains
            info!("No pending crawl jobs found. Crawling common URLs.");
            let mut urls_to_try: Vec<String> = common_urls.iter().map(|s| s.to_string()).collect();

            // Attempt to discover as many domains as possible (within reason)
            // WARNING: The DNS root zone contains thousands of TLDs and millions of domains.
            // Here, we demonstrate a systematic approach, but in practice, this is limited by DNS, rate limits, and ethics.
            // We'll enumerate common TLDs and prefixes, but not brute-force the entire DNS space.

            let tlds = vec![
                "com", "net", "org", "io", "co", "ai", "dev", "app", "info", "biz", "us", "uk", "ca", "de", "jp", "fr", "au", "ru", "ch", "it", "nl", "se", "no", "es", "cz", "in", "br", "pl", "me", "tv", "xyz", "site", "online", "store", "tech", "pro", "club", "top", "vip", "live", "news", "cloud", "fun", "world", "today", "agency", "solutions", "digital", "media", "group", "center", "systems", "works", "company", "services", "network", "consulting", "support", "software", "design", "studio", "marketing", "events", "finance", "capital", "ventures", "partners", "law", "legal", "health", "care", "doctor", "clinic", "school", "academy", "education", "university", "college", "gov", "mil", "int", "edu", "museum", "travel", "jobs", "mobi", "name", "coop", "aero", "arpa"
            ];
            let prefixes = vec![
                "www", "mail", "blog", "shop", "store", "news", "app", "api", "dev", "test", "portal", "home", "web", "en", "es", "fr", "de", "it", "pt", "jp", "cn", "ru", "in", "us", "uk", "ca", "au", "br", "mx", "za", "nl", "se", "no", "fi", "dk", "pl", "cz", "tr", "kr", "id", "vn", "th", "my", "sg", "hk", "tw", "il", "ae", "sa", "ir", "eg", "ng", "ke", "gh", "ar", "cl", "co", "pe", "ve"
            ];
            let words = vec![
                "google", "facebook", "youtube", "twitter", "instagram", "wikipedia", 
                "amazon", "reddit", "yahoo", "linkedin", "netflix", "microsoft", 
                "apple", "github", "stackoverflow", "wordpress", "blogspot", 
                "tumblr", "pinterest", "paypal", "dropbox", "adobe", "slack", 
                "zoom", "twitch", "ebay", "bing", "duckduckgo", "quora", "imdb", 
                "bbc", "cnn", "nytimes", "forbes", "weather", "booking", "airbnb", 
                "uber", "lyft", "spotify", "soundcloud", "medium", "vimeo", "flickr",
                "news", "sports", "games", "movies", "music", "photos", "video", "live",
                "shop", "store", "market", "sale", "deal", "offer", "buy", "sell",
                "jobs", "career", "work", "hire", "resume", "apply", "school", "college",
                "university", "learn", "study", "teach", "class", "course", "academy",
                "health", "doctor", "clinic", "hospital", "care", "med", "pharmacy",
                "finance", "bank", "money", "loan", "credit", "card", "pay", "fund",
                "insurance", "tax", "invest", "trade", "stock", "crypto", "bitcoin",
                "weather", "travel", "trip", "flight", "hotel", "car", "rent", "map",
                "food", "pizza", "burger", "cafe", "bar", "restaurant", "menu", "order",
                "blog", "forum", "chat", "mail", "email", "message", "note", "wiki",
                "photo", "pic", "image", "gallery", "album", "camera", "snap", "art",
                "design", "dev", "code", "app", "site", "web", "cloud", "host", "server",
                "data", "ai", "bot", "robot", "smart", "tech", "digital", "media",
                "news", "press", "report", "story", "magazine", "journal", "book",
                "library", "archive", "docs", "file", "pdf", "doc", "sheet", "slide",
                "event", "meet", "party", "club", "group", "team", "community", "social",
                "network", "connect", "link", "share", "like", "follow", "friend",
                "support", "help", "faq", "guide", "info", "about", "contact", "home",
                "login", "signup", "register", "account", "profile", "user", "admin",
                "dashboard", "panel", "console", "system", "manager", "control", "settings",
                "tools", "tool", "kit", "box", "lab", "test", "beta", "demo", "sample",
                "random", "fun", "play", "game", "quiz", "test", "try", "beta", "alpha",
                "pro", "plus", "max", "prime", "vip", "elite", "gold", "silver", "basic",
                "free", "cheap", "deal", "sale", "discount", "offer", "promo", "gift",
                "shop", "store", "cart", "checkout", "buy", "sell", "order", "track",
                "review", "rate", "star", "top", "best", "hot", "new", "now", "today",
                "fast", "quick", "easy", "simple", "safe", "secure", "trusted", "official",
                "global", "world", "local", "city", "town", "village", "place", "zone",
                "area", "region", "state", "country", "nation", "gov", "org", "edu",
                "science", "math", "physics", "chemistry", "bio", "earth", "space",
                "astro", "geo", "eco", "env", "nature", "animal", "plant", "tree",
                "flower", "garden", "farm", "pet", "dog", "cat", "fish", "bird", "horse",
                "car", "bike", "bus", "train", "plane", "boat", "ship", "auto", "motor",
                "drive", "ride", "fly", "move", "run", "walk", "jump", "swim", "climb",
                "build", "make", "create", "craft", "draw", "paint", "write", "read",
                "speak", "talk", "listen", "hear", "see", "watch", "look", "view",
                "open", "close", "start", "stop", "go", "come", "join", "leave", "exit",
                "enter", "begin", "end", "finish", "win", "lose", "score", "goal",
                "plan", "project", "task", "todo", "list", "note", "memo", "remind",
                "alert", "alarm", "clock", "time", "date", "calendar", "schedule",
                "event", "meet", "call", "video", "voice", "chat", "message", "mail",
                "email", "post", "tweet", "blog", "forum", "board", "thread", "topic",
                "news", "press", "media", "tv", "radio", "movie", "film", "show",
                "music", "song", "album", "band", "artist", "dj", "mix", "play", "pause",
                "stop", "record", "edit", "cut", "copy", "paste", "save", "load",
                "send", "receive", "upload", "download", "sync", "backup", "restore",
                "scan", "print", "fax", "copy", "photo", "pic", "image", "video",
                "camera", "lens", "screen", "display", "monitor", "tv", "projector",
                "light", "lamp", "bulb", "fan", "ac", "heater", "fridge", "oven",
                "microwave", "washer", "dryer", "vacuum", "cleaner", "robot", "drone",
                "sensor", "alarm", "lock", "key", "door", "gate", "window", "wall",
                "roof", "floor", "room", "house", "home", "apartment", "flat", "villa",
                "hotel", "motel", "inn", "resort", "camp", "tent", "cabin", "hostel",
                "office", "desk", "chair", "table", "sofa", "bed", "bath", "toilet",
                "kitchen", "cook", "chef", "food", "meal", "dish", "snack", "drink",
                "water", "juice", "milk", "tea", "coffee", "beer", "wine", "bar",
                "pub", "club", "party", "event", "festival", "concert", "show",
                "exhibit", "expo", "fair", "market", "shop", "store", "mall", "plaza",
                "park", "garden", "zoo", "museum", "gallery", "library", "theater",
                "cinema", "stadium", "arena", "gym", "pool", "court", "field", "track",
                "ring", "course", "trail", "road", "street", "avenue", "boulevard",
                "drive", "lane", "way", "path", "route", "highway", "freeway", "bridge",
                "tunnel", "station", "stop", "terminal", "port", "harbor", "dock",
                "airport", "runway", "tower", "building", "block", "lot", "yard",
                "garden", "farm", "field", "forest", "mountain", "hill", "valley",
                "lake", "river", "sea", "ocean", "beach", "island", "bay", "coast",
                "shore", "cliff", "cave", "desert", "plain", "plateau", "volcano",
                "glacier", "reef", "coral", "delta", "marsh", "swamp", "pond", "pool",
                "spring", "well", "fountain", "waterfall", "cascade", "geyser",
            ];

            // Try all combinations of [prefix].[word].[tld]
            use futures::stream::{FuturesUnordered, StreamExt};

            // Prepare domain lists for concurrent lookup
            let mut rng = SmallRng::from_entropy();

            // [word].[tld] and [prefix].[word].[tld]
            let mut domains = Vec::new();
            for tld in &tlds {
                let mut sampled_words = words.clone();
                sampled_words.shuffle(&mut rng);
                for word in sampled_words.iter().take(1) {
                    domains.push(format!("{}.{}", word, tld));
                    for prefix in &prefixes {
                        domains.push(format!("{}.{}.{}", prefix, word, tld));
                    }
                }
            }
            // [prefix].[tld]
            for tld in &tlds {
                for prefix in &prefixes {
                    domains.push(format!("{}.{}", prefix, tld));
                }
            }
            // [word].[tld] (again, for completeness)
            for tld in &tlds {
                let mut sampled_words = words.clone();
                sampled_words.shuffle(&mut rng);
                for word in sampled_words.iter().take(1) {
                    domains.push(format!("{}.{}", word, tld));
                }
            }

            log::info!("Found {} domains to check", domains.len());

            // Sort the domains to ensure consistent order
            // This is important for deduplication and shuffling
            domains.sort();

            // Deduplicate the domains
            domains.dedup();

            // Shuffle the domains to randomize the order
            domains.shuffle(&mut rng);
            

            // Limit the number of domains to check
            let max_domains = num_cpus::get() * 10;
            let domains = &domains[..std::cmp::min(domains.len(), max_domains)];


            // Perform DNS lookups concurrently (limit batch size to avoid overload)
            let batch_size = num_cpus::get() / 2;
            for batch in domains.chunks(batch_size) {
                let found = lookup_domains(&resolver, batch.iter().cloned()).await;
                for domain in found {
                    urls_to_try.push(format!("https://{}/", domain));
                }
            }
            // Remove duplicates
            urls_to_try.sort();
            urls_to_try.dedup();

            // Crawl multiple URLs concurrently (limit concurrency to avoid overload)
            let concurrency = num_cpus::get() / 2;
            let mut url_iter = urls_to_try.into_iter();
            loop {
                let mut handles = Vec::new();
                for _ in 0..concurrency {
                    if let Some(url) = url_iter.next() {
                        let mut rng = SmallRng::from_entropy();
                        let dummy_job_oid: String = rng
                            .sample_iter(&Alphanumeric)
                            .take(15)
                            .map(char::from)
                            .collect();
                        handles.push(tokio::spawn(async move {
                            // Pass depth = 0 for top-level crawl
                            match crawl_url_boxed(dummy_job_oid, url.clone(), 0).await {
                                Ok(_page) => {
                                    log::info!("Crawled (no job): {}", url);
                                }
                                Err(e) => {
                                    info!("Crawler error (no job): {}", e);
                                    log::error!("Crawler error (no job): {}", e);
                                }
                            }
                        }));
                    }
                }
                if handles.is_empty() {
                    break;
                }
                for handle in handles {
                    let _ = handle.await;
                }
            }
        }
        // Sleep before next check
        sleep(Duration::from_secs(10)).await;
    }
}
