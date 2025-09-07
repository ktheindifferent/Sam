//! Per-domain rate limiting and crawl delay management
//! 
//! This module implements sophisticated rate limiting that respects robots.txt crawl-delay
//! directives and adapts based on server response times.

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use log::{debug, info, warn};
use url::Url;

/// Configuration for adaptive rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Default delay between requests to the same domain (milliseconds)
    pub default_delay_ms: u64,
    
    /// Minimum delay between requests (milliseconds)
    pub min_delay_ms: u64,
    
    /// Maximum delay between requests (milliseconds)
    pub max_delay_ms: u64,
    
    /// Factor to increase delay on slow responses
    pub slow_response_factor: f64,
    
    /// Factor to decrease delay on fast responses
    pub fast_response_factor: f64,
    
    /// Response time threshold for "slow" (milliseconds)
    pub slow_threshold_ms: u64,
    
    /// Response time threshold for "fast" (milliseconds)
    pub fast_threshold_ms: u64,
    
    /// Maximum concurrent requests per domain
    pub max_concurrent_per_domain: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_delay_ms: 1000,
            min_delay_ms: 100,
            max_delay_ms: 30000,
            slow_response_factor: 1.5,
            fast_response_factor: 0.9,
            slow_threshold_ms: 5000,
            fast_threshold_ms: 500,
            max_concurrent_per_domain: 2,
        }
    }
}

/// Per-domain statistics and rate limiting state
#[derive(Debug, Clone)]
struct DomainState {
    /// Last access time
    last_access: Instant,
    
    /// Current delay for this domain (milliseconds)
    current_delay_ms: u64,
    
    /// Average response time (milliseconds)
    avg_response_time_ms: f64,
    
    /// Number of requests made
    request_count: u64,
    
    /// Number of currently active requests
    active_requests: usize,
    
    /// Crawl delay from robots.txt (if any)
    robots_crawl_delay_ms: Option<u64>,
    
    /// Last time we got a 429/503 response
    last_rate_limit_response: Option<Instant>,
    
    /// Retry-After header value (if received)
    retry_after: Option<Instant>,
}

impl DomainState {
    fn new(robots_delay_ms: Option<u64>, default_delay_ms: u64) -> Self {
        Self {
            last_access: Instant::now().checked_sub(Duration::from_secs(60)).unwrap_or_else(Instant::now),
            current_delay_ms: robots_delay_ms.unwrap_or(default_delay_ms),
            avg_response_time_ms: 0.0,
            request_count: 0,
            active_requests: 0,
            robots_crawl_delay_ms: robots_delay_ms,
            last_rate_limit_response: None,
            retry_after: None,
        }
    }
    
    /// Update statistics with a new response
    fn update_stats(&mut self, response_time_ms: u64) {
        self.request_count += 1;
        
        // Calculate exponential moving average
        let alpha = 0.2; // Smoothing factor
        if self.avg_response_time_ms == 0.0 {
            self.avg_response_time_ms = response_time_ms as f64;
        } else {
            self.avg_response_time_ms = alpha * response_time_ms as f64 + 
                                        (1.0 - alpha) * self.avg_response_time_ms;
        }
    }
    
    /// Adapt delay based on response time
    fn adapt_delay(&mut self, response_time_ms: u64, config: &RateLimitConfig) {
        // Don't go below robots.txt specified delay
        let min_delay = self.robots_crawl_delay_ms
            .unwrap_or(config.min_delay_ms)
            .max(config.min_delay_ms);
        
        if response_time_ms > config.slow_threshold_ms {
            // Slow response - increase delay
            self.current_delay_ms = (self.current_delay_ms as f64 * config.slow_response_factor) as u64;
            debug!("Slow response ({}ms), increasing delay to {}ms", 
                   response_time_ms, self.current_delay_ms);
        } else if response_time_ms < config.fast_threshold_ms && self.request_count > 10 {
            // Fast response and we've made enough requests to judge - decrease delay
            self.current_delay_ms = (self.current_delay_ms as f64 * config.fast_response_factor) as u64;
            debug!("Fast response ({}ms), decreasing delay to {}ms", 
                   response_time_ms, self.current_delay_ms);
        }
        
        // Enforce limits
        self.current_delay_ms = self.current_delay_ms.clamp(min_delay, config.max_delay_ms);
    }
}

/// Adaptive per-domain rate limiter
pub struct AdaptiveRateLimiter {
    /// Configuration
    config: RateLimitConfig,
    
    /// Per-domain state
    domains: Arc<RwLock<HashMap<String, DomainState>>>,
    
    /// Global rate limit (requests per second across all domains)
    global_rps_limit: Option<u32>,
    
    /// Last global request time
    last_global_request: Arc<RwLock<Instant>>,
}

impl AdaptiveRateLimiter {
    /// Create a new adaptive rate limiter
    pub fn new(config: RateLimitConfig, global_rps_limit: Option<u32>) -> Self {
        Self {
            config,
            domains: Arc::new(RwLock::new(HashMap::new())),
            global_rps_limit,
            last_global_request: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Extract domain from URL
    fn extract_domain(url: &str) -> Result<String> {
        let parsed = Url::parse(url).context("Failed to parse URL")?;
        let domain = parsed.host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in URL"))?
            .to_string();
        Ok(domain)
    }
    
    /// Wait for rate limit before making a request
    pub async fn wait_for_slot(&self, url: &str, robots_delay_seconds: Option<f64>) -> Result<()> {
        let domain = Self::extract_domain(url)?;
        
        // Check global rate limit
        if let Some(global_limit) = self.global_rps_limit {
            let mut last_global = self.last_global_request.write().await;
            let min_interval = Duration::from_millis(1000 / global_limit as u64);
            let elapsed = last_global.elapsed();
            
            if elapsed < min_interval {
                let wait_time = min_interval - elapsed;
                debug!("Global rate limit: waiting {:?}", wait_time);
                sleep(wait_time).await;
            }
            *last_global = Instant::now();
        }
        
        // Get or create domain state
        let mut domains = self.domains.write().await;
        let mut state = domains.entry(domain.clone()).or_insert_with(|| {
            let robots_delay_ms = robots_delay_seconds.map(|s| (s * 1000.0) as u64);
            DomainState::new(robots_delay_ms, self.config.default_delay_ms)
        });
        
        // Check if we're in a retry-after period
        if let Some(retry_after) = state.retry_after {
            if Instant::now() < retry_after {
                let wait_time = retry_after.duration_since(Instant::now());
                info!("Respecting Retry-After header for {}: waiting {:?}", domain, wait_time);
                sleep(wait_time).await;
                state.retry_after = None;
            }
        }
        
        // Wait if we have too many concurrent requests
        while state.active_requests >= self.config.max_concurrent_per_domain {
            debug!("Too many concurrent requests for {}, waiting...", domain);
            drop(domains);
            sleep(Duration::from_millis(100)).await;
            domains = self.domains.write().await;
            state = domains.get_mut(&domain).unwrap();
        }
        
        // Calculate wait time based on last access
        let elapsed = state.last_access.elapsed();
        let required_delay = Duration::from_millis(state.current_delay_ms);
        
        if elapsed < required_delay {
            let wait_time = required_delay - elapsed;
            debug!("Rate limiting {}: waiting {:?} (delay: {}ms)", 
                   domain, wait_time, state.current_delay_ms);
            sleep(wait_time).await;
        }
        
        // Update state
        state.last_access = Instant::now();
        state.active_requests += 1;
        
        Ok(())
    }
    
    /// Record request completion and update statistics
    pub async fn record_request_complete(
        &self, 
        url: &str, 
        response_time: Duration,
        status_code: Option<u16>,
        retry_after_header: Option<u64>,
    ) -> Result<()> {
        let domain = Self::extract_domain(url)?;
        let response_time_ms = response_time.as_millis() as u64;
        
        let mut domains = self.domains.write().await;
        if let Some(state) = domains.get_mut(&domain) {
            // Decrease active requests
            state.active_requests = state.active_requests.saturating_sub(1);
            
            // Update statistics
            state.update_stats(response_time_ms);
            
            // Handle rate limit responses
            match status_code {
                Some(429) | Some(503) => {
                    warn!("Got rate limit response ({:?}) from {}", status_code, domain);
                    state.last_rate_limit_response = Some(Instant::now());
                    
                    // Double the delay on rate limit
                    state.current_delay_ms = (state.current_delay_ms * 2)
                        .min(self.config.max_delay_ms);
                    
                    // Handle Retry-After header
                    if let Some(retry_seconds) = retry_after_header {
                        state.retry_after = Some(Instant::now() + Duration::from_secs(retry_seconds));
                        info!("Set retry-after for {} to {} seconds", domain, retry_seconds);
                    }
                }
                Some(code) if code < 400 => {
                    // Successful response - adapt delay based on response time
                    state.adapt_delay(response_time_ms, &self.config);
                }
                _ => {
                    // Error response - slightly increase delay
                    state.current_delay_ms = (state.current_delay_ms as f64 * 1.1) as u64;
                    state.current_delay_ms = state.current_delay_ms.min(self.config.max_delay_ms);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get current statistics for a domain
    pub async fn get_domain_stats(&self, domain: &str) -> Option<DomainStats> {
        let domains = self.domains.read().await;
        domains.get(domain).map(|state| DomainStats {
            current_delay_ms: state.current_delay_ms,
            avg_response_time_ms: state.avg_response_time_ms,
            request_count: state.request_count,
            active_requests: state.active_requests,
            has_rate_limited: state.last_rate_limit_response.is_some(),
        })
    }
    
    /// Get statistics for all domains
    pub async fn get_all_stats(&self) -> HashMap<String, DomainStats> {
        let domains = self.domains.read().await;
        domains.iter().map(|(domain, state)| {
            (domain.clone(), DomainStats {
                current_delay_ms: state.current_delay_ms,
                avg_response_time_ms: state.avg_response_time_ms,
                request_count: state.request_count,
                active_requests: state.active_requests,
                has_rate_limited: state.last_rate_limit_response.is_some(),
            })
        }).collect()
    }
    
    /// Clear statistics for old domains (not accessed in the last hour)
    pub async fn cleanup_old_domains(&self) {
        let mut domains = self.domains.write().await;
        let cutoff = Duration::from_secs(3600);
        
        domains.retain(|domain, state| {
            let should_keep = state.last_access.elapsed() < cutoff || state.active_requests > 0;
            if !should_keep {
                debug!("Removing old domain stats for {}", domain);
            }
            should_keep
        });
    }
}

/// Public statistics for a domain
#[derive(Debug, Clone)]
pub struct DomainStats {
    pub current_delay_ms: u64,
    pub avg_response_time_ms: f64,
    pub request_count: u64,
    pub active_requests: usize,
    pub has_rate_limited: bool,
}

/// Global rate limiter instance
static RATE_LIMITER: once_cell::sync::OnceCell<AdaptiveRateLimiter> = once_cell::sync::OnceCell::new();

/// Initialize the global rate limiter
pub fn init_rate_limiter(config: RateLimitConfig, global_rps_limit: Option<u32>) {
    let limiter = AdaptiveRateLimiter::new(config, global_rps_limit);
    if RATE_LIMITER.set(limiter).is_err() {
        warn!("Rate limiter already initialized");
    }
}

/// Get the global rate limiter
pub fn get_rate_limiter() -> &'static AdaptiveRateLimiter {
    RATE_LIMITER.get_or_init(|| {
        AdaptiveRateLimiter::new(RateLimitConfig::default(), Some(100))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = AdaptiveRateLimiter::new(
            RateLimitConfig {
                default_delay_ms: 100,
                ..Default::default()
            },
            None
        );
        
        let url = "https://example.com/test";
        
        // First request should go through immediately
        let start = Instant::now();
        limiter.wait_for_slot(url, None).await.unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));
        
        // Second request should be delayed
        let start = Instant::now();
        limiter.wait_for_slot(url, None).await.unwrap();
        assert!(start.elapsed() >= Duration::from_millis(90));
    }
    
    #[tokio::test]
    async fn test_adaptive_delay() {
        let limiter = AdaptiveRateLimiter::new(RateLimitConfig::default(), None);
        let url = "https://example.com/test";
        
        // Simulate slow response
        limiter.wait_for_slot(url, None).await.unwrap();
        limiter.record_request_complete(
            url,
            Duration::from_millis(6000),
            Some(200),
            None
        ).await.unwrap();
        
        // Check that delay increased
        let stats = limiter.get_domain_stats("example.com").await.unwrap();
        assert!(stats.current_delay_ms > 1000);
    }
}