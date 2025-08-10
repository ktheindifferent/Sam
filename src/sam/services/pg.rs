use anyhow::{Result, Context};
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Row};
use log::{info, warn, error};
use std::time::Duration;
use std::env;

static mut POOL: Option<Pool> = None;

pub async fn connect() -> Result<Pool> {
    // Check if pool already exists
    unsafe {
        if let Some(ref pool) = POOL {
            return Ok(pool.clone());
        }
    }
    
    // Create new pool
    let pool = create_pool().await?;
    
    // Store for future use
    unsafe {
        POOL = Some(pool.clone());
    }
    
    Ok(pool)
}

async fn create_pool() -> Result<Pool> {
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
}

pub async fn health_check() -> Result<()> {
    let pool = connect().await?;
    let client = pool.get().await
        .context("Failed to get client for health check")?;
    
    let rows = client.query("SELECT version()", &[])
        .await
        .context("Health check query failed")?;
    
    if !rows.is_empty() {
        let version: &str = rows[0].get(0);
        info!("PostgreSQL health check passed: {}", version);
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
    let client = pool.get().await?;
    
    client.query(query, params)
        .await
        .context("Failed to execute query")
}

pub async fn execute_statement(query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
    let pool = connect().await?;
    let client = pool.get().await?;
    
    client.execute(query, params)
        .await
        .context("Failed to execute statement")
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
    
    Ok(format!(
        "Pool Status - Size: {}, Available: {}, Waiting: {}",
        status.size, status.available, status.waiting
    ))
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