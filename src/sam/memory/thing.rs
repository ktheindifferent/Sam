//! Thing module
//!
//! Provides synchronous and asynchronous methods for interacting with thing/device records in a PostgreSQL database.

use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

/// Represents a device or thing in the system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thing {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Name of the thing.
    pub name: String,
    /// OID of the associated room.
    pub room_oid: String,
    /// Type of the thing (e.g., lifx, rtsp, etc).
    pub thing_type: String,
    /// Username for the thing (if applicable).
    pub username: String,
    /// Password for the thing (if applicable).
    pub password: String,
    /// IP address of the thing.
    pub ip_address: String,
    /// Online identifiers for the thing.
    pub online_identifiers: Vec<String>,
    /// Local identifiers for the thing.
    pub local_identifiers: Vec<String>,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64,
}

impl Default for Thing {
    fn default() -> Self {
        Self::new()
    }
}

impl Thing {
    /// Creates a new Thing with a random OID and current timestamps.
    pub fn new() -> Thing {
        let oid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        let empty_vec: Vec<String> = Vec::new();
        Thing {
            id: 0,
            oid,
            name: String::new(),
            room_oid: String::new(),
            thing_type: String::new(),
            username: String::new(),
            password: String::new(),
            ip_address: String::new(),
            online_identifiers: empty_vec.clone(),
            local_identifiers: empty_vec.clone(),
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

    /// Returns the SQL table name for the things table.
    pub fn sql_table_name() -> String {
        "things".to_string()
    }

    /// Returns the SQL statement to create the things table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.things (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            room_oid varchar NULL,
            thing_type varchar NULL,
            username varchar NULL,
            password varchar NULL,
            ip_address varchar NULL,
            online_identifiers varchar NULL,
            local_identifiers varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT things_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the things table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.things ADD COLUMN username varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN ip_address varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.things ADD COLUMN updated_at BIGINT NULL;",
        ]
    }

    /// Saves the Thing to the database. Updates if OID exists, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        let rows = Self::select(None, None, None, Some(pg_query)).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO things (oid, name, room_oid, thing_type, username, password, ip_address, online_identifiers, local_identifiers, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                &[&self.oid.clone(),
                &self.name,
                &self.room_oid,
                &self.thing_type,
                &self.username,
                &self.password,
                &self.ip_address,
                &self.online_identifiers.join(","),
                &self.local_identifiers.join(","),
                &self.created_at,
                &self.updated_at]
            )?;
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE things SET name = $1, room_oid = $2, thing_type = $3, username = $4, password = $5, ip_address = $6, online_identifiers = $7, local_identifiers = $8 WHERE oid = $9;", 
                &[
                    &self.name,
                    &self.room_oid,
                    &self.thing_type,
                    &self.username,
                    &self.password,
                    &self.ip_address,
                    &self.online_identifiers.join(","),
                    &self.local_identifiers.join(","),
                    &ads.oid
                ])?;
            }
        }
        Ok(self)
    }

    /// Selects Thing entries from the database with optional limit, offset, order, and query.
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

    /// Constructs a Thing from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut online_identifiers: Vec<String> = Vec::new();
        let sql_online_identifiers: Option<String> = row.get("online_identifiers");
        if let Some(ts) = sql_online_identifiers {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec {
                newvec.push(v.to_string());
            }
            online_identifiers = newvec;
        }
        let mut local_identifiers: Vec<String> = Vec::new();
        let sql_local_identifiers: Option<String> = row.get("local_identifiers");
        if let Some(ts) = sql_local_identifiers {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec {
                newvec.push(v.to_string());
            }
            local_identifiers = newvec;
        }
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"),
            room_oid: row.get("room_oid"),
            thing_type: row.get("thing_type"),
            username: row.get("username"),
            password: row.get("password"),
            ip_address: row.get("ip_address"),
            online_identifiers,
            local_identifiers,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Deletes a Thing from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "things".to_string())
    }

    /// Asynchronously saves the Thing to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO things (oid, name, room_oid, thing_type, username, password, ip_address, online_identifiers, local_identifiers, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                &[&self.oid.clone(),
                &self.name,
                &self.room_oid,
                &self.thing_type,
                &self.username,
                &self.password,
                &self.ip_address,
                &self.online_identifiers.join(","),
                &self.local_identifiers.join(","),
                &self.created_at,
                &self.updated_at]
            ).await?;
        } else {
            let ads = rows[0].clone();
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE things SET name = $1, room_oid = $2, thing_type = $3, username = $4, password = $5, ip_address = $6, online_identifiers = $7, local_identifiers = $8 WHERE oid = $9;",
                &[
                    &self.name,
                    &self.room_oid,
                    &self.thing_type,
                    &self.username,
                    &self.password,
                    &self.ip_address,
                    &self.online_identifiers.join(","),
                    &self.local_identifiers.join(","),
                    &ads.oid
                ]).await?;
            }
        }
        Ok(self)
    }

    /// Asynchronously selects Thing entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a Thing from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        Self::from_row(row)
    }

    /// Asynchronously deletes a Thing from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "things".to_string()).await
    }
}
