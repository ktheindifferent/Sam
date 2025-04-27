use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use tokio_postgres::Row;
use crate::sam::memory::{Config, PostgresQueries};
use crate::sam::memory::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebCrawlExtResult {
    pub id: i32,
    pub oid: String,
    pub url: String,
    pub summaries: HashMap<String, Option<String>>, // e.g., {"llama": Some("..."), "gpt4": None}
    pub links: Vec<String>,
    pub timestamp: i64,
}

impl Default for WebCrawlExtResult {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

impl WebCrawlExtResult {
    pub fn new(url: String) -> Self {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Self {
            id: 0,
            oid,
            url,
            summaries: HashMap::new(),
            links: Vec::new(),
            timestamp,
        }
    }

    pub fn sql_table_name() -> String {
        "cache_web_crawl_ext_results".to_string()
    }

    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.cache_web_crawl_ext_results (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            url varchar NOT NULL,
            summaries jsonb NULL,
            links jsonb NULL,
            timestamp BIGINT DEFAULT 0,
            CONSTRAINT cache_web_crawl_ext_results_pkey PRIMARY KEY (id));"
    }

    pub fn migrations() -> Vec<&'static str> {
        vec![
            "",
        ]
    }

    pub fn save(object: Self) -> Result<Self> {
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
        )?;

        let summaries_json = serde_json::to_string(&object.summaries).unwrap();
        let links_json = serde_json::to_string(&object.links).unwrap();

        if rows.is_empty() {
            client.execute(
                "INSERT INTO cache_web_crawl_ext_results (oid, url, summaries, links, timestamp) VALUES ($1, $2, $3, $4, $5)",
                &[&object.oid, &object.url, &summaries_json, &links_json, &object.timestamp]
            )?;

            // Search for OID matches
            let rows_two = Self::select(
                None,
                None,
                None,
                Some(pg_query)
            )?;
            Ok(rows_two[0].clone())
        } else {
            let ads = rows[0].clone();
            client.execute(
                "UPDATE cache_web_crawl_ext_results SET url = $1, summaries = $2, links = $3, timestamp = $4 WHERE oid = $5;",
                &[&object.url, &summaries_json, &links_json, &object.timestamp, &ads.oid]
            )?;

            let statement_two = client.prepare("SELECT * FROM cache_web_crawl_ext_results WHERE oid = $1")?;
            let rows_two = client.query(&statement_two, &[&object.oid])?;
            Self::from_row(&rows_two[0])
        }
    }

    pub fn select(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>
    ) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(
            Self::sql_table_name(),
            None,
            limit,
            offset,
            order,
            query, None
        )?;

        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    pub fn from_row(row: &Row) -> Result<Self> {
        let summaries_json: Option<String> = row.get("summaries");
        let summaries: HashMap<String, Option<String>> = if let Some(s) = summaries_json {
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let links_json: Option<String> = row.get("links");
        let links: Vec<String> = if let Some(l) = links_json {
            serde_json::from_str(&l).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            url: row.get("url"),
            summaries,
            links,
            timestamp: row.get("timestamp"),
        })
    }

    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, Self::sql_table_name())
    }
}
