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

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_MANAGER: OnceCell<RedisClient> = OnceCell::new();

async fn redis_client() -> redis::RedisResult<MultiplexedConnection> {
    let client = REDIS_MANAGER.get_or_init(|| RedisClient::open(REDIS_URL).unwrap());
    client.get_multiplexed_async_connection().await
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