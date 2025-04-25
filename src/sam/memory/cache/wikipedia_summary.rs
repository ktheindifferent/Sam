use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WikipediaSummary {
    pub id: i32,
    pub oid: String,
    pub topics: Vec<String>,
    pub summary: String,
    pub timestamp: i64
}
impl Default for WikipediaSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaSummary {
    pub fn new() -> WikipediaSummary {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let topics: Vec<String> = Vec::new();
        WikipediaSummary { 
            id: 0,
            oid,
            topics,
            summary: String::new(),
            timestamp
        }
    }
    pub fn sql_table_name() -> String {
        "cached_wikipedia_summaries".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.cached_wikipedia_summaries (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            topics varchar NULL,
            summary varchar NULL,
            timestamp BIGINT DEFAULT 0,
            CONSTRAINT cached_wikipedia_summaries_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "",
        ]
    }
    pub fn save(object: Self) -> Result<Self>{
        let mut client = Config::client()?;
        
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


            client.execute("INSERT INTO cached_wikipedia_summaries (oid, topics, summary, timestamp) VALUES ($1, $2, $3, $4)",
                &[&object.oid.clone(),
                &object.topics.join(","),
                &object.summary,
                &object.timestamp]
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


            // Only save if newer than stored information
            // if objec.updated_at > ads.updated_at {
                client.execute("UPDATE cached_wikipedia_summaries SET topics = $1, summary = $2, timestamp = $3 WHERE oid = $4;", 
                &[&object.topics.join(","),
                &object.summary,
                &object.timestamp,
                &ads.oid])?;
            // }

            let statement_two = client.prepare("SELECT * FROM cached_wikipedia_summaries WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[
                &object.oid, 
            ])?;

            Self::from_row(&rows_two[0])
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


        let mut topics: Vec<String> = Vec::new();
        let sql_topics: Option<String> = row.get("topics");
        if let Some(ts) = sql_topics {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            topics = newvec;
        }
   


        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            topics, 
            summary: row.get("summary"),
            timestamp: row.get("timestamp"),
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "cached_wikipedia_summaries".to_string())
    }
}