use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Default requests per window for authenticated users
    pub default_authenticated_limit: u32,
    /// Default requests per window for anonymous users
    pub default_anonymous_limit: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Enable distributed rate limiting via Redis
    pub use_redis: bool,
    /// Endpoint-specific limits
    pub endpoint_limits: HashMap<String, EndpointLimit>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut endpoint_limits = HashMap::new();

        // Configure specific endpoint limits
        endpoint_limits.insert(
            "/api/auth/login".to_string(),
            EndpointLimit {
                authenticated: 10,
                anonymous: 5,
                window_seconds: 300, // 5 minutes
                burst_size: 3,
            },
        );

        endpoint_limits.insert(
            "/api/auth/register".to_string(),
            EndpointLimit {
                authenticated: 5,
                anonymous: 3,
                window_seconds: 3600, // 1 hour
                burst_size: 2,
            },
        );

        endpoint_limits.insert(
            "/api/voice/transcribe".to_string(),
            EndpointLimit {
                authenticated: 100,
                anonymous: 10,
                window_seconds: 60,
                burst_size: 5,
            },
        );

        endpoint_limits.insert(
            "/api/crawler/crawl".to_string(),
            EndpointLimit {
                authenticated: 50,
                anonymous: 5,
                window_seconds: 300,
                burst_size: 3,
            },
        );

        endpoint_limits.insert(
            "/api/ai/generate".to_string(),
            EndpointLimit {
                authenticated: 30,
                anonymous: 0, // No access for anonymous
                window_seconds: 60,
                burst_size: 5,
            },
        );

        RateLimitConfig {
            default_authenticated_limit: 1000,
            default_anonymous_limit: 100,
            window_seconds: 60,
            use_redis: true,
            endpoint_limits,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EndpointLimit {
    pub authenticated: u32,
    pub anonymous: u32,
    pub window_seconds: u64,
    pub burst_size: u32,
}

/// Rate limiter bucket
#[derive(Debug, Clone)]
struct RateLimitBucket {
    count: u32,
    window_start: Instant,
    burst_tokens: u32,
    last_refill: Instant,
}

impl RateLimitBucket {
    fn new(burst_size: u32) -> Self {
        RateLimitBucket {
            count: 0,
            window_start: Instant::now(),
            burst_tokens: burst_size,
            last_refill: Instant::now(),
        }
    }

    fn reset(&mut self, burst_size: u32) {
        self.count = 0;
        self.window_start = Instant::now();
        self.burst_tokens = burst_size;
        self.last_refill = Instant::now();
    }

    fn refill_burst(&mut self, burst_size: u32, refill_rate: Duration) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() / refill_rate.as_secs_f64()) as u32;

        if tokens_to_add > 0 {
            self.burst_tokens = (self.burst_tokens + tokens_to_add).min(burst_size);
            self.last_refill = now;
        }
    }
}

/// Rate limiter implementation
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
    redis_client: Option<Arc<RwLock<redis::Client>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let redis_client = if config.use_redis {
            Self::init_redis_client()
        } else {
            None
        };

        RateLimiter {
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
            redis_client,
        }
    }

    /// Initialize Redis client for distributed rate limiting
    fn init_redis_client() -> Option<Arc<RwLock<redis::Client>>> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        match redis::Client::open(redis_url) {
            Ok(client) => {
                info!("Redis client initialized for distributed rate limiting");
                Some(Arc::new(RwLock::new(client)))
            }
            Err(e) => {
                warn!("Failed to connect to Redis for rate limiting: {}. Using in-memory rate limiting.", e);
                None
            }
        }
    }

    /// Check if a request should be rate limited
    pub async fn check_rate_limit(
        &self,
        endpoint: &str,
        client_id: &str,
        is_authenticated: bool,
    ) -> Result<RateLimitStatus, RateLimitError> {
        // Use Redis if available
        if self.redis_client.is_some() {
            return self
                .check_redis_rate_limit(endpoint, client_id, is_authenticated)
                .await;
        }

        // Fall back to in-memory rate limiting
        self.check_memory_rate_limit(endpoint, client_id, is_authenticated)
            .await
    }

    /// Check rate limit using Redis
    async fn check_redis_rate_limit(
        &self,
        endpoint: &str,
        client_id: &str,
        is_authenticated: bool,
    ) -> Result<RateLimitStatus, RateLimitError> {
        let redis_client = self.redis_client.as_ref().ok_or_else(|| {
            RateLimitError::RedisError("Redis client is not configured".to_string())
        })?;
        let mut conn = redis_client
            .read()
            .await
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        let limit_config = self.get_limit_config(endpoint, is_authenticated);
        let key = format!("rate_limit:{}:{}", endpoint, client_id);
        let window = Duration::from_secs(limit_config.window_seconds);

        // Use Redis INCR with TTL
        let count: u32 = deadpool_redis::redis::cmd("INCR")
            .arg(&key)
            .query_async::<u32>(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        if count == 1 {
            // First request in window, set TTL
            deadpool_redis::redis::cmd("EXPIRE")
                .arg(&key)
                .arg(window.as_secs())
                .query_async::<i32>(&mut conn)
                .await
                .map_err(|e| RateLimitError::RedisError(e.to_string()))?;
        }

        if count > limit_config.limit {
            // Get TTL for retry-after header
            let ttl: i64 = deadpool_redis::redis::cmd("TTL")
                .arg(&key)
                .query_async::<i64>(&mut conn)
                .await
                .unwrap_or(window.as_secs() as i64);

            Ok(RateLimitStatus::Limited {
                retry_after_seconds: ttl.max(1) as u64,
                limit: limit_config.limit,
                remaining: 0,
                reset_at: Instant::now() + Duration::from_secs(ttl as u64),
            })
        } else {
            Ok(RateLimitStatus::Allowed {
                limit: limit_config.limit,
                remaining: limit_config.limit - count,
                reset_at: Instant::now() + window,
            })
        }
    }

    /// Check rate limit using in-memory storage
    async fn check_memory_rate_limit(
        &self,
        endpoint: &str,
        client_id: &str,
        is_authenticated: bool,
    ) -> Result<RateLimitStatus, RateLimitError> {
        let limit_config = self.get_limit_config(endpoint, is_authenticated);
        let key = format!("{}:{}", endpoint, client_id);
        let window = Duration::from_secs(limit_config.window_seconds);

        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| RateLimitBucket::new(limit_config.burst_size));

        let now = Instant::now();

        // Check if window has expired
        if now.duration_since(bucket.window_start) >= window {
            bucket.reset(limit_config.burst_size);
        }

        // Refill burst tokens
        bucket.refill_burst(limit_config.burst_size, Duration::from_secs(1));

        // Check if request can proceed
        if bucket.count >= limit_config.limit {
            // Check burst tokens
            if bucket.burst_tokens > 0 {
                bucket.burst_tokens -= 1;
                bucket.count += 1;

                Ok(RateLimitStatus::AllowedWithBurst {
                    limit: limit_config.limit,
                    remaining: 0,
                    burst_remaining: bucket.burst_tokens,
                    reset_at: bucket.window_start + window,
                })
            } else {
                let retry_after = window - now.duration_since(bucket.window_start);

                Ok(RateLimitStatus::Limited {
                    retry_after_seconds: retry_after.as_secs(),
                    limit: limit_config.limit,
                    remaining: 0,
                    reset_at: bucket.window_start + window,
                })
            }
        } else {
            bucket.count += 1;

            Ok(RateLimitStatus::Allowed {
                limit: limit_config.limit,
                remaining: limit_config.limit - bucket.count,
                reset_at: bucket.window_start + window,
            })
        }
    }

    /// Get limit configuration for an endpoint
    fn get_limit_config(&self, endpoint: &str, is_authenticated: bool) -> LimitConfig {
        if let Some(endpoint_limit) = self.config.endpoint_limits.get(endpoint) {
            LimitConfig {
                limit: if is_authenticated {
                    endpoint_limit.authenticated
                } else {
                    endpoint_limit.anonymous
                },
                window_seconds: endpoint_limit.window_seconds,
                burst_size: endpoint_limit.burst_size,
            }
        } else {
            LimitConfig {
                limit: if is_authenticated {
                    self.config.default_authenticated_limit
                } else {
                    self.config.default_anonymous_limit
                },
                window_seconds: self.config.window_seconds,
                burst_size: 10, // Default burst size
            }
        }
    }

    /// Clean up old buckets (for memory-based rate limiting)
    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        buckets.retain(|_, bucket| {
            now.duration_since(bucket.window_start) < Duration::from_secs(3600) // Keep for 1 hour
        });

        debug!(
            "Cleaned up old rate limit buckets. Remaining: {}",
            buckets.len()
        );
    }

    /// Get statistics about rate limiting
    pub async fn get_stats(&self) -> RateLimitStats {
        let buckets = self.buckets.read().await;

        RateLimitStats {
            total_buckets: buckets.len(),
            redis_enabled: self.redis_client.is_some(),
            config: self.config.clone(),
        }
    }
}

/// Internal limit configuration
struct LimitConfig {
    limit: u32,
    window_seconds: u64,
    burst_size: u32,
}

/// Rate limit status
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum RateLimitStatus {
    Allowed {
        limit: u32,
        remaining: u32,
        reset_at: Instant,
    },
    AllowedWithBurst {
        limit: u32,
        remaining: u32,
        burst_remaining: u32,
        reset_at: Instant,
    },
    Limited {
        retry_after_seconds: u64,
        limit: u32,
        remaining: u32,
        reset_at: Instant,
    },
}

impl RateLimitStatus {
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            RateLimitStatus::Allowed { .. } | RateLimitStatus::AllowedWithBurst { .. }
        )
    }

    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        match self {
            RateLimitStatus::Allowed {
                limit, remaining, ..
            }
            | RateLimitStatus::AllowedWithBurst {
                limit, remaining, ..
            } => {
                headers.insert("X-RateLimit-Limit".to_string(), limit.to_string());
                headers.insert("X-RateLimit-Remaining".to_string(), remaining.to_string());
            }
            RateLimitStatus::Limited {
                retry_after_seconds,
                limit,
                ..
            } => {
                headers.insert("X-RateLimit-Limit".to_string(), limit.to_string());
                headers.insert("X-RateLimit-Remaining".to_string(), "0".to_string());
                headers.insert("Retry-After".to_string(), retry_after_seconds.to_string());
            }
        }

        headers
    }
}

/// Rate limit error
#[derive(Debug, Clone, Serialize)]
pub enum RateLimitError {
    RedisError(String),
    ConfigError(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::RedisError(e) => write!(f, "Redis error: {}", e),
            RateLimitError::ConfigError(e) => write!(f, "Configuration error: {}", e),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Rate limiting statistics
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStats {
    pub total_buckets: usize,
    pub redis_enabled: bool,
    pub config: RateLimitConfig,
}

/// Middleware function for rate limiting
pub async fn rate_limit_middleware(
    request: &rouille::Request,
    rate_limiter: &RateLimiter,
) -> Option<rouille::Response> {
    let endpoint = request.url();
    let client_id = get_client_id(request);
    let is_authenticated = is_authenticated_request(request);

    match rate_limiter
        .check_rate_limit(endpoint, &client_id, is_authenticated)
        .await
    {
        Ok(status) => {
            if !status.is_allowed() {
                let headers = status.to_headers();
                let mut response =
                    rouille::Response::text("Rate limit exceeded").with_status_code(429);

                for (key, value) in headers {
                    response = response.with_additional_header(key, value);
                }

                Some(response)
            } else {
                None // Allow request to proceed
            }
        }
        Err(e) => {
            warn!("Rate limiting error: {}. Allowing request to proceed.", e);
            None // Allow request on error
        }
    }
}

/// Extract client ID from request
fn get_client_id(request: &rouille::Request) -> String {
    // Try to get authenticated user ID
    if let Some(user_id) = request.header("X-User-Id") {
        return user_id.to_string();
    }

    // Try to get session ID
    if let Some(session_id) = request.header("X-Session-Id") {
        return session_id.to_string();
    }

    // Fall back to IP address
    request.remote_addr().to_string()
}

/// Check if request is authenticated
fn is_authenticated_request(request: &rouille::Request) -> bool {
    request.header("Authorization").is_some()
        || request.header("X-API-Key").is_some()
        || request.header("X-User-Id").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.default_authenticated_limit, 1000);
        assert_eq!(config.default_anonymous_limit, 100);
        assert!(config.endpoint_limits.contains_key("/api/auth/login"));
    }

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);

        let stats = limiter.get_stats().await;
        assert_eq!(stats.total_buckets, 0);
    }

    #[tokio::test]
    async fn test_memory_rate_limiting() {
        let mut config = RateLimitConfig::default();
        config.use_redis = false;
        config.default_anonymous_limit = 3;
        config.window_seconds = 1;

        let limiter = RateLimiter::new(config);

        // First 3 requests should be allowed
        for i in 1..=3 {
            let status = limiter
                .check_rate_limit("/api/test", "client1", false)
                .await
                .unwrap();
            assert!(status.is_allowed(), "Request {} should be allowed", i);
        }

        // 4th request should be limited
        let status = limiter
            .check_rate_limit("/api/test", "client1", false)
            .await
            .unwrap();
        assert!(!status.is_allowed(), "4th request should be limited");

        // Wait for window to reset
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be allowed again
        let status = limiter
            .check_rate_limit("/api/test", "client1", false)
            .await
            .unwrap();
        assert!(
            status.is_allowed(),
            "Request should be allowed after window reset"
        );
    }

    #[test]
    fn test_rate_limit_status_headers() {
        let status = RateLimitStatus::Allowed {
            limit: 100,
            remaining: 75,
            reset_at: Instant::now() + Duration::from_secs(60),
        };

        let headers = status.to_headers();
        assert_eq!(headers.get("X-RateLimit-Limit").unwrap(), "100");
        assert_eq!(headers.get("X-RateLimit-Remaining").unwrap(), "75");
    }
}
