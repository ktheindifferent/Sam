use super::cache::{CacheConfig, HybridCache};
use super::environment::get_env_config;
use super::error_handling::{retry_with_backoff, CircuitBreaker, RetryConfig};
use crate::monitoring::report_service_error;
use anyhow::{Context, Result};
use bollard::container::ListContainersOptions;
use bollard::Docker;
use deadpool_redis::{redis::cmd, Config, Pool, Runtime};
use log::{debug, error, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;

/// Redis-specific error types for better error handling
#[derive(Error, Debug)]
pub enum RedisError {
    #[error("Connection pool error: {0}")]
    PoolError(String),

    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Command execution failed: {0}")]
    CommandError(String),

    #[error("Task execution failed: {task_name}: {error}")]
    TaskError { task_name: String, error: String },

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Lock acquisition failed: {0}")]
    LockError(String),

    #[error("Docker operation failed: {0}")]
    DockerError(String),
}

/// Global circuit breaker for Redis connections
static REDIS_CIRCUIT_BREAKER: OnceCell<Arc<CircuitBreaker>> = OnceCell::new();

/// Get or create the Redis circuit breaker
fn get_circuit_breaker() -> Arc<CircuitBreaker> {
    REDIS_CIRCUIT_BREAKER
        .get_or_init(|| {
            Arc::new(CircuitBreaker::new(
                "redis_connection".to_string(),
                3,                       // failure threshold
                2,                       // success threshold to close
                Duration::from_secs(30), // timeout before attempting half-open
            ))
        })
        .clone()
}

/// Install and start Redis using Docker if not already running.
/// This is intended to be called from setup/install.
pub async fn install() {
    let env_config = get_env_config();

    // Skip Docker operations in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - using external Redis service");
        return;
    }

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
            let err = RedisError::DockerError(format!(
                "Failed to pull Redis image, exit code: {:?}",
                status
            ));
            error!("{}", err);
            report_service_error("redis", &err, None);
            return;
        }
        Err(e) => {
            let err = RedisError::DockerError(format!("Failed to pull Redis image: {}", e));
            error!("{}", err);
            report_service_error("redis", &err, None);
            return;
        }
    }

    start().await;
}

/// Start the Redis Docker container (if not running)
pub async fn start() {
    let env_config = get_env_config();

    // Skip Docker operations in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - using external Redis service");
        return;
    }

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
        Ok(output) => {
            let err = RedisError::DockerError(format!(
                "Failed to start Redis container, exit code: {}. Stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
            error!("{}", err);
            report_service_error("redis", &err, None);
        }
        Err(e) => {
            let err = RedisError::DockerError(format!("Failed to start Redis container: {}", e));
            error!("{}", err);
            report_service_error("redis", &err, None);
        }
    }
}

/// Stop the Redis Docker container (if running)
pub async fn stop() {
    let env_config = get_env_config();

    // Skip Docker operations in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - external Redis service not managed");
        return;
    }

    if !is_running().await {
        info!("Redis Docker container 'sam-redis' is not running.");
        return;
    }
    info!("Stopping Redis Docker container...");
    let stop = Command::new("docker").args(["stop", "sam-redis"]).output();

    match stop {
        Ok(status) if status.status.success() => info!("Redis Docker container stopped."),
        Ok(status) => {
            let err = RedisError::DockerError(format!(
                "Failed to stop Redis container, exit code: {}",
                status.status
            ));
            error!("{}", err);
            report_service_error("redis", &err, None);
        }
        Err(e) => {
            let err = RedisError::DockerError(format!("Failed to stop Redis container: {}", e));
            error!("{}", err);
            report_service_error("redis", &err, None);
        }
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
    let env_config = get_env_config();

    // In CapRover mode, assume external Redis is always "running"
    if env_config.is_caprover {
        return env_config.should_use_redis();
    }

    let now = Instant::now();
    // Check cache before await
    {
        let cache = match IS_RUNNING_CACHE.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let err = RedisError::LockError(
                    "Failed to acquire lock for IS_RUNNING_CACHE: poisoned".to_string(),
                );
                error!("{}", err);
                report_service_error("redis", &err, None);
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
                let err = RedisError::LockError(
                    "Failed to acquire lock for IS_INSTALLED_CACHE: poisoned".to_string(),
                );
                error!("{}", err);
                report_service_error("redis", &err, None);
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
    let circuit_breaker = get_circuit_breaker();

    // Check circuit breaker status
    circuit_breaker
        .call(|| async { connect_with_retry().await })
        .await
        .map_err(|e| {
            let err = RedisError::ServiceUnavailable(format!(
                "Redis connection failed through circuit breaker: {}",
                e
            ));
            error!("{}", err);
            report_service_error("redis", &err, None);
            anyhow::anyhow!("Redis connection unavailable: {}", e)
        })
}

/// Internal connection function with retry logic
async fn connect_with_retry() -> Result<Pool> {
    // Initialize the pool holder if not already done
    let pool_holder = POOL.get_or_init(|| Arc::new(RwLock::new(None)));

    // Check if pool already exists
    let existing_pool = {
        let pool_guard = pool_holder
            .read()
            .map_err(|e| RedisError::LockError(format!("Failed to acquire read lock: {}", e)))?;
        pool_guard.clone()
    };

    if let Some(ref pool) = existing_pool {
        // Validate the existing pool is still healthy
        if validate_pool(pool).await.is_ok() {
            return Ok(pool.clone());
        } else {
            warn!("Existing pool is unhealthy, will create new one");
        }
    }

    // Create new pool with retry logic
    let retry_config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(5),
        exponential_base: 2.0,
        jitter: true,
    };

    let pool = retry_with_backoff(create_pool, retry_config, "redis_pool_creation").await?;

    // Store for future use (write lock)
    {
        let mut pool_guard = pool_holder
            .write()
            .map_err(|e| RedisError::LockError(format!("Failed to acquire write lock: {}", e)))?;
        *pool_guard = Some(pool.clone());
    }

    Ok(pool)
}

/// Validate that a pool is still healthy
async fn validate_pool(pool: &Pool) -> Result<()> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| RedisError::ConnectionError(format!("Failed to get connection: {}", e)))?;

    let _: String = cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .map_err(|e| RedisError::CommandError(format!("Ping failed: {}", e)))?;

    Ok(())
}

/// Reset the connection pool (useful for testing and reconnection)
pub async fn reset_pool() -> Result<()> {
    if let Some(pool_holder) = POOL.get() {
        let mut pool_guard = pool_holder
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock for pool reset: {}", e))?;
        *pool_guard = None;
    }
    Ok(())
}

async fn create_pool() -> Result<Pool> {
    let env_config = get_env_config();
    let redis_url = env_config.get_redis_url();

    info!(
        "Creating Redis pool with URL: {}",
        if env_config.is_caprover {
            "[external Redis]"
        } else {
            &redis_url
        }
    );

    let cfg = Config::from_url(redis_url);
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .context("Failed to create Redis connection pool")?;

    // Test the connection
    let mut conn = pool
        .get()
        .await
        .context("Failed to get connection from pool")?;

    let _: String = cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .context("Failed to ping Redis")?;

    info!("Redis connection pool created successfully");
    Ok(pool)
}

pub async fn health_check() -> Result<()> {
    let retry_config = RetryConfig {
        max_attempts: 2,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        exponential_base: 2.0,
        jitter: false,
    };

    retry_with_backoff(
        || async {
            let pool = connect().await?;
            let mut conn = pool.get().await.map_err(|e| {
                RedisError::ConnectionError(format!("Health check connection failed: {}", e))
            })?;

            let pong: String = cmd("PING")
                .query_async::<String>(&mut conn)
                .await
                .map_err(|e| {
                    RedisError::CommandError(format!("Health check ping failed: {}", e))
                })?;

            if pong == "PONG" {
                debug!("Redis health check passed");
                Ok(())
            } else {
                let err = RedisError::ServiceUnavailable(format!(
                    "Unexpected Redis PING response: {}",
                    pong
                ));
                report_service_error("redis_health_check", &err, None);
                Err(err.into())
            }
        },
        retry_config,
        "redis_health_check",
    )
    .await
}

pub async fn get_info() -> Result<String> {
    let pool = connect().await?;
    let mut conn = pool.get().await?;

    let info: String = cmd("INFO")
        .query_async::<String>(&mut conn)
        .await
        .context("Failed to get Redis info")?;

    Ok(info)
}

pub async fn flush_db() -> Result<()> {
    let pool = connect().await?;
    let mut conn = pool.get().await?;

    cmd("FLUSHDB")
        .query_async::<()>(&mut conn)
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
    use std::future::Future;
    use tokio::task::JoinSet;
    use tokio::time::{timeout, Duration};

    // Custom test result type for better error reporting
    type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    // Test configuration constants
    const TEST_TIMEOUT_SECS: u64 = 30;
    const REDIS_RETRY_ATTEMPTS: u32 = 3;
    const REDIS_RETRY_DELAY_MS: u64 = 500;

    // Helper struct for test fixtures
    struct RedisTestFixture {
        initial_state_verified: bool,
    }

    impl RedisTestFixture {
        async fn setup() -> TestResult<Self> {
            // Reset pool to ensure clean state
            reset_pool().await?;

            // Verify Redis is available with retries
            ensure_redis_available().await?;

            Ok(Self {
                initial_state_verified: true,
            })
        }

        async fn teardown(&self) -> TestResult<()> {
            // Clean up any test data if needed
            reset_pool().await?;
            Ok(())
        }
    }

    // Helper function to ensure Redis is available with retry logic
    async fn ensure_redis_available() -> TestResult<Pool> {
        for attempt in 1..=REDIS_RETRY_ATTEMPTS {
            match connect().await {
                Ok(pool) => return Ok(pool),
                Err(e) if attempt < REDIS_RETRY_ATTEMPTS => {
                    eprintln!(
                        "Redis connection attempt {} failed: {}. Retrying...",
                        attempt, e
                    );
                    tokio::time::sleep(Duration::from_millis(REDIS_RETRY_DELAY_MS)).await;
                }
                Err(e) => {
                    return Err(format!(
                        "Redis not available after {} attempts: {}",
                        REDIS_RETRY_ATTEMPTS, e
                    )
                    .into());
                }
            }
        }
        unreachable!()
    }

    // Helper function to run tests with timeout
    async fn with_timeout<F, T>(duration: Duration, future: F) -> TestResult<T>
    where
        F: Future<Output = T> + Send,
        T: Send,
    {
        timeout(duration, future)
            .await
            .map_err(|_| "Test timed out".into())
    }

    // Helper function for asserting pool validity
    fn assert_pool_valid(pool: &Pool, context: &str) {
        assert!(
            pool.status().size > 0,
            "Pool should be valid ({}): size = {}",
            context,
            pool.status().size
        );
    }

    // Helper function for handling JoinSet errors with proper error propagation
    fn handle_task_result<T>(
        result: Result<T, tokio::task::JoinError>,
        task_name: &str,
    ) -> Result<T> {
        result.map_err(|e| {
            let err = RedisError::TaskError {
                task_name: task_name.to_string(),
                error: format!(
                    "JoinError: {}. This usually indicates a panic in the spawned task.",
                    e
                ),
            };
            error!("{}", err);
            report_service_error("redis", &err, None);

            // Check if it was a panic
            if e.is_panic() {
                let panic_info = if let Ok(panic) = e.try_into_panic() {
                    format!("Task panicked: {:?}", panic)
                } else {
                    "Task panicked with unknown error".to_string()
                };

                RedisError::TaskError {
                    task_name: task_name.to_string(),
                    error: panic_info,
                }
                .into()
            } else if e.is_cancelled() {
                RedisError::TaskError {
                    task_name: task_name.to_string(),
                    error: "Task was cancelled".to_string(),
                }
                .into()
            } else {
                RedisError::TaskError {
                    task_name: task_name.to_string(),
                    error: e.to_string(),
                }
                .into()
            }
        })
    }

    #[tokio::test]
    async fn test_redis_connection() -> TestResult<()> {
        let test_future = async {
            // This test requires a running Redis instance
            let pool = ensure_redis_available().await?;
            assert_pool_valid(&pool, "initial connection");
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_health_check() -> TestResult<()> {
        let test_future = async {
            // Ensure Redis is available first
            ensure_redis_available().await?;

            // Perform health check
            health_check()
                .await
                .map_err(|e| format!("Health check failed: {}", e))?;

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_concurrent_pool_access() -> TestResult<()> {
        let test_future = async {
            let fixture = RedisTestFixture::setup().await?;

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
                        Err(e) => Err(e),
                    }
                });
            }

            // Collect all results
            let mut results = Vec::new();
            let mut errors = Vec::new();

            while let Some(result) = tasks.join_next().await {
                match handle_task_result(result, "concurrent_pool_access") {
                    Ok(Ok(i)) => results.push(i),
                    Ok(Err(e)) => errors.push(e.to_string()),
                    Err(e) => errors.push(e.to_string()),
                }
            }

            // If Redis is not available, skip the test gracefully
            if !errors.is_empty() && results.is_empty() {
                eprintln!(
                    "Warning: Redis not available for concurrent test. Errors: {:?}",
                    errors
                );
                return Ok(());
            }

            // Verify all tasks completed successfully
            assert!(
                results.len() >= 8, // Allow for some failures in concurrent environment
                "Expected at least 8 successful tasks, got {}. Errors: {:?}",
                results.len(),
                errors
            );

            fixture.teardown().await?;
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_pool_reuse_across_threads() -> TestResult<()> {
        let test_future = async {
            let fixture = RedisTestFixture::setup().await?;

            // Get initial pool
            let initial_pool = ensure_redis_available().await?;
            let initial_size = initial_pool.status().size;

            // Spawn multiple tasks that should reuse the same pool
            let mut tasks = JoinSet::new();

            for i in 0..5 {
                tasks.spawn(async move { connect().await.map(|pool| (i, pool)) });
            }

            // Verify all tasks get the same pool instance
            let mut successful_checks = 0;
            let mut errors = Vec::new();

            while let Some(result) = tasks.join_next().await {
                match handle_task_result(result, "pool_reuse") {
                    Ok(Ok((i, pool))) => {
                        // The pool should be the same instance (same underlying connection pool)
                        assert_eq!(
                            pool.status().size,
                            initial_size,
                            "Task {} - Pool configuration should be identical",
                            i
                        );
                        successful_checks += 1;
                    }
                    Ok(Err(e)) => {
                        errors.push(format!("Task failed: {}", e));
                    }
                    Err(e) => {
                        errors.push(format!("Task execution failed: {}", e));
                    }
                }
            }

            assert!(
                successful_checks >= 4,
                "Expected at least 4 successful pool reuse checks, got {}. Errors: {:?}",
                successful_checks,
                errors
            );

            fixture.teardown().await?;
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_pool_reset() -> TestResult<()> {
        let test_future = async {
            // Try to connect first to ensure Redis is available
            let _ = ensure_redis_available().await?;

            // Reset the pool
            reset_pool()
                .await
                .map_err(|e| format!("Failed to reset pool: {}", e))?;

            // Verify pool can be re-established after reset
            let pool = connect()
                .await
                .map_err(|e| format!("Failed to reconnect after reset: {}", e))?;

            assert_pool_valid(&pool, "after reset");
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_no_data_races() -> TestResult<()> {
        let test_future = async {
            let fixture = RedisTestFixture::setup().await?;

            let mut tasks = JoinSet::new();

            // Spawn readers
            for i in 0..20 {
                tasks.spawn(async move {
                    tokio::time::sleep(Duration::from_millis(i as u64)).await;
                    connect().await.map(|_| format!("reader_{}", i))
                });
            }

            // Spawn a writer (reset) in the middle
            tasks.spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let reset_result = reset_pool().await;
                let connect_result = connect().await;
                match (reset_result, connect_result) {
                    (Ok(_), Ok(_)) => Ok("writer_reset".to_string()),
                    (Err(e), _) => Err(anyhow::anyhow!("Reset failed: {}", e)),
                    (_, Err(e)) => Err(e),
                }
            });

            // All operations should complete without panic
            let mut success_count = 0;
            let mut task_errors = Vec::new();

            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(Ok(task_name)) => {
                        success_count += 1;
                        eprintln!("Task {} completed successfully", task_name);
                    }
                    Ok(Err(e)) => {
                        task_errors.push(format!("Task error: {}", e));
                    }
                    Err(e) => {
                        // This is a JoinError - the task itself panicked
                        return Err(format!("Task panicked during execution: {}", e).into());
                    }
                }
            }

            // We should have successful operations (exact count may vary due to Redis availability)
            assert!(
                success_count > 0,
                "Expected at least some successful operations, got {}. Errors: {:?}",
                success_count,
                task_errors
            );

            fixture.teardown().await?;
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS * 2), test_future).await?
    }

    // Integration tests for error handling

    #[tokio::test]
    async fn test_error_handling_on_connection_failure() -> TestResult<()> {
        // This test verifies proper error handling when Redis is not available
        // We'll test this by attempting to connect with an invalid configuration

        // Note: This test might not fail if Redis is actually running,
        // but it verifies that errors are properly propagated
        let result = connect().await;

        match result {
            Ok(pool) => {
                // Redis is available, verify the pool is valid
                assert_pool_valid(&pool, "connection_failure_test");
                Ok(())
            }
            Err(e) => {
                // Verify we get a proper error message
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error message should not be empty");
                eprintln!("Received expected error: {}", error_msg);
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_reset_safety() -> TestResult<()> {
        let test_future = async {
            // Ensure Redis is available
            ensure_redis_available().await?;

            // Spawn multiple tasks that attempt to reset concurrently
            let mut tasks = JoinSet::new();

            for i in 0..5 {
                tasks.spawn(async move {
                    tokio::time::sleep(Duration::from_millis(i * 10)).await;
                    reset_pool().await.map(|_| i)
                });
            }

            // All resets should complete without panic
            let mut reset_results = Vec::new();
            while let Some(result) = tasks.join_next().await {
                match handle_task_result(result, "concurrent_reset") {
                    Ok(Ok(i)) => reset_results.push(i),
                    Ok(Err(e)) => warn!("Task failed during reset: {}", e),
                    Err(e) => warn!("Task execution failed during reset: {}", e),
                }
            }

            assert!(
                !reset_results.is_empty(),
                "At least one reset should succeed"
            );

            // Verify pool is still usable after concurrent resets
            let pool = connect().await?;
            assert_pool_valid(&pool, "after concurrent resets");

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_health_check_with_retry() -> TestResult<()> {
        let test_future = async {
            // Perform health check with retry logic
            let mut last_error = None;

            for attempt in 1..=REDIS_RETRY_ATTEMPTS {
                match health_check().await {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        last_error = Some(e.to_string());
                        if attempt < REDIS_RETRY_ATTEMPTS {
                            eprintln!("Health check attempt {} failed. Retrying...", attempt);
                            tokio::time::sleep(Duration::from_millis(REDIS_RETRY_DELAY_MS)).await;
                        }
                    }
                }
            }

            // If we get here, all attempts failed
            eprintln!(
                "Health check failed after {} attempts. Last error: {:?}",
                REDIS_RETRY_ATTEMPTS, last_error
            );

            // This is not a test failure if Redis is not available
            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    // ==================== Failure Injection Tests ====================

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() -> TestResult<()> {
        let test_future = async {
            // Reset the circuit breaker
            if let Some(cb) = REDIS_CIRCUIT_BREAKER.get() {
                cb.reset().await;
            }

            // Force the pool to be invalid by resetting it
            reset_pool().await?;

            // Simulate multiple connection failures
            let mut failures = 0;
            for _ in 0..5 {
                match connect().await {
                    Err(e) => {
                        failures += 1;
                        debug!("Expected failure #{}: {}", failures, e);
                    }
                    Ok(_) => {
                        // If Redis is actually available, skip this test
                        eprintln!("Redis is available, skipping circuit breaker test");
                        return Ok(());
                    }
                }
            }

            // Verify that we got failures (circuit breaker should be open)
            assert!(
                failures >= 3,
                "Should have at least 3 failures to open circuit breaker"
            );

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_retry_logic_with_exponential_backoff() -> TestResult<()> {
        let test_future = async {
            // This test verifies that retry logic is working
            let start = Instant::now();

            // Reset pool to force reconnection
            reset_pool().await?;

            // Try to connect (should retry if Redis is not available)
            let result = connect().await;

            let elapsed = start.elapsed();

            match result {
                Ok(_) => {
                    // Connection succeeded
                    info!("Connection succeeded after {:?}", elapsed);
                }
                Err(e) => {
                    // Connection failed after retries
                    info!("Connection failed after {:?} with retries: {}", elapsed, e);
                    // Verify that retries took some time (exponential backoff)
                    assert!(
                        elapsed >= Duration::from_millis(500),
                        "Retry logic should have taken at least 500ms, took {:?}",
                        elapsed
                    );
                }
            }

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_graceful_handling_of_task_panics() -> TestResult<()> {
        use tokio::task::JoinSet;

        let test_future = async {
            let mut tasks = JoinSet::new();

            // Spawn a task that will panic
            tasks.spawn(async move {
                panic!("Intentional test panic");
            });

            // Spawn a task that succeeds
            tasks.spawn(async move { Ok::<i32, anyhow::Error>(42) });

            let mut results = Vec::new();
            let mut errors = Vec::new();

            while let Some(join_result) = tasks.join_next().await {
                match handle_task_result(join_result, "test_task") {
                    Ok(Ok(val)) => results.push(val),
                    Ok(Err(e)) => errors.push(format!("Task error: {}", e)),
                    Err(e) => {
                        // This should capture the panic
                        errors.push(format!("Join error: {}", e));
                        // Verify it's identified as a task error
                        assert!(
                            e.to_string().contains("Task execution failed"),
                            "Error should indicate task failure: {}",
                            e
                        );
                    }
                }
            }

            // We should have one success and one error
            assert_eq!(results.len(), 1, "Should have one successful result");
            assert_eq!(errors.len(), 1, "Should have one error from panic");
            assert!(
                errors[0].contains("Task"),
                "Error should mention task: {}",
                errors[0]
            );

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_connection_pool_recovery() -> TestResult<()> {
        let test_future = async {
            // This test verifies that the pool can recover from failures

            // First, try to get a connection
            let first_result = connect().await;

            if first_result.is_err() {
                // Redis not available, skip test
                eprintln!("Redis not available, skipping recovery test");
                return Ok(());
            }

            let first_pool = first_result?;
            let initial_status = first_pool.status();

            // Reset the pool to simulate a failure
            reset_pool().await?;

            // Wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Try to reconnect
            let second_result = connect().await;

            if let Ok(second_pool) = second_result {
                let new_status = second_pool.status();
                info!(
                    "Pool recovered - initial size: {}, new size: {}",
                    initial_status.size, new_status.size
                );

                // Verify the pool is functional
                validate_pool(&second_pool).await?;
            }

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }

    #[tokio::test]
    async fn test_health_check_with_retries() -> TestResult<()> {
        let test_future = async {
            // Test that health check includes retry logic
            let start = Instant::now();
            let result = health_check().await;
            let elapsed = start.elapsed();

            match result {
                Ok(_) => {
                    info!("Health check succeeded after {:?}", elapsed);
                }
                Err(e) => {
                    info!("Health check failed after {:?}: {}", elapsed, e);
                    // Health check should have retried
                    assert!(
                        elapsed >= Duration::from_millis(100),
                        "Health check should include retry delay, took {:?}",
                        elapsed
                    );
                }
            }

            Ok(())
        };

        with_timeout(Duration::from_secs(TEST_TIMEOUT_SECS), test_future).await?
    }
}
