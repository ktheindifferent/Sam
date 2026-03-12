//! Base provider implementation to reduce duplication across LLM providers

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

/// Common provider metrics and state
#[derive(Debug, Clone)]
pub struct ProviderState {
    /// Provider name for identification
    pub name: String,
    /// Last successful response time
    pub last_success: Option<SystemTime>,
    /// Last failure time
    pub last_failure: Option<SystemTime>,
    /// Success count in current window
    pub success_count: u64,
    /// Failure count in current window
    pub failure_count: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Current availability status
    pub is_available: bool,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Default for ProviderState {
    fn default() -> Self {
        Self {
            name: String::new(),
            last_success: None,
            last_failure: None,
            success_count: 0,
            failure_count: 0,
            avg_response_time_ms: 0.0,
            is_available: true,
            metadata: HashMap::new(),
        }
    }
}

/// Base provider implementation with common functionality
pub struct BaseProvider<T> {
    /// Provider-specific implementation
    implementation: T,
    /// Shared state
    state: Arc<RwLock<ProviderState>>,
    /// Rate limiting
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Retry configuration
    retry_config: RetryConfig,
}

/// Rate limiter for providers
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum requests per window
    max_requests: usize,
    /// Time window for rate limiting
    window: Duration,
    /// Request timestamps
    requests: Vec<Instant>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: Vec::new(),
        }
    }

    /// Check if request is allowed and record it
    pub fn allow_request(&mut self) -> bool {
        let now = Instant::now();

        // Remove old requests outside the window
        self.requests.retain(|&req| now.duration_since(req) < self.window);

        if self.requests.len() < self.max_requests {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get time until next request is allowed
    pub fn time_until_next_request(&self) -> Option<Duration> {
        if self.requests.len() < self.max_requests {
            return None;
        }

        let oldest = self.requests.first()?;
        let elapsed = Instant::now().duration_since(*oldest);

        if elapsed >= self.window {
            None
        } else {
            Some(self.window - elapsed)
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Exponential backoff multiplier
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// Provider implementation trait
#[async_trait]
pub trait ProviderImpl: Send + Sync {
    /// Generate response for the given prompt
    async fn generate_impl(&self, prompt: &str, model: &str) -> Result<String>;

    /// Check if the provider is available
    async fn is_available_impl(&self) -> bool;

    /// List available models
    async fn list_models_impl(&self) -> Result<Vec<String>>;

    /// Get provider name
    fn name(&self) -> &str;
}

impl<T: ProviderImpl> BaseProvider<T> {
    pub fn new(implementation: T, max_requests_per_minute: usize) -> Self {
        let name = implementation.name().to_string();
        Self {
            implementation,
            state: Arc::new(RwLock::new(ProviderState {
                name,
                is_available: true,
                ..Default::default()
            })),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(
                max_requests_per_minute,
                Duration::from_secs(60),
            ))),
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Generate response with retry logic and metrics
    pub async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
        // Check rate limit
        {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.allow_request() {
                if let Some(wait_time) = limiter.time_until_next_request() {
                    warn!("Rate limit exceeded, need to wait {:?}", wait_time);
                    return Err(anyhow::anyhow!("Rate limit exceeded, retry after {:?}", wait_time));
                }
            }
        }

        let start_time = Instant::now();
        let mut last_error = None;
        let mut delay = self.retry_config.initial_delay;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                info!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;

                // Exponential backoff
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * self.retry_config.multiplier)
                        .min(self.retry_config.max_delay.as_secs_f64())
                );
            }

            match self.implementation.generate_impl(prompt, model).await {
                Ok(response) => {
                    let duration = start_time.elapsed();

                    // Update metrics
                    {
                        let mut state = self.state.write().await;
                        state.last_success = Some(SystemTime::now());
                        state.success_count += 1;
                        state.is_available = true;

                        // Update average response time
                        let total_requests = state.success_count + state.failure_count;
                        if total_requests == 1 {
                            state.avg_response_time_ms = duration.as_millis() as f64;
                        } else {
                            state.avg_response_time_ms =
                                (state.avg_response_time_ms * (total_requests - 1) as f64
                                 + duration.as_millis() as f64) / total_requests as f64;
                        }
                    }

                    debug!("Provider {} generated response in {:?}", self.implementation.name(), duration);
                    return Ok(response);
                }
                Err(e) => {
                    error!("Provider {} error on attempt {}: {}",
                           self.implementation.name(), attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        // All retries failed
        {
            let mut state = self.state.write().await;
            state.last_failure = Some(SystemTime::now());
            state.failure_count += 1;

            // Mark as unavailable if too many failures
            if state.failure_count > 5 &&
               state.success_count < state.failure_count / 2 {
                state.is_available = false;
                warn!("Provider {} marked as unavailable due to high failure rate",
                      self.implementation.name());
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    /// Check availability with caching
    pub async fn is_available(&self) -> bool {
        let state = self.state.read().await;

        // If marked as unavailable and less than 5 minutes since last check
        if !state.is_available {
            if let Some(last_failure) = state.last_failure {
                if SystemTime::now().duration_since(last_failure)
                    .unwrap_or(Duration::from_secs(0)) < Duration::from_secs(300) {
                    return false;
                }
            }
        }

        drop(state);  // Release the lock before making the async call

        // Perform actual availability check
        let available = self.implementation.is_available_impl().await;

        // Update state
        {
            let mut state = self.state.write().await;
            state.is_available = available;
            if available {
                debug!("Provider {} is available", self.implementation.name());
            } else {
                warn!("Provider {} is not available", self.implementation.name());
            }
        }

        available
    }

    /// List models with caching
    pub async fn list_models(&self) -> Result<Vec<String>> {
        self.implementation.list_models_impl().await
    }

    /// Get provider metrics
    pub async fn get_metrics(&self) -> ProviderMetrics {
        let state = self.state.read().await;
        ProviderMetrics {
            name: state.name.clone(),
            total_requests: state.success_count + state.failure_count,
            success_count: state.success_count,
            failure_count: state.failure_count,
            success_rate: if state.success_count + state.failure_count > 0 {
                state.success_count as f64 / (state.success_count + state.failure_count) as f64
            } else {
                0.0
            },
            avg_response_time_ms: state.avg_response_time_ms,
            is_available: state.is_available,
            last_success: state.last_success,
            last_failure: state.last_failure,
        }
    }

    /// Reset provider metrics
    pub async fn reset_metrics(&self) {
        let mut state = self.state.write().await;
        state.success_count = 0;
        state.failure_count = 0;
        state.avg_response_time_ms = 0.0;
        state.last_success = None;
        state.last_failure = None;
        info!("Reset metrics for provider {}", state.name);
    }

    /// Get provider name
    pub fn name(&self) -> &str {
        self.implementation.name()
    }
}

/// Provider metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub name: String,
    pub total_requests: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub is_available: bool,
    pub last_success: Option<SystemTime>,
    pub last_failure: Option<SystemTime>,
}

/// Circuit breaker for provider resilience
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    failures: u32,
    last_failure_time: Option<Instant>,
    state: CircuitState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            failures: 0,
            last_failure_time: None,
            state: CircuitState::Closed,
        }
    }

    pub fn is_available(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if Instant::now().duration_since(last_failure) >= self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.state = CircuitState::Closed;
                self.failures = 0;
                self.last_failure_time = None;
            }
            _ => {
                self.failures = 0;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failures >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(1));

        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(!limiter.allow_request());

        // Wait for window to reset
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(limiter.allow_request());
    }

    #[test]
    fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        assert!(breaker.is_available());

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        // Should be open now
        assert!(!breaker.is_available());

        // Success in half-open should close the circuit
        breaker.state = CircuitState::HalfOpen;
        breaker.record_success();
        assert!(breaker.is_available());
    }
}