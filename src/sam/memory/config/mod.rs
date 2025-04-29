use crate::sam::memory::PostgresServer;
use crate::sam::memory::Result;
use crate::sam::memory::{PGCol, PostgresQueries};
use deadpool_postgres::Manager;
use deadpool_postgres::Pool;
use native_tls::TlsConnector;
use once_cell::sync::OnceCell;
use postgres_native_tls::MakeTlsConnector;
use rouille::Response;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::thread;
use tokio_postgres::Row;

pub mod file_storage_location;
pub mod service;
pub mod setting;

pub use file_storage_location::FileStorageLocation;
pub use service::Service;
pub use setting::Setting;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub postgres: crate::sam::memory::PostgresServer,
    pub version_installed: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Creates a new Config instance with default values.
    pub fn new() -> Config {
        Config {
            postgres: PostgresServer::new(),
            version_installed: option_env!("CARGO_PKG_VERSION")
                .unwrap_or("unknown")
                .to_string(),
        }
    }

    /// Initializes the PostgreSQL database and starts the HTTP server.
    /// TODO: Make http a service
    pub async fn init(&self) {
        match self.create_db().await {
            Ok(_) => log::info!("Database created successfully"),
            Err(e) => log::debug!("failed to create database: {}", e),
        }

        match self.build_tables().await {
            Ok(_) => log::info!("Tables created successfully"),
            Err(e) => log::debug!("failed to create tables: {}", e),
        }

        let _config = self.clone();
        thread::spawn(move || {
            rouille::start_server("0.0.0.0:8000".to_string().as_str(), move |request| {
                match crate::sam::http::handle(request) {
                    Ok(request) => request,
                    Err(err) => {
                        log::error!("HTTP_ERROR: {}", err);
                        Response::empty_404()
                    }
                }
            });
        });
    }

    /// Returns a new PostgreSQL client connection.
    pub async fn connect(&self) -> Result<tokio_postgres::Client> {
        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

        // Connect to the PostgreSQL database
        let (client, connection) = tokio_postgres::connect(
            &format!(
                "postgresql://{}:{}@{}/{}?sslmode=prefer",
                self.postgres.username,
                self.postgres.password,
                self.postgres.address,
                self.postgres.db_name
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

    /// Returns a new PostgreSQL client connection.
    /// Returns a new Deadpool PostgreSQL connection from a pool.
    /// Initializes a pool if not already created.
    /// This function is thread-safe and can be called from multiple threads.
    ///
    /// # Returns
    /// Returns a `Result` containing a `deadpool_postgres::Client` if successful, or an error otherwise.
    ///
    /// # Example
    /// ```
    /// use crate::sam::memory::Config;
    /// use deadpool_postgres::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = Config::new();
    ///     match config.connect_pool().await {
    ///         Ok(client) => println!("Connected to PostgreSQL"),
    ///         Err(e) => println!("Failed to connect: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn connect_pool(&self) -> Result<deadpool_postgres::Client> {
        static POOL: OnceCell<Pool> = OnceCell::new();

        let pool = POOL.get_or_init(|| {
            let connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let connector = MakeTlsConnector::new(connector);

            // let config_str = format!(
            //     "host={} user={} password={} dbname={} sslmode=prefer",
            //     self.postgres.address,
            //     self.postgres.username,
            //     self.postgres.password,
            //     self.postgres.db_name
            // );
            let mut pg_config = tokio_postgres::Config::new();
            pg_config
                .host(&self.postgres.address)
                .user(&self.postgres.username)
                .password(&self.postgres.password)
                .dbname(&self.postgres.db_name);

            let mgr = Manager::from_config(
                pg_config,
                connector,
                deadpool_postgres::ManagerConfig {
                    recycling_method: deadpool_postgres::RecyclingMethod::Fast,
                },
            );
            Pool::builder(mgr).max_size(16).build().unwrap()
        });

        let client = pool.get().await.map_err(crate::sam::memory::Error::from)?;
        Ok(client)
    }

    /// Builds all tables in the PostgreSQL database.
    ///
    /// This function connects to the PostgreSQL database and executes the SQL statements
    /// to create the tables defined in the `tables` array. It also applies any migrations
    /// specified for each table.
    ///
    /// # Returns
    /// Returns `Ok(())` if all tables were created successfully, or an error otherwise.
    ///
    /// # Errors
    /// Returns an error if the connection fails or any of the table creation or migrations fail.
    ///
    /// # Example
    /// ```
    /// use crate::sam::memory::Config;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = Config::new();
    ///     match config.build_tables().await {
    ///         Ok(_) => println!("Tables created successfully"),
    ///         Err(e) => println!("Failed to create tables: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    /// This function is intended to be called during the initialization phase of the application.
    /// It is responsible for setting up the database schema and ensuring that all required tables
    /// are present before the application starts handling requests.
    /// It is not intended to be called during normal operation.
    /// It is recommended to call this function only once during the application's lifecycle.
    /// It is also recommended to call this function only after the database has been created.
    pub async fn build_tables(&self) -> Result<()> {
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

        let (client, connection) = tokio_postgres::connect(
            format!(
                "postgresql://{}:{}@{}/{}?sslmode=prefer",
                &self.postgres.username,
                &self.postgres.password,
                &self.postgres.address,
                &self.postgres.db_name
            )
            .as_str(),
            connector,
        )
        .await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });

        // Build all tables in sequence
        let tables = [
            (
                crate::sam::memory::cache::WikipediaSummary::sql_table_name(),
                crate::sam::memory::cache::WikipediaSummary::sql_build_statement(),
                crate::sam::memory::cache::WikipediaSummary::migrations(),
            ),
            (
                crate::sam::memory::Human::sql_table_name(),
                crate::sam::memory::Human::sql_build_statement(),
                crate::sam::memory::Human::migrations(),
            ),
            (
                crate::sam::memory::human::FaceEncoding::sql_table_name(),
                crate::sam::memory::human::FaceEncoding::sql_build_statement(),
                crate::sam::memory::human::FaceEncoding::migrations(),
            ),
            (
                crate::sam::memory::location::Location::sql_table_name(),
                crate::sam::memory::location::Location::sql_build_statement(),
                crate::sam::memory::location::Location::migrations(),
            ),
            (
                crate::sam::memory::room::Room::sql_table_name(),
                crate::sam::memory::room::Room::sql_build_statement(),
                crate::sam::memory::room::Room::migrations(),
            ),
            (
                crate::sam::memory::config::Service::sql_table_name(),
                crate::sam::memory::config::Service::sql_build_statement(),
                crate::sam::memory::config::Service::migrations(),
            ),
            (
                crate::sam::memory::Thing::sql_table_name(),
                crate::sam::memory::Thing::sql_build_statement(),
                crate::sam::memory::Thing::migrations(),
            ),
            (
                crate::sam::memory::Observation::sql_table_name(),
                crate::sam::memory::Observation::sql_build_statement(),
                crate::sam::memory::Observation::migrations(),
            ),
            (
                crate::sam::memory::config::Setting::sql_table_name(),
                crate::sam::memory::config::Setting::sql_build_statement(),
                crate::sam::memory::config::Setting::migrations(),
            ),
            (
                crate::sam::memory::cache::WebSessions::sql_table_name(),
                crate::sam::memory::cache::WebSessions::sql_build_statement(),
                crate::sam::memory::cache::WebSessions::migrations(),
            ),
            (
                crate::sam::memory::config::FileStorageLocation::sql_table_name(),
                crate::sam::memory::config::FileStorageLocation::sql_build_statement(),
                crate::sam::memory::config::FileStorageLocation::migrations(),
            ),
            (
                crate::sam::memory::storage::File::sql_table_name(),
                crate::sam::memory::storage::File::sql_build_statement(),
                crate::sam::memory::storage::File::migrations(),
            ),
            (
                crate::sam::memory::human::Notification::sql_table_name(),
                crate::sam::memory::human::Notification::sql_build_statement(),
                crate::sam::memory::human::Notification::migrations(),
            ),
            (
                crate::sam::services::crawler::CrawlJob::sql_table_name(),
                crate::sam::services::crawler::CrawlJob::sql_build_statement(),
                crate::sam::services::crawler::CrawlJob::migrations(),
            ),
            (
                crate::sam::services::crawler::CrawledPage::sql_table_name(),
                crate::sam::services::crawler::CrawledPage::sql_build_statement(),
                crate::sam::services::crawler::CrawledPage::migrations(),
            ),
        ];

        let mut current_client = client;
        for (table_name, build_statement, migrations) in tables {
            // Run crawler index migrations after creating crawler tables
            if table_name == crate::sam::services::crawler::CrawlJob::sql_table_name() {
                for idx_sql in crate::sam::services::crawler::CrawlJob::sql_indexes() {
                    match current_client.batch_execute(idx_sql).await {
                        Ok(_) => {
                            log::info!("POSTGRES: Created index for '{}': {}", table_name, idx_sql)
                        }
                        Err(e) => log::debug!(
                            "POSTGRES: Failed to create index for '{}': {:?} ({})",
                            table_name,
                            idx_sql,
                            e
                        ),
                    }
                }
            }
            if table_name == crate::sam::services::crawler::CrawledPage::sql_table_name() {
                for idx_sql in crate::sam::services::crawler::CrawledPage::sql_indexes() {
                    match current_client.batch_execute(idx_sql).await {
                        Ok(_) => {
                            log::info!("POSTGRES: Created index for '{}': {}", table_name, idx_sql)
                        }
                        Err(e) => log::debug!(
                            "POSTGRES: Failed to create index for '{}': {:?} ({})",
                            table_name,
                            idx_sql,
                            e
                        ),
                    }
                }
            }
            current_client =
                Self::build_table(current_client, table_name, build_statement, migrations).await;
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
    ///
    /// # Example
    /// ```
    /// use crate::sam::memory::Config;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = Config::new();
    ///     match config.create_db().await {
    ///         Ok(_) => println!("Database created successfully"),
    ///         Err(e) => println!("Failed to create database: {}", e),
    ///     }
    /// }
    /// ```
    /// # Note
    /// This function is intended to be called during the initialization phase of the application.
    /// It is responsible for ensuring that the database exists before the application starts
    /// handling requests. It is not intended to be called during normal operation.
    /// It is recommended to call this function only once during the application's lifecycle.
    /// It is also recommended to call this function only after the PostgreSQL server is running.
    pub async fn create_db(&self) -> Result<()> {
        // Build a TLS connector that skips certificate verification (for self-signed certs)
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

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
                    log::debug!(
                        "Failed to create database '{}': {}",
                        self.postgres.db_name,
                        e
                    );
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
    ///
    /// # Errors
    /// Returns an error if the connection fails or any of the SQL statements fail.
    ///
    /// # Example
    /// ```
    /// use crate::sam::memory::Config;
    /// use tokio_postgres::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = Config::new();
    ///     let client = config.connect().await.unwrap();
    ///     let table_name = "example_table".to_string();
    ///     let build_statement = "CREATE TABLE example_table (id SERIAL PRIMARY KEY, name VARCHAR(50))";
    ///     let migrations = vec!["ALTER TABLE example_table ADD COLUMN age INT"];
    ///     match config.build_table(client, table_name, build_statement, migrations).await {
    ///         Ok(_) => println!("Table created and migrations applied successfully"),
    ///         Err(e) => println!("Failed to create table or apply migrations: {}", e),
    ///     }
    /// }
    /// ```
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
    ///
    /// # Errors
    /// Returns an error if the connection fails or the SQL statement fails.
    ///
    /// # Example
    /// ```
    /// use crate::sam::memory::Config;
    /// use tokio_postgres::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = Config::new();
    ///     let oid = "some_oid".to_string();
    ///     let table_name = "example_table".to_string();
    ///     match config.destroy_row(oid, table_name) {
    ///         Ok(deleted) => println!("Row deleted: {}", deleted),
    ///         Err(e) => println!("Failed to delete row: {}", e),
    ///     }
    /// }
    /// ```
    pub fn destroy_row(oid: String, table_name: String) -> Result<bool> {
        let mut client = Config::client()?;

        // Use parameterized queries to prevent SQL injection
        client.execute(&format!("DELETE FROM {table_name} WHERE oid = $1"), &[&oid])?;

        let rows = client.query(
            &format!("SELECT 1 FROM {table_name} WHERE oid = $1"),
            &[&oid],
        )?;

        Ok(rows.is_empty())
    }

    /// Asynchronously deletes a row from the specified table by OID.
    ///
    /// # Arguments
    /// * `oid` - The OID of the row to delete.
    /// * `table_name` - The name of the table from which to delete the row.
    /// * `client` - An established tokio_postgres::Client.
    ///
    /// # Returns
    /// Returns `Ok(true)` if the row was deleted (no longer exists), `Ok(false)` if the row still exists,
    /// or an error if the operation failed.
    ///
    /// # Errors
    /// Returns an error if the connection fails or the SQL statement fails.
    ///
    pub async fn destroy_row_async(oid: String, table_name: String) -> Result<bool> {
        let config = crate::sam::memory::Config::new();
        let client = config.connect_pool().await?;
        // Use parameterized queries to prevent SQL injection
        client
            .execute(&format!("DELETE FROM {table_name} WHERE oid = $1"), &[&oid])
            .await?;

        let rows = client
            .query(
                &format!("SELECT 1 FROM {table_name} WHERE oid = $1"),
                &[&oid],
            )
            .await?;

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
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

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
            format!("SELECT {cols} FROM {table_name}")
        } else {
            format!("SELECT * FROM {table_name}")
        };

        // Build WHERE clause if query is provided
        if let Some(pg_query) = query.clone() {
            let mut counter = 1;
            for col in pg_query.query_columns {
                if counter == 1 {
                    execquery = format!("{execquery} WHERE {col} ${counter}");
                } else {
                    execquery = format!("{execquery} {col} ${counter}");
                }
                counter += 1;
            }
        }

        // Add ORDER BY clause
        execquery = match order {
            Some(order_val) => format!("{execquery} ORDER BY {order_val}"),
            None => format!("{execquery} ORDER BY id DESC"),
        };

        // Add LIMIT and OFFSET
        if let Some(limit_val) = limit {
            execquery = format!("{execquery} LIMIT {limit_val}");
        }
        if let Some(offset_val) = offset {
            execquery = format!("{execquery} OFFSET {offset_val}");
        }

        // Prepare to collect results
        let mut parsed_rows: Vec<String> = Vec::new();

        // Execute query with or without parameters
        if let Some(pg_query) = query {
            let query_values: Vec<_> = pg_query
                .queries
                .iter()
                .map(|x| match x {
                    PGCol::String(y) => y as &(dyn postgres::types::ToSql + Sync),
                    PGCol::Number(y) => y as &(dyn postgres::types::ToSql + Sync),
                    PGCol::Boolean(y) => y as &(dyn postgres::types::ToSql + Sync),
                })
                .collect();

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
        client: deadpool_postgres::Client,
    ) -> Result<Vec<String>> {
        let mut parsed_rows: Vec<String> = Vec::new();

        // Build SELECT clause
        let mut execquery = if let Some(cols) = &columns {
            format!("SELECT {cols} FROM {table_name}")
        } else {
            format!("SELECT * FROM {table_name}")
        };

        // Build WHERE clause if query is provided
        if let Some(pg_query) = query.clone() {
            let mut counter = 1;
            for col in pg_query.query_columns.iter() {
                if counter == 1 {
                    execquery = format!("{execquery} WHERE {col} ${counter}");
                } else {
                    execquery = format!("{execquery} {col} ${counter}");
                }
                counter += 1;
            }
        }

        // Add ORDER BY clause
        execquery = match order {
            Some(order_val) => format!("{execquery} ORDER BY {order_val}"),
            None => format!("{execquery} ORDER BY id DESC"),
        };

        // Add LIMIT and OFFSET
        if let Some(limit_val) = limit {
            execquery = format!("{execquery} LIMIT {limit_val}");
        }
        if let Some(offset_val) = offset {
            execquery = format!("{execquery} OFFSET {offset_val}");
        }

        // Execute query with or without parameters
        if let Some(pg_query) = query {
            let query_values: Vec<_> = pg_query
                .queries
                .iter()
                .map(|x| match x {
                    PGCol::String(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                    PGCol::Number(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                    PGCol::Boolean(y) => y as &(dyn tokio_postgres::types::ToSql + Sync),
                })
                .collect();

            for row in client
                .query(execquery.as_str(), query_values.as_slice())
                .await?
            {
                Self::serialize_row_async(&table_name, &columns, &row, &mut parsed_rows).await?;
            }
        } else {
            for row in client.query(execquery.as_str(), &[]).await? {
                Self::serialize_row_async(&table_name, &columns, &row, &mut parsed_rows).await?;
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
            t if t == crate::sam::memory::cache::WikipediaSummary::sql_table_name() => {
                push_json!(crate::sam::memory::cache::WikipediaSummary, from_row);
            }
            t if t == crate::sam::memory::Human::sql_table_name() => {
                push_json!(crate::sam::memory::Human, from_row);
            }
            t if t == crate::sam::memory::human::FaceEncoding::sql_table_name() => {
                push_json!(crate::sam::memory::human::FaceEncoding, from_row);
            }
            t if t == crate::sam::memory::Location::sql_table_name() => {
                push_json!(crate::sam::memory::Location, from_row);
            }
            t if t == crate::sam::memory::human::Notification::sql_table_name() => {
                push_json!(crate::sam::memory::human::Notification, from_row);
            }
            t if t == crate::sam::memory::Room::sql_table_name() => {
                push_json!(crate::sam::memory::Room, from_row);
            }
            t if t == crate::sam::memory::config::Service::sql_table_name() => {
                push_json!(crate::sam::memory::config::Service, from_row);
            }
            t if t == crate::sam::memory::Thing::sql_table_name() => {
                push_json!(crate::sam::memory::Thing, from_row);
            }
            t if t == crate::sam::memory::Observation::sql_table_name() => {
                if columns.is_none() {
                    push_json!(crate::sam::memory::Observation, from_row);
                } else {
                    push_json!(crate::sam::memory::Observation, from_row_lite);
                }
            }
            t if t == crate::sam::memory::config::Setting::sql_table_name() => {
                push_json!(crate::sam::memory::config::Setting, from_row);
            }
            t if t == crate::sam::memory::cache::WebSessions::sql_table_name() => {
                push_json!(crate::sam::memory::cache::WebSessions, from_row);
            }
            t if t == crate::sam::memory::config::FileStorageLocation::sql_table_name() => {
                push_json!(crate::sam::memory::config::FileStorageLocation, from_row);
            }
            t if t == crate::sam::memory::storage::File::sql_table_name() => {
                if columns.is_none() {
                    push_json!(crate::sam::memory::storage::File, from_row);
                } else {
                    push_json!(crate::sam::memory::storage::File, from_row_lite);
                }
            }
            t if t == crate::sam::services::crawler::CrawlJob::sql_table_name() => {
                push_json!(crate::sam::services::crawler::CrawlJob, from_row);
            }
            t if t == crate::sam::services::crawler::CrawledPage::sql_table_name() => {
                push_json!(crate::sam::services::crawler::CrawledPage, from_row);
            }
            _ => {}
        }
        Ok(())
    }

    /// Asynchronous helper function to serialize a row to JSON based on table and columns.
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
    async fn serialize_row_async(
        table_name: &str,
        columns: &Option<String>,
        row: &tokio_postgres::Row,
        parsed_rows: &mut Vec<String>,
    ) -> Result<()> {
        macro_rules! push_json_async {
            ($ty:ty, $from_row_async:ident) => {
                parsed_rows
                    .push(serde_json::to_string(&<$ty>::$from_row_async(row).await?).unwrap())
            };
        }

        match table_name {
            t if t == crate::sam::memory::cache::WikipediaSummary::sql_table_name() => {
                push_json_async!(crate::sam::memory::cache::WikipediaSummary, from_row_async);
            }
            t if t == crate::sam::memory::Human::sql_table_name() => {
                push_json_async!(crate::sam::memory::Human, from_row_async);
            }
            t if t == crate::sam::memory::human::FaceEncoding::sql_table_name() => {
                push_json_async!(crate::sam::memory::human::FaceEncoding, from_row_async);
            }
            t if t == crate::sam::memory::Location::sql_table_name() => {
                push_json_async!(crate::sam::memory::Location, from_row_async);
            }
            t if t == crate::sam::memory::human::Notification::sql_table_name() => {
                push_json_async!(crate::sam::memory::human::Notification, from_row_async);
            }
            t if t == crate::sam::memory::Room::sql_table_name() => {
                push_json_async!(crate::sam::memory::Room, from_row_async);
            }
            t if t == crate::sam::memory::config::Service::sql_table_name() => {
                push_json_async!(crate::sam::memory::config::Service, from_row_async);
            }
            t if t == crate::sam::memory::Thing::sql_table_name() => {
                push_json_async!(crate::sam::memory::Thing, from_row_async);
            }
            t if t == crate::sam::memory::Observation::sql_table_name() => {
                if columns.is_none() {
                    push_json_async!(crate::sam::memory::Observation, from_row_async);
                } else {
                    push_json_async!(crate::sam::memory::Observation, from_row_lite_async);
                }
            }
            t if t == crate::sam::memory::config::Setting::sql_table_name() => {
                push_json_async!(crate::sam::memory::config::Setting, from_row_async);
            }
            t if t == crate::sam::memory::cache::WebSessions::sql_table_name() => {
                push_json_async!(crate::sam::memory::cache::WebSessions, from_row_async);
            }
            t if t == crate::sam::memory::config::FileStorageLocation::sql_table_name() => {
                push_json_async!(
                    crate::sam::memory::config::FileStorageLocation,
                    from_row_async
                );
            }
            t if t == crate::sam::memory::storage::File::sql_table_name() => {
                if columns.is_none() {
                    push_json_async!(crate::sam::memory::storage::File, from_row_async);
                } else {
                    push_json_async!(crate::sam::memory::storage::File, from_row_lite_async);
                }
            }
            t if t == crate::sam::services::crawler::CrawlJob::sql_table_name() => {
                push_json_async!(crate::sam::services::crawler::CrawlJob, from_row_async);
            }
            t if t == crate::sam::services::crawler::CrawledPage::sql_table_name() => {
                push_json_async!(crate::sam::services::crawler::CrawledPage, from_row_async);
            }
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
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

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
        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let connector = MakeTlsConnector::new(connector);

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
        match Command::new("psql").arg("--version").output() {
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

    #[cfg(target_os = "windows")]
    pub fn create_user_and_database(user: &str) -> Result<()> {
        println!("Creating user and database...");
        // Create user 'sam' if not exists
        let create_user = "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sam') THEN CREATE ROLE sam LOGIN PASSWORD 'sam'; END IF; END $$;";
        let status_user = Command::new("psql")
            .arg("postgres")
            .arg("-U")
            .arg(user)
            .arg("-c")
            .arg(create_user)
            .status()?;
        if !status_user.success() {
            log::warn!(
                "Could not create user 'sam' (may already exist or insufficient privileges)"
            );
        }
        // Create database 'sam' owned by 'sam' if not exists
        // NOTE: CREATE DATABASE cannot be run inside DO/PLPGSQL blocks.
        // So we must check existence and run CREATE DATABASE as a separate statement.
        let check_db = "SELECT 1 FROM pg_database WHERE datname = 'sam';";
        let output = Command::new("psql")
            .arg("postgres")
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

    /// Creates the user 'sam' and database 'sam' if they do not exist.
    /// Requires superuser privileges (may prompt for password).
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn create_user_and_database(user: &str) -> Result<()> {
        println!("Creating user and database...");
        // Create user 'sam' if not exists
        let create_user = "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sam') THEN CREATE ROLE sam LOGIN PASSWORD 'sam'; END IF; END $$;";
        let status_user = Command::new("psql")
            .arg("-U")
            .arg(user)
            .arg("-c")
            .arg(create_user)
            .status()?;
        if !status_user.success() {
            log::warn!(
                "Could not create user 'sam' (may already exist or insufficient privileges)"
            );
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
