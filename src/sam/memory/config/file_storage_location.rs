use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileStorageLocation {
    pub id: i32,
    pub oid: String,
    pub storage_type: String,
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for FileStorageLocation {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorageLocation {
    pub fn new() -> FileStorageLocation {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        FileStorageLocation { 
            id: 0,
            oid,
            storage_type: String::new(), 
            endpoint: String::new(), 
            username: String::new(), 
            password: String::new(), 
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "storage_locations".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.storage_locations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            storage_type varchar NULL,
            endpoint varchar NULL,
            username varchar NULL,
            password varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT storage_locations_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.storage_locations ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.storage_locations ADD COLUMN updated_at BIGINT NULL;"
        ]
    }
    pub fn save(&self) -> Result<&Self>{

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
            client.execute("INSERT INTO storage_locations (oid, storage_type, endpoint, username, password, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
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
                client.execute("UPDATE storage_locations SET storage_type = $1, endpoint = $2, username = $3, password = $4, updated_at = $5 WHERE oid = $6;", 
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
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
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
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "storage_locations".to_string())
    }
}