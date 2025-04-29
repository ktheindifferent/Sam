//! WikipediaSummary cache module
//!
//! Provides synchronous and asynchronous methods for interacting with cached Wikipedia summaries in a PostgreSQL database.

use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;

/// Represents a cached Wikipedia summary.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WikipediaSummary {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// List of topics associated with the summary.
    pub topics: Vec<String>,
    /// The summary text.
    pub summary: String,
    /// Timestamp of the cache entry (seconds since UNIX_EPOCH).
    pub timestamp: i64
}

impl Default for WikipediaSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaSummary {
    /// Creates a new WikipediaSummary with a random OID and current timestamp.
    pub fn new() -> WikipediaSummary {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let topics: Vec<String> = Vec::new();
        WikipediaSummary { 
            id: 0,
            oid,
            topics,
            summary: String::new(),
            timestamp
        }
    }

    /// Returns the SQL table name for the cache.
    pub fn sql_table_name() -> String {
        "cache_wikipedia_summaries".to_string()
    }

    /// Returns the SQL statement to create the cache table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.cache_wikipedia_summaries (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            topics varchar NULL,
            summary varchar NULL,
            timestamp BIGINT DEFAULT 0,
            CONSTRAINT cache_wikipedia_summaries_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the cache table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "",
        ]
    }

    /// Saves the WikipediaSummary to the database. Updates if OID exists, inserts otherwise.
    pub fn save(object: Self) -> Result<Self> {
        let mut client = Config::client()?;
        
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(object.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        // Search for OID matches
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query.clone())
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO cache_wikipedia_summaries (oid, topics, summary, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.topics.join(","),
                &object.summary,
                &object.timestamp]
            ).unwrap();
            let rows_two = Self::select(
                None, 
                None, 
                None, 
                Some(pg_query)
            ).unwrap();
            Ok(rows_two[0].clone())
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE cache_wikipedia_summaries SET topics = $1, summary = $2, timestamp = $3 WHERE oid = $4;", 
                &[&object.topics.join(","),
                &object.summary,
                &object.timestamp,
                &ads.oid])?;
            let statement_two = client.prepare("SELECT * FROM cache_wikipedia_summaries WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[
                &object.oid, 
            ])?;
            Self::from_row(&rows_two[0])
        }
    }

    /// Selects WikipediaSummary entries from the database with optional limit, offset, order, and query.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Constructs a WikipediaSummary from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut topics: Vec<String> = Vec::new();
        let sql_topics: Option<String> = row.get("topics");
        if let Some(ts) = sql_topics {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            topics = newvec;
        }
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            topics, 
            summary: row.get("summary"),
            timestamp: row.get("timestamp"),
        })
    }

    /// Deletes a WikipediaSummary from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "cache_wikipedia_summaries".to_string())
    }

    /// Asynchronously saves the WikipediaSummary to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(object: Self) -> Result<Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(object.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        // Search for OID matches
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO cache_wikipedia_summaries (oid, topics, summary, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.topics.join(","),
                &object.summary,
                &object.timestamp]
            ).await?;
            let rows_two = Self::select_async(None, None, None, Some(pg_query)).await?;
            Ok(rows_two[0].clone())
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE cache_wikipedia_summaries SET topics = $1, summary = $2, timestamp = $3 WHERE oid = $4;",
                &[&object.topics.join(","),
                &object.summary,
                &object.timestamp,
                &ads.oid]).await?;
            let statement_two = client.prepare("SELECT * FROM cache_wikipedia_summaries WHERE oid = $1").await?;
            let rows_two = client.query(&statement_two, &[&object.oid]).await?;
            Self::from_row(&rows_two[0])
        }
    }

    /// Asynchronously selects WikipediaSummary entries from the database with optional limit, offset, order, and query.
    pub async fn select_async(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let config = crate::sam::memory::Config::new();
let client = config.connect_pool().await?;
        let jsons = crate::sam::memory::Config::pg_select_async(Self::sql_table_name(), None, limit, offset, order, query, client).await?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Asynchronously constructs a WikipediaSummary from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a WikipediaSummary from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "cache_wikipedia_summaries".to_string()).await
    }
}