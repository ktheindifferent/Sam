use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebSessions {
    pub id: i32,
    pub oid: String,
    pub sid: String,
    pub human_oid: String,
    pub ip_address: String,
    pub authenticated: bool,
    pub timestamp: i64,
}
impl WebSessions {
    pub fn new(sid: String) -> WebSessions {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        WebSessions { 
            id: 0,
            oid,
            sid,
            human_oid: String::new(), 
            ip_address: String::new(),
            authenticated: false,
            timestamp,
        }
    }
    pub fn sql_table_name() -> String {
        "web_sessions".to_string()
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
           ""
        ]
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.web_sessions (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            sid varchar NOT NULL UNIQUE,
            human_oid varchar NULL,
            ip_address varchar NULL,
            authenticated bool NULL DEFAULT FALSE,
            timestamp BIGINT NULL,
            CONSTRAINT web_sessions_pkey PRIMARY KEY (id));"
    }
    pub fn save(&self) -> Result<&Self>{
        
        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.sid.clone()));
        pg_query.query_columns.push(" OR sid =".to_string());
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query)
        )?;

        if rows.is_empty() {



            client.execute("INSERT INTO web_sessions (oid, sid, human_oid, ip_address, authenticated, timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.sid,
                &self.human_oid, 
                &self.ip_address.to_string(), 
                &self.authenticated, 
                &self.timestamp]
            ).unwrap();

        
            return Ok(self);
        
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


        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            sid: row.get("sid"),
            human_oid: row.get("human_oid"), 
            ip_address: row.get("ip_address"), 
            authenticated: row.get("authenticated"),
            timestamp: row.get("timestamp"),
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "web_sessions".to_string())
    }
}