use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Setting {
    pub id: i32,
    pub oid: String,
    pub key: String,
    pub values: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Setting {
    fn default() -> Self {
        Self::new()
    }
}


impl Setting {
    pub fn new() -> Setting {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let empty_vec: Vec<String> = Vec::new();
        Setting { 
            id: 0,
            oid,
            key: String::new(), 
            values: empty_vec,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "settings".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.settings (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            key varchar NULL,
            values varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT settings_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.settings ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.settings ADD COLUMN updated_at BIGINT NULL;"
        ]
    }
    pub fn save(&self) -> Result<&Self>{

        let mut client = Config::client()?;
        
        // Search for OID matches
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.key.clone()));
        pg_query.query_columns.push(" OR key =".to_string());
        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query)
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO settings (oid, key, values, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                &[&self.oid.clone(),
                &self.key,
                &self.values.join(","),
                &self.created_at,
                &self.updated_at]
            )?;        
            Ok(self)
        
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE settings SET key = $1, values = $2, updated_at = $3 WHERE oid = $4;", 
                &[
                    &self.key,
                    &self.values.join(","),
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
     

           
        let mut values: Vec<String> = Vec::new();
        let sql_values: Option<String> = row.get("values");
        if let Some(ts) = sql_values {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            values = newvec;
        }  

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            key: row.get("key"), 
            values,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "settings".to_string())
    }
}