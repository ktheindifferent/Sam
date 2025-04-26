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
use crate::sam::memory::{PostgresQueries, PGCol};
use crate::sam::memory::Result;
use crate::sam::memory::cache::{WebSessions};
use crate::sam::memory::human::{Human, FaceEncoding, Notification};
use crate::sam::memory::location::{Location};
use crate::sam::memory::room::Room;
use crate::sam::memory::PostgresServer;
use tokio::sync::MutexGuard;


pub mod service;
pub mod file_storage_location;
pub mod setting;

pub use service::Service;
pub use file_storage_location::FileStorageLocation;
pub use setting::Setting;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub postgres: crate::sam::memory::PostgresServer,
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
            version_installed: option_env!("CARGO_PKG_VERSION").unwrap_or("unknown").to_string()
        }
    }
    pub async fn init(&self){

        match self.create_db().await{
            Ok(_) => log::info!("Database created successfully"),
            Err(e) => log::debug!("failed to create database: {}", e),
        }

        match self.build_tables().await{
            Ok(_) => log::info!("Tables created successfully"),
            Err(e) => log::debug!("failed to create tables: {}", e),
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

    pub async fn connect(&self) -> Result<tokio_postgres::Client> {
        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());

        // Connect to the PostgreSQL database
        let (client, connection) = tokio_postgres::connect(
            &format!(
                "postgresql://{}:{}@{}/{}?sslmode=prefer",
                self.postgres.username, self.postgres.password, self.postgres.address, self.postgres.db_name
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

        Ok(client)
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
            (crate::sam::memory::cache::WikipediaSummary::sql_table_name(),
             crate::sam::memory::cache::WikipediaSummary::sql_build_statement(), 
             crate::sam::memory::cache::WikipediaSummary::migrations()),
            (crate::sam::memory::Human::sql_table_name(), crate::sam::memory::Human::sql_build_statement(), crate::sam::memory::Human::migrations()),
            (crate::sam::memory::human::FaceEncoding::sql_table_name(), crate::sam::memory::human::FaceEncoding::sql_build_statement(), crate::sam::memory::human::FaceEncoding::migrations()),
            (crate::sam::memory::location::Location::sql_table_name(), crate::sam::memory::location::Location::sql_build_statement(), crate::sam::memory::location::Location::migrations()),
            (crate::sam::memory::room::Room::sql_table_name(), crate::sam::memory::room::Room::sql_build_statement(), crate::sam::memory::room::Room::migrations()),
            (crate::sam::memory::config::Service::sql_table_name(), crate::sam::memory::config::Service::sql_build_statement(), crate::sam::memory::config::Service::migrations()),
            (crate::sam::memory::Thing::sql_table_name(), crate::sam::memory::Thing::sql_build_statement(), crate::sam::memory::Thing::migrations()),
            (crate::sam::memory::Observation::sql_table_name(), crate::sam::memory::Observation::sql_build_statement(), crate::sam::memory::Observation::migrations()),
            (crate::sam::memory::config::Setting::sql_table_name(), crate::sam::memory::config::Setting::sql_build_statement(), crate::sam::memory::config::Setting::migrations()),
            (crate::sam::memory::cache::WebSessions::sql_table_name(), crate::sam::memory::cache::WebSessions::sql_build_statement(), crate::sam::memory::cache::WebSessions::migrations()),
            (crate::sam::memory::config::FileStorageLocation::sql_table_name(), crate::sam::memory::config::FileStorageLocation::sql_build_statement(), crate::sam::memory::config::FileStorageLocation::migrations()),
            (crate::sam::memory::storage::File::sql_table_name(), crate::sam::memory::storage::File::sql_build_statement(), crate::sam::memory::storage::File::migrations()),
            (crate::sam::memory::human::Notification::sql_table_name(), crate::sam::memory::human::Notification::sql_build_statement(), crate::sam::memory::human::Notification::migrations()),
            (crate::sam::services::crawler::CrawlJob::sql_table_name(), crate::sam::services::crawler::CrawlJob::sql_build_statement(), crate::sam::services::crawler::CrawlJob::migrations()),
            (crate::sam::services::crawler::CrawledPage::sql_table_name(), crate::sam::services::crawler::CrawledPage::sql_build_statement(), crate::sam::services::crawler::CrawledPage::migrations()),
        ];

        let mut current_client = client;
        for (table_name, build_statement, migrations) in tables {
           
            // Run crawler index migrations after creating crawler tables
            if table_name == crate::sam::services::crawler::CrawlJob::sql_table_name() {
                for idx_sql in crate::sam::services::crawler::CrawlJob::sql_indexes() {
                    match current_client.batch_execute(idx_sql).await {
                        Ok(_) => log::info!("POSTGRES: Created index for '{}': {}", table_name, idx_sql),
                        Err(e) => log::debug!("POSTGRES: Failed to create index for '{}': {:?} ({})", table_name, idx_sql, e),
                    }
                }
            }
            if table_name == crate::sam::services::crawler::CrawledPage::sql_table_name() {
                for idx_sql in crate::sam::services::crawler::CrawledPage::sql_indexes() {
                    match current_client.batch_execute(idx_sql).await {
                        Ok(_) => log::info!("POSTGRES: Created index for '{}': {}", table_name, idx_sql),
                        Err(e) => log::debug!("POSTGRES: Failed to create index for '{}': {:?} ({})", table_name, idx_sql, e),
                    }
                }
            }
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
                    log::debug!("Failed to create database '{}': {}", self.postgres.db_name, e);
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
            Err(e) => log::debug!("POSTGRES: Failed to create '{}': {:?}", table_name, e),
        }

        // Apply migrations in order
        for migration in migrations {
            // Skip empty migration strings
            if migration.trim().is_empty() {
                continue;
            }
            match client.batch_execute(migration).await {
                Ok(_) => log::info!("POSTGRES: MIGRATED '{}' TABLE", table_name),
                Err(e) => log::debug!("POSTGRES: Migration failed for '{}': {:?}", table_name, e),
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
                log::debug!("connection error: {}", e);
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
        mut established_client: Option<postgres::Client>,
    ) -> Result<Vec<String>> {
        let mut client = if let Some(client) = established_client.take() {
            client
        } else {
            Config::client()?
        };

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
        if established_client.is_none() {
            if let Err(e) = client.close() {
                log::debug!("Failed to close PG-SQL Client: {}", e);
            }
        }

        Ok(parsed_rows)
    }

    /// Asynchronously executes a SELECT query on the specified PostgreSQL table with optional filtering, ordering, and pagination.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to query.
    /// * `columns` - Optional comma-separated list of columns to select. If `None`, selects all columns.
    /// * `limit` - Optional maximum number of rows to return.
    /// * `offset` - Optional number of rows to skip.
    /// * `order` - Optional ORDER BY clause (e.g., "id DESC").
    /// * `query` - Optional `PostgresQueries` for WHERE clause and parameterized values.
    /// * `established_client` - Optionally provide an already established tokio_postgres::Client.
    ///
    /// # Returns
    /// Returns a `Result` containing a vector of JSON strings, each representing a row.
    pub async fn pg_select_async(
        table_name: String,
        columns: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
        order: Option<String>,
        query: Option<PostgresQueries>,
        established_clients: Vec<std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>>,
    ) -> Result<Vec<String>> {

        let mut parsed_rows: Vec<String> = Vec::new();

        // Lock the client for the duration of the query
        // Try to acquire a lock on any of the provided clients
        let mut client_guard = None;
        for client_arc in &established_clients {
            if let Ok(guard) = client_arc.try_lock() {
            client_guard = Some(guard);
            break;
            }
        }
        let mut client_guard = match client_guard {
            Some(guard) => guard,
            None => {
            // If none are available, just wait for the first one
            established_clients
                .get(0)
                .expect("No clients provided")
                .lock()
                .await
            }
        };

        // Build SELECT clause
        let mut execquery = if let Some(cols) = &columns {
            format!("SELECT {} FROM {}", cols, table_name)
        } else {
            format!("SELECT * FROM {}", table_name)
        };

        // Build WHERE clause if query is provided
        if let Some(pg_query) = query.clone() {
            let mut counter = 1;
            for col in pg_query.query_columns.iter() {
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

        // Execute query with or without parameters
        if let Some(pg_query) = query {
            let query_values: Vec<_> = pg_query.queries.iter().map(|x| {
                match x {
                    PGCol::String(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                    PGCol::Number(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                    PGCol::Boolean(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                }
            }).collect();

            for row in client_guard.query(execquery.as_str(), query_values.as_slice()).await? {
                Self::serialize_row(&table_name, &columns, &row, &mut parsed_rows)?;
            }
        } else {
            for row in client_guard.query(execquery.as_str(), &[]).await? {
                Self::serialize_row(&table_name, &columns, &row, &mut parsed_rows)?;
            }
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
            t if t == crate::sam::memory::cache::WikipediaSummary::sql_table_name() => { push_json!(crate::sam::memory::cache::WikipediaSummary, from_row); },
            t if t == crate::sam::memory::Human::sql_table_name() => { push_json!(crate::sam::memory::Human, from_row); },
            t if t == crate::sam::memory::human::FaceEncoding::sql_table_name() => { push_json!(crate::sam::memory::human::FaceEncoding, from_row); },
            t if t == crate::sam::memory::Location::sql_table_name() => { push_json!(crate::sam::memory::Location, from_row); },
            t if t == crate::sam::memory::human::Notification::sql_table_name() => { push_json!(crate::sam::memory::human::Notification, from_row); },
            t if t == crate::sam::memory::Room::sql_table_name() => { push_json!(crate::sam::memory::Room, from_row); },
            t if t == crate::sam::memory::config::Service::sql_table_name() => { push_json!(crate::sam::memory::config::Service, from_row); },
            t if t == crate::sam::memory::Thing::sql_table_name() => { push_json!(crate::sam::memory::Thing, from_row); },
            t if t == crate::sam::memory::Observation::sql_table_name() => {
                if columns.is_none() {
                    push_json!(crate::sam::memory::Observation, from_row);
                } else {
                    push_json!(crate::sam::memory::Observation, from_row_lite);
                }
            },
            t if t == crate::sam::memory::config::Setting::sql_table_name() => { push_json!(crate::sam::memory::config::Setting, from_row); },
            t if t == crate::sam::memory::cache::WebSessions::sql_table_name() => { push_json!(crate::sam::memory::cache::WebSessions, from_row); },
            t if t == crate::sam::memory::config::FileStorageLocation::sql_table_name() => { push_json!(crate::sam::memory::config::FileStorageLocation, from_row); },
            t if t == crate::sam::memory::storage::File::sql_table_name() => {
                if columns.is_none() {
                    push_json!(crate::sam::memory::storage::File, from_row);
                } else {
                    push_json!(crate::sam::memory::storage::File, from_row_lite);
                }
            },
            t if t == crate::sam::services::crawler::CrawlJob::sql_table_name() => { push_json!(crate::sam::services::crawler::CrawlJob, from_row); },
            t if t == crate::sam::services::crawler::CrawledPage::sql_table_name() => { push_json!(crate::sam::services::crawler::CrawledPage, from_row); },
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

    /// Asynchronously creates and returns a tokio_postgres::Client using the current configuration.
    ///
    /// This function builds a TLS connector (with certificate verification disabled for self-signed certs)
    /// and connects to the configured PostgreSQL database using the `tokio_postgres` crate.
    ///
    /// # Returns
    /// Returns a `Result` containing a `tokio_postgres::Client` on success, or an error otherwise.
    pub async fn client_async() -> Result<tokio_postgres::Client> {
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
        let (client, connection) = tokio_postgres::connect(&conn_str, connector).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });
        Ok(client)
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