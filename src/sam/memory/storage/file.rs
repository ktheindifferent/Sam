use serde::{Serialize, Deserialize};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct File {
    pub id: i32,
    pub oid: String,
    pub file_name: String,
    pub file_type: String,
    pub file_data: Option<Vec<u8>>,
    pub file_folder_tree: Option<Vec<String>>,
    pub storage_location_oid: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for File {
    fn default() -> Self {
        Self::new()
    }
}

impl File {
    pub fn new() -> File {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        File { 
            id: 0,
            oid,
            file_name: String::new(), 
            file_type: String::new(), 
            file_data: None, 
            file_folder_tree: None, 
            storage_location_oid: String::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "file_storage".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.file_storage (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            file_name varchar NULL,
            file_type varchar NULL,
            file_data BYTEA NULL,
            file_folder_tree varchar NULL,
            storage_location_oid varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT file_storage_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.file_storage ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.file_storage ADD COLUMN updated_at BIGINT NULL;"
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
            Some(pg_query.clone())
        )?;

        if rows.is_empty() {
            client.execute("INSERT INTO file_storage (oid, file_name, file_type, storage_location_oid, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.file_name,
                &self.file_type,
                &self.storage_location_oid,
                &self.created_at,
                &self.updated_at]
            )?;        

        
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE file_storage SET file_name = $1, file_type = $2, storage_location_oid = $3, updated_at = $4 WHERE oid = $5;", 
                &[
                    &self.file_name,
                    &self.file_type,
                    &self.storage_location_oid,
                    &self.updated_at,
                    &ads.oid
                ])?;
            }

        }

        let rows = Self::select(
            None, 
            None, 
            None, 
            Some(pg_query)
        )?;
        let ads = rows[0].clone();

        if let Some(folder_tree) = self.file_folder_tree.clone() {
            client.execute("UPDATE file_storage SET file_folder_tree = $1, updated_at = $2 WHERE oid = $3;", 
            &[
                &folder_tree.join("/"),
                &self.updated_at,
                &ads.oid
            ])?;  
        }

        if let Some(file_data) = self.file_data.clone() {
            client.execute("UPDATE file_storage SET file_data = $1, updated_at = $2 WHERE oid = $3;", 
            &[
                &file_data,
                &self.updated_at,
                &ads.oid
            ])?;  
        }

        Ok(self)


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
    pub fn select_lite(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = Config::pg_select(Self::sql_table_name(), Some("id, oid, file_name, file_type, file_folder_tree, storage_location_oid, created_at, updated_at".to_string()), limit, offset, order, query, None)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    pub fn from_row(row: &Row) -> Result<Self> {

        let mut file_folder_tree: Option<Vec<String>> = None;
        let sql_file_folder_tree: Option<String> = row.get("file_folder_tree");
        if let Some(ts) = sql_file_folder_tree {
            let split = ts.split('/');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            file_folder_tree = Some(newvec);
        }

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            file_name: row.get("file_name"), 
            file_type: row.get("file_type"), 
            file_data: row.get("file_data"), 
            file_folder_tree, 
            storage_location_oid: row.get("storage_location_oid"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn from_row_lite(row: &Row) -> Result<Self> {

        let mut file_folder_tree: Option<Vec<String>> = None;
        let sql_file_folder_tree: Option<String> = row.get("file_folder_tree");
        if let Some(ts) = sql_file_folder_tree {
            let split = ts.split('/');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            file_folder_tree = Some(newvec);
        }


        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            file_name: row.get("file_name"), 
            file_type: row.get("file_type"), 
            file_data: None, 
            file_folder_tree, 
            storage_location_oid: row.get("storage_location_oid"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "file_storage".to_string())
    }
    pub fn cache_all() -> Result<()>{
        let files_without_data = File::select_lite(None, None, None, None)?;

        for file in files_without_data{

            if !Path::new(file.path_on_disk().as_str()).exists(){


                if file.storage_location_oid == *"SQL"{
                    let mut pg_query = PostgresQueries::default();
                    pg_query.queries.push(crate::sam::memory::PGCol::String(file.oid.clone()));
                    pg_query.query_columns.push("oid =".to_string());
        
                    let files_with_data = File::select(None, None, None, Some(pg_query))?;
                    let ffile = files_with_data[0].clone();
                    ffile.cache()?;
                } else if file.storage_location_oid == *"DROPBOX"{
                    // crate::sam::services::dropbox::download_file("/Sam/test.png", file.path_on_disk().as_str());
                }
        
            }

        }

        Ok(())
    }
    pub fn cache(&self) -> Result<()>{
        if let Some(data) = self.file_data.clone() {
            std::fs::write(self.path_on_disk().clone(), data)?;
        }
        Ok(())
    }
    pub fn path_on_disk(&self) -> String{
        format!("/opt/sam/files/{}", self.oid.clone())
    }
}