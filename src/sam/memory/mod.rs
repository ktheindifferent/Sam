pub mod config;
pub mod human;
pub mod location;
pub mod observation;
pub mod thing;
// pub mod service;
// pub mod file_storage;
// pub mod notification;
pub mod room;
// pub mod setting;
// pub mod storage_location;
// pub mod web_sessions;
// pub mod cached_wikipedia_summary;
// pub mod human_face_encoding;
pub mod cache;
pub mod storage;
pub mod observation_objects;

// Re-export types for convenience
pub use config::Config;
pub use human::Human;
pub use location::*;
pub use observation::*;
pub use observation_objects::ObservationObjects;
pub use thing::*;
// pub use service::*;
// pub use file_storage::*;
// pub use notification::*;
pub use room::*;
// pub use setting::*;
// pub use storage_location::*;
// pub use web_sessions::*;
// pub use cached_wikipedia_summary::*;
// pub use human_face_encoding::*;

// ===== Shared error_chain! block =====
// Remove error_chain and use thiserror/anyhow
use thiserror::Error;
pub type Result<T> = anyhow::Result<T>;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Tokio Postgres error: {0}")]
    TokioPg(#[from] tokio_postgres::Error),
    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),
    #[error("Post input error: {0}")]
    PostError(#[from] rouille::input::post::PostError),
    #[error("Parse float error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Deadpool Postgres error: {0}")]
    DeadpoolPostgresError(#[from] deadpool_postgres::PoolError),
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Error = MemoryError;

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

// ===== Shared utility types and enums =====

use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

// PostgresServer config struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresServer {
    pub db_name: String,
    pub username: String,
    pub password: String,
    pub address: String,
}
impl Default for PostgresServer {
    fn default() -> Self {
        Self::new()
    }
}
impl PostgresServer {
    pub fn new() -> PostgresServer {
        let db_name = env::var("PG_DBNAME").expect("$PG_DBNAME is not set");
        let username = env::var("PG_USER").expect("$PG_USER is not set");
        let password = env::var("PG_PASS").expect("$PG_PASS is not set");
        let address = env::var("PG_ADDRESS").expect("$PG_ADDRESS is not set");
        PostgresServer {
            db_name,
            username,
            password,
            address,
        }
    }
}

// Not tracked in SQL
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PostgresQueries {
    pub queries: Vec<PGCol>,
    pub query_columns: Vec<String>,
    pub append: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PGCol {
    String(String),
    Number(i32),
    Boolean(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeepVisionResult {
    pub id: String,
    pub whoio: Option<WhoioResult>,
    pub probability: f64,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WhoioResult {
    pub id: String,
    pub directory: String,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ObservationType {
    UNKNOWN,
    SEEN,
    HEARD,
    Motion,
    Object,
}
impl fmt::Display for ObservationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::str::FromStr for ObservationType {
    type Err = String;
    fn from_str(input: &str) -> std::result::Result<ObservationType, Self::Err> {
        match input {
            "UNKNOWN" => Ok(ObservationType::UNKNOWN),
            "SEEN" => Ok(ObservationType::SEEN),
            "HEARD" => Ok(ObservationType::HEARD),
            "Motion" => Ok(ObservationType::Motion),
            "Object" => Ok(ObservationType::Object),
            _ => Err(format!("Unknown observation type: {}", input)),
        }
    }
}

// ObservationObjects moved to observation_objects.rs
