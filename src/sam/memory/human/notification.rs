use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::Config;
use crate::sam::memory::Result;
use crate::sam::memory::PostgresQueries;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notification {
    pub id: i32,
    pub oid: String,
    pub sid: String,
    pub human_oid: String,
    pub message: String,
    pub seen: bool,
    pub timestamp: i64
}
impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Notification {
    pub fn new() -> Notification {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Notification { 
            id: 0,
            oid,
            sid: String::new(),
            human_oid: String::new(),
            message: String::new(),
            seen: false,
            timestamp
        }
    }
    pub fn sql_table_name() -> String {
        "notifications".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.notifications (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            sid varchar NULL,
            human_oid varchar NULL,
            message varchar NULL,
            seen bool DEFAULT false,
            timestamp BIGINT DEFAULT 0,
            CONSTRAINT notifications_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "",
        ]
    }
    pub fn save(&self) -> Result<Self>{
        let mut client = Config::client()?;
        
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());


        // Search for OID matches
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query.clone())
        ).unwrap();

        if rows.is_empty() {

            client.execute("INSERT INTO notifications (oid, sid, human_oid, message, seen, timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.sid,
                &self.human_oid,
                &self.message,
                &self.seen,
                &self.timestamp]
            ).unwrap();

    
            // Search for OID matches
            let rows_two = Self::select(
                None, 
                None, 
                None, 
                Some(pg_query)
            ).unwrap();

            Ok(rows_two[0].clone())
        
        } else {
            let ads = rows[0].clone();


 
            client.execute("UPDATE notifications SET message = $1, seen = $2 WHERE oid = $3;", 
            &[&self.message,
            &self.seen,
            &ads.oid])?;


            let statement_two = client.prepare("SELECT * FROM notifications WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[
                &self.oid, 
            ])?;

            Self::from_row(&rows_two[0])
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
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            sid: row.get("sid"),
            human_oid: row.get("human_oid"),
            message: row.get("message"),
            seen: row.get("seen"),
            timestamp: row.get("timestamp")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "notifications".to_string())
    }
}