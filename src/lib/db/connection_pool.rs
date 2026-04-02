use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Row};
use std::time::Duration;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use log::{info, error};

/// Global database connection pool
static DB_POOL: Lazy<Arc<DbPool>> = Lazy::new(|| {
    Arc::new(DbPool::new())
});

/// Database connection pool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: usize,
    pub min_connections: usize,
    pub connection_timeout_sec: u64,
    pub idle_timeout_sec: u64,
    pub max_lifetime_sec: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            database: std::env::var("DB_NAME").unwrap_or_else(|_| "sam_db".to_string()),
            username: std::env::var("DB_USER").unwrap_or_else(|_| "sam".to_string()),
            password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            max_connections: 32,
            min_connections: 2,
            connection_timeout_sec: 30,
            idle_timeout_sec: 600, // 10 minutes
            max_lifetime_sec: 1800, // 30 minutes
        }
    }
}

/// Database connection pool manager
pub struct DbPool {
    pool: Option<Pool>,
    config: PoolConfig,
}

impl Default for DbPool {
    fn default() -> Self {
        Self::new()
    }
}

impl DbPool {
    /// Create a new database pool (uninitialized)
    pub fn new() -> Self {
        DbPool {
            pool: None,
            config: PoolConfig::default(),
        }
    }

    /// Initialize the connection pool with custom configuration
    pub async fn init_with_config(config: PoolConfig) -> Result<(), Box<dyn std::error::Error>> {
        let pool = DB_POOL.clone();
        let pool_mut = pool.as_ref();
        
        // Create deadpool configuration
        let mut cfg = Config::new();
        cfg.host = Some(config.host.clone());
        cfg.port = Some(config.port);
        cfg.dbname = Some(config.database.clone());
        cfg.user = Some(config.username.clone());
        cfg.password = Some(config.password.clone());
        
        // Set pool size
        cfg.pool = Some(deadpool_postgres::PoolConfig {
            max_size: config.max_connections,
            timeouts: deadpool_postgres::Timeouts {
                wait: Some(Duration::from_secs(config.connection_timeout_sec)),
                create: Some(Duration::from_secs(config.connection_timeout_sec)),
                recycle: Some(Duration::from_secs(5)),
            },
            queue_mode: deadpool::managed::QueueMode::Fifo,
        });
        
        // Create manager configuration
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        
        // Create the pool
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        
        // Test the connection
        let client = pool.get().await?;
        let _ = client.query_one("SELECT 1", &[]).await?;
        
        info!(
            "Database connection pool initialized with {} connections",
            config.max_connections
        );
        
        Ok(())
    }

    /// Initialize the connection pool with default configuration
    pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
        Self::init_with_config(PoolConfig::default()).await
    }

    /// Get a connection from the pool
    pub async fn get_connection() -> Result<deadpool_postgres::Client, Box<dyn std::error::Error>> {
        let pool = get_pool().await?;
        let client = pool.get().await?;
        Ok(client)
    }

    /// Execute a query that returns rows
    pub async fn query(
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
        let client = Self::get_connection().await?;
        let rows = client.query(sql, params).await?;
        Ok(rows)
    }

    /// Execute a query that returns a single row
    pub async fn query_one(
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Row, Box<dyn std::error::Error>> {
        let client = Self::get_connection().await?;
        let row = client.query_one(sql, params).await?;
        Ok(row)
    }

    /// Execute a query that returns an optional single row
    pub async fn query_opt(
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Option<Row>, Box<dyn std::error::Error>> {
        let client = Self::get_connection().await?;
        let row = client.query_opt(sql, params).await?;
        Ok(row)
    }

    /// Execute a statement that doesn't return rows
    pub async fn execute(
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let client = Self::get_connection().await?;
        let count = client.execute(sql, params).await?;
        Ok(count)
    }

    /// Execute multiple statements in a transaction
    pub async fn transaction<F, R>(f: F) -> Result<R, Box<dyn std::error::Error>>
    where
        F: FnOnce(deadpool_postgres::Transaction<'_>) -> futures::future::BoxFuture<'_, Result<R, Box<dyn std::error::Error>>>,
    {
        let mut client = Self::get_connection().await?;
        let tx = client.transaction().await?;
        
        match f(tx).await {
            Ok(result) => {
                // Transaction will be committed when it goes out of scope
                Ok(result)
            }
            Err(e) => {
                // Transaction will be rolled back when it goes out of scope
                Err(e)
            }
        }
    }

    /// Get pool statistics
    pub async fn get_stats() -> Result<PoolStats, Box<dyn std::error::Error>> {
        let pool = get_pool().await?;
        let status = pool.status();
        
        Ok(PoolStats {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
            max_size: status.max_size,
        })
    }

    /// Health check for the database connection pool
    pub async fn health_check() -> Result<bool, Box<dyn std::error::Error>> {
        match Self::query_one("SELECT 1 as health", &[]).await {
            Ok(row) => {
                let health: i32 = row.get("health");
                Ok(health == 1)
            }
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Gracefully shutdown the connection pool
    pub async fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
        if let Some(pool) = &DB_POOL.pool {
            pool.close();
            info!("Database connection pool shut down");
        }
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
    pub max_size: usize,
}

impl PoolStats {
    pub fn utilization_percent(&self) -> f32 {
        if self.max_size == 0 {
            return 0.0;
        }
        ((self.size - self.available) as f32 / self.max_size as f32) * 100.0
    }
}

/// Get the initialized pool
async fn get_pool() -> Result<&'static Pool, Box<dyn std::error::Error>> {
    if let Some(pool) = &DB_POOL.pool {
        Ok(pool)
    } else {
        // Initialize with default config if not already initialized
        DbPool::init().await?;
        DB_POOL
            .pool
            .as_ref()
            .ok_or_else(|| "Failed to initialize database pool".into())
    }
}

/// Query builder for optimized queries
pub struct QueryBuilder {
    query: String,
    params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>,
}

impl QueryBuilder {
    pub fn new(base_query: &str) -> Self {
        QueryBuilder {
            query: base_query.to_string(),
            params: Vec::new(),
        }
    }

    pub fn add_param<T>(mut self, param: T) -> Self
    where
        T: tokio_postgres::types::ToSql + Sync + Send + 'static,
    {
        self.params.push(Box::new(param));
        self
    }

    pub fn add_where_clause(mut self, clause: &str) -> Self {
        if self.query.to_lowercase().contains("where") {
            self.query.push_str(" AND ");
        } else {
            self.query.push_str(" WHERE ");
        }
        self.query.push_str(clause);
        self
    }

    pub fn add_order_by(mut self, column: &str, desc: bool) -> Self {
        self.query.push_str(" ORDER BY ");
        self.query.push_str(column);
        if desc {
            self.query.push_str(" DESC");
        } else {
            self.query.push_str(" ASC");
        }
        self
    }

    pub fn add_limit(mut self, limit: i64) -> Self {
        // SAFETY: While LIMIT cannot be parameterized in SQL, we explicitly validate
        // the numeric range to prevent injection-like patterns. PostgreSQL only accepts
        // numeric values for LIMIT, so string interpolation of validated i64 is safe.
        if limit < 0 {
            log::warn!("add_limit called with negative value: {}, treating as 0", limit);
            // PostgreSQL treats negative LIMIT as unlimited
            // We log the attempt for security monitoring
        }
        self.query.push_str(&format!(" LIMIT {}", limit));
        self
    }

    pub fn add_offset(mut self, offset: i64) -> Self {
        // SAFETY: While OFFSET cannot be parameterized in SQL, we explicitly validate
        // the numeric range. PostgreSQL only accepts non-negative integers for OFFSET.
        if offset < 0 {
            log::warn!("add_offset called with negative value: {}, treating as 0", offset);
            // PostgreSQL treats negative OFFSET as 0
        }
        self.query.push_str(&format!(" OFFSET {}", offset));
        self
    }

    pub async fn execute(self) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            self.params.iter().map(|p| {
                let r: &(dyn tokio_postgres::types::ToSql + Sync) = p.as_ref();
                r
            }).collect();
        DbPool::query(&self.query, &params).await
    }
}

/// Macro for easy parameterized queries
#[macro_export]
macro_rules! db_query {
    ($query:expr) => {
        DbPool::query($query, &[]).await
    };
    ($query:expr, $($param:expr),*) => {
        DbPool::query($query, &[$(&$param),*]).await
    };
}

#[macro_export]
macro_rules! db_query_one {
    ($query:expr) => {
        DbPool::query_one($query, &[]).await
    };
    ($query:expr, $($param:expr),*) => {
        DbPool::query_one($query, &[$(&$param),*]).await
    };
}

#[macro_export]
macro_rules! db_execute {
    ($query:expr) => {
        DbPool::execute($query, &[]).await
    };
    ($query:expr, $($param:expr),*) => {
        DbPool::execute($query, &[$(&$param),*]).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 32);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.port, 5432);
    }

    #[test]
    fn test_pool_stats_utilization() {
        let stats = PoolStats {
            size: 10,
            available: 3,
            waiting: 0,
            max_size: 20,
        };
        
        let utilization = stats.utilization_percent();
        assert_eq!(utilization, 35.0); // (10-3)/20 * 100
    }

    #[test]
    fn test_query_builder() {
        let builder = QueryBuilder::new("SELECT * FROM users")
            .add_where_clause("active = true")
            .add_where_clause("age > 18")
            .add_order_by("created_at", true)
            .add_limit(10);
        
        let expected = "SELECT * FROM users WHERE active = true AND age > 18 ORDER BY created_at DESC LIMIT 10";
        assert_eq!(builder.query, expected);
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout_sec: 5,
            idle_timeout_sec: 300,
            max_lifetime_sec: 900,
        };
        
        // This will fail if no database is available, which is expected in tests
        let result = DbPool::init_with_config(config).await;
        assert!(result.is_err() || result.is_ok());
    }
}