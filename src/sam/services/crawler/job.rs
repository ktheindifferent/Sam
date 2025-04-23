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

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_MANAGER: OnceCell<RedisClient> = OnceCell::new();

async fn redis_client() -> redis::RedisResult<MultiplexedConnection> {
    let client = REDIS_MANAGER.get_or_try_init(|| RedisClient::open(REDIS_URL))
        .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Failed to create Redis client", format!("{:?}", e))))?;
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
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(_) => 0, // fallback to 0 if system time is before UNIX_EPOCH
        };
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
            let object: Self = serde_json::from_str(&j)
                .map_err(|e| crate::sam::memory::Error::with_chain(e, "Failed to deserialize CrawlJob"))?;
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
        let result = tokio::task::spawn_blocking(move || Self::select(limit, offset, order, query))
            .await
            .map_err(|e| crate::sam::memory::Error::with_chain(e, "JoinError in select_async"))??;
        Ok(result)
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
        match tokio::task::spawn_blocking(move || this.save()).await {
            Ok(res) => res,
            Err(e) => Err(crate::sam::memory::Error::with_chain(e, "JoinError in save_async")),
        }
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
        let val = match serde_json::to_string(self) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to serialize CrawlJob for Redis: {}", e);
                return Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Serialization error")));
            }
        };
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