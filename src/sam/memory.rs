// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

pub mod pg;

use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rouille::Response;
use serde::{Serialize, Deserialize};
use std::env;
use std::fmt;
use std::str::FromStr;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::Row;
use std::path::Path;
use std::process::Command;

use error_chain::error_chain;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        TokioPg(tokio_postgres::Error);
        Hound(hound::Error);
        PostError(rouille::input::post::PostError);
        ParseFloatError(std::num::ParseFloatError);
        // TchError(tch::TchError);
    }
}

// store application version as a const
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub postgres: PostgresServer,
    pub version_installed: String
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Config {
        Config{
            postgres: PostgresServer::new(),
            version_installed: VERSION.unwrap_or("unknown").to_string()
        }
    }
    pub async fn init(&self){

        match self.create_db().await{
            Ok(_) => log::info!("Database created successfully"),
            Err(e) => log::error!("failed to create database: {}", e),
        }

        match self.build_tables().await{
            Ok(_) => log::info!("Tables created successfully"),
            Err(e) => log::error!("failed to create tables: {}", e),
        }
    

        let _config = self.clone();
        thread::spawn(move || {

            rouille::start_server("0.0.0.0:8000".to_string().as_str(), move |request| {
            
                match crate::sam::http::handle(request){
                    Ok(request) => {
                        request
                    },
                    Err(err) => {
                        log::error!("HTTP_ERROR: {}", err);
                        Response::empty_404()
                    }
                }

            });
        });
    }


    
    pub async fn build_tables(&self) -> Result<()>{
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());

        let (client, connection) = tokio_postgres::connect(format!("postgresql://{}:{}@{}/{}?sslmode=prefer", &self.postgres.username, &self.postgres.password, &self.postgres.address, &self.postgres.db_name).as_str(), connector).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });

        // Build all tables in sequence
        let tables = [
            (CachedWikipediaSummary::sql_table_name(), CachedWikipediaSummary::sql_build_statement(), CachedWikipediaSummary::migrations()),
            (Human::sql_table_name(), Human::sql_build_statement(), Human::migrations()),
            (HumanFaceEncoding::sql_table_name(), HumanFaceEncoding::sql_build_statement(), HumanFaceEncoding::migrations()),
            (Location::sql_table_name(), Location::sql_build_statement(), Location::migrations()),
            (Room::sql_table_name(), Room::sql_build_statement(), Room::migrations()),
            (Service::sql_table_name(), Service::sql_build_statement(), Service::migrations()),
            (Thing::sql_table_name(), Thing::sql_build_statement(), Thing::migrations()),
            (Observation::sql_table_name(), Observation::sql_build_statement(), Observation::migrations()),
            (Setting::sql_table_name(), Setting::sql_build_statement(), Setting::migrations()),
            (WebSessions::sql_table_name(), WebSessions::sql_build_statement(), WebSessions::migrations()),
            (StorageLocation::sql_table_name(), StorageLocation::sql_build_statement(), StorageLocation::migrations()),
            (FileStorage::sql_table_name(), FileStorage::sql_build_statement(), FileStorage::migrations()),
            (Notification::sql_table_name(), Notification::sql_build_statement(), Notification::migrations()),
            (crate::sam::crawler::CrawlJob::sql_table_name(), crate::sam::crawler::CrawlJob::sql_build_statement(), crate::sam::crawler::CrawlJob::migrations()),
            (crate::sam::crawler::CrawledPage::sql_table_name(), crate::sam::crawler::CrawledPage::sql_build_statement(), crate::sam::crawler::CrawledPage::migrations()),
        ];

        let mut current_client = client;
        for (table_name, build_statement, migrations) in tables {
            current_client = Self::build_table(current_client, table_name, build_statement, migrations).await;
        }

        Ok(())
    }

    /// Attempts to create the configured PostgreSQL database if it does not exist.
    /// 
    /// Connects to the PostgreSQL server (without specifying a database), then issues a
    /// `CREATE DATABASE` statement for the configured database name. If the database already
    /// exists, this will return an error.
    ///
    /// # Errors
    /// Returns an error if the connection fails or the database cannot be created.
    pub async fn create_db(&self) -> Result<()> {
        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());

        // Connect to the server without specifying a database
        let conn_str = format!(
            "postgresql://{}:{}@{}?sslmode=prefer",
            self.postgres.username, self.postgres.password, self.postgres.address
        );
        let (client, connection) = tokio_postgres::connect(&conn_str, connector).await?;

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("Postgres connection error: {}", e);
            }
        });

        // Attempt to create the database
        let create_db_sql = format!("CREATE DATABASE {}", self.postgres.db_name);
        match client.batch_execute(&create_db_sql).await {
            Ok(_) => log::info!("Database '{}' created successfully", self.postgres.db_name),
            Err(e) => {
                // If the database already exists, log and ignore the error
                if e.to_string().contains("already exists") {
                    log::info!("Database '{}' already exists", self.postgres.db_name);
                } else {
                    log::error!("Failed to create database '{}': {}", self.postgres.db_name, e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    /// Builds a table in the PostgreSQL database and applies any migrations.
    ///
    /// This function executes the provided `build_statement` to create the table if it does not exist,
    /// and then sequentially applies each migration in the `migrations` vector.
    ///
    /// # Arguments
    /// * `client` - The PostgreSQL client to use for executing statements.
    /// * `table_name` - The name of the table being created/migrated (for logging).
    /// * `build_statement` - The SQL statement to create the table.
    /// * `migrations` - A vector of SQL migration statements to apply after table creation.
    ///
    /// # Returns
    /// Returns the same `tokio_postgres::Client` for further use.
    pub async fn build_table(
        client: tokio_postgres::Client,
        table_name: String,
        build_statement: &str,
        migrations: Vec<&str>,
    ) -> tokio_postgres::Client {
        // Attempt to create the table
        match client.batch_execute(build_statement).await {
            Ok(_) => log::info!("POSTGRES: CREATED '{}' TABLE", table_name),
            Err(e) => log::error!("POSTGRES: Failed to create '{}': {:?}", table_name, e),
        }

        // Apply migrations in order
        for migration in migrations {
            // Skip empty migration strings
            if migration.trim().is_empty() {
                continue;
            }
            match client.batch_execute(migration).await {
                Ok(_) => log::info!("POSTGRES: MIGRATED '{}' TABLE", table_name),
                Err(e) => log::error!("POSTGRES: Migration failed for '{}': {:?}", table_name, e),
            }
        }

        client
    }

    /// Deletes a row from the specified table by OID.
    ///
    /// # Arguments
    /// * `oid` - The OID of the row to delete.
    /// * `table_name` - The name of the table from which to delete the row.
    ///
    /// # Returns
    /// Returns `Ok(true)` if the row was deleted (no longer exists), `Ok(false)` if the row still exists,
    /// or an error if the operation failed.
    pub fn destroy_row(oid: String, table_name: String) -> Result<bool> {
        let mut client = Config::client()?;

        // Use parameterized queries to prevent SQL injection
        client.execute(
            &format!("DELETE FROM {} WHERE oid = $1", table_name),
            &[&oid],
        )?;

        let rows = client.query(
            &format!("SELECT 1 FROM {} WHERE oid = $1", table_name),
            &[&oid],
        )?;

        Ok(rows.is_empty())
    }

    /// Drops all tables in the current PostgreSQL schema asynchronously.
    ///
    /// This function connects to the configured PostgreSQL database and executes a
    /// DO block that iterates over all tables in the current schema, dropping each one.
    /// This is a destructive operation and should be used with caution.
    ///
    /// # Returns
    /// Returns `Ok(())` if all tables were dropped successfully, or an error otherwise.
    pub async fn nuke_async() -> Result<()> {
        // Load configuration and get PostgreSQL connection info
        let config = crate::sam::memory::Config::new();
        let postgres = config.postgres.clone();

        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());

        // Connect to the PostgreSQL database
        let (client, connection) = tokio_postgres::connect(
            &format!(
                "postgresql://{}:{}@{}/{}?sslmode=prefer",
                &postgres.username, &postgres.password, &postgres.address, &postgres.db_name
            ),
            connector,
        )
        .await?;

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });

        // Drop all tables in the current schema
        client
            .batch_execute(
                r#"
                DO $$
                DECLARE
                    r RECORD;
                BEGIN
                    FOR r IN (
                        SELECT table_name
                        FROM information_schema.tables
                        WHERE table_schema = current_schema()
                    )
                    LOOP
                        EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.table_name) || ' CASCADE';
                    END LOOP;
                END
                $$;
                "#,
            )
            .await?;

        Ok(())
    }


    /// Executes a SELECT query on the specified PostgreSQL table with optional filtering, ordering, and pagination.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to query.
    /// * `columns` - Optional comma-separated list of columns to select. If `None`, selects all columns.
    /// * `limit` - Optional maximum number of rows to return.
    /// * `offset` - Optional number of rows to skip.
    /// * `order` - Optional ORDER BY clause (e.g., "id DESC").
    /// * `query` - Optional `PostgresQueries` for WHERE clause and parameterized values.
    ///
    /// # Returns
    /// Returns a `Result` containing a vector of JSON strings, each representing a row.
    pub fn pg_select(
        table_name: String,
        columns: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
    ) -> Result<Vec<String>> {
        let mut client = Config::client()?;

        // Build SELECT clause
        let mut execquery = if let Some(cols) = &columns {
            format!("SELECT {} FROM {}", cols, table_name)
        } else {
            format!("SELECT * FROM {}", table_name)
        };

        // Build WHERE clause if query is provided
        if let Some(pg_query) = query.clone() {
            let mut counter = 1;
            for col in pg_query.query_columns {
                if counter == 1 {
                    execquery = format!("{} WHERE {} ${}", execquery, col, counter);
                } else {
                    execquery = format!("{} {} ${}", execquery, col, counter);
                }
                counter += 1;
            }
        }

        // Add ORDER BY clause
        execquery = match order {
            Some(order_val) => format!("{} ORDER BY {}", execquery, order_val),
            None => format!("{} ORDER BY id DESC", execquery),
        };

        // Add LIMIT and OFFSET
        if let Some(limit_val) = limit {
            execquery = format!("{} LIMIT {}", execquery, limit_val);
        }
        if let Some(offset_val) = offset {
            execquery = format!("{} OFFSET {}", execquery, offset_val);
        }

        // Prepare to collect results
        let mut parsed_rows: Vec<String> = Vec::new();

        // Execute query with or without parameters
        if let Some(pg_query) = query {
            let query_values: Vec<_> = pg_query.queries.iter().map(|x| {
                match x {
                    PGCol::String(y) => y as &(dyn postgres::types::ToSql + Sync),
                    PGCol::Number(y) => y as &(dyn postgres::types::ToSql + Sync),
                    PGCol::Boolean(y) => y as &(dyn postgres::types::ToSql + Sync),
                }
            }).collect();

            for row in client.query(execquery.as_str(), query_values.as_slice())? {
                Self::serialize_row(&table_name, &columns, &row, &mut parsed_rows)?;
            }
        } else {
            for row in client.query(execquery.as_str(), &[])? {
                Self::serialize_row(&table_name, &columns, &row, &mut parsed_rows)?;
            }
        }

        // Close the client connection
        if let Err(e) = client.close() {
            log::error!("Failed to close PG-SQL Client: {}", e);
        }

        Ok(parsed_rows)
    }

    /// Helper function to serialize a row to JSON based on table and columns.
    /// Serializes a database row into a JSON string and pushes it to the parsed_rows vector.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to determine the struct type for deserialization.
    /// * `columns` - Optional comma-separated list of columns; if `None`, full struct is serialized.
    /// * `row` - The database row to serialize.
    /// * `parsed_rows` - The vector to which the resulting JSON string will be appended.
    ///
    /// # Returns
    /// Returns `Ok(())` on success, or an error if serialization fails.
    fn serialize_row(
        table_name: &str,
        columns: &Option<String>,
        row: &Row,
        parsed_rows: &mut Vec<String>,
    ) -> Result<()> {
        macro_rules! push_json {
            ($ty:ty, $from_row:ident) => {
                parsed_rows.push(serde_json::to_string(&<$ty>::$from_row(row)?).unwrap())
            };
        }

        match table_name {
            t if t == CachedWikipediaSummary::sql_table_name() => { push_json!(CachedWikipediaSummary, from_row); },
            t if t == Human::sql_table_name() => { push_json!(Human, from_row); },
            t if t == HumanFaceEncoding::sql_table_name() => { push_json!(HumanFaceEncoding, from_row); },
            t if t == Location::sql_table_name() => { push_json!(Location, from_row); },
            t if t == Notification::sql_table_name() => { push_json!(Notification, from_row); },
            t if t == Room::sql_table_name() => { push_json!(Room, from_row); },
            t if t == Service::sql_table_name() => { push_json!(Service, from_row); },
            t if t == Thing::sql_table_name() => { push_json!(Thing, from_row); },
            t if t == Observation::sql_table_name() => {
                if columns.is_none() {
                    push_json!(Observation, from_row);
                } else {
                    push_json!(Observation, from_row_lite);
                }
            },
            t if t == Setting::sql_table_name() => { push_json!(Setting, from_row); },
            t if t == WebSessions::sql_table_name() => { push_json!(WebSessions, from_row); },
            t if t == StorageLocation::sql_table_name() => { push_json!(StorageLocation, from_row); },
            t if t == FileStorage::sql_table_name() => {
                if columns.is_none() {
                    push_json!(FileStorage, from_row);
                } else {
                    push_json!(FileStorage, from_row_lite);
                }
            },
            t if t == crate::sam::crawler::CrawlJob::sql_table_name() => { push_json!(crate::sam::crawler::CrawlJob, from_row); },
            t if t == crate::sam::crawler::CrawledPage::sql_table_name() => { push_json!(crate::sam::crawler::CrawledPage, from_row); },
            _ => {}
        }
        Ok(())
    }

    /// Creates and returns a synchronous PostgreSQL client using the current configuration.
    ///
    /// This function builds a TLS connector (with certificate verification disabled for self-signed certs)
    /// and connects to the configured PostgreSQL database using the `postgres` crate.
    ///
    /// # Returns
    /// Returns a `Result` containing a `postgres::Client` on success, or an error otherwise.
    pub fn client() -> Result<postgres::Client> {
        let config = Config::new();

        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());

        // Construct the connection string
        let conn_str = format!(
            "postgresql://{}:{}@{}/{}?sslmode=prefer",
            config.postgres.username,
            config.postgres.password,
            config.postgres.address,
            config.postgres.db_name
        );

        // Connect and return the client
        Ok(postgres::Client::connect(&conn_str, connector)?)
    }

    /// Checks if PostgreSQL (psql) is installed and available in PATH.
    pub fn check_postgres_installed() -> bool {
        match Command::new("psql")
            .arg("--version")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return false;
                }
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Check for error messages indicating missing server/socket
                let error_patterns = [
                    "Is the server running locally",
                    "could not connect to server",
                ];
                for pattern in error_patterns.iter() {
                    if stdout.contains(pattern) || stderr.contains(pattern) {
                        return false;
                    }
                }
                true
            }
            Err(_) => false,
        }
    }

    /// Creates the user 'sam' and database 'sam' if they do not exist.
    /// Requires superuser privileges (may prompt for password).
    pub fn create_user_and_database(user: &str) -> Result<()> {
        // Create user 'sam' if not exists
        let create_user = "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sam') THEN CREATE ROLE sam LOGIN PASSWORD 'sam'; END IF; END $$;";
        let status_user = Command::new("psql")
            .arg("-U")
            .arg(user)
            .arg("-c")
            .arg(create_user)
            .status()?;
        if !status_user.success() {
            log::warn!("Could not create user 'sam' (may already exist or insufficient privileges)");
        }

        // Create database 'sam' owned by 'sam' if not exists
        // NOTE: CREATE DATABASE cannot be run inside DO/PLPGSQL blocks.
        // So we must check existence and run CREATE DATABASE as a separate statement.
        let check_db = "SELECT 1 FROM pg_database WHERE datname = 'sam';";
        let output = Command::new("psql")
            .arg("-U")
            .arg(user)
            .arg("-tAc")
            .arg(check_db)
            .output()?;
        let db_exists = String::from_utf8_lossy(&output.stdout).trim() == "1";
        if !db_exists {
            let status_db = Command::new("psql")
                .arg("-U")
                .arg(user)
                .arg("-c")
                .arg("CREATE DATABASE sam OWNER sam;")
                .status()?;
            if !status_db.success() {
                log::warn!("Could not create database 'sam' (may already exist or insufficient privileges)");
            }
        } else {
            log::info!("Database 'sam' already exists");
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedWikipediaSummary {
    pub id: i32,
    pub oid: String,
    pub topics: Vec<String>,
    pub summary: String,
    pub timestamp: i64
}
impl Default for CachedWikipediaSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedWikipediaSummary {
    pub fn new() -> CachedWikipediaSummary {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let topics: Vec<String> = Vec::new();
        CachedWikipediaSummary { 
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
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    fn from_row(row: &Row) -> Result<Self> {


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

// A human can have many face encodings for accuracy
// A human may or may not have an email address
// Unknown humans will be assiged a name
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Human {
    pub id: i32,
    pub oid: String,
    pub name: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub phone_number: Option<String>,
    pub heard_count: i64,
    pub seen_count: i64,
    pub authorization_level: i64,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Human {
    fn default() -> Self {
        Self::new()
    }
}

impl Human {
    pub fn new() -> Human {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        Human { 
            id: 0,
            oid: oid.clone(),
            name: format!("unknown-{}", oid), 
            email: None,
            password: None,
            phone_number: None,
            heard_count: 0,
            seen_count: 0,
            authorization_level: 0,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "humans".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.humans (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            email varchar NULL,
            password varchar NULL,
            phone_number varchar NULL,
            heard_count BIGINT NULL,
            seen_count BIGINT NULL,
            authorization_level BIGINT NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT humans_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.humans ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.humans ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.humans ADD COLUMN updated_at BIGINT NULL;"
        ]
    }
    pub fn count() -> Result<i64>{

        let mut client = Config::client()?;


        let execquery = format!("SELECT COUNT(*)
        FROM {}", Self::sql_table_name());


        let mut counter: i64 = 0;
        for row in client.query(execquery.as_str(), &[])? {
           counter = row.get("count");
        }

        match client.close(){
            Ok(_) => {},
            Err(e) => log::error!("failed to close connection to database: {}", e),
        }

        Ok(counter)
    }
    pub fn save(&self) -> Result<&Self>{
        let mut client = Config::client()?;
        
        // Search for OID matches
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


            client.execute("INSERT INTO humans (oid, name, heard_count, seen_count, authorization_level, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[  &self.oid.clone(),
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.created_at,
                    &self.updated_at
                ]
            ).unwrap();

            if self.phone_number.is_some() {
                client.execute("UPDATE humans SET phone_number = $1 WHERE oid = $2;", 
                &[
                    &self.phone_number.clone().unwrap(),
                    &self.oid
                ])?;
            }

            if self.email.is_some() {
                client.execute("UPDATE humans SET email = $1 WHERE oid = $2;", 
                &[
                    &self.email.clone().unwrap(),
                    &self.oid
                ])?;
            }

            if self.password.is_some() {
                client.execute("UPDATE humans SET password = $1 WHERE oid = $2;", 
                &[
                    &self.password.clone().unwrap(),
                    &self.oid
                ])?;
            }
    
   
            
            Ok(self)
        
         
        } else {
            // TODO Impliment Update

            let ads = rows[0].clone();


            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE humans SET name = $1, heard_count = $2, seen_count = $3, authorization_level = $4, updated_at = $5 WHERE oid = $6;", 
                &[
                    &self.name,
                    &self.heard_count,
                    &self.seen_count,
                    &self.authorization_level,
                    &self.updated_at,
                    &ads.oid
                ])?;


                if self.phone_number.is_some() {
                    client.execute("UPDATE humans SET phone_number = $1 WHERE oid = $2;", 
                    &[
                        &self.phone_number.clone().unwrap(),
                        &ads.oid
                    ])?;
                }

                if self.email.is_some() {
                    client.execute("UPDATE humans SET email = $1 WHERE oid = $2;", 
                    &[
                        &self.email.clone().unwrap(),
                        &ads.oid
                    ])?;
                }

                if self.password.is_some() {
                    client.execute("UPDATE humans SET password = $1 WHERE oid = $2;", 
                    &[
                        &self.password.clone().unwrap(),
                        &self.oid
                    ])?;
                }
        
            }


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
    fn from_row(row: &Row) -> Result<Self> {

        let sql_email: Option<String> = row.get("email");

        let sql_password: Option<String> = row.get("password");

        let sql_phone_number: Option<String> = row.get("phone_number");

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"), 
            email: sql_email,
            password: sql_password,
            phone_number: sql_phone_number,
            heard_count: row.get("heard_count"),
            seen_count: row.get("seen_count"),
            authorization_level: row.get("authorization_level"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "humans".to_string())
    }
}

// Face encodings for humans
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HumanFaceEncoding {
    pub id: i32,
    pub oid: String,
    pub encoding: Vec<u8>,
    pub human_oid: String,
    pub timestamp: i64
}
impl Default for HumanFaceEncoding {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanFaceEncoding {
    pub fn new() -> HumanFaceEncoding {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let encoding: Vec<u8> = Vec::new();
        HumanFaceEncoding { 
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
    fn from_row(row: &Row) -> Result<Self> {




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

// Locations can have many rooms
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    pub id: i32,
    pub oid: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub lifx_api_key: Option<String>,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Location {
    fn default() -> Self {
        Self::new()
    }
}

impl Location {
    pub fn new() -> Location {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        Location { 
            id: 0,
            oid,
            name: String::new(), 
            address: String::new(),
            city: String::new(),
            state: String::new(),
            zip_code: String::new(),
            lifx_api_key: None,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "locations".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.locations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            address varchar NULL,
            city varchar NULL,
            state varchar NULL,
            zip_code varchar NULL,
            lifx_api_key varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT locations_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.locations ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.locations ADD COLUMN updated_at BIGINT NULL;",
            "ALTER TABLE public.locations ADD COLUMN lifx_api_key VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN city VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN state VARCHAR NULL;",
            "ALTER TABLE public.locations ADD COLUMN zip_code VARCHAR NULL;"
        ]
    }
    pub fn count() -> Result<i64>{

        let mut client = Config::client()?;

        let execquery = format!("SELECT COUNT(*)
        FROM {}", Self::sql_table_name());

        let mut counter: i64 = 0;
        for row in client.query(execquery.as_str(), &[])? {
           counter = row.get("count");
        }

        match client.close(){
            Ok(_) => {},
            Err(e) => log::error!("failed to close connection to database: {}", e),
        }

        Ok(counter)
    }
    pub fn save(&self) -> Result<&Self>{

        let mut client = Config::client()?;

        // Search for OID matches
        let statement = client.prepare("SELECT * FROM locations WHERE oid = $1 OR name ilike $2")?;
        let rows = client.query(&statement, &[
            &self.oid, 
            &self.name,
        ])?;

        if rows.is_empty() {
            client.execute("INSERT INTO locations (oid, name, address, city, state, zip_code, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);",
                &[&self.oid.clone(),
                &self.name,
                &self.address,
                &self.city,
                &self.state,
                &self.zip_code,
                &self.created_at,
                &self.updated_at]
            ).unwrap();


            if self.lifx_api_key.is_some() {
                client.execute("UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;", 
                &[
                    &self.lifx_api_key.clone().unwrap(),
                    &self.oid
                ])?;
            }

            
            let statement = client.prepare("SELECT * FROM locations WHERE oid = $1")?;
            let _rows_two = client.query(&statement, &[
                &self.oid, 
            ])?;
        
            Ok(self)
        
        } else {
            let ads = Self::from_row(&rows[0]).unwrap();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
   
                client.execute("UPDATE locations SET name = $1, address = $2, city = $3, state = $4, zip_code = $5, updated_at = $6 WHERE oid = $7;", 
                &[
                    &self.name,
                    &self.address,
                    &self.city,
                    &self.state,
                    &self.zip_code,
                    &self.updated_at,
                    &ads.oid
                ])?;


                if self.lifx_api_key.is_some() {
                    client.execute("UPDATE locations SET lifx_api_key = $1 WHERE oid = $2;", 
                    &[
                        &self.lifx_api_key.clone().unwrap(),
                        &ads.oid
                    ])?;
                }

            }

            let statement_two = client.prepare("SELECT * FROM locations WHERE oid = $1")?;
            let _rows_two = client.query(&statement_two, &[
                &self.oid, 
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
    fn from_row(row: &Row) -> Result<Self> {


        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"), 
            address: row.get("address"), 
            city: row.get("city"), 
            state: row.get("state"), 
            zip_code: row.get("zip_code"),
            lifx_api_key: row.get("lifx_api_key"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "locations".to_string())
    }
}



// TODO: Add progress_bar {}
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
    fn from_row(row: &Row) -> Result<Self> {
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








#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub id: i32,
    pub oid: String,
    pub name: String,
    pub icon: String,
    pub location_oid: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
}

impl Room {
    pub fn new() -> Room {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        Room { 
            id: 0,
            oid,
            name: String::new(), 
            icon: "fa fa-solid fa-cube".to_string(),
            location_oid: String::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "rooms".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.rooms (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            icon varchar NULL,
            location_oid varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT rooms_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.rooms ADD COLUMN icon varchar NULL;",
            "ALTER TABLE public.rooms ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.rooms ADD COLUMN updated_at BIGINT NULL;"
        ]
    }
    pub fn save(&self) -> Result<&Self>{

        let mut client = Config::client()?;
        
        // Search for OID matches
        let statement = client.prepare("SELECT * FROM rooms WHERE oid = $1 OR (location_oid = $2 AND name = $3)")?;
        let rows = client.query(&statement, &[
            &self.oid, 
            &self.location_oid,
            &self.name,
        ])?;

        if rows.is_empty() {
            client.execute("INSERT INTO rooms (oid, name, icon, location_oid, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                &[&self.oid.clone(),
                &self.name,
                &self.icon,
                &self.location_oid,
                &self.created_at,
                &self.updated_at]
            ).unwrap();
        } else {
            let ads = Self::from_row(&rows[0]).unwrap();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE rooms SET name = $1, icon = $2, location_oid = $3 WHERE oid = $4;", 
                &[
                    &self.name,
                    &self.icon,
                    &self.location_oid,
                    &ads.oid
                ])?;
            }
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
    fn from_row(row: &Row) -> Result<Self> {

        let mut icon: String = "fa fa-solid fa-cube".to_string();

        if let Some(val) = row.get("icon") {
            icon = val;
        }

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"), 
            icon, 
            location_oid: row.get("location_oid"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "rooms".to_string())
    }
}

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
    fn from_row(row: &Row) -> Result<Self> {


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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thing {
    pub id: i32,
    pub oid: String,
    pub name: String,
    pub room_oid: String,
    pub thing_type: String, // lifx, rtsp, etc
    pub username: String,
    pub password: String,
    pub ip_address: String,
    pub online_identifiers: Vec<String>,
    pub local_identifiers: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for Thing {
    fn default() -> Self {
        Self::new()
    }
}

impl Thing {
    pub fn new() -> Thing {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        let empty_vec: Vec<String> = Vec::new();
        Thing { 
            id: 0,
            oid,
            name: String::new(), 
            room_oid: String::new(),
            thing_type: String::new(),
            username: String::new(), 
            password: String::new(), 
            ip_address: String::new(), 
            online_identifiers: empty_vec.clone(),
            local_identifiers: empty_vec.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "things".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.things (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            name varchar NULL,
            room_oid varchar NULL,
            thing_type varchar NULL,
            username varchar NULL,
            password varchar NULL,
            ip_address varchar NULL,
            online_identifiers varchar NULL,
            local_identifiers varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT things_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.things ADD COLUMN username varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN password varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN ip_address varchar NULL;",
            "ALTER TABLE public.things ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.things ADD COLUMN updated_at BIGINT NULL;"
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
            Some(pg_query)
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO things (oid, name, room_oid, thing_type, username, password, ip_address, online_identifiers, local_identifiers, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                &[&self.oid.clone(),
                &self.name,
                &self.room_oid,
                &self.thing_type,
                &self.username,
                &self.password,
                &self.ip_address,
                &self.online_identifiers.join(","),
                &self.local_identifiers.join(","),
                &self.created_at,
                &self.updated_at]
            )?;        
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE things SET name = $1, room_oid = $2, thing_type = $3, username = $4, password = $5, ip_address = $6, online_identifiers = $7, local_identifiers = $8 WHERE oid = $9;", 
                &[
                    &self.name,
                    &self.room_oid,
                    &self.thing_type,
                    &self.username,
                    &self.password,
                    &self.ip_address,
                    &self.online_identifiers.join(","),
                    &self.local_identifiers.join(","),
                    &ads.oid
                ])?;
            }
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
    fn from_row(row: &Row) -> Result<Self> {
        let mut online_identifiers: Vec<String> = Vec::new();
        let sql_online_identifiers: Option<String> = row.get("online_identifiers");
        if let Some(ts) = sql_online_identifiers {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            online_identifiers = newvec;
        }  
            

           
        let mut local_identifiers: Vec<String> = Vec::new();
        let sql_local_identifiers: Option<String> = row.get("local_identifiers");
        if let Some(ts) = sql_local_identifiers {
            let split = ts.split(',');
            let vec = split.collect::<Vec<&str>>();
            let mut newvec: Vec<String> = Vec::new();
            for v in vec{
                newvec.push(v.to_string());
            }
            local_identifiers = newvec;
        }  

        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            name: row.get("name"), 
            room_oid: row.get("room_oid"),
            thing_type: row.get("thing_type"),
            username: row.get("username"),
            password: row.get("password"),
            ip_address: row.get("ip_address"),
            online_identifiers,
            local_identifiers,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "things".to_string())
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Observation {
    pub id: i32,
    pub oid: String,
    pub timestamp: i64,
    pub observation_type: ObservationType,
    pub observation_objects: Vec<ObservationObjects>,
    pub observation_humans: Vec<Human>,
    pub observation_notes: Vec<String>,
    pub observation_file: Option<Vec<u8>>,
    pub deep_vision: Vec<DeepVisionResult>,
    pub deep_vision_json: Option<String>,
    pub thing: Option<Thing>,
    pub web_session: Option<WebSessions>,
}
impl Default for Observation {
    fn default() -> Self {
        Self::new()
    }
}

impl Observation {
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
    pub fn sql_table_name() -> String {
        "observations".to_string()
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.observations ADD COLUMN observation_file bytea NULL;",
            "ALTER TABLE public.observations ADD COLUMN deep_vision_json varchar NULL;",
            "ALTER TABLE public.observations ADD COLUMN thing_oid varchar NULL;",
            "ALTER TABLE public.observations ADD COLUMN web_session_id varchar NULL;",
        ]
    }
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
    pub fn save(&self) -> Result<Self>{

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
    pub fn select(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    pub fn select_lite(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = Config::pg_select(Self::sql_table_name(), Some("id, oid, timestamp, observation_type, observation_objects, observation_humans, observation_notes, deep_vision_json".to_string()), limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    fn from_row(row: &Row) -> Result<Self> {

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
    fn from_row_lite(row: &Row) -> Result<Self> {

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
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "observations".to_string())
    }
}


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
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    fn from_row(row: &Row) -> Result<Self> {
     

           
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageLocation {
    pub id: i32,
    pub oid: String,
    pub storage_type: String, // unique
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for StorageLocation {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageLocation {
    pub fn new() -> StorageLocation {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        StorageLocation { 
            id: 0,
            oid,
            storage_type: String::new(), 
            endpoint: String::new(), 
            username: String::new(), 
            password: String::new(), 
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        }
    }
    pub fn sql_table_name() -> String {
        "storage_locations".to_string()
    }
    pub fn sql_build_statement() -> &'static str {
        "CREATE TABLE public.storage_locations (
            id serial NOT NULL,
            oid varchar NOT NULL UNIQUE,
            storage_type varchar NULL,
            endpoint varchar NULL,
            username varchar NULL,
            password varchar NULL,
            created_at BIGINT NULL,
            updated_at BIGINT NULL,
            CONSTRAINT storage_locations_pkey PRIMARY KEY (id));"
    }
    pub fn migrations() -> Vec<&'static str> {
        vec![
            "ALTER TABLE public.storage_locations ADD COLUMN created_at BIGINT NULL;",
            "ALTER TABLE public.storage_locations ADD COLUMN updated_at BIGINT NULL;"
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
            Some(pg_query)
        ).unwrap();

        if rows.is_empty() {
            client.execute("INSERT INTO storage_locations (oid, storage_type, endpoint, username, password, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[&self.oid.clone(),
                &self.storage_type,
                &self.endpoint,
                &self.username,
                &self.password,
                &self.created_at,
                &self.updated_at]
            )?;        
            Ok(self)
        
        } else {
            let ads = rows[0].clone();

            // Only save if newer than stored information
            if self.updated_at > ads.updated_at {
                client.execute("UPDATE storage_locations SET storage_type = $1, endpoint = $2, username = $3, password = $4, updated_at = $5 WHERE oid = $6;", 
                &[
                    &self.storage_type,
                    &self.endpoint,
                    &self.username,
                    &self.password,
                    &self.updated_at,
                    &ads.oid
                ])?;


             
            }

   
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
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id"),
            oid: row.get("oid"),
            storage_type: row.get("storage_type"), 
            endpoint: row.get("endpoint"), 
            username: row.get("username"), 
            password: row.get("password"), 
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at")
        })
    }
    pub fn destroy(oid: String) -> Result<bool>{
        crate::sam::memory::Config::destroy_row(oid, "storage_locations".to_string())
    }
}

pub struct FileMetadataPermissions {
    pub shared_with_humans: Vec<String>,
    pub public: bool,
    pub public_url: String
}

pub struct FileMetadata {
    pub file_name: String,
    pub mime_type: String,
    pub owner: String,
    pub permissions: FileMetadataPermissions,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileStorage {
    pub id: i32,
    pub oid: String,
    pub file_name: String, // unique
    pub file_type: String,
    pub file_data: Option<Vec<u8>>,
    pub file_folder_tree: Option<Vec<String>>,
    pub storage_location_oid: String,
    pub created_at: i64,
    pub updated_at: i64
}
impl Default for FileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorage {
    pub fn new() -> FileStorage {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        FileStorage { 
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
        let jsons = crate::sam::memory::Config::pg_select(Self::sql_table_name(), None, limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    pub fn select_lite(limit: Option<usize>, offset: Option<usize>, order: Option<String>, query: Option<PostgresQueries>) -> Result<Vec<Self>>{
        let mut parsed_rows: Vec<Self> = Vec::new();
        let jsons = Config::pg_select(Self::sql_table_name(), Some("id, oid, file_name, file_type, file_folder_tree, storage_location_oid, created_at, updated_at".to_string()), limit, offset, order, query)?;

        for j in jsons{
            let object: Self = serde_json::from_str(&j).unwrap();
            parsed_rows.push(object);
        }
        

        Ok(parsed_rows)
    }
    fn from_row(row: &Row) -> Result<Self> {

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
    fn from_row_lite(row: &Row) -> Result<Self> {

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
        let files_without_data = FileStorage::select_lite(None, None, None, None)?;

        for file in files_without_data{

            if !Path::new(file.path_on_disk().as_str()).exists(){


                if file.storage_location_oid == *"SQL"{
                    let mut pg_query = PostgresQueries::default();
                    pg_query.queries.push(crate::sam::memory::PGCol::String(file.oid.clone()));
                    pg_query.query_columns.push("oid =".to_string());
        
                    let files_with_data = FileStorage::select(None, None, None, Some(pg_query))?;
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
    fn from_row(row: &Row) -> Result<Self> {


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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresServer {
	pub db_name: String,
    pub username: String,
    pub password: String,
	pub address: String
}
impl Default for PostgresServer {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresServer {
    pub fn new() -> PostgresServer {

        let db_name = env::var("PG_DBNAME").expect("$PG_DBNAME is not set");
        let username = env::var("PG_USER").expect("$PG_USER is not set");
        let password = env::var("PG_PASS").expect("$PG_PASS is not set");
        let address = env::var("PG_ADDRESS").expect("$PG_ADDRESS is not set");


        PostgresServer{
            db_name, 
            username, 
            password, 
            address
        }
    }
}

// Not tracked in SQL
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PostgresQueries {
    pub queries: Vec<PGCol>, 
    pub query_columns: Vec<String>,
    pub append: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PGCol {
    String(String),
    Number(i32),
    Boolean(bool),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeepVisionResult {
    pub id: String,
    pub whoio: Option<WhoioResult>,
    pub probability: f64,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WhoioResult {
    pub id: String,
    pub directory: String,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ObservationType {
    UNKNOWN,
    SEEN,
    HEARD
}
impl fmt::Display for ObservationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::str::FromStr for ObservationType {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<ObservationType, Self::Err> {
        match input {
            "UNKNOWN"  => Ok(ObservationType::UNKNOWN),
            "SEEN"  => Ok(ObservationType::SEEN),
            "HEARD"  => Ok(ObservationType::HEARD),
            _      => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ObservationObjects {
    #[allow(non_camel_case_types)]
    QR_CODE,
    #[allow(non_camel_case_types)]
    PERSON,
    #[allow(non_camel_case_types)]
    BICYCLE,
    #[allow(non_camel_case_types)]
    CAR,
    #[allow(non_camel_case_types)]
    MOTORBIKE,
    #[allow(non_camel_case_types)]
    AEROPLANE,
    #[allow(non_camel_case_types)]
    BUS,
    #[allow(non_camel_case_types)]
    TRAIN,
    #[allow(non_camel_case_types)]
    TRUCK,
    #[allow(non_camel_case_types)]
    BOAT,
    #[allow(non_camel_case_types)]
    TRAFFIC_LIGHT,
    #[allow(non_camel_case_types)]
    FIRE_HYDRANT,
    #[allow(non_camel_case_types)]
    STOP_SIGN,
    #[allow(non_camel_case_types)]
    PARKING_METER,
    #[allow(non_camel_case_types)]
    BENCH,
    #[allow(non_camel_case_types)]
    BIRD,
    #[allow(non_camel_case_types)]
    CAT,
    #[allow(non_camel_case_types)]
    DOG,
    #[allow(non_camel_case_types)]
    HORSE,
    #[allow(non_camel_case_types)]
    SHEEP,
    #[allow(non_camel_case_types)]
    COW,
    #[allow(non_camel_case_types)]
    ELEPHANT,
    #[allow(non_camel_case_types)]
    BEAR,
    #[allow(non_camel_case_types)]
    ZEBRA,
    #[allow(non_camel_case_types)]
    GIRAFFE,
    #[allow(non_camel_case_types)]
    BACKPACK,
    #[allow(non_camel_case_types)]
    UMBRELLA,
    #[allow(non_camel_case_types)]
    HANDBAG,
    #[allow(non_camel_case_types)]
    TIE,
    #[allow(non_camel_case_types)]
    SUITCASE,
    #[allow(non_camel_case_types)]
    FRISBEE,
    #[allow(non_camel_case_types)]
    SKIS,
    #[allow(non_camel_case_types)]
    SNOWBOARD,
    #[allow(non_camel_case_types)]
    SPORTS_BALL,
    #[allow(non_camel_case_types)]
    KITE,
    #[allow(non_camel_case_types)]
    BASEBALL_BAT,
    #[allow(non_camel_case_types)]
    SKATEBOARD,
    #[allow(non_camel_case_types)]
    SURFBOARD,
    #[allow(non_camel_case_types)]
    TENNIS_RACKET,
    #[allow(non_camel_case_types)]
    BOTTLE,
    #[allow(non_camel_case_types)]
    WINE_GLASS,
    #[allow(non_camel_case_types)]
    CUP,
    #[allow(non_camel_case_types)]
    FORK,
    #[allow(non_camel_case_types)]
    KNIFE,
    #[allow(non_camel_case_types)]
    SPOON,
    #[allow(non_camel_case_types)]
    BOWL,
    #[allow(non_camel_case_types)]
    BANANA,
    #[allow(non_camel_case_types)]
    APPLE,
    #[allow(non_camel_case_types)]
    SANDWICH,
    #[allow(non_camel_case_types)]
    ORANGE,
    #[allow(non_camel_case_types)]
    BROCCOLI,
    #[allow(non_camel_case_types)]
    CARROT,
    #[allow(non_camel_case_types)]
    HOT_DOG,
    #[allow(non_camel_case_types)]
    PIZZA,
    #[allow(non_camel_case_types)]
    DONUT,
    #[allow(non_camel_case_types)]
    CAKE,
    #[allow(non_camel_case_types)]
    CHAIR,
    #[allow(non_camel_case_types)]
    SOFA,
    #[allow(non_camel_case_types)]
    POTTED_PLANT,
    #[allow(non_camel_case_types)]
    BED,
    #[allow(non_camel_case_types)]
    DINING_TABLE,
    #[allow(non_camel_case_types)]
    TOILET,
    #[allow(non_camel_case_types)]
    TV_MONITOR,
    #[allow(non_camel_case_types)]
    LAPTOP,
    #[allow(non_camel_case_types)]
    MOUSE,
    #[allow(non_camel_case_types)]
    REMOTE,
    #[allow(non_camel_case_types)]
    KEYBOARD,
    #[allow(non_camel_case_types)]
    CELL_PHONE,
    #[allow(non_camel_case_types)]
    MICROWAVE,
    #[allow(non_camel_case_types)]
    OVEN,
    #[allow(non_camel_case_types)]
    TOASTER,
    #[allow(non_camel_case_types)]
    SINK,
    #[allow(non_camel_case_types)]
    REFRIGERATOR,
    #[allow(non_camel_case_types)]
    BOOK,
    #[allow(non_camel_case_types)]
    CLOCK,
    #[allow(non_camel_case_types)]
    VASE,
    #[allow(non_camel_case_types)]
    SCISSORS,
    #[allow(non_camel_case_types)]
    TEDDY_BEAR,
    #[allow(non_camel_case_types)]
    HAIR_DRIER,
    #[allow(non_camel_case_types)]
    TOOTHBRUSH
}
impl fmt::Display for ObservationObjects {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::str::FromStr for ObservationObjects {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<ObservationObjects, Self::Err> {
        match input {
            "QR_CODE"  => Ok(ObservationObjects::QR_CODE),
            "PERSON"  => Ok(ObservationObjects::PERSON),
            "BICYCLE"  => Ok(ObservationObjects::BICYCLE),
            "CAR"  => Ok(ObservationObjects::CAR),
            "MOTORBIKE"  => Ok(ObservationObjects::MOTORBIKE),
            "AEROPLANE"  => Ok(ObservationObjects::AEROPLANE),
            "BUS"  => Ok(ObservationObjects::BUS),
            "TRAIN"  => Ok(ObservationObjects::TRAIN),
            "TRUCK"  => Ok(ObservationObjects::TRUCK),
            "BOAT"  => Ok(ObservationObjects::BOAT),
            "TRAFFIC_LIGHT"  => Ok(ObservationObjects::TRAFFIC_LIGHT),
            "FIRE_HYDRANT"  => Ok(ObservationObjects::FIRE_HYDRANT),
            "STOP_SIGN"  => Ok(ObservationObjects::STOP_SIGN),
            "PARKING_METER"  => Ok(ObservationObjects::PARKING_METER),
            "BENCH"  => Ok(ObservationObjects::BENCH),
            "BIRD"  => Ok(ObservationObjects::BIRD),
            "CAT"  => Ok(ObservationObjects::CAT),
            "DOG"  => Ok(ObservationObjects::DOG),
            "HORSE"  => Ok(ObservationObjects::HORSE),
            "SHEEP"  => Ok(ObservationObjects::SHEEP),
            "COW"  => Ok(ObservationObjects::COW),
            "ELEPHANT"  => Ok(ObservationObjects::ELEPHANT),
            "BEAR"  => Ok(ObservationObjects::BEAR),
            "ZEBRA"  => Ok(ObservationObjects::ZEBRA),
            "GIRAFFE"  => Ok(ObservationObjects::GIRAFFE),
            "BACKPACK"  => Ok(ObservationObjects::BACKPACK),
            "UMBRELLA"  => Ok(ObservationObjects::UMBRELLA),
            "HANDBAG"  => Ok(ObservationObjects::HANDBAG),
            "TIE"  => Ok(ObservationObjects::TIE),
            "SUITCASE"  => Ok(ObservationObjects::SUITCASE),
            "FRISBEE"  => Ok(ObservationObjects::FRISBEE),
            "SKIS"  => Ok(ObservationObjects::SKIS),
            "SNOWBOARD"  => Ok(ObservationObjects::SNOWBOARD),
            "SPORTS_BALL"  => Ok(ObservationObjects::SPORTS_BALL),
            "KITE"  => Ok(ObservationObjects::KITE),
            "BASEBALL_BAT"  => Ok(ObservationObjects::BASEBALL_BAT),
            "SKATEBOARD"  => Ok(ObservationObjects::SKATEBOARD),
            "SURFBOARD"  => Ok(ObservationObjects::SURFBOARD),
            "TENNIS_RACKET"  => Ok(ObservationObjects::TENNIS_RACKET),
            "BOTTLE"  => Ok(ObservationObjects::BOTTLE),
            "WINE_GLASS"  => Ok(ObservationObjects::WINE_GLASS),
            "CUP"  => Ok(ObservationObjects::CUP),
            "FORK"  => Ok(ObservationObjects::FORK),
            "KNIFE"  => Ok(ObservationObjects::KNIFE),
            "SPOON"  => Ok(ObservationObjects::SPOON),
            "BOWL"  => Ok(ObservationObjects::BOWL),
            "BANANA"  => Ok(ObservationObjects::BANANA),
            "APPLE"  => Ok(ObservationObjects::APPLE),
            "SANDWICH"  => Ok(ObservationObjects::SANDWICH),
            "ORANGE"  => Ok(ObservationObjects::ORANGE),
            "BROCCOLI"  => Ok(ObservationObjects::BROCCOLI),
            "CARROT"  => Ok(ObservationObjects::CARROT),
            "HOT_DOG"  => Ok(ObservationObjects::HOT_DOG),
            "PIZZA"  => Ok(ObservationObjects::PIZZA),
            "DONUT"  => Ok(ObservationObjects::DONUT),
            "CAKE"  => Ok(ObservationObjects::CAKE),
            "CHAIR"  => Ok(ObservationObjects::CHAIR),
            "SOFA"  => Ok(ObservationObjects::SOFA),
            "POTTED_PLANT"  => Ok(ObservationObjects::POTTED_PLANT),
            "BED"  => Ok(ObservationObjects::BED),
            "DINING_TABLE"  => Ok(ObservationObjects::DINING_TABLE),
            "TOILET"  => Ok(ObservationObjects::TOILET),
            "TV_MONITOR"  => Ok(ObservationObjects::TV_MONITOR),
            "LAPTOP"  => Ok(ObservationObjects::LAPTOP),
            "MOUSE"  => Ok(ObservationObjects::MOUSE),
            "REMOTE"  => Ok(ObservationObjects::REMOTE),
            "KEYBOARD"  => Ok(ObservationObjects::KEYBOARD),
            "CELL_PHONE"  => Ok(ObservationObjects::CELL_PHONE),
            "MICROWAVE"  => Ok(ObservationObjects::MICROWAVE),
            "OVEN"  => Ok(ObservationObjects::OVEN),
            "SINK"  => Ok(ObservationObjects::SINK),
            "REFRIGERATOR"  => Ok(ObservationObjects::REFRIGERATOR),
            "BOOK"  => Ok(ObservationObjects::BOOK),
            "CLOCK"  => Ok(ObservationObjects::CLOCK),
            "VASE"  => Ok(ObservationObjects::VASE),
            "SCISSORS"  => Ok(ObservationObjects::SCISSORS),
            "TEDDY_BEAR"  => Ok(ObservationObjects::TEDDY_BEAR),
            "HAIR_DRIER"  => Ok(ObservationObjects::HAIR_DRIER),
            "TOOTHBRUSH"  => Ok(ObservationObjects::TOOTHBRUSH),
            _      => Err(()),
        }
    }
}