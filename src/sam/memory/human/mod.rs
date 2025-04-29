//! Human module
//!
//! Provides synchronous and asynchronous methods for interacting with human records in a PostgreSQL database.

use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

pub mod face_encoding;
pub mod notification;

pub use face_encoding::*;
pub use notification::*;

/// Represents a human user in the system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Human {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Human's name.
    pub name: String,
    /// Optional email address.
    pub email: Option<String>,
    /// Optional password hash.
    pub password: Option<String>,
    /// Optional phone number.
    pub phone_number: Option<String>,
    /// Number of times heard.
    pub heard_count: i64,
    /// Number of times seen.
    pub seen_count: i64,
    /// Authorization level.
    pub authorization_level: i64,
    /// Creation timestamp (seconds since UNIX_EPOCH).
    pub created_at: i64,
    /// Last update timestamp (seconds since UNIX_EPOCH).
    pub updated_at: i64,
}

impl Default for Human {
    fn default() -> Self {
        Self::new()
    }
}

impl Human {
    /// Creates a new Human with a random OID and current timestamps.
    pub fn new() -> Human {
        let oid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        Human {
            id: 0,
            oid: oid.clone(),
            name: format!("unknown-{oid}"),
            email: None,
            password: None,
            phone_number: None,
            heard_count: 0,
            seen_count: 0,
            authorization_level: 0,
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

    /// Returns the SQL table name for the humans table.
    pub fn sql_table_name() -> String {
        "humans".to_string()
    }

    /// Returns the SQL statement to create the humans table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.humans (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            email varchar NULL,
            password varchar NULL,
            phone_number varchar NULL,
            heard_count BIGINT NULL,
            seen_count BIGINT NULL,
            authorization_level BIGINT NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT humans_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the humans table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.humans ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.humans ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.humans ADD COLUMN updated_at BIGINT NULL;",
        ]
    }

    /// Returns the number of human records in the database.
    pub fn count() -> Result<i64> {
        let mut client = Config::client()?;
        let execquery = format!("SELECT COUNT(*) FROM {}", Self::sql_table_name());
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

    /// Saves the Human to the database. Updates if OID exists, inserts otherwise.
    pub fn save(&self) -> Result<&Self> {
        let mut client = Config::client()?;
        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        // Search for OID matches
        let rows = Self::select(None, None, None, Some(pg_query.clone())).unwrap();
        if rows.is_empty() {
            client.execute("INSERT INTO humans (oid, name, heard_count, seen_count, authorization_level, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[  &self.oid.clone(),
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.created_at,
                    &self.updated_at
                ]
            ).unwrap();
            if self.phone_number.is_some() {
                client.execute(
                    "UPDATE humans SET phone_number = $1 WHERE oid = $2;",
                    &[&self.phone_number.clone().unwrap(), &self.oid],
                )?;
            }
            if self.email.is_some() {
                client.execute(
                    "UPDATE humans SET email = $1 WHERE oid = $2;",
                    &[&self.email.clone().unwrap(), &self.oid],
                )?;
            }
            if self.password.is_some() {
                client.execute(
                    "UPDATE humans SET password = $1 WHERE oid = $2;",
                    &[&self.password.clone().unwrap(), &self.oid],
                )?;
            }
            Ok(self)
        } else {
            // TODO Impliment Update
            let ads = rows[0].clone();
            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE humans SET name = $1, heard_count = $2, seen_count = $3, authorization_level = $4, updated_at = $5 WHERE oid = $6;", 
                &[
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.updated_at,
                    &ads.oid
                ])?;
                if self.phone_number.is_some() {
                    client.execute(
                        "UPDATE humans SET phone_number = $1 WHERE oid = $2;",
                        &[&self.phone_number.clone().unwrap(), &ads.oid],
                    )?;
                }
                if self.email.is_some() {
                    client.execute(
                        "UPDATE humans SET email = $1 WHERE oid = $2;",
                        &[&self.email.clone().unwrap(), &ads.oid],
                    )?;
                }
                if self.password.is_some() {
                    client.execute(
                        "UPDATE humans SET password = $1 WHERE oid = $2;",
                        &[&self.password.clone().unwrap(), &self.oid],
                    )?;
                }
            }
            Ok(self)
        }
    }

    /// Selects Human entries from the database with optional limit, offset, order, and query.
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

    /// Constructs a Human from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let sql_email: Option<String> = row.get("email");
        let sql_password: Option<String> = row.get("password");
        let sql_phone_number: Option<String> = row.get("phone_number");
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"),
            email: sql_email,
            password: sql_password,
            phone_number: sql_phone_number,
            heard_count: row.get("heard_count"),
            seen_count: row.get("seen_count"),
            authorization_level: row.get("authorization_level"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Deletes a Human from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "humans".to_string())
    }

    /// Asynchronously returns the number of human records in the database.
    pub async fn count_async() -> Result<i64> {
        let client = Config::client_async().await?;
        let execquery = format!("SELECT COUNT(*) FROM {}", Self::sql_table_name());
        let rows = client.query(execquery.as_str(), &[]).await?;
        let counter: i64 = rows[0].get("count");
        Ok(counter)
    }

    /// Asynchronously saves the Human to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<&Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO humans (oid, name, heard_count, seen_count, authorization_level, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[  &self.oid.clone(),
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.created_at,
                    &self.updated_at
                ]
            ).await?;
            if self.phone_number.is_some() {
                client
                    .execute(
                        "UPDATE humans SET phone_number = $1 WHERE oid = $2;",
                        &[&self.phone_number.clone().unwrap(), &self.oid],
                    )
                    .await?;
            }
            if self.email.is_some() {
                client
                    .execute(
                        "UPDATE humans SET email = $1 WHERE oid = $2;",
                        &[&self.email.clone().unwrap(), &self.oid],
                    )
                    .await?;
            }
            if self.password.is_some() {
                client
                    .execute(
                        "UPDATE humans SET password = $1 WHERE oid = $2;",
                        &[&self.password.clone().unwrap(), &self.oid],
                    )
                    .await?;
            }
            Ok(self)
        } else {
            let ads = rows[0].clone();
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE humans SET name = $1, heard_count = $2, seen_count = $3, authorization_level = $4, updated_at = $5 WHERE oid = $6;",
                &[
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.updated_at,
                    &ads.oid
                ]).await?;
                if self.phone_number.is_some() {
                    client
                        .execute(
                            "UPDATE humans SET phone_number = $1 WHERE oid = $2;",
                            &[&self.phone_number.clone().unwrap(), &ads.oid],
                        )
                        .await?;
                }
                if self.email.is_some() {
                    client
                        .execute(
                            "UPDATE humans SET email = $1 WHERE oid = $2;",
                            &[&self.email.clone().unwrap(), &ads.oid],
                        )
                        .await?;
                }
                if self.password.is_some() {
                    client
                        .execute(
                            "UPDATE humans SET password = $1 WHERE oid = $2;",
                            &[&self.password.clone().unwrap(), &self.oid],
                        )
                        .await?;
                }
            }
            Ok(self)
        }
    }

    /// Asynchronously selects Human entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a Human from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        Self::from_row(row)
    }

    /// Asynchronously deletes a Human from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "humans".to_string()).await
    }
}
