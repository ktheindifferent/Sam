use bollard::container::ListContainersOptions;
use bollard::Docker;
use log::{error, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use std::process::Command;
use std::sync::{Arc, RwLock, Mutex};
use std::time::Duration;
use std::time::Instant;
use anyhow::{Result, Context};
use deadpool_redis::{Config, Runtime, Pool, redis::cmd};
use super::cache::{HybridCache, CacheConfig};

/// Install and start Redis using Docker if not already running.
/// This is intended to be called from setup/install.
pub async fn install() {
    info!("Checking for running Redis Docker container...");
    if is_running().await {
        info!("Redis Docker container 'sam-redis' is already running.");
        return;
    }
    info!("Pulling Redis Docker image...");
    let pull = Command::new("docker").args(["pull", "redis:7"]).output();

    match pull {
        Ok(status) if status.status.success() => info!("Redis Docker image pulled successfully."),
        Ok(status) => {
            error!("Failed to pull Redis image, exit code: {:?}", status);
            return;
        }
        Err(e) => {
            error!("Failed to pull Redis image: {}", e);
            return;
        }
    }

    start().await;
}

/// Start the Redis Docker container (if not running)
pub async fn start() {
    if is_running().await {
        info!("Redis Docker container 'sam-redis' is already running.");
        return;
    }
    info!("Starting Redis Docker container...");
    let run = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            "sam-redis",
            "-p",
            "6379:6379",
            "--restart",
            "unless-stopped",
            "redis:7",
        ])
        .output(); // changed from .status() to .output()

    match run {
        Ok(output) if output.status.success() => {
            info!("Redis Docker container started as 'sam-redis'.");
            // Optionally log container id: String::from_utf8_lossy(&output.stdout)
        }
        Ok(output) => error!(
            "Failed to start Redis container, exit code: {}. Stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) => error!("Failed to start Redis container: {}", e),
    }
}

/// Stop the Redis Docker container (if running)
pub async fn stop() {
    if !is_running().await {
        info!("Redis Docker container 'sam-redis' is not running.");
        return;
    }
    info!("Stopping Redis Docker container...");
    let stop = Command::new("docker").args(["stop", "sam-redis"]).output();

    match stop {
        Ok(status) if status.status.success() => info!("Redis Docker container stopped."),
        Ok(status) => error!(
            "Failed to stop Redis container, exit code: {}",
            status.status
        ),
        Err(e) => error!("Failed to stop Redis container: {}", e),
    }
    // Optionally remove the container after stopping
    let rm = Command::new("docker").args(["rm", "sam-redis"]).output();
    match rm {
        Ok(status) if status.status.success() => info!("Redis Docker container removed."),
        Ok(_) => {} // ignore errors if already removed
        Err(_) => {}
    }
}

/// Return the status of the Redis Docker container: "running", "stopped", or "not installed"
pub async fn status() -> &'static str {
    if is_running().await {
        "running"
    } else if is_installed().await {
        "stopped"
    } else {
        "not installed"
    }
}

/// Helper: check if the Redis Docker container is running
// Native Rust cannot directly interact with Docker without using its CLI or a Docker API client.
// For a faster, native approach, use the `bollard` crate (Docker API client for Rust).
// Add `bollard = "0.15"` to your Cargo.toml dependencies.

struct RunningCache {
    value: Option<(bool, Instant)>,
}

static IS_RUNNING_CACHE: Lazy<Mutex<RunningCache>> =
    Lazy::new(|| Mutex::new(RunningCache { value: None }));

pub async fn is_running() -> bool {
    let now = Instant::now();
    // Check cache before await
    {
        let cache = match IS_RUNNING_CACHE.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Failed to acquire lock for IS_RUNNING_CACHE: poisoned");
                poisoned.into_inner()
            }
        };
        if let Some((cached, timestamp)) = cache.value {
            if now.duration_since(timestamp) < Duration::from_secs(600) {
                return cached;
            }
        }
    }
    // Not cached or expired, check Docker
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => {
            if let Ok(mut cache) = IS_RUNNING_CACHE.lock() {
                cache.value = Some((false, now));
            }
            return false;
        }
    };
    let options = Some(ListContainersOptions::<String> {
        all: false, // Only running containers
        filters: {
            let mut map = std::collections::HashMap::new();
            map.insert("name".to_string(), vec!["sam-redis".to_string()]);
            map
        },
        ..Default::default()
    });
    let result = match docker.list_containers(options).await {
        Ok(containers) => containers.iter().any(|c| {
            c.names
                .as_ref()
                .is_some_and(|names| names.iter().any(|n| n.contains("sam-redis")))
        }),
        Err(_) => false,
    };
    if let Ok(mut cache) = IS_RUNNING_CACHE.lock() {
        cache.value = Some((result, now));
    }
    result
}

/// Helper: check if the Redis Docker container exists (installed)
struct InstalledCache {
    value: Option<(bool, Instant)>,
}

static IS_INSTALLED_CACHE: Lazy<Mutex<InstalledCache>> =
    Lazy::new(|| Mutex::new(InstalledCache { value: None }));

pub async fn is_installed() -> bool {
    let now = Instant::now();
    // Check cache before await
    {
        let cache = match IS_INSTALLED_CACHE.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Failed to acquire lock for IS_INSTALLED_CACHE: poisoned");
                poisoned.into_inner()
            }
        };
        if let Some((cached, timestamp)) = cache.value {
            if now.duration_since(timestamp) < Duration::from_secs(600) {
                return cached;
            }
        }
    }
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => {
            if let Ok(mut cache) = IS_INSTALLED_CACHE.lock() {
                cache.value = Some((false, now));
            }
            return false;
        }
    };
    let options = Some(ListContainersOptions::<String> {
        all: true, // Include stopped containers
        filters: {
            let mut map = std::collections::HashMap::new();
            map.insert("name".to_string(), vec!["sam-redis".to_string()]);
            map
        },
        ..Default::default()
    });
    let result = match docker.list_containers(options).await {
        Ok(containers) => containers.iter().any(|c| {
            c.names
                .as_ref()
                .is_some_and(|names| names.iter().any(|n| n.contains("sam-redis")))
        }),
        Err(_) => false,
    };
    if let Ok(mut cache) = IS_INSTALLED_CACHE.lock() {
        cache.value = Some((result, now));
    }
    result
}

// Thread-safe connection pool management using OnceCell
static POOL: OnceCell<Arc<RwLock<Option<Pool>>>> = OnceCell::new();

pub async fn connect() -> Result<Pool> {
    // Initialize the pool holder if not already done
    let pool_holder = POOL.get_or_init(|| Arc::new(RwLock::new(None)));
    
    // Check if pool already exists
    {
        let pool_guard = pool_holder.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock for pool: {}", e))?;
        if let Some(ref pool) = *pool_guard {
            return Ok(pool.clone());
        }
    }
    
    // Create new pool
    let pool = create_pool().await?;
    
    // Store for future use (write lock)
    {
        let mut pool_guard = pool_holder.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock for pool: {}", e))?;
        *pool_guard = Some(pool.clone());
    }
    
    Ok(pool)
}

/// Reset the connection pool (useful for testing and reconnection)
pub async fn reset_pool() -> Result<()> {
    if let Some(pool_holder) = POOL.get() {
        let mut pool_guard = pool_holder.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock for pool reset: {}", e))?;
        *pool_guard = None;
    }
    Ok(())
}

async fn create_pool() -> Result<Pool> {
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    let cfg = Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))
        .context("Failed to create Redis connection pool")?;
    
    // Test the connection
    let mut conn = pool.get().await
        .context("Failed to get connection from pool")?;
    
    let _: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .context("Failed to ping Redis")?;
    
    info!("Redis connection pool created successfully");
    Ok(pool)
}

pub async fn health_check() -> Result<()> {
    let pool = connect().await?;
    let mut conn = pool.get().await
        .context("Failed to get connection for health check")?;
    
    let pong: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .context("Redis health check failed")?;
    
    if pong == "PONG" {
        info!("Redis health check passed");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Redis health check failed: unexpected response"))
    }
}

pub async fn get_info() -> Result<String> {
    let pool = connect().await?;
    let mut conn = pool.get().await?;
    
    let info: String = cmd("INFO")
        .query_async(&mut conn)
        .await
        .context("Failed to get Redis info")?;
    
    Ok(info)
}

pub async fn flush_db() -> Result<()> {
    let pool = connect().await?;
    let mut conn = pool.get().await?;
    
    cmd("FLUSHDB")
        .query_async(&mut conn)
        .await
        .context("Failed to flush Redis database")?;
    
    warn!("Redis database flushed");
    Ok(())
}

pub async fn get_pool_status() -> Result<String> {
    let pool = connect().await?;
    let status = pool.status();
    
    Ok(format!(
        "Pool Status - Size: {}, Available: {}, Waiting: {}",
        status.size, status.available, status.waiting
    ))
}

/// Create a new HybridCache instance with the Redis pool
pub async fn create_cache() -> Result<HybridCache> {
    let pool = connect().await?;
    let config = CacheConfig::default();
    HybridCache::new(pool, config).await
}

/// Create a new HybridCache instance with custom configuration
pub async fn create_cache_with_config(config: CacheConfig) -> Result<HybridCache> {
    let pool = connect().await?;
    HybridCache::new(pool, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_redis_connection() {
        // This test requires a running Redis instance
        match connect().await {
            Ok(pool) => {
                assert!(pool.status().size > 0);
            }
            Err(e) => {
                eprintln!("Skipping test - Redis not available: {}", e);
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
                eprintln!("Skipping test - Redis not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_pool_access() {
        // Reset pool to ensure clean state
        let _ = reset_pool().await;
        
        // Spawn multiple concurrent tasks to access the pool
        let mut tasks = JoinSet::new();
        
        for i in 0..10 {
            tasks.spawn(async move {
                let result = connect().await;
                match result {
                    Ok(pool) => {
                        // Verify pool is valid
                        assert!(pool.status().size > 0, "Task {} got invalid pool", i);
                        Ok(i)
                    }
                    Err(e) => Err(e)
                }
            });
        }
        
        // Collect all results
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(i)) => results.push(i),
                Ok(Err(e)) => {
                    eprintln!("Skipping concurrent test - Redis not available: {}", e);
                    return;
                }
                Err(e) => panic!("Task panicked: {}", e),
            }
        }
        
        // Verify all tasks completed successfully
        assert_eq!(results.len(), 10, "Not all tasks completed successfully");
    }

    #[tokio::test]
    async fn test_pool_reuse_across_threads() {
        // Reset pool to ensure clean state
        let _ = reset_pool().await;
        
        // Get initial pool
        let initial_pool = match connect().await {
            Ok(pool) => pool,
            Err(e) => {
                eprintln!("Skipping test - Redis not available: {}", e);
                return;
            }
        };
        
        // Spawn multiple tasks that should reuse the same pool
        let mut tasks = JoinSet::new();
        
        for _ in 0..5 {
            tasks.spawn(async move {
                connect().await
            });
        }
        
        // Verify all tasks get the same pool instance
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(pool)) => {
                    // The pool should be the same instance (same underlying connection pool)
                    assert_eq!(
                        pool.status().size, 
                        initial_pool.status().size,
                        "Pool configuration should be identical"
                    );
                }
                Ok(Err(e)) => {
                    eprintln!("Error in concurrent access: {}", e);
                    return;
                }
                Err(e) => panic!("Task panicked: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_pool_reset() {
        // Connect to establish a pool
        let _ = connect().await;
        
        // Reset the pool
        reset_pool().await.expect("Failed to reset pool");
        
        // Verify pool can be re-established after reset
        match connect().await {
            Ok(pool) => {
                assert!(pool.status().size > 0, "Pool should be valid after reset");
            }
            Err(e) => {
                eprintln!("Skipping test - Redis not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_no_data_races() {
        // This test verifies thread safety by attempting concurrent reads and writes
        let _ = reset_pool().await;
        
        let mut tasks = JoinSet::new();
        
        // Spawn readers
        for i in 0..20 {
            tasks.spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(i as u64)).await;
                connect().await
            });
        }
        
        // Spawn a writer (reset) in the middle
        tasks.spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            reset_pool().await.expect("Failed to reset");
            connect().await
        });
        
        // All operations should complete without panic
        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            if let Ok(Ok(_)) = result {
                success_count += 1;
            }
        }
        
        // We should have successful operations (exact count may vary due to Redis availability)
        assert!(success_count > 0, "At least some operations should succeed");
    }
}
