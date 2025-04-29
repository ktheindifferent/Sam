//! Setting configuration module
//!
//! Provides synchronous and asynchronous methods for interacting with config_settings in a PostgreSQL database.

use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

/// Represents a key-value setting.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Setting {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// The key for the setting.
    pub key: String,
    /// The values for the setting.
    pub values: Vec<String>,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64,
}

impl Default for Setting {
    fn default() -> Self {
        Self::new()
    }
}

impl Setting {
    /// Creates a new Setting with a random OID and current timestamps.
    pub fn new() -> Setting {
        let oid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        let empty_vec: Vec<String> = Vec::new();
        Setting {
            id: 0,
            oid,
            key: String::new(),
            values: empty_vec,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    /// Returns the SQL table name for the config_settings table.
    pub fn sql_table_name() -> String {
        "config_settings".to_string()
    }

    /// Returns the SQL statement to create the config_settings table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.config_settings (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            key varchar NULL,
            values varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT settings_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the config_settings table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.config_settings ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.config_settings ADD COLUMN updated_at BIGINT NULL;",
        ]
    }

    /// Saves the Setting to the database. Updates if OID or key exists, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.key.clone()));
        pg_query.query_columns.push(" OR key =".to_string());
        let rows = Self::select(None, None, None, Some(pg_query)).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO config_settings (oid, key, values, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                &[&self.oid.clone(),
                &self.key,
                &self.values.join(","),
                &self.created_at,
                &self.updated_at]
            )?;
            Ok(self)
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE config_settings SET key = $1, values = $2, updated_at = $3 WHERE oid = $4;", 
                &[
                    &self.key,
                    &self.values.join(","),
                    &self.updated_at,
                    &ads.oid
                ])?;
            }
            Ok(self)
        }
    }

    /// Selects Setting entries from the database with optional limit, offset, order, and query.
    pub fn select(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(
            Self::sql_table_name(),
            None,
            limit,
            offset,
            order,
            query,
            None,
        )?;

        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Constructs a Setting from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut values: Vec<String> = Vec::new();
        let sql_values: Option<String> = row.get("values");
        if let Some(ts) = sql_values {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec {
                newvec.push(v.to_string());
            }
            values = newvec;
        }
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            key: row.get("key"),
            values,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Deletes a Setting from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "config_settings".to_string())
    }

    /// Asynchronously saves the Setting to the database. Updates if OID or key exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.key.clone()));
        pg_query.query_columns.push(" OR key =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query)).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO config_settings (oid, key, values, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                &[&self.oid.clone(),
                &self.key,
                &self.values.join(","),
                &self.created_at,
                &self.updated_at]
            ).await?;
            Ok(self)
        } else {
            let ads = rows[0].clone();
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE config_settings SET key = $1, values = $2, updated_at = $3 WHERE oid = $4;",
                &[
                    &self.key,
                    &self.values.join(","),
                    &self.updated_at,
                    &ads.oid
                ]).await?;
            }
            Ok(self)
        }
    }

    /// Asynchronously selects Setting entries from the database with optional limit, offset, order, and query.
    pub async fn select_async(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
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
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Asynchronously constructs a Setting from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a Setting from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "config_settings".to_string()).await
    }
}
