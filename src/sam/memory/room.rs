use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::Config;
use crate::sam::memory::Result;
use rand::Rng;
use crate::sam::memory::PostgresQueries;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub id: i32,
    pub oid: String,
    pub name: String,
    pub icon: String,
    pub location_oid: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
}

impl Room {
    pub fn new() -> Room {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        Room { 
            id: 0,
            oid,
            name: String::new(), 
            icon: "fa fa-solid fa-cube".to_string(),
            location_oid: String::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "rooms".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.rooms (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            icon varchar NULL,
            location_oid varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT rooms_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.rooms ADD COLUMN icon varchar NULL;",
            "ALTER TABLE public.rooms ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.rooms ADD COLUMN updated_at BIGINT NULL;"
        ]
    }
    pub fn save(&self) -> Result<&Self>{

        let mut client = Config::client()?;
        
        // Search for OID matches
        let statement = client.prepare("SELECT * FROM rooms WHERE oid = $1 OR (location_oid = $2 AND name = $3)")?;
        let rows = client.query(&statement, &[
            &self.oid, 
            &self.location_oid,
            &self.name,
        ])?;

        if rows.is_empty() {
            client.execute("INSERT INTO rooms (oid, name, icon, location_oid, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.name,
                &self.icon,
                &self.location_oid,
                &self.created_at,
                &self.updated_at]
            ).unwrap();
        } else {
            let ads = Self::from_row(&rows[0]).unwrap();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE rooms SET name = $1, icon = $2, location_oid = $3 WHERE oid = $4;", 
                &[
                    &self.name,
                    &self.icon,
                    &self.location_oid,
                    &ads.oid
                ])?;
            }
        }
        Ok(self)
        
    }
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    pub fn from_row(row: &Row) -> Result<Self> {

        let mut icon: String = "fa fa-solid fa-cube".to_string();

        if let Some(val) = row.get("icon") {
            icon = val;
        }

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"), 
            icon, 
            location_oid: row.get("location_oid"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "rooms".to_string())
    }
}