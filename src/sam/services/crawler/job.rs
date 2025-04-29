//! Crawler job definition and persistence layer.
//! 
//! Provides the CrawlJob struct and async/sync DB/Redis persistence for crawl jobs.

use serde::{Serialize, Deserialize};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::sam::memory::{Config, PostgresQueries};
use tokio_postgres::Row;
use serde_json;
use log;
use native_tls::{TlsConnector};
use postgres_native_tls::MakeTlsConnector;

/// Represents a crawl job (start URL, status, timestamps).
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
    /// Create a new CrawlJob with random OID and current timestamps.
    pub fn new() -> CrawlJob {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        CrawlJob {
            id: 0,
            oid,
            start_url: String::new(),
            status: "pending".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
    /// Table name for SQL.
    pub fn sql_table_name() -> String { "crawl_jobs".to_string() }
    /// SQL for table creation.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE IF NOT EXISTS crawl_jobs (\n            id serial PRIMARY KEY,\n            oid varchar NOT NULL UNIQUE,\n            start_url varchar NOT NULL,\n            status varchar NOT NULL,\n            created_at BIGINT,\n            updated_at BIGINT\n        );"
    }
    /// SQL index statements.
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
    /// Build from DB row.
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


    /// Build from async DB row.
    pub async fn from_row_async(row: &Row) -> crate::sam::memory::Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            start_url: row.get("start_url"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Select jobs from DB.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> crate::sam::memory::Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j)
                .map_err(|e| crate::sam::memory::Error::with_chain(e, "Failed to deserialize CrawlJob"))?;
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }
    /// Async select (no Redis support).
    pub async fn select_async(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> crate::sam::memory::Result<Vec<Self>> {
        let result = tokio::task::spawn_blocking(move || Self::select(limit, offset, order, query))
            .await
            .map_err(|e| crate::sam::memory::Error::with_chain(e, "JoinError in select_async"))??;
        Ok(result)
    }
    /// Save to DB (insert or update by oid).
    pub fn save(&self) -> crate::sam::memory::Result<Self> {
        let mut client = Config::client()?;
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
    /// Async save (no Redis support).
    pub async fn save_async(&self) -> crate::sam::memory::Result<Self> {
        let this = self.clone();
        match tokio::task::spawn_blocking(move || this.save()).await {
            Ok(res) => res,
            Err(e) => Err(crate::sam::memory::Error::with_chain(e, "JoinError in save_async")),
        }
    }


    /// Async destroy by oid (removes from DB only).
    pub async fn destroy_async(oid: String) -> crate::sam::memory::Result<bool> {
        let config = Config::new();

        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

        // Construct the connection string
        let conn_str = format!(
            "postgresql://{}:{}@{}/{}?sslmode=prefer",
            config.postgres.username,
            config.postgres.password,
            config.postgres.address,
            config.postgres.db_name
        );

        // Connect and return the client
        let (pg_client, connection) = tokio_postgres::connect(&conn_str, connector).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });

        // Remove from Postgres
        let table = Self::sql_table_name();
        let query = format!("DELETE FROM {} WHERE oid = $1", table);
       
        // let pg_client = crate::sam::memory::Config::client_async().await.unwrap();
        // Spawn the connection to drive it
        // tokio::spawn(connection);
        let rows = pg_client.execute(&query, &[&oid]).await.map_err(|e| crate::sam::memory::Error::with_chain(e, "Failed to delete crawl job"))?;
        Ok(rows > 0)
    }


    /// Destroy by oid.
    pub fn destroy(oid: String) -> crate::sam::memory::Result<bool> {
        Config::destroy_row(oid, Self::sql_table_name())
    }
}