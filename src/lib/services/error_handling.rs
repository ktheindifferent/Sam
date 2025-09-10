// Enhanced Error Handling Module for SAM Services
// Provides comprehensive error handling utilities, retry logic, and circuit breaker patterns

use std::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;
use anyhow::Result;
use log::{error, warn, info, debug};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// ==================== Error Types ====================

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),
    
    #[error("Timeout after {0:?}")]
    TimeoutError(Duration),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Invalid input: {0}")]
    ValidationError(String),
    
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Circuit breaker open for: {0}")]
    CircuitBreakerOpen(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Retry limit exceeded after {attempts} attempts: {reason}")]
    RetryExhausted { attempts: u32, reason: String },
    
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

// ==================== Retry Logic ====================

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;
    
    loop {
        attempt += 1;
        debug!("Attempting {} (attempt {}/{})", operation_name, attempt, config.max_attempts);
        
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("Successfully completed {} after {} attempts", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) if attempt >= config.max_attempts => {
                error!("Failed {} after {} attempts: {}", operation_name, attempt, e);
                return Err(ServiceError::RetryExhausted {
                    attempts: attempt,
                    reason: e.to_string(),
                }.into());
            }
            Err(e) => {
                warn!("Attempt {} failed for {}: {}", attempt, operation_name, e);
                
                // Apply jitter if configured
                let mut actual_delay = delay;
                if config.jitter {
                    use rand::Rng;
                    let jitter_range = delay.as_millis() as f64 * 0.1;
                    let jitter = rand::thread_rng().gen_range(-jitter_range..jitter_range) as u64;
                    actual_delay = Duration::from_millis(delay.as_millis() as u64 + jitter);
                }
                
                tokio::time::sleep(actual_delay).await;
                
                // Calculate next delay with exponential backoff
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.exponential_base).min(config.max_delay.as_secs_f64())
                );
            }
        }
    }
}

// ==================== Circuit Breaker ====================

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open { opened_at: DateTime<Utc> },
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: String, failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_threshold,
            success_threshold,
            timeout,
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            name,
        }
    }
    
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit is open
        let state = self.state.read().await.clone();
        if let CircuitState::Open { opened_at } = state {
            if Utc::now().signed_duration_since(opened_at).to_std().expect("Duration should be valid") >= self.timeout {
                // Transition to half-open
                *self.state.write().await = CircuitState::HalfOpen;
                *self.success_count.write().await = 0;
                info!("Circuit breaker '{}' transitioning to half-open", self.name);
            } else {
                return Err(ServiceError::CircuitBreakerOpen(self.name.clone()).into());
            }
        }
        
        // Execute operation
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
    
    async fn on_success(&self) {
        let state = self.state.read().await.clone();
        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;
                
                if *success_count >= self.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    info!("Circuit breaker '{}' closed after successful recovery", self.name);
                }
            }
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let state = self.state.read().await.clone();
        match state {
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open { opened_at: Utc::now() };
                *self.failure_count.write().await = 0;
                warn!("Circuit breaker '{}' opened due to failure in half-open state", self.name);
            }
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;
                
                if *failure_count >= self.failure_threshold {
                    *self.state.write().await = CircuitState::Open { opened_at: Utc::now() };
                    error!("Circuit breaker '{}' opened after {} failures", self.name, *failure_count);
                }
            }
            _ => {}
        }
    }
    
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }
    
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        info!("Circuit breaker '{}' manually reset", self.name);
    }
}

// ==================== Rate Limiter ====================

pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    requests: Arc<RwLock<Vec<DateTime<Utc>>>>,
    name: String,
}

impl RateLimiter {
    pub fn new(name: String, max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: Arc::new(RwLock::new(Vec::new())),
            name,
        }
    }
    
    pub async fn check_rate_limit(&self) -> Result<()> {
        let now = Utc::now();
        let mut requests = self.requests.write().await;
        
        // Remove old requests outside the window
        let cutoff = now - chrono::Duration::from_std(self.window).expect("Window duration should be valid");
        requests.retain(|&req| req > cutoff);
        
        if requests.len() >= self.max_requests as usize {
            return Err(ServiceError::RateLimitExceeded(
                format!("{}: {} requests in {:?}", self.name, self.max_requests, self.window)
            ).into());
        }
        
        requests.push(now);
        Ok(())
    }
}

// ==================== Global Error Handler ====================

pub struct ErrorHandler {
    handlers: HashMap<String, Box<dyn Fn(&dyn std::error::Error) + Send + Sync>>,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
    
    pub fn register_handler<F>(&mut self, error_type: String, handler: F)
    where
        F: Fn(&dyn std::error::Error) + Send + Sync + 'static,
    {
        self.handlers.insert(error_type, Box::new(handler));
    }
    
    pub fn handle_error(&self, error: &dyn std::error::Error) {
        let error_type = std::any::type_name_of_val(error);
        
        if let Some(handler) = self.handlers.get(error_type) {
            handler(error);
        } else {
            // Default error handling
            error!("Unhandled error: {}", error);
            
            // Log error chain
            let mut current_error = error.source();
            while let Some(err) = current_error {
                error!("Caused by: {}", err);
                current_error = err.source();
            }
        }
    }
}

// ==================== Helper Functions ====================

/// Safely unwrap a Result or log and return a default value
pub fn safe_unwrap_or<T>(result: Result<T>, default: T, context: &str) -> T {
    match result {
        Ok(val) => val,
        Err(e) => {
            error!("Error in {}: {}. Using default value.", context, e);
            default
        }
    }
}

/// Convert any error to a ServiceError with context
pub fn to_service_error<E: std::error::Error>(error: E, context: &str) -> ServiceError {
    ServiceError::Unexpected(format!("{}: {}", context, error))
}

/// Graceful degradation wrapper
pub async fn with_fallback<F, Fut, T, G, Gut>(
    primary: F,
    fallback: G,
    operation_name: &str,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
    G: FnOnce() -> Gut,
    Gut: std::future::Future<Output = Result<T>>,
{
    match primary().await {
        Ok(result) => Ok(result),
        Err(primary_error) => {
            warn!("Primary operation '{}' failed: {}. Attempting fallback.", operation_name, primary_error);
            
            match fallback().await {
                Ok(result) => {
                    info!("Fallback for '{}' succeeded", operation_name);
                    Ok(result)
                }
                Err(fallback_error) => {
                    error!("Both primary and fallback failed for '{}'. Primary: {}, Fallback: {}", 
                           operation_name, primary_error, fallback_error);
                    Err(primary_error)
                }
            }
        }
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let attempt = Arc::new(AtomicU32::new(0));
        let attempt_clone = attempt.clone();
        
        let result = retry_with_backoff(
            move || {
                let attempt = attempt_clone.clone();
                async move {
                    let current_attempt = attempt.fetch_add(1, Ordering::SeqCst) + 1;
                    if current_attempt < 3 {
                        Err(anyhow::anyhow!("Temporary failure"))
                    } else {
                        Ok("Success")
                    }
                }
            },
            RetryConfig::default(),
            "test_operation",
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.expect("Operation should succeed after retry"), "Success");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let cb = CircuitBreaker::new("test".to_string(), 2, 2, Duration::from_secs(1));
        
        // First failure
        let _: Result<()> = cb.call(|| async { Err(anyhow::anyhow!("Error")) }).await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        // Second failure - should open
        let _: Result<()> = cb.call(|| async { Err(anyhow::anyhow!("Error")) }).await;
        
        match cb.get_state().await {
            CircuitState::Open { .. } => {}
            _ => panic!("Circuit should be open"),
        }
        
        // Should reject calls when open
        let result = cb.call(|| async { Ok("Should not execute") }).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new("test".to_string(), 2, Duration::from_secs(1));
        
        // First two requests should succeed
        assert!(limiter.check_rate_limit().await.is_ok());
        assert!(limiter.check_rate_limit().await.is_ok());
        
        // Third request should fail
        assert!(limiter.check_rate_limit().await.is_err());
        
        // After waiting, should succeed again
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(limiter.check_rate_limit().await.is_ok());
    }
}