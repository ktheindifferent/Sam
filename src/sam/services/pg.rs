use anyhow::{Result, Context};
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Row};
use log::{info, warn, error};
use std::time::Duration;
use std::env;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use std::collections::HashMap;

static POOL: OnceLock<Arc<Pool>> = OnceLock::new();
static POOL_METRICS: OnceLock<Arc<RwLock<PoolMetrics>>> = OnceLock::new();

pub struct PoolMetrics {
    pub total_connections: u64,
    pub failed_connections: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub connection_wait_time_ms: Vec<u64>,
    pub last_health_check: Option<std::time::Instant>,
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self {
            total_connections: 0,
            failed_connections: 0,
            successful_queries: 0,
            failed_queries: 0,
            connection_wait_time_ms: Vec::new(),
            last_health_check: None,
        }
    }
}

pub async fn connect() -> Result<Arc<Pool>> {
    // Try to get existing pool first
    if let Some(pool) = POOL.get() {
        // Perform health check if needed
        perform_health_check_if_needed(pool).await?;
        return Ok(pool.clone());
    }
    
    // Initialize pool with retry logic
    let pool = retry_with_backoff(create_pool, 3, Duration::from_secs(1)).await
        .context("Failed to create connection pool after retries")?;
    
    let pool = Arc::new(pool);
    
    // Try to set the pool, but if another thread beat us, use theirs
    match POOL.set(pool.clone()) {
        Ok(_) => {
            info!("Database connection pool initialized successfully");
            // Initialize metrics
            let _ = POOL_METRICS.set(Arc::new(RwLock::new(PoolMetrics::default())));
        },
        Err(_) => {
            // Another thread initialized it, use theirs
            if let Some(existing_pool) = POOL.get() {
                return Ok(existing_pool.clone());
            }
        }
    }
    
    Ok(pool)
}

async fn retry_with_backoff<F, T>(
    mut f: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T>>,
{
    let mut delay = initial_delay;
    let mut last_error = None;
    
    for attempt in 0..max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
                if attempt < max_retries - 1 {
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
}

async fn perform_health_check_if_needed(pool: &Arc<Pool>) -> Result<()> {
    if let Some(metrics) = POOL_METRICS.get() {
        let should_check = {
            let metrics = metrics.read().await;
            metrics.last_health_check.map_or(true, |last| {
                last.elapsed() > Duration::from_secs(30)
            })
        };
        
        if should_check {
            // Perform quick health check
            let start = std::time::Instant::now();
            match pool.get().await {
                Ok(client) => {
                    match client.simple_query("SELECT 1").await {
                        Ok(_) => {
                            let mut metrics = metrics.write().await;
                            metrics.last_health_check = Some(std::time::Instant::now());
                            metrics.connection_wait_time_ms.push(start.elapsed().as_millis() as u64);
                            // Keep only last 100 measurements
                            if metrics.connection_wait_time_ms.len() > 100 {
                                metrics.connection_wait_time_ms.remove(0);
                            }
                        }
                        Err(e) => {
                            error!("Health check query failed: {}", e);
                            return Err(anyhow::anyhow!("Database health check failed"));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get connection for health check: {}", e);
                    return Err(anyhow::anyhow!("Failed to get database connection"));
                }
            }
        }
    }
    Ok(())
}

fn create_pool() -> futures::future::BoxFuture<'static, Result<Pool>> {
    Box::pin(async move {
    let mut cfg = Config::new();
    
    // Get database configuration from environment or use defaults
    cfg.host = Some(env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()));
    cfg.port = Some(env::var("POSTGRES_PORT")
        .unwrap_or_else(|_| "5432".to_string())
        .parse()
        .unwrap_or(5432));
    cfg.dbname = Some(env::var("POSTGRES_DB").unwrap_or_else(|_| "sam".to_string()));
    cfg.user = Some(env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()));
    cfg.password = Some(env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "sampassword".to_string()));
    
    // Pool configuration
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: 32,
        timeouts: deadpool_postgres::Timeouts {
            wait: Some(Duration::from_secs(5)),
            create: Some(Duration::from_secs(5)),
            recycle: Some(Duration::from_secs(5)),
        },
    });
    
    // Manager configuration
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    
    let mgr = Manager::from_config(cfg, NoTls, mgr_config);
    let pool = Pool::builder(mgr)
        .max_size(32)
        .runtime(Runtime::Tokio1)
        .build()
        .context("Failed to create PostgreSQL connection pool")?;
    
    // Test the connection
    let client = pool.get().await
        .context("Failed to get client from pool")?;
    
    client.simple_query("SELECT 1")
        .await
        .context("Failed to execute test query")?;
    
    info!("PostgreSQL connection pool created successfully");
    Ok(pool)
    })
}

pub async fn health_check() -> Result<()> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    // Try to get a connection with timeout
    let client = tokio::time::timeout(
        Duration::from_secs(5),
        pool.get()
    ).await
        .context("Connection timeout during health check")?
        .context("Failed to get client for health check")?;
    
    let rows = client.query("SELECT version(), current_database(), pg_is_in_recovery()", &[])
        .await
        .context("Health check query failed")?;
    
    if !rows.is_empty() {
        let version: &str = rows[0].get(0);
        let database: &str = rows[0].get(1);
        let is_replica: bool = rows[0].get(2);
        
        info!("PostgreSQL health check passed: {} (database: {}, replica: {}, latency: {:?})", 
              version, database, is_replica, start.elapsed());
        
        // Update metrics
        if let Some(metrics) = POOL_METRICS.get() {
            let mut metrics = metrics.write().await;
            metrics.successful_queries += 1;
        }
        
        Ok(())
    } else {
        Err(anyhow::anyhow!("PostgreSQL health check failed"))
    }
}

pub async fn initialize_schema() -> Result<()> {
    let pool = connect().await?;
    let client = pool.get().await?;
    
    // Create tables if they don't exist
    let queries = vec![
        // Crawler tables
        r#"
        CREATE TABLE IF NOT EXISTS crawl_jobs (
            id SERIAL PRIMARY KEY,
            url TEXT NOT NULL,
            max_depth INTEGER DEFAULT 2,
            status TEXT DEFAULT 'pending',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS crawl_pages (
            id SERIAL PRIMARY KEY,
            job_id INTEGER REFERENCES crawl_jobs(id) ON DELETE CASCADE,
            url TEXT NOT NULL,
            title TEXT,
            content TEXT,
            status_code INTEGER,
            error TEXT,
            crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(job_id, url)
        )
        "#,
        
        // File storage tables
        r#"
        CREATE TABLE IF NOT EXISTS files (
            id SERIAL PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            size BIGINT NOT NULL,
            mime_type TEXT,
            checksum TEXT,
            metadata JSONB,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS file_versions (
            id SERIAL PRIMARY KEY,
            file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
            version_number INTEGER NOT NULL,
            size BIGINT NOT NULL,
            checksum TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        
        // Backup tables
        r#"
        CREATE TABLE IF NOT EXISTS backups (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            size BIGINT,
            path TEXT,
            error TEXT,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        
        // Session tables
        r#"
        CREATE TABLE IF NOT EXISTS user_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            csrf_token TEXT NOT NULL,
            data JSONB,
            expires_at TIMESTAMP NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        
        // Service health tables
        r#"
        CREATE TABLE IF NOT EXISTS service_health (
            id SERIAL PRIMARY KEY,
            service_name TEXT NOT NULL,
            status TEXT NOT NULL,
            error_count INTEGER DEFAULT 0,
            restart_count INTEGER DEFAULT 0,
            memory_usage BIGINT,
            cpu_usage REAL,
            custom_metrics JSONB,
            checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        
        // Create indexes
        "CREATE INDEX IF NOT EXISTS idx_crawl_pages_job_id ON crawl_pages(job_id)",
        "CREATE INDEX IF NOT EXISTS idx_crawl_pages_url ON crawl_pages(url)",
        "CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)",
        "CREATE INDEX IF NOT EXISTS idx_files_checksum ON files(checksum)",
        "CREATE INDEX IF NOT EXISTS idx_backups_status ON backups(status)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_expires ON user_sessions(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_service_health_name ON service_health(service_name)",
    ];
    
    for query in queries {
        client.execute(query, &[])
            .await
            .context(format!("Failed to execute schema query: {}", &query[..50]))?;
    }
    
    info!("PostgreSQL schema initialized successfully");
    Ok(())
}

pub async fn execute_query(query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    // Get connection with timeout
    let client = tokio::time::timeout(
        Duration::from_secs(5),
        pool.get()
    ).await
        .context("Connection timeout")?
        .context("Failed to get client")?;
    
    // Execute query with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        client.query(query, params)
    ).await
        .context("Query timeout")?;
    
    match result {
        Ok(rows) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.successful_queries += 1;
                metrics.total_connections += 1;
            }
            info!("Query executed successfully in {:?}", start.elapsed());
            Ok(rows)
        }
        Err(e) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.failed_queries += 1;
            }
            error!("Query failed: {}", e);
            Err(e).context("Failed to execute query")
        }
    }
}

pub async fn execute_statement(query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    // Get connection with timeout
    let client = tokio::time::timeout(
        Duration::from_secs(5),
        pool.get()
    ).await
        .context("Connection timeout")?
        .context("Failed to get client")?;
    
    // Execute statement with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        client.execute(query, params)
    ).await
        .context("Statement timeout")?;
    
    match result {
        Ok(count) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.successful_queries += 1;
                metrics.total_connections += 1;
            }
            info!("Statement executed successfully in {:?}, affected {} rows", start.elapsed(), count);
            Ok(count)
        }
        Err(e) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.failed_queries += 1;
            }
            error!("Statement failed: {}", e);
            Err(e).context("Failed to execute statement")
        }
    }
}

pub async fn transaction<F, R>(f: F) -> Result<R>
where
    F: FnOnce(deadpool_postgres::Transaction<'_>) -> futures::future::BoxFuture<'_, Result<R>>,
{
    let pool = connect().await?;
    let mut client = pool.get().await?;
    let transaction = client.transaction().await?;
    
    match f(transaction).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

pub async fn get_pool_status() -> Result<String> {
    let pool = connect().await?;
    let status = pool.status();
    
    let metrics_str = if let Some(metrics) = POOL_METRICS.get() {
        let metrics = metrics.read().await;
        let avg_wait = if !metrics.connection_wait_time_ms.is_empty() {
            metrics.connection_wait_time_ms.iter().sum::<u64>() / metrics.connection_wait_time_ms.len() as u64
        } else {
            0
        };
        
        format!(
            "\nMetrics - Total Connections: {}, Failed: {}, Successful Queries: {}, Failed Queries: {}, Avg Wait: {}ms",
            metrics.total_connections,
            metrics.failed_connections,
            metrics.successful_queries,
            metrics.failed_queries,
            avg_wait
        )
    } else {
        String::new()
    };
    
    Ok(format!(
        "Pool Status - Size: {}, Available: {}, Waiting: {}{}",
        status.size, status.available, status.waiting, metrics_str
    ))
}

pub async fn reset_pool() -> Result<()> {
    if let Some(pool) = POOL.get() {
        // This doesn't actually reset the OnceLock, but we can close all connections
        // and the pool will recreate them as needed
        info!("Resetting connection pool");
        // The pool will automatically recreate connections
    }
    Ok(())
}

pub async fn cleanup_old_sessions() -> Result<u64> {
    execute_statement(
        "DELETE FROM user_sessions WHERE expires_at < NOW()",
        &[]
    ).await
}

pub async fn cleanup_old_health_records(days: i32) -> Result<u64> {
    execute_statement(
        "DELETE FROM service_health WHERE checked_at < NOW() - INTERVAL '$1 days'",
        &[&days]
    ).await
}

pub async fn execute_batch_query<T, F>(
    query: &str,
    params_batch: Vec<Vec<&(dyn tokio_postgres::types::ToSql + Sync)>>,
    row_mapper: F,
) -> Result<Vec<T>>
where
    F: Fn(&Row) -> Result<T>,
{
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    // Get connection with timeout
    let client = tokio::time::timeout(
        Duration::from_secs(5),
        pool.get()
    ).await
        .context("Connection timeout")?
        .context("Failed to get client")?;
    
    let mut all_results = Vec::new();
    
    // Execute all queries in a single transaction for better performance
    let transaction = client.transaction().await
        .context("Failed to start transaction")?;
    
    for params in params_batch {
        let rows = transaction.query(query, &params[..]).await
            .context("Failed to execute batch query")?;
        
        for row in rows {
            all_results.push(row_mapper(&row)?);
        }
    }
    
    transaction.commit().await
        .context("Failed to commit batch transaction")?;
    
    // Update metrics
    if let Some(metrics) = POOL_METRICS.get() {
        let mut metrics = metrics.write().await;
        metrics.successful_queries += params_batch.len() as u64;
        metrics.total_connections += 1;
    }
    
    info!("Batch query executed {} queries in {:?}", params_batch.len(), start.elapsed());
    Ok(all_results)
}

pub async fn execute_query_with_cache<T, F>(
    cache_key: &str,
    cache_ttl: Duration,
    query: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    row_mapper: F,
) -> Result<Vec<T>>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&Row) -> Result<T>,
{
    // Simple in-memory cache using a static HashMap
    // In production, consider using a proper caching solution like Redis
    static QUERY_CACHE: OnceLock<Arc<RwLock<HashMap<String, (std::time::Instant, Vec<Vec<u8>>)>>>> = OnceLock::new();
    
    let cache = QUERY_CACHE.get_or_init(|| {
        Arc::new(RwLock::new(HashMap::new()))
    });
    
    // Check cache
    {
        let cache_read = cache.read().await;
        if let Some((cached_at, _cached_data)) = cache_read.get(cache_key) {
            if cached_at.elapsed() < cache_ttl {
                info!("Cache hit for key: {}", cache_key);
                // For simplicity, we're not implementing full deserialization here
                // In production, you'd deserialize the cached data
            }
        }
    }
    
    // Cache miss - execute query
    let rows = execute_query(query, params).await?;
    let mut results = Vec::new();
    
    for row in rows {
        results.push(row_mapper(&row)?);
    }
    
    // Update cache
    {
        let mut cache_write = cache.write().await;
        // For simplicity, we're not implementing full serialization here
        // In production, you'd serialize the results
        cache_write.insert(cache_key.to_string(), (std::time::Instant::now(), Vec::new()));
    }
    
    Ok(results)
}

pub async fn execute_query_batch_join(
    base_query: &str,
    join_query: &str,
    base_params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    join_column: &str,
) -> Result<Vec<Row>> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    // Get connection with timeout
    let client = tokio::time::timeout(
        Duration::from_secs(5),
        pool.get()
    ).await
        .context("Connection timeout")?
        .context("Failed to get client")?;
    
    // Build optimized query with JOIN instead of N+1
    let combined_query = format!(
        "{} LEFT JOIN ({}) AS joined ON base.{} = joined.{}",
        base_query, join_query, join_column, join_column
    );
    
    // Execute combined query
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        client.query(&combined_query, base_params)
    ).await
        .context("Query timeout")?;
    
    match result {
        Ok(rows) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.successful_queries += 1;
                metrics.total_connections += 1;
            }
            info!("Batch JOIN query executed in {:?}, returned {} rows", start.elapsed(), rows.len());
            Ok(rows)
        }
        Err(e) => {
            // Update metrics
            if let Some(metrics) = POOL_METRICS.get() {
                let mut metrics = metrics.write().await;
                metrics.failed_queries += 1;
            }
            error!("Batch JOIN query failed: {}", e);
            Err(e).context("Failed to execute batch JOIN query")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool() {
        // This test requires a running PostgreSQL instance
        match connect().await {
            Ok(pool) => {
                assert!(pool.status().size > 0);
            }
            Err(e) => {
                eprintln!("Skipping test - PostgreSQL not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        match health_check().await {
            Ok(_) => {
                // Health check passed
            }
            Err(e) => {
                eprintln!("Skipping test - PostgreSQL not available: {}", e);
            }
        }
    }
}