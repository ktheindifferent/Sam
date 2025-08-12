use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use deadpool_redis::{Pool};
use redis::AsyncCommands;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Whether to use Redis for distributed rate limiting
    pub use_redis: bool,
    /// Burst allowance (requests that can exceed the limit temporarily)
    pub burst_size: u32,
    /// Block duration for exceeded limits (in seconds)
    pub block_duration: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            max_requests: 100,
            window_seconds: 60,
            use_redis: true,
            burst_size: 10,
            block_duration: 300, // 5 minutes
        }
    }
}

/// DOS protection configuration
#[derive(Debug, Clone)]
pub struct DosProtectionConfig {
    /// Maximum concurrent connections per IP
    pub max_connections_per_ip: usize,
    /// Maximum request body size (in bytes)
    pub max_body_size: usize,
    /// Request timeout (in seconds)
    pub request_timeout: u64,
    /// Enable SYN flood protection
    pub syn_flood_protection: bool,
    /// Maximum requests per second per IP
    pub max_requests_per_second: u32,
}

impl Default for DosProtectionConfig {
    fn default() -> Self {
        DosProtectionConfig {
            max_connections_per_ip: 10,
            max_body_size: 10 * 1024 * 1024, // 10MB
            request_timeout: 30,
            syn_flood_protection: true,
            max_requests_per_second: 10,
        }
    }
}

/// Request tracking information
#[derive(Debug, Clone)]
struct RequestInfo {
    count: u32,
    first_request: Instant,
    last_request: Instant,
    blocked_until: Option<Instant>,
}

/// HTTP Security Middleware
pub struct HttpSecurityMiddleware {
    rate_limit_config: RateLimitConfig,
    dos_config: DosProtectionConfig,
    redis_pool: Option<Pool>,
    local_cache: Arc<RwLock<HashMap<IpAddr, RequestInfo>>>,
    connection_count: Arc<RwLock<HashMap<IpAddr, usize>>>,
}

impl HttpSecurityMiddleware {
    /// Create new middleware instance
    pub async fn new(
        rate_limit_config: RateLimitConfig,
        dos_config: DosProtectionConfig,
        redis_pool: Option<Pool>,
    ) -> Self {
        let middleware = HttpSecurityMiddleware {
            rate_limit_config,
            dos_config,
            redis_pool,
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            connection_count: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start cleanup task
        middleware.start_cleanup_task();
        
        middleware
    }
    
    /// Check rate limit for an IP address
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<bool, String> {
        // First check if IP is blocked
        if self.is_ip_blocked(ip).await? {
            return Ok(false);
        }
        
        if self.rate_limit_config.use_redis && self.redis_pool.is_some() {
            self.check_rate_limit_redis(ip).await
        } else {
            self.check_rate_limit_local(ip).await
        }
    }
    
    /// Check rate limit using Redis (distributed)
    async fn check_rate_limit_redis(&self, ip: IpAddr) -> Result<bool, String> {
        let pool = self.redis_pool.as_ref().ok_or("Redis pool not available")?;
        let mut conn = pool.get().await.map_err(|e| format!("Redis connection error: {}", e))?;
        
        let key = format!("rate_limit:{}", ip);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Use Redis sliding window algorithm
        let window_start = now - self.rate_limit_config.window_seconds;
        
        // Remove old entries
        let _: Result<(), _> = conn.zremrangebyscore(&key, 0, window_start as f64).await;
        
        // Count current requests in window
        let count: u32 = conn.zcount(&key, window_start as f64, now as f64)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;
        
        // Check if limit exceeded
        if count >= self.rate_limit_config.max_requests {
            // Check burst allowance
            if count >= self.rate_limit_config.max_requests + self.rate_limit_config.burst_size {
                // Block the IP
                self.block_ip(ip).await?;
                return Ok(false);
            }
        }
        
        // Add current request
        let _: Result<(), _> = conn.zadd(&key, now as f64, now).await;
        
        // Set expiration
        let _: Result<(), _> = conn.expire(&key, self.rate_limit_config.window_seconds as i64).await;
        
        Ok(true)
    }
    
    /// Check rate limit using local cache
    async fn check_rate_limit_local(&self, ip: IpAddr) -> Result<bool, String> {
        let mut cache = self.local_cache.write().await;
        let now = Instant::now();
        
        let info = cache.entry(ip).or_insert(RequestInfo {
            count: 0,
            first_request: now,
            last_request: now,
            blocked_until: None,
        });
        
        // Check if blocked
        if let Some(blocked_until) = info.blocked_until {
            if now < blocked_until {
                return Ok(false);
            } else {
                info.blocked_until = None;
            }
        }
        
        // Reset counter if window expired
        if now.duration_since(info.first_request) > Duration::from_secs(self.rate_limit_config.window_seconds) {
            info.count = 0;
            info.first_request = now;
        }
        
        // Check rate limit
        if info.count >= self.rate_limit_config.max_requests {
            // Check burst allowance
            if info.count >= self.rate_limit_config.max_requests + self.rate_limit_config.burst_size {
                info.blocked_until = Some(now + Duration::from_secs(self.rate_limit_config.block_duration));
                return Ok(false);
            }
        }
        
        // Check requests per second
        if now.duration_since(info.last_request) < Duration::from_secs(1) {
            let requests_per_second = info.count;
            if requests_per_second > self.dos_config.max_requests_per_second {
                info.blocked_until = Some(now + Duration::from_secs(self.rate_limit_config.block_duration));
                return Ok(false);
            }
        }
        
        info.count += 1;
        info.last_request = now;
        
        Ok(true)
    }
    
    /// Check if IP is blocked
    async fn is_ip_blocked(&self, ip: IpAddr) -> Result<bool, String> {
        if self.redis_pool.is_some() {
            let pool = self.redis_pool.as_ref().unwrap();
            let mut conn = pool.get().await.map_err(|e| format!("Redis error: {}", e))?;
            let key = format!("blocked:{}", ip);
            let is_blocked: bool = conn.exists(&key).await.map_err(|e| format!("Redis error: {}", e))?;
            Ok(is_blocked)
        } else {
            let cache = self.local_cache.read().await;
            if let Some(info) = cache.get(&ip) {
                if let Some(blocked_until) = info.blocked_until {
                    Ok(Instant::now() < blocked_until)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        }
    }
    
    /// Block an IP address
    async fn block_ip(&self, ip: IpAddr) -> Result<(), String> {
        if self.redis_pool.is_some() {
            let pool = self.redis_pool.as_ref().unwrap();
            let mut conn = pool.get().await.map_err(|e| format!("Redis error: {}", e))?;
            let key = format!("blocked:{}", ip);
            conn.set_ex(key, "1", self.rate_limit_config.block_duration)
                .await
                .map_err(|e| format!("Redis error: {}", e))?;
        }
        
        log::warn!("Blocked IP {} for {} seconds due to rate limit violation", 
                   ip, self.rate_limit_config.block_duration);
        
        Ok(())
    }
    
    /// Check connection limit for an IP
    pub async fn check_connection_limit(&self, ip: IpAddr) -> Result<bool, String> {
        let mut connections = self.connection_count.write().await;
        let count = connections.entry(ip).or_insert(0);
        
        if *count >= self.dos_config.max_connections_per_ip {
            log::warn!("Connection limit exceeded for IP {}: {} connections", ip, count);
            return Ok(false);
        }
        
        *count += 1;
        Ok(true)
    }
    
    /// Decrement connection count
    pub async fn decrement_connection(&self, ip: IpAddr) {
        let mut connections = self.connection_count.write().await;
        if let Some(count) = connections.get_mut(&ip) {
            if *count > 0 {
                *count -= 1;
            }
            if *count == 0 {
                connections.remove(&ip);
            }
        }
    }
    
    /// Validate request body size
    pub fn validate_body_size(&self, size: usize) -> bool {
        size <= self.dos_config.max_body_size
    }
    
    /// Get request timeout duration
    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.dos_config.request_timeout)
    }
    
    /// Start cleanup task for local cache
    fn start_cleanup_task(&self) {
        let cache = self.local_cache.clone();
        let window_seconds = self.rate_limit_config.window_seconds;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut cache = cache.write().await;
                let now = Instant::now();
                
                // Remove old entries
                cache.retain(|_, info| {
                    // Keep if within window or blocked
                    now.duration_since(info.last_request) < Duration::from_secs(window_seconds * 2)
                        || info.blocked_until.is_some()
                });
            }
        });
    }
}

/// Helper functions for HTTP headers
pub mod headers {
    use std::net::IpAddr;
    
    /// Extract client IP from headers (considering proxies)
    pub fn extract_client_ip(
        remote_addr: Option<IpAddr>,
        x_forwarded_for: Option<&str>,
        x_real_ip: Option<&str>,
    ) -> Option<IpAddr> {
        // Try X-Forwarded-For first (for proxies)
        if let Some(forwarded) = x_forwarded_for {
            if let Some(first_ip) = forwarded.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
        
        // Try X-Real-IP
        if let Some(real_ip) = x_real_ip {
            if let Ok(ip) = real_ip.parse::<IpAddr>() {
                return Some(ip);
            }
        }
        
        // Fall back to remote address
        remote_addr
    }
    
    /// Add security headers to response with nonce support
    pub fn add_security_headers_with_nonce(nonce: Option<String>) -> Vec<(String, String)> {
        let csp = if let Some(nonce_value) = nonce {
            format!(
                "default-src 'self'; \
                script-src 'self' 'nonce-{}' 'strict-dynamic'; \
                style-src 'self' 'nonce-{}'; \
                img-src 'self' data: https:; \
                font-src 'self' data:; \
                connect-src 'self'; \
                frame-src 'none'; \
                object-src 'none'; \
                base-uri 'self'; \
                form-action 'self'; \
                frame-ancestors 'none'; \
                upgrade-insecure-requests;",
                nonce_value, nonce_value
            )
        } else {
            // Fallback CSP without nonces (more restrictive)
            String::from(
                "default-src 'self'; \
                script-src 'self'; \
                style-src 'self'; \
                img-src 'self' data: https:; \
                font-src 'self' data:; \
                connect-src 'self'; \
                frame-src 'none'; \
                object-src 'none'; \
                base-uri 'self'; \
                form-action 'self'; \
                frame-ancestors 'none'; \
                upgrade-insecure-requests;"
            )
        };
        
        vec![
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            ("X-Frame-Options".to_string(), "DENY".to_string()),
            ("X-XSS-Protection".to_string(), "1; mode=block".to_string()),
            ("Referrer-Policy".to_string(), "strict-origin-when-cross-origin".to_string()),
            ("Content-Security-Policy".to_string(), csp),
            ("Strict-Transport-Security".to_string(), "max-age=31536000; includeSubDomains; preload".to_string()),
            ("Permissions-Policy".to_string(), "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".to_string()),
            ("Cross-Origin-Opener-Policy".to_string(), "same-origin".to_string()),
            ("Cross-Origin-Resource-Policy".to_string(), "same-origin".to_string()),
            ("Cross-Origin-Embedder-Policy".to_string(), "require-corp".to_string()),
        ]
    }
    
    /// Add security headers to response (backward compatibility)
    pub fn add_security_headers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("X-Content-Type-Options", "nosniff"),
            ("X-Frame-Options", "DENY"),
            ("X-XSS-Protection", "1; mode=block"),
            ("Referrer-Policy", "strict-origin-when-cross-origin"),
            ("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests;"),
            ("Strict-Transport-Security", "max-age=31536000; includeSubDomains; preload"),
            ("Permissions-Policy", "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()"),
        ]
    }
    
    /// Generate a secure nonce for CSP
    pub fn generate_csp_nonce() -> String {
        use base64::{Engine as _, engine::general_purpose};
        use rand::Rng;
        
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 16] = rng.gen();
        general_purpose::STANDARD.encode(random_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    
    #[tokio::test]
    async fn test_rate_limit_local() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_seconds: 1,
            use_redis: false,
            burst_size: 2,
            block_duration: 5,
        };
        
        let middleware = HttpSecurityMiddleware::new(
            config,
            DosProtectionConfig::default(),
            None,
        ).await;
        
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(middleware.check_rate_limit(ip).await.unwrap());
        }
        
        // Should allow burst (2 more)
        for _ in 0..2 {
            assert!(middleware.check_rate_limit(ip).await.unwrap());
        }
        
        // Should block after burst exceeded
        assert!(!middleware.check_rate_limit(ip).await.unwrap());
    }
    
    #[test]
    fn test_extract_client_ip() {
        use headers::extract_client_ip;
        
        let remote = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        let forwarded = Some("192.168.1.1, 10.0.0.1");
        let real_ip = Some("172.16.0.1");
        
        // Should prefer X-Forwarded-For
        let ip = extract_client_ip(remote, forwarded, real_ip);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        
        // Should use X-Real-IP if no X-Forwarded-For
        let ip = extract_client_ip(remote, None, real_ip);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        
        // Should fall back to remote address
        let ip = extract_client_ip(remote, None, None);
        assert_eq!(ip, remote);
    }
    
    #[tokio::test]
    async fn test_connection_limit() {
        let middleware = HttpSecurityMiddleware::new(
            RateLimitConfig::default(),
            DosProtectionConfig {
                max_connections_per_ip: 2,
                ..Default::default()
            },
            None,
        ).await;
        
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Should allow first 2 connections
        assert!(middleware.check_connection_limit(ip).await.unwrap());
        assert!(middleware.check_connection_limit(ip).await.unwrap());
        
        // Should block 3rd connection
        assert!(!middleware.check_connection_limit(ip).await.unwrap());
        
        // Should allow after decrement
        middleware.decrement_connection(ip).await;
        assert!(middleware.check_connection_limit(ip).await.unwrap());
    }
}