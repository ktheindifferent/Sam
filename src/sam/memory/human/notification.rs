//! Notification module
//!
//! Provides synchronous and asynchronous methods for interacting with human human_notifications in a PostgreSQL database.

use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::Config;
use crate::sam::memory::Result;
use crate::sam::memory::PostgresQueries;
use rand::Rng;

/// Represents a notification for a human user.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notification {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Session or source identifier.
    pub sid: String,
    /// OID of the associated human.
    pub human_oid: String,
    /// Notification message.
    pub message: String,
    /// Whether the notification has been seen.
    pub seen: bool,
    /// Timestamp of the notification (seconds since UNIX_EPOCH).
    pub timestamp: i64
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Notification {
    /// Creates a new Notification with a random OID and current timestamp.
    pub fn new() -> Notification {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Notification { 
            id: 0,
            oid,
            sid: String::new(),
            human_oid: String::new(),
            message: String::new(),
            seen: false,
            timestamp
        }
    }

    /// Returns the SQL table name for the human_notifications table.
    pub fn sql_table_name() -> String {
        "human_notifications".to_string()
    }

    /// Returns the SQL statement to create the human_notifications table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.human_notifications (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            sid varchar NULL,
            human_oid varchar NULL,
            message varchar NULL,
            seen bool DEFAULT false,
            timestamp BIGINT DEFAULT 0,
            CONSTRAINT notifications_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the human_notifications table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "",
        ]
    }

    /// Saves the Notification to the database. Updates if OID exists, inserts otherwise.
    pub fn save(&self) -> Result<Self> {
        let mut client = Config::client()?;
        
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());


        // Search for OID matches
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query.clone())
        ).unwrap();

        if rows.is_empty() {

            client.execute("INSERT INTO human_notifications (oid, sid, human_oid, message, seen, timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.sid,
                &self.human_oid,
                &self.message,
                &self.seen,
                &self.timestamp]
            ).unwrap();

    
            // Search for OID matches
            let rows_two = Self::select(
                None, 
                None, 
                None, 
                Some(pg_query)
            ).unwrap();

            Ok(rows_two[0].clone())
        
        } else {
            let ads = rows[0].clone();


 
            client.execute("UPDATE human_notifications SET message = $1, seen = $2 WHERE oid = $3;", 
            &[&self.message,
            &self.seen,
            &ads.oid])?;


            let statement_two = client.prepare("SELECT * FROM human_notifications WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[
                &self.oid, 
            ])?;

            Self::from_row(&rows_two[0])
        }
    }

    /// Selects Notification entries from the database with optional limit, offset, order, and query.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }

    /// Constructs a Notification from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            sid: row.get("sid"),
            human_oid: row.get("human_oid"),
            message: row.get("message"),
            seen: row.get("seen"),
            timestamp: row.get("timestamp")
        })
    }

    /// Deletes a Notification from the database by OID.
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "human_notifications".to_string())
    }

    /// Asynchronously saves the Notification to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<Self> {
        let mut client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO human_notifications (oid, sid, human_oid, message, seen, timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.sid,
                &self.human_oid,
                &self.message,
                &self.seen,
                &self.timestamp]
            ).await?;
            let rows_two = Self::select_async(None, None, None, Some(pg_query)).await?;
            Ok(rows_two[0].clone())
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE human_notifications SET message = $1, seen = $2 WHERE oid = $3;",
                &[&self.message,
                &self.seen,
                &ads.oid]).await?;
            let statement_two = client.prepare("SELECT * FROM human_notifications WHERE oid = $1").await?;
            let rows_two = client.query(&statement_two, &[&self.oid]).await?;
            Self::from_row(&rows_two[0])
        }
    }

    /// Asynchronously selects Notification entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a Notification from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a Notification from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "human_notifications".to_string()).await
    }
}