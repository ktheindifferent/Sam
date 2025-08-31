// TODO: Ext Crawler
// TODO: Use redis for dns cache if available

use once_cell::sync::Lazy;
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod circuit_breaker;
pub mod enhanced;
pub mod job;
pub mod metrics;
pub mod page;
pub mod robots;
pub mod runner;
pub mod sitemap;

pub use job::CrawlJob;
pub use page::CrawledPage;
pub use runner::{crawl_url, service_status, start_service, start_service_async, stop_service};
pub use robots::{is_url_allowed, DEFAULT_USER_AGENT};
pub use sitemap::{extract_urls_from_sitemaps, fetch_sitemap};
pub use circuit_breaker::{is_domain_allowed, record_domain_failure, record_domain_success};
pub use metrics::{get_crawler_metrics, generate_metrics_report, record_crawl_success, record_crawl_failure};
pub use enhanced::{EnhancedCrawler, EnhancedCrawlResult};

/// Global database connection pool for crawler threads
static DB_POOL: Lazy<Arc<RwLock<Option<Pool>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(None))
});

/// Initialize the database connection pool for the crawler
/// This should be called once at application startup
pub async fn initialize_db_pool() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        format!(
            "postgresql://{}:{}@{}/{}",
            std::env::var("PG_USER").unwrap_or_else(|_| "sam".to_string()),
            std::env::var("PG_PASS").unwrap_or_else(|_| "sam".to_string()),
            std::env::var("PG_ADDRESS").unwrap_or_else(|_| "localhost".to_string()),
            std::env::var("PG_DBNAME").unwrap_or_else(|_| "sam".to_string())
        )
    });
    
    let mut cfg = Config::new();
    cfg.url = Some(db_url);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: 20, // Maximum number of connections in the pool
        timeouts: deadpool_postgres::Timeouts {
            wait: Some(std::time::Duration::from_secs(5)),
            create: Some(std::time::Duration::from_secs(5)),
            recycle: Some(std::time::Duration::from_secs(5)),
        },
        queue_mode: deadpool_postgres::QueueMode::Fifo,
    });
    
    let pool = cfg.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)?;
    
    // Test the connection
    let client = pool.get().await?;
    client.query_one("SELECT 1", &[]).await?;
    
    let mut pool_guard = DB_POOL.write().await;
    *pool_guard = Some(pool);
    
    log::info!("Crawler database connection pool initialized successfully");
    Ok(())
}

/// Get a database connection from the pool
/// Returns None if the pool is not initialized
pub async fn get_db_connection() -> Option<deadpool_postgres::Client> {
    let pool_guard = DB_POOL.read().await;
    if let Some(pool) = pool_guard.as_ref() {
        match pool.get().await {
            Ok(client) => Some(client),
            Err(e) => {
                log::error!("Failed to get database connection from pool: {}", e);
                None
            }
        }
    } else {
        log::warn!("Database pool not initialized");
        None
    }
}

/// Get the database pool for direct access
/// Useful for passing to functions that need the pool itself
pub async fn get_db_pool() -> Option<Pool> {
    let pool_guard = DB_POOL.read().await;
    pool_guard.clone()
}

/// Shutdown the database pool gracefully
pub async fn shutdown_db_pool() {
    let mut pool_guard = DB_POOL.write().await;
    if let Some(pool) = pool_guard.take() {
        // Pool will be dropped and connections closed
        drop(pool);
        log::info!("Crawler database connection pool shut down");
    }
}
