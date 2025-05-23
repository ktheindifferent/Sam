//! Location module
//!
//! Provides synchronous and asynchronous methods for interacting with location records in a PostgreSQL database.

use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

/// Represents a location in the system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Name of the location.
    pub name: String,
    /// Address of the location.
    pub address: String,
    /// City of the location.
    pub city: String,
    /// State of the location.
    pub state: String,
    /// Zip code of the location.
    pub zip_code: String,
    /// Optional LIFX API key for the location.
    pub lifx_api_key: Option<String>,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64,
}

impl Default for Location {
    fn default() -> Self {
        Self::new()
    }
}

impl Location {
    /// Creates a new Location with a random OID and current timestamps.
    pub fn new() -> Location {
        let oid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        Location {
            id: 0,
            oid,
            name: String::new(),
            address: String::new(),
            city: String::new(),
            state: String::new(),
            zip_code: String::new(),
            lifx_api_key: None,
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

    /// Returns the SQL table name for the locations table.
    pub fn sql_table_name() -> String {
        "locations".to_string()
    }

    /// Returns the SQL statement to create the locations table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.locations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            address varchar NULL,
            city varchar NULL,
            state varchar NULL,
            zip_code varchar NULL,
            lifx_api_key varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT locations_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the locations table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.locations ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.locations ADD COLUMN updated_at BIGINT NULL;",
            "ALTER TABLE public.locations ADD COLUMN lifx_api_key VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN city VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN state VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN zip_code VARCHAR NULL;",
        ]
    }

    /// Returns the number of location records in the database.
    pub fn count() -> Result<i64> {
        let mut client = Config::client()?;
        let execquery = format!(
            "SELECT COUNT(*)
        FROM {}",
            Self::sql_table_name()
        );
        let mut counter: i64 = 0;
        for row in client.query(execquery.as_str(), &[])? {
            counter = row.get("count");
        }
        match client.close() {
            Ok(_) => {}
            Err(e) => log::error!("failed to close connection to database: {}", e),
        }
        Ok(counter)
    }

    /// Saves the Location to the database. Updates if OID exists, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;
        // Search for OID matches
        let statement =
            client.prepare("SELECT * FROM locations WHERE oid = $1 OR name ilike $2")?;
        let rows = client.query(&statement, &[&self.oid, &self.name])?;
        if rows.is_empty() {
            client.execute("INSERT INTO locations (oid, name, address, city, state, zip_code, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);",
                &[&self.oid.clone(),
                &self.name,
                &self.address,
                &self.city,
                &self.state,
                &self.zip_code,
                &self.created_at,
                &self.updated_at]
            ).unwrap();
            if self.lifx_api_key.is_some() {
                client.execute(
                    "UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;",
                    &[&self.lifx_api_key.clone().unwrap(), &self.oid],
                )?;
            }
            let statement = client.prepare("SELECT * FROM locations WHERE oid = $1")?;
            let _rows_two = client.query(&statement, &[&self.oid])?;
            Ok(self)
        } else {
            let ads = Self::from_row(&rows[0]).unwrap();
            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE locations SET name = $1, address = $2, city = $3, state = $4, zip_code = $5, updated_at = $6 WHERE oid = $7;", 
                &[
                    &self.name,
                    &self.address,
                    &self.city,
                    &self.state,
                    &self.zip_code,
                    &self.updated_at,
                    &ads.oid
                ])?;
                if self.lifx_api_key.is_some() {
                    client.execute(
                        "UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;",
                        &[&self.lifx_api_key.clone().unwrap(), &ads.oid],
                    )?;
                }
            }
            let statement_two = client.prepare("SELECT * FROM locations WHERE oid = $1")?;
            let _rows_two = client.query(&statement_two, &[&self.oid])?;
            Ok(self)
        }
    }

    /// Selects Location entries from the database with optional limit, offset, order, and query.
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

    /// Constructs a Location from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"),
            address: row.get("address"),
            city: row.get("city"),
            state: row.get("state"),
            zip_code: row.get("zip_code"),
            lifx_api_key: row.get("lifx_api_key"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Deletes a Location from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "locations".to_string())
    }

    /// Asynchronously returns the number of location records in the database.
    pub async fn count_async() -> Result<i64> {
        let client = Config::client_async().await?;
        let execquery = format!("SELECT COUNT(*) FROM {}", Self::sql_table_name());
        let rows = client.query(execquery.as_str(), &[]).await?;
        let counter: i64 = rows[0].get("count");
        Ok(counter)
    }

    /// Asynchronously saves the Location to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let client = Config::client_async().await?;
        let statement = client
            .prepare("SELECT * FROM locations WHERE oid = $1 OR name ilike $2")
            .await?;
        let rows = client.query(&statement, &[&self.oid, &self.name]).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO locations (oid, name, address, city, state, zip_code, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);",
                &[&self.oid.clone(),
                &self.name,
                &self.address,
                &self.city,
                &self.state,
                &self.zip_code,
                &self.created_at,
                &self.updated_at]
            ).await?;
            if self.lifx_api_key.is_some() {
                client
                    .execute(
                        "UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;",
                        &[&self.lifx_api_key.clone().unwrap(), &self.oid],
                    )
                    .await?;
            }
            Ok(self)
        } else {
            let ads = Self::from_row(&rows[0]).unwrap();
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE locations SET name = $1, address = $2, city = $3, state = $4, zip_code = $5, updated_at = $6 WHERE oid = $7;",
                &[
                    &self.name,
                    &self.address,
                    &self.city,
                    &self.state,
                    &self.zip_code,
                    &self.updated_at,
                    &ads.oid
                ]).await?;
                if self.lifx_api_key.is_some() {
                    client
                        .execute(
                            "UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;",
                            &[&self.lifx_api_key.clone().unwrap(), &ads.oid],
                        )
                        .await?;
                }
            }
            Ok(self)
        }
    }

    /// Asynchronously selects Location entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a Location from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        Self::from_row(row)
    }

    /// Asynchronously deletes a Location from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "locations".to_string()).await
    }
}
