use crate::sam::memory::Result;
use crate::sam::memory::{Config, PostgresQueries};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebCrawl {
    pub id: i32,
    pub url: String,
}

impl Default for WebCrawl {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

impl WebCrawl {
    pub fn new(url: String) -> Self {
        Self { id: 0, url }
    }

    pub fn sql_table_name() -> String {
        "cache_web_crawl".to_string()
    }

    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.cache_web_crawl (
            id serial NOT NULL,
            url varchar NOT NULL UNIQUE,
            CONSTRAINT cache_web_crawl_ext_results_pkey PRIMARY KEY (id));"
    }

    pub fn migrations() -> Vec<&'static str> {
        vec![""]
    }

    pub fn save(object: Self) -> Result<Self> {
        let mut client = Config::client()?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(object.url.clone()));
        pg_query.query_columns.push("url =".to_string());

        // Search for OID matches
        let rows = Self::select(None, None, None, Some(pg_query.clone()))?;

        if rows.is_empty() {
            client.execute(
                "INSERT INTO cache_web_crawl (url) VALUES ($1)",
                &[&object.url],
            )?;

            // Search for OID matches
            let rows_two = Self::select(None, None, None, Some(pg_query))?;
            Ok(rows_two[0].clone())
        } else {
            Ok(rows[0].clone())
        }
    }

    pub async fn save_batch_async(objects: Vec<Self>) -> Result<Vec<Self>> {
        if objects.is_empty() {
            return Ok(Vec::new());
        }

        let mut client = Config::client_async().await?;

        // Collect all URLs for batch checking
        let urls: Vec<String> = objects.iter().map(|o| o.url.clone()).collect();

        // Build a single query to check all URLs at once
        let placeholders: Vec<String> = (1..=urls.len()).map(|i| format!("${}", i)).collect();
        let query = format!(
            "SELECT url FROM cache_web_crawl WHERE url IN ({})",
            placeholders.join(", ")
        );

        // Convert urls to references for the query
        let url_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = urls
            .iter()
            .map(|u| u as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // Get existing URLs
        let rows = client.query(&query, &url_refs[..]).await?;
        let existing_urls: std::collections::HashSet<String> =
            rows.iter().map(|row| row.get::<_, String>("url")).collect();

        // Filter out existing URLs
        let new_objects: Vec<Self> = objects
            .into_iter()
            .filter(|o| !existing_urls.contains(&o.url))
            .collect();

        if new_objects.is_empty() {
            return Ok(Vec::new());
        }

        // Bulk insert new URLs using UNNEST for better performance
        let new_urls: Vec<String> = new_objects.iter().map(|o| o.url.clone()).collect();

        client
            .execute(
                "INSERT INTO cache_web_crawl (url) 
             SELECT DISTINCT * FROM UNNEST($1::varchar[]) 
             ON CONFLICT (url) DO NOTHING",
                &[&new_urls],
            )
            .await?;

        // Return the inserted objects
        Ok(new_objects)
    }

    pub async fn save_async(object: Self) -> Result<Self> {
        let mut client = Config::client_async().await?;
        let mut pg_query = PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(object.url.clone()));
        pg_query.query_columns.push("url =".to_string());

        // Search for OID matches
        let rows = Self::select_async(None, None, None, Some(pg_query.clone())).await?;

        if rows.is_empty() {
            client
                .execute(
                    "INSERT INTO cache_web_crawl (url) VALUES ($1)",
                    &[&object.url],
                )
                .await?;

            // Search for OID matches
            let rows_two = Self::select_async(None, None, None, Some(pg_query)).await?;
            Ok(rows_two[0].clone())
        } else {
            Ok(rows[0].clone())
        }
    }

    pub fn select(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(
            Self::sql_table_name(),
            None,
            limit,
            offset,
            order,
            query,
            None,
        )?;

        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    pub async fn select_async(
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> Result<Vec<Self>> {
        let mut parsed_rows: Vec<Self> = Vec::new();
        let config = crate::sam::memory::Config::new();
        let client = config.connect_pool().await?;
        let jsons = crate::sam::memory::Config::pg_select_async(
            Self::sql_table_name(),
            None,
            limit,
            offset,
            order,
            query,
            client,
        )
        .await?;
        for j in jsons {
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        Ok(parsed_rows)
    }

    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            url: row.get("url"),
        })
    }

    pub async fn from_row_async(row: &Row) -> Result<Self> {
        // This function is synchronous in practice, but provided for API symmetry.
        Self::from_row(row)
    }

    pub fn destroy(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row(oid, Self::sql_table_name())
    }

    pub async fn destroy_async(oid: String) -> Result<bool> {
        crate::sam::memory::Config::destroy_row_async(oid, Self::sql_table_name()).await
    }
}
