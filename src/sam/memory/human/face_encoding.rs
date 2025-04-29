//! FaceEncoding module
//!
//! Provides synchronous and asynchronous methods for interacting with human face encodings in a PostgreSQL database.

use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

/// Represents a face encoding for a human.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaceEncoding {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// The face encoding as a byte vector.
    pub encoding: Vec<u8>,
    /// The OID of the associated human.
    pub human_oid: String,
    /// Timestamp of the encoding (seconds since UNIX_EPOCH).
    pub timestamp: i64,
}

impl Default for FaceEncoding {
    fn default() -> Self {
        Self::new()
    }
}

impl FaceEncoding {
    /// Creates a new FaceEncoding with a random OID and current timestamp.
    pub fn new() -> FaceEncoding {
        let oid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        let encoding: Vec<u8> = Vec::new();
        FaceEncoding {
            id: 0,
            oid,
            encoding,
            human_oid: String::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    /// Returns the SQL table name for the face encodings table.
    pub fn sql_table_name() -> String {
        "human_face_encodings".to_string()
    }

    /// Returns the SQL statement to create the face encodings table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.human_face_encodings (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            encoding bytea NULL,
            human_oid varchar NULL,
            timestamp BIGINT NULL,
            CONSTRAINT human_face_encodings_pkey PRIMARY KEY (id));"
    }

    /// Returns a list of SQL migration statements for the face encodings table.
    pub fn migrations() -> Vec<&'static str> {
        vec!["ALTER TABLE public.human_face_encodings ADD COLUMN timestamp BIGINT NULL;"]
    }

    /// Saves the FaceEncoding to the database. Updates if OID exists, inserts otherwise.
    pub fn save(object: Self) -> Result<Self> {
        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(object.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        // Search for OID matches
        let rows = Self::select(None, None, None, Some(pg_query.clone())).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO human_face_encodings (oid, encoding, human_oid, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.encoding,
                &object.human_oid,
                &object.timestamp]
            ).unwrap();

            let rows_two = Self::select(None, None, None, Some(pg_query)).unwrap();

            return Ok(rows_two[0].clone());
        }

        Ok(object)
    }

    /// Selects FaceEncoding entries from the database with optional limit, offset, order, and query.
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

    /// Constructs a FaceEncoding from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            encoding: row.get("encoding"),
            human_oid: row.get("human_oid"),
            timestamp: row.get("timestamp"),
        })
    }

    /// Deletes a FaceEncoding from the database by OID.
    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, "human_face_encodings".to_string())
    }

    /// Asynchronously saves the FaceEncoding to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(object: Self) -> Result<Self> {
        let client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(object.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        if rows.is_empty() {
            client.execute("INSERT INTO human_face_encodings (oid, encoding, human_oid, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.encoding,
                &object.human_oid,
                &object.timestamp]
            ).await?;
            let rows_two = Self::select_async(None, None, None, Some(pg_query)).await?;
            Ok(rows_two[0].clone())
        } else {
            Ok(object)
        }
    }

    /// Asynchronously selects FaceEncoding entries from the database with optional limit, offset, order, and query.
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

    /// Asynchronously constructs a FaceEncoding from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    /// Asynchronously deletes a FaceEncoding from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "human_face_encodings".to_string()).await
    }
}
