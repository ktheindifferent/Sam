//! Observation module
//!
//! Provides synchronous and asynchronous methods for interacting with observation records in a PostgreSQL database.

use serde::{Serialize, Deserialize};
use std::fmt;
use std::str::FromStr;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries, Human, Thing, DeepVisionResult, ObservationType, ObservationObjects};
use crate::sam::memory::cache::WebSessions;
use crate::sam::memory::Result;
use rand::Rng;

/// Represents an observation event in the system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Observation {
    /// Database ID (primary key).
    pub id: i32,
    /// Unique object identifier.
    pub oid: String,
    /// Timestamp of the observation (seconds since UNIX_EPOCH).
    pub timestamp: i64,
    /// Type of the observation.
    pub observation_type: ObservationType,
    /// Objects observed.
    pub observation_objects: Vec<ObservationObjects>,
    /// Humans observed.
    pub observation_humans: Vec<Human>,
    /// Notes about the observation.
    pub observation_notes: Vec<String>,
    /// Optional file data associated with the observation.
    pub observation_file: Option<Vec<u8>>,
    /// Deep vision results.
    pub deep_vision: Vec<DeepVisionResult>,
    /// Optional deep vision results as JSON.
    pub deep_vision_json: Option<String>,
    /// Associated thing/device (if any).
    pub thing: Option<Thing>,
    /// Associated web session (if any).
    pub web_session: Option<WebSessions>,
}

impl Default for Observation {
    fn default() -> Self {
        Self::new()
    }
}

impl Observation {
    /// Creates a new Observation with a random OID and current timestamp.
    pub fn new() -> Observation {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let observation_objects: Vec<ObservationObjects> = Vec::new();
        let observation_humans: Vec<Human> = Vec::new();
        let observation_notes: Vec<String> = Vec::new();
        let deep_vision: Vec<DeepVisionResult> = Vec::new();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Observation { 
            id: 0,
            oid,
            timestamp,
            observation_type: ObservationType::UNKNOWN,
            observation_objects,
            observation_humans,
            observation_notes,
            observation_file: None,
            deep_vision,
            deep_vision_json: None,
            thing: None,
            web_session: None,
        }
    }

    /// Returns the SQL table name for the observations table.
    pub fn sql_table_name() -> String {
        "observations".to_string()
    }

    /// Returns a list of SQL migration statements for the observations table.
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.observations ADD COLUMN observation_file bytea NULL;",
            "ALTER TABLE public.observations ADD COLUMN deep_vision_json varchar NULL;",
            "ALTER TABLE public.observations ADD COLUMN thing_oid varchar NULL;",
            "ALTER TABLE public.observations ADD COLUMN web_session_id varchar NULL;",
        ]
    }

    /// Returns the SQL statement to create the observations table.
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.observations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            timestamp BIGINT NULL,
            observation_type varchar NULL,
            observation_objects varchar NULL,
            observation_humans varchar NULL,
            observation_notes varchar NULL,
            observation_file bytea NULL,
            deep_vision_json varchar NULL,
            thing_oid varchar NULL,
            web_session_id varchar NULL,
            CONSTRAINT observations_pkey PRIMARY KEY (id));"
    }

    /// Saves the Observation to the database. Updates if OID exists, inserts otherwise.
    pub fn save(&self) -> Result<Self> {
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

            let mut obb_obv_str = String::new();
            for obv in &self.observation_objects{
                obb_obv_str += format!("{},", obv).as_str();
            }

            let mut obb_humans_str = String::new();
            for hum in &self.observation_humans{
                obb_humans_str += format!("{},", hum.oid).as_str();
            }


            let mut obb_thing_str = String::new();
            if let Some(thing) = &self.thing {
                obb_thing_str = thing.oid.clone();
            }

            let mut obb_web_session_str = String::new();
            if let Some(web_session) = &self.web_session {
                obb_web_session_str = web_session.sid.clone();
            }

            client.execute("INSERT INTO observations (oid, timestamp, observation_type, thing_oid, web_session_id, observation_objects, observation_humans, observation_notes, observation_file) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[&self.oid.clone(),
                &self.timestamp,
                &self.observation_type.to_string(), 
                &obb_thing_str, 
                &obb_web_session_str,
                &obb_obv_str, 
                &obb_humans_str, 
                &self.observation_notes.join(","),
                &self.observation_file]
            ).unwrap();


            if self.deep_vision_json.is_some() {
                client.execute("UPDATE observations SET deep_vision_json = $1 WHERE oid = $2;", 
                &[
                    &self.deep_vision_json.clone().unwrap(),
                    &self.oid
                ])?;
            }


            let mut pg_query = PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
            pg_query.query_columns.push("oid =".to_string());
             let rows_two = Self::select(
                None, 
                None, 
                None, 
                Some(pg_query)
            ).unwrap();
        
            Ok(rows_two[0].clone())
        
        } else {


            let ads = rows[0].clone();


            let mut obb_obv_str = String::new();
            for obv in &self.observation_objects{
                obb_obv_str += format!("{},", obv).as_str();
            }

            let mut obb_humans_str = String::new();
            for hum in &self.observation_humans{
                obb_humans_str += format!("{},", hum.oid).as_str();
            }




            client.execute("UPDATE observations SET observation_type = $1, observation_objects = $2, observation_humans = $3, observation_notes = $4, observation_file = $5 WHERE oid = $6;", 
            &[&self.observation_type.to_string(), 
            &obb_obv_str, 
            &obb_humans_str, 
            &self.observation_notes.join(","),
            &self.observation_file,
            &ads.oid])?;

            if self.deep_vision_json.is_some() {
                client.execute("UPDATE observations SET deep_vision_json = $1 WHERE oid = $2;", 
                &[
                    &self.deep_vision_json.clone().unwrap(),
                    &self.oid
                ])?;
            }


    

            let statement_two = client.prepare("SELECT * FROM observations WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[
                &self.oid, 
            ])?;

            Self::from_row(&rows_two[0])

        }
    }

    /// Asynchronously saves the Observation to the database. Updates if OID exists, inserts otherwise.
    pub async fn save_async(&self) -> Result<Self> {
        let mut client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(self.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;
        let mut obb_obv_str = String::new();
        for obv in &self.observation_objects {
            obb_obv_str += format!("{},", obv).as_str();
        }
        let mut obb_humans_str = String::new();
        for hum in &self.observation_humans {
            obb_humans_str += format!("{},", hum.oid).as_str();
        }
        let mut obb_thing_str = String::new();
        if let Some(thing) = &self.thing {
            obb_thing_str = thing.oid.clone();
        }
        let mut obb_web_session_str = String::new();
        if let Some(web_session) = &self.web_session {
            obb_web_session_str = web_session.sid.clone();
        }
        if rows.is_empty() {
            client.execute("INSERT INTO observations (oid, timestamp, observation_type, thing_oid, web_session_id, observation_objects, observation_humans, observation_notes, observation_file) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[&self.oid.clone(),
                &self.timestamp,
                &self.observation_type.to_string(),
                &obb_thing_str,
                &obb_web_session_str,
                &obb_obv_str,
                &obb_humans_str,
                &self.observation_notes.join(","),
                &self.observation_file]
            ).await?;
            if self.deep_vision_json.is_some() {
                client.execute("UPDATE observations SET deep_vision_json = $1 WHERE oid = $2;",
                &[
                    &self.deep_vision_json.clone().unwrap(),
                    &self.oid
                ]).await?;
            }
            let rows_two = Self::select_async(None, None, None, Some(pg_query)).await?;
            Ok(rows_two[0].clone())
        } else {
            let ads = rows[0].clone();
            client.execute("UPDATE observations SET observation_type = $1, observation_objects = $2, observation_humans = $3, observation_notes = $4, observation_file = $5 WHERE oid = $6;",
                &[&self.observation_type.to_string(),
                &obb_obv_str,
                &obb_humans_str,
                &self.observation_notes.join(","),
                &self.observation_file,
                &ads.oid]).await?;
            if self.deep_vision_json.is_some() {
                client.execute("UPDATE observations SET deep_vision_json = $1 WHERE oid = $2;",
                &[
                    &self.deep_vision_json.clone().unwrap(),
                    &self.oid
                ]).await?;
            }
            let statement_two = client.prepare("SELECT * FROM observations WHERE oid = $1").await?;
            let rows_two = client.query(&statement_two, &[&self.oid]).await?;
            Self::from_row(&rows_two[0])
        }
    }

    /// Selects Observation entries from the database with optional limit, offset, order, and query.
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }

    /// Asynchronously selects Observation entries from the database with optional limit, offset, order, and query.
    pub async fn select_async(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let config = crate::sam::memory::Config::new();
let client = config.connect_pool().await?;
        let jsons = crate::sam::memory::Config::pg_select_async(Self::sql_table_name(), None, limit, offset, order, query, client).await?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Selects Observation entries (without file data) from the database with optional limit, offset, order, and query.
    pub fn select_lite(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = Config::pg_select(Self::sql_table_name(), Some("id, oid, timestamp, observation_type, observation_objects, observation_humans, observation_notes, deep_vision_json".to_string()), limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }

    /// Asynchronously selects Observation entries (without file data) from the database with optional limit, offset, order, and query.
    pub async fn select_lite_async(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let config = crate::sam::memory::Config::new();
let client = config.connect_pool().await?;
        let jsons = Config::pg_select_async(Self::sql_table_name(), Some("id, oid, timestamp, observation_type, observation_objects, observation_humans, observation_notes, deep_vision_json".to_string()), limit, offset, order, query, client).await?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    /// Constructs an Observation from a PostgreSQL row.
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut deep_vision: Vec<DeepVisionResult> = Vec::new();

        let deep_vision_json = row.get("deep_vision_json");

        if let Some(deep_vision_json_val) = deep_vision_json {
            deep_vision = serde_json::from_str(deep_vision_json_val).unwrap();
        }


    
        let mut observation_type = ObservationType::UNKNOWN;
        let sql_observation_type: Option<String> = row.get("observation_type");
        if let Some(object) = sql_observation_type {
            let obj = ObservationType::from_str(&object).unwrap();
            observation_type = obj.clone();
        }
        


        let mut observation_objects: Vec<ObservationObjects> = Vec::new();
        let sql_observation_objects: Option<String> = row.get("observation_objects");
        if let Some(object) = sql_observation_objects {
            let split = object.split(",");
            for s in split {
                if !s.is_empty() {
                    let obj = ObservationObjects::from_str(s);
                    match obj{
                        Ok(obj) => observation_objects.push(obj),
                        Err(err) => log::error!("sql_observation_objects: {:?}: {:?}",observation_objects.clone(), err)
                    }
                }
            }
        }
        

        let mut observation_humans: Vec<Human> = Vec::new();
        let sql_observation_humans: Option<String> = row.get("observation_humans");
        if let Some(object) = sql_observation_humans {
            let split = object.split(",");
            let vec = split.collect::<Vec<&str>>();
            for oidx in vec {

                // Search for OID matches
                let mut pg_query = PostgresQueries::default();
                pg_query.queries.push(crate::sam::memory::PGCol::String(oidx.to_string()));
                pg_query.query_columns.push("oid ilike".to_string());


                let observation_humansx = Human::select(
                    None, 
                    None, 
                    None, 
                    Some(pg_query)
                ).unwrap(); 

                for human in observation_humansx{
                    observation_humans.push(human);
                }

                // if rows.len() > 0 {
                //     observation_humans.push(rows[0].clone());
                // }
            }
        }
        

        let mut observation_notes: Vec<String> = Vec::new();
        let sql_observation_notes: Option<String> = row.get("observation_notes");
        if let Some(object) = sql_observation_notes {
            let split = object.split(",");
            for s in split {
                observation_notes.push(s.to_string());
            }
        }
        

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            timestamp: row.get("timestamp"), 
            observation_type,
            observation_objects,
            observation_humans,
            observation_notes,
            observation_file: row.get("observation_file"),
            deep_vision,
            deep_vision_json: row.get("deep_vision_json"),
            thing: None,
            web_session: None,
        })
    }

    /// Asynchronously constructs an Observation from a PostgreSQL row.
    pub async fn from_row_async(row: &Row) -> Result<Self> {
        Self::from_row(row)
    }

    /// Constructs an Observation from a PostgreSQL row (without file data).
    pub fn from_row_lite(row: &Row) -> Result<Self> {
        let mut deep_vision: Vec<DeepVisionResult> = Vec::new();

        let deep_vision_json = row.get("deep_vision_json");

        if let Some(deep_vision_json_val) = deep_vision_json {
            deep_vision = serde_json::from_str(deep_vision_json_val).unwrap();
        }


    
        let mut observation_type = ObservationType::UNKNOWN;
        let sql_observation_type: Option<String> = row.get("observation_type");
        if let Some(object) = sql_observation_type {
            let obj = ObservationType::from_str(&object).unwrap();
            observation_type = obj.clone();
        }
        


        let mut observation_objects: Vec<ObservationObjects> = Vec::new();
        let sql_observation_objects: Option<String> = row.get("observation_objects");
        if let Some(object) = sql_observation_objects {
            let split = object.split(",");
            for s in split {
                if !s.is_empty() {
                    let obj = ObservationObjects::from_str(s);
                    match obj{
                        Ok(obj) => observation_objects.push(obj),
                        Err(err) => log::error!("sql_observation_objects2: {:?}: {:?}",observation_objects.clone(), err)
                    }
                }
            }
        }
        

        let mut observation_humans: Vec<Human> = Vec::new();
        let sql_observation_humans: Option<String> = row.get("observation_humans");
        if let Some(object) = sql_observation_humans {
            let split = object.split(",");
            let vec = split.collect::<Vec<&str>>();
            for oidx in vec {
                if !oidx.is_empty() {
                    let mut xperson = Human::new();
                    xperson.oid = oidx.to_string();
                    observation_humans.push(xperson);
                }
            }
        }
        

        let mut observation_notes: Vec<String> = Vec::new();
        let sql_observation_notes: Option<String> = row.get("observation_notes");
        if let Some(object) = sql_observation_notes {
            let split = object.split(",");
            for s in split {
                observation_notes.push(s.to_string());
            }
        }
        

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            timestamp: row.get("timestamp"), 
            observation_type,
            observation_objects,
            observation_humans,
            observation_notes,
            observation_file: None,
            deep_vision,
            deep_vision_json: row.get("deep_vision_json"),
            thing: None,
            web_session: None,
        })
    }

    /// Asynchronously constructs an Observation from a PostgreSQL row (without file data).
    pub async fn from_row_lite_async(row: &Row) -> Result<Self> {
        Self::from_row_lite(row)
    }

    /// Deletes an Observation from the database by OID.
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "observations".to_string())
    }

    /// Asynchronously deletes an Observation from the database by OID.
    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, "observations".to_string()).await
    }
}