use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceSetting {
    pub tag: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    pub id: i32,
    pub oid: String,
    pub identifier: String,
    pub key: String,
    pub secret: String,
    pub username: String,
    pub password: String,
    pub endpoint: String,
    pub settings: Vec<ServiceSetting>,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}
impl Service {
    pub fn new() -> Service {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let settings: Vec<ServiceSetting> = Vec::new();
        Service { 
            id: 0,
            oid,
            identifier: String::new(),
            key: String::new(),
            secret: String::new(),
            username: String::new(),
            password: String::new(),
            endpoint: String::new(),
            settings,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64

        }
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.services ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.services ADD COLUMN updated_at BIGINT NULL;",
            "ALTER TABLE public.services ADD COLUMN username varchar NULL;",
            "ALTER TABLE public.services ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.services ADD COLUMN settings varchar NULL;",
        ]
    }
    pub fn sql_table_name() -> String {
        "services".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.services (
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
    pub fn save(&self) -> Result<&Self>{

        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.identifier.clone()));
        pg_query.query_columns.push(" OR identifier =".to_string());
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query)
        ).unwrap();

        // Save New Service
        if rows.is_empty() {
            let settings = serde_json::to_string(&self.settings).unwrap();
            client.execute("INSERT INTO services (oid, identifier, key, secret, username, password, endpoint, settings, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
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
        
        } 
        // Update existing service
        else {

            let ads = rows[0].clone();
            let settings = serde_json::to_string(&self.settings).unwrap();
            // Only save if newer than stored information
            client.execute("UPDATE services SET key = $1, secret = $2, settings = $3, updated_at = $4 WHERE oid = $5;", 
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
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "services".to_string())
    }
}