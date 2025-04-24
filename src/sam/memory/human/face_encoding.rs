use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaceEncoding {
    pub id: i32,
    pub oid: String,
    pub encoding: Vec<u8>,
    pub human_oid: String,
    pub timestamp: i64
}
impl Default for FaceEncoding {
    fn default() -> Self {
        Self::new()
    }
}
impl FaceEncoding {
    pub fn new() -> FaceEncoding {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let encoding: Vec<u8> = Vec::new();
        FaceEncoding { 
            id: 0,
            oid,
            encoding, 
            human_oid: String::new(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        }
    }
    pub fn sql_table_name() -> String {
        "human_face_encodings".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.human_face_encodings (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            encoding bytea NULL,
            human_oid varchar NULL,
            timestamp BIGINT NULL,
            CONSTRAINT human_face_encodings_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.human_face_encodings ADD COLUMN timestamp BIGINT NULL;"
        ]
    }
    pub fn save(object: Self) -> Result<Self>{

        let mut client = Config::client()?;

        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(object.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());


        // Search for OID matches
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query.clone())
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO human_face_encodings (oid, encoding, human_oid, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.encoding,
                &object.human_oid,
                &object.timestamp]
            ).unwrap();
            
             let rows_two = Self::select(
                None, 
                None, 
                None, 
                Some(pg_query)
            ).unwrap();
        
            return Ok(rows_two[0].clone());
        
        }
        
    
        Ok(object)
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
            encoding: row.get("encoding"), 
            human_oid:  row.get("human_oid"),
            timestamp: row.get("timestamp"),
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "human_face_encodings".to_string())
    }
}