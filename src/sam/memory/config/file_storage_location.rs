//! FileStorageLocation configuration module
//!
//! Provides synchronous and asynchronous methods for interacting with file storage location configurations in a PostgreSQL database.

use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;

/// Represents a file storage location configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileStorageLocation {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Storage type (e.g., S3, local, etc.).
    pub storage_type: String,
    /// Endpoint URL or path.
    pub endpoint: String,
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64
}

impl Default for FileStorageLocation {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorageLocation {
    /// Creates a new FileStorageLocation with a random OID and current timestamps.
    pub fn new() -> FileStorageLocation {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        FileStorageLocation { 
            id: 0,
            oid,
            storage_type: String::new(), 
            endpoint: String::new(), 
            username: String::new(), 
            password: String::new(), 
            created_at: now,
            updated_at: now
        }
    }

    /// Returns the SQL table name for the file storage locations.
    pub fn sql_table_name() -> String {
        "config_file_storage_locations".to_string()
    }

    /// Returns the SQL statement to create the file storage locations table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.config_file_storage_locations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            storage_type varchar NULL,
            endpoint varchar NULL,
            username varchar NULL,
            password varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT config_file_storage_locations_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the file storage locations table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.config_file_storage_locations ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.config_file_storage_locations ADD COLUMN updated_at BIGINT NULL;"
        ]
    }

    /// Saves the FileStorageLocation to the database. Updates if OID exists and is older, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query)
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO config_file_storage_locations (oid, storage_type, endpoint, username, password, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[&self.oid.clone(),
                &self.storage_type,
                &self.endpoint,
                &self.username,
                &self.password,
                &self.created_at,
                &self.updated_at]
            )?;        
            Ok(self)
        
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE config_file_storage_locations SET storage_type = $1, endpoint = $2, username = $3, password = $4, updated_at = $5 WHERE oid = $6;", 
                &[
                    &self.storage_type,
                    &self.endpoint,
                    &self.username,
                    &self.password,
                    &self.updated_at,
                    &ads.oid
                ])?;
            }
            Ok(self)
        }
    }

    /// Selects FileStorageLocation entries from the database with optional limit, offset, order, and query.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Constructs a FileStorageLocation from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            storage_type: row.get("storage_type"), 
            endpoint: row.get("endpoint"), 
            username: row.get("username"), 
            password: row.get("password"), 
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }

    /// Deletes a FileStorageLocation from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "config_file_storage_locations".to_string())
    }

    /// Asynchronously saves the FileStorageLocation to the database. Updates if OID exists and is older, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let mut client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query)).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO config_file_storage_locations (oid, storage_type, endpoint, username, password, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[&self.oid.clone(),
                &self.storage_type,
                &self.endpoint,
                &self.username,
                &self.password,
                &self.created_at,
                &self.updated_at]
            ).await?;
            Ok(self)
        } else {
            let ads = rows[0].clone();
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE config_file_storage_locations SET storage_type = $1, endpoint = $2, username = $3, password = $4, updated_at = $5 WHERE oid = $6;",
                &[
                    &self.storage_type,
                    &self.endpoint,
                    &self.username,
                    &self.password,
                    &self.updated_at,
                    &ads.oid
                ]).await?;
            }
            Ok(self)
        }
    }

    /// Asynchronously selects FileStorageLocation entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a FileStorageLocation from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a FileStorageLocation from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "config_file_storage_locations".to_string()).await
    }
}