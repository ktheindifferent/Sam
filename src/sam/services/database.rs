use anyhow::{Result, Context};
use log::{info, warn, error};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use crate::sam::db::database_engine::{DatabaseEngine, DatabasePool, Value, Row};

static DB_POOL: OnceLock<Arc<DatabasePool>> = OnceLock::new();

pub async fn connect() -> Result<Arc<DatabasePool>> {
    if let Some(pool) = DB_POOL.get() {
        pool.health_check().await?;
        return Ok(pool.clone());
    }
    
    let engine = DatabaseEngine::from_env();
    info!("Initializing database with engine: {:?}", engine);
    
    let pool = DatabasePool::new(engine).await
        .context("Failed to create database pool")?;
    
    let pool = Arc::new(pool);
    
    match DB_POOL.set(pool.clone()) {
        Ok(_) => {
            info!("Database pool initialized successfully");
        },
        Err(_) => {
            if let Some(existing_pool) = DB_POOL.get() {
                return Ok(existing_pool.clone());
            }
        }
    }
    
    Ok(pool)
}

pub async fn health_check() -> Result<()> {
    let pool = connect().await?;
    pool.health_check().await?;
    info!("Database health check passed (engine: {:?})", pool.engine());
    Ok(())
}

pub async fn initialize_schema() -> Result<()> {
    info!("Initializing database schema");
    let pool = connect().await?;
    
    match pool.engine() {
        DatabaseEngine::SQLite => initialize_sqlite_schema(pool).await,
        DatabaseEngine::PostgreSQL => initialize_postgres_schema(pool).await,
        _ => Err(anyhow::anyhow!("Schema initialization not implemented for {:?}", pool.engine())),
    }
}

async fn initialize_sqlite_schema(pool: Arc<DatabasePool>) -> Result<()> {
    info!("Initializing SQLite schema");
    
    let schema = r#"
        -- Crawler tables
        CREATE TABLE IF NOT EXISTS crawl_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL,
            max_depth INTEGER DEFAULT 2,
            status TEXT DEFAULT 'pending',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE TABLE IF NOT EXISTS crawl_pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER REFERENCES crawl_jobs(id) ON DELETE CASCADE,
            url TEXT NOT NULL,
            title TEXT,
            content TEXT,
            status_code INTEGER,
            error TEXT,
            crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(job_id, url)
        );
        
        -- File storage tables
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            size INTEGER NOT NULL,
            mime_type TEXT,
            checksum TEXT,
            metadata TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE TABLE IF NOT EXISTS file_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
            version_number INTEGER NOT NULL,
            size INTEGER NOT NULL,
            checksum TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Backup tables
        CREATE TABLE IF NOT EXISTS backups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            size INTEGER,
            path TEXT,
            error TEXT,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Session tables
        CREATE TABLE IF NOT EXISTS user_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            csrf_token TEXT NOT NULL,
            data TEXT,
            expires_at TIMESTAMP NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Service health tables
        CREATE TABLE IF NOT EXISTS service_health (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            service_name TEXT NOT NULL,
            status TEXT NOT NULL,
            error_count INTEGER DEFAULT 0,
            restart_count INTEGER DEFAULT 0,
            memory_usage INTEGER,
            cpu_usage REAL,
            metadata TEXT,
            checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Migration tables
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            checksum TEXT NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            execution_time_ms INTEGER
        );
        
        -- Job queue tables
        CREATE TABLE IF NOT EXISTS job_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_type TEXT NOT NULL,
            payload TEXT,
            status TEXT DEFAULT 'pending',
            priority INTEGER DEFAULT 0,
            retry_count INTEGER DEFAULT 0,
            max_retries INTEGER DEFAULT 3,
            error TEXT,
            scheduled_at TIMESTAMP,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE TABLE IF NOT EXISTS dead_letter_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER,
            job_type TEXT NOT NULL,
            payload TEXT,
            error TEXT,
            retry_count INTEGER,
            failed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Security tables
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            user_id TEXT,
            ip_address TEXT,
            resource TEXT,
            action TEXT,
            result TEXT,
            metadata TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE TABLE IF NOT EXISTS rate_limits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            count INTEGER DEFAULT 0,
            window_start TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        -- Create indexes for better performance
        CREATE INDEX IF NOT EXISTS idx_crawl_pages_job_id ON crawl_pages(job_id);
        CREATE INDEX IF NOT EXISTS idx_crawl_pages_url ON crawl_pages(url);
        CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
        CREATE INDEX IF NOT EXISTS idx_files_checksum ON files(checksum);
        CREATE INDEX IF NOT EXISTS idx_user_sessions_expires ON user_sessions(expires_at);
        CREATE INDEX IF NOT EXISTS idx_service_health_service ON service_health(service_name);
        CREATE INDEX IF NOT EXISTS idx_service_health_checked ON service_health(checked_at);
        CREATE INDEX IF NOT EXISTS idx_job_queue_status ON job_queue(status);
        CREATE INDEX IF NOT EXISTS idx_job_queue_scheduled ON job_queue(scheduled_at);
        CREATE INDEX IF NOT EXISTS idx_audit_log_event ON audit_log(event_type);
        CREATE INDEX IF NOT EXISTS idx_audit_log_user ON audit_log(user_id);
        CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);
        CREATE INDEX IF NOT EXISTS idx_rate_limits_key ON rate_limits(key);
    "#;
    
    for statement in schema.split(';').filter(|s| !s.trim().is_empty()) {
        let stmt = format!("{};", statement.trim());
        pool.execute(&stmt, vec![]).await
            .with_context(|| format!("Failed to execute schema statement: {}", stmt))?;
    }
    
    info!("SQLite schema initialized successfully");
    Ok(())
}

async fn initialize_postgres_schema(pool: Arc<DatabasePool>) -> Result<()> {
    info!("Using existing PostgreSQL migration system");
    crate::sam::services::pg::initialize_schema().await
}

pub async fn execute_query(query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        pool.query(query, params)
    ).await
        .context("Query timeout")?;
    
    match result {
        Ok(rows) => {
            info!("Query executed successfully in {:?}", start.elapsed());
            Ok(rows)
        }
        Err(e) => {
            error!("Query failed: {}", e);
            Err(e).context("Failed to execute query")
        }
    }
}

pub async fn execute_statement(query: &str, params: Vec<Value>) -> Result<u64> {
    let pool = connect().await?;
    let start = std::time::Instant::now();
    
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        pool.execute(query, params)
    ).await
        .context("Statement timeout")?;
    
    match result {
        Ok(count) => {
            info!("Statement executed successfully in {:?}, affected {} rows", start.elapsed(), count);
            Ok(count)
        }
        Err(e) => {
            error!("Statement failed: {}", e);
            Err(e).context("Failed to execute statement")
        }
    }
}

pub async fn cleanup_old_sessions() -> Result<u64> {
    let pool = connect().await?;
    
    let query = match pool.engine() {
        DatabaseEngine::SQLite => {
            "DELETE FROM user_sessions WHERE expires_at < datetime('now')"
        }
        DatabaseEngine::PostgreSQL => {
            "DELETE FROM user_sessions WHERE expires_at < NOW()"
        }
        _ => return Err(anyhow::anyhow!("Cleanup not implemented for {:?}", pool.engine())),
    };
    
    execute_statement(query, vec![]).await
}

pub async fn cleanup_old_health_records(days: i32) -> Result<u64> {
    let pool = connect().await?;
    
    let query = match pool.engine() {
        DatabaseEngine::SQLite => {
            format!("DELETE FROM service_health WHERE checked_at < datetime('now', '-{} days')", days)
        }
        DatabaseEngine::PostgreSQL => {
            format!("DELETE FROM service_health WHERE checked_at < NOW() - INTERVAL '{} days'", days)
        }
        _ => return Err(anyhow::anyhow!("Cleanup not implemented for {:?}", pool.engine())),
    };
    
    execute_statement(&query, vec![]).await
}

pub fn value_null() -> Value {
    Value::Null
}

pub fn value_bool(b: bool) -> Value {
    Value::Bool(b)
}

pub fn value_i32(i: i32) -> Value {
    Value::Int32(i)
}

pub fn value_i64(i: i64) -> Value {
    Value::Int64(i)
}

pub fn value_f32(f: f32) -> Value {
    Value::Float(f)
}

pub fn value_f64(f: f64) -> Value {
    Value::Double(f)
}

pub fn value_text(s: String) -> Value {
    Value::Text(s)
}

pub fn value_bytes(b: Vec<u8>) -> Value {
    Value::Bytes(b)
}

pub fn value_json(j: serde_json::Value) -> Value {
    Value::Json(j)
}

pub fn value_timestamp(t: chrono::DateTime<chrono::Utc>) -> Value {
    Value::Timestamp(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection() {
        match connect().await {
            Ok(pool) => {
                println!("Connected to database with engine: {:?}", pool.engine());
            }
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        match health_check().await {
            Ok(_) => {
                println!("Database health check passed");
            }
            Err(e) => {
                eprintln!("Database health check failed: {}", e);
            }
        }
    }
}