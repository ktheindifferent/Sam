// TODO: Ext Crawler
// TODO: Use redis for dns cache if available

use once_cell::sync::Lazy;
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod circuit_breaker;
pub mod content_storage;
pub mod enhanced;
pub mod feed_parser;
pub mod job;
pub mod job_config;
pub mod job_queue;
pub mod memory_optimized;
pub mod metrics;
pub mod page;
pub mod prometheus_metrics;
pub mod rate_limiter;
pub mod rejected;
pub mod robots;
pub mod runner;
pub mod sitemap;
pub mod url_patterns;
pub mod webhooks;

pub use job::CrawlJob;
pub use job_config::{CrawlJobConfig, ConfigurableCrawlJob};
pub use job_queue::{PersistentJobQueue, QueuedJob, JobStatus, QueueStats, DistributedLock};
pub use memory_optimized::{OptimizedUrlTracker, BoundedUrlQueue, MemoryConfig};
pub use content_storage::{CrawledContent, DeduplicationStats};
pub use page::CrawledPage;
pub use rejected::{CrawlRejected, RejectionReason};
pub use rate_limiter::{AdaptiveRateLimiter, RateLimitConfig, DomainStats, init_rate_limiter, get_rate_limiter};
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
    // Check if running in CapRover environment
    let is_caprover = std::env::var("CAPROVER").is_ok();
    
    let db_url = if is_caprover {
        // Use CapRover-specific connection string with explicit host
        std::env::var("POSTGRES_URL").unwrap_or_else(|_| {
            format!(
                "postgresql://{}:{}@srv-captain--sam-db:5432/{}",
                std::env::var("PG_USER").unwrap_or_else(|_| "sam".to_string()),
                std::env::var("PG_PASS").unwrap_or_else(|_| "sam".to_string()),
                std::env::var("PG_DBNAME").unwrap_or_else(|_| "sam".to_string())
            )
        })
    } else {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "postgresql://{}:{}@{}/{}",
                std::env::var("PG_USER").unwrap_or_else(|_| "sam".to_string()),
                std::env::var("PG_PASS").unwrap_or_else(|_| "sam".to_string()),
                std::env::var("PG_ADDRESS").unwrap_or_else(|_| "localhost".to_string()),
                std::env::var("PG_DBNAME").unwrap_or_else(|_| "sam".to_string())
            )
        })
    };
    
    log::info!("Initializing crawler DB pool (CapRover: {})", is_caprover);
    
    let mut cfg = Config::new();
    cfg.url = Some(db_url.clone());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    
    // Adjust timeouts based on environment
    let (wait_timeout, create_timeout, recycle_timeout, max_connections) = if is_caprover {
        // Extended timeouts for CapRover environment
        (30, 30, 30, 10) // Longer timeouts, fewer connections
    } else {
        // Standard timeouts for local/development
        (5, 5, 5, 20)
    };
    
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: max_connections,
        timeouts: deadpool_postgres::Timeouts {
            wait: Some(std::time::Duration::from_secs(wait_timeout)),
            create: Some(std::time::Duration::from_secs(create_timeout)),
            recycle: Some(std::time::Duration::from_secs(recycle_timeout)),
        },
        queue_mode: deadpool::managed::QueueMode::Fifo,
    });
    
    // Retry connection with exponential backoff for CapRover
    let mut attempts = 0;
    let max_attempts = if is_caprover { 5 } else { 3 };
    let mut delay = std::time::Duration::from_secs(1);
    
    loop {
        attempts += 1;
        log::info!("DB connection attempt {} of {}", attempts, max_attempts);
        
        match cfg.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls) {
            Ok(pool) => {
                // Test the connection with timeout
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    pool.get()
                ).await {
                    Ok(Ok(client)) => {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            client.query_one("SELECT 1", &[])
                        ).await {
                            Ok(Ok(_)) => {
                                let mut pool_guard = DB_POOL.write().await;
                                *pool_guard = Some(pool);
                                log::info!("Crawler database connection pool initialized successfully");
                                return Ok(());
                            }
                            Ok(Err(e)) => {
                                log::warn!("DB test query failed: {}", e);
                            }
                            Err(_) => {
                                log::warn!("DB test query timed out");
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        log::warn!("Failed to get connection from pool: {}", e);
                    }
                    Err(_) => {
                        log::warn!("Connection pool get timed out");
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to create pool: {}", e);
            }
        }
        
        if attempts >= max_attempts {
            return Err(format!("Failed to initialize DB pool after {} attempts", max_attempts).into());
        }
        
        log::info!("Retrying in {:?}...", delay);
        tokio::time::sleep(delay).await;
        delay *= 2; // Exponential backoff
    }
}

/// Get a database connection from the pool with timeout handling
/// Returns None if the pool is not initialized or connection fails
pub async fn get_db_connection() -> Option<deadpool_postgres::Client> {
    let pool_guard = DB_POOL.read().await;
    if let Some(pool) = pool_guard.as_ref() {
        // Use timeout for CapRover environments
        let timeout_duration = if std::env::var("CAPROVER").is_ok() {
            std::time::Duration::from_secs(30)
        } else {
            std::time::Duration::from_secs(5)
        };
        
        match tokio::time::timeout(timeout_duration, pool.get()).await {
            Ok(Ok(client)) => Some(client),
            Ok(Err(e)) => {
                log::error!("Failed to get database connection from pool: {}", e);
                None
            }
            Err(_) => {
                log::error!("Database connection timed out after {:?}", timeout_duration);
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
