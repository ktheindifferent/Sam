//! Service configuration module
//!
//! Provides synchronous and asynchronous methods for interacting with service configurations in a PostgreSQL database.

use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;

/// Represents a key-value setting for a service.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceSetting {
    /// The tag or key for the setting.
    pub tag: String,
    /// The value for the setting.
    pub value: String,
}

/// Represents a service configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Service identifier (e.g., name or type).
    pub identifier: String,
    /// Service key (API key or similar).
    pub key: String,
    /// Service secret (API secret or similar).
    pub secret: String,
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
    /// Endpoint URL or address.
    pub endpoint: String,
    /// List of additional settings for the service.
    pub settings: Vec<ServiceSetting>,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64
}

impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}

impl Service {
    /// Creates a new Service with a random OID and current timestamps.
    pub fn new() -> Service {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Service { 
            id: 0,
            oid,
            identifier: String::new(),
            key: String::new(),
            secret: String::new(),
            username: String::new(),
            password: String::new(),
            endpoint: String::new(),
            settings: Vec::new(),
            created_at: now,
            updated_at: now
        }
    }

    /// Returns a list of SQL migration statements for the config_services table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.config_services ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.config_services ADD COLUMN updated_at BIGINT NULL;",
            "ALTER TABLE public.config_services ADD COLUMN username varchar NULL;",
            "ALTER TABLE public.config_services ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.config_services ADD COLUMN settings varchar NULL;",
        ]
    }

    /// Returns the SQL table name for the config_services table.
    pub fn sql_table_name() -> String {
        "config_services".to_string()
    }

    /// Returns the SQL statement to create the config_services table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.config_services (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            identifier varchar NULL,
            key varchar NULL,
            secret varchar NULL,
            endpoint varchar NULL,
            settings varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT services_pkey PRIMARY KEY (id));"
    }

    /// Saves the Service to the database. Updates if OID or identifier exists, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;
        // Search for OID or identifier matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.identifier.clone()));
        pg_query.query_columns.push(" OR identifier =".to_string());
        let rows = Self::select(None, None, None, Some(pg_query)).unwrap();
        let settings = serde_json::to_string(&self.settings).unwrap();
        if rows.is_empty() {
            client.execute("INSERT INTO config_services (oid, identifier, key, secret, username, password, endpoint, settings, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[&self.oid.clone(),
                &self.identifier,
                &self.key,
                &self.secret,
                &self.username,
                &self.password,
                &self.endpoint,
                &settings,
                &self.created_at,
                &self.updated_at]
            ).unwrap();
            Ok(self)
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE config_services SET key = $1, secret = $2, settings = $3, updated_at = $4 WHERE oid = $5;",
                &[
                    &self.key,
                    &self.secret,
                    &settings,
                    &(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64),
                    &ads.oid
                ])?;
            Ok(self)
        }
    }

    /// Selects Service entries from the database with optional limit, offset, order, and query.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Constructs a Service from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut settings: Vec<ServiceSetting> = Vec::new();
        if let Some(settings_str) = row.get("settings") {
            settings = serde_json::from_str(settings_str).unwrap();  
        }
        Ok(Self {
            id: row.get("id"),
            oid:  row.get("oid"),
            identifier: row.get("identifier"),
            key: row.get("key"),
            secret: row.get("secret"),
            username: row.get("username"),
            password: row.get("password"),
            endpoint: row.get("endpoint"),
            settings,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Deletes a Service from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "config_services".to_string())
    }

    /// Asynchronously saves the Service to the database. Updates if OID or identifier exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.identifier.clone()));
        pg_query.query_columns.push(" OR identifier =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query)).await?;
        let settings = serde_json::to_string(&self.settings).unwrap();
        if rows.is_empty() {
            client.execute("INSERT INTO config_services (oid, identifier, key, secret, username, password, endpoint, settings, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[&self.oid.clone(),
                &self.identifier,
                &self.key,
                &self.secret,
                &self.username,
                &self.password,
                &self.endpoint,
                &settings,
                &self.created_at,
                &self.updated_at]
            ).await?;
            Ok(self)
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE config_services SET key = $1, secret = $2, settings = $3, updated_at = $4 WHERE oid = $5;",
                &[
                    &self.key,
                    &self.secret,
                    &settings,
                    &(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64),
                    &ads.oid
                ]).await?;
            Ok(self)
        }
    }

    /// Asynchronously selects Service entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a Service from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a Service from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "config_services".to_string()).await
    }
}