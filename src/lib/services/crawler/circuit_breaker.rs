//! # Circuit Breaker Module
//!
//! This module implements a circuit breaker pattern for the web crawler to handle
//! consistently failing domains gracefully and prevent resource waste.
//!
//! ## Features
//! - Track failure rates per domain
//! - Automatic circuit breaking for failing domains
//! - Exponential backoff for retries
//! - Automatic recovery detection
//! - Configurable thresholds and timeouts

use log::{debug, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed, requests allowed
    Closed,
    /// Circuit is open, requests blocked
    Open,
    /// Circuit is half-open, limited requests allowed for testing
    HalfOpen,
}

/// Statistics for a domain
#[derive(Debug, Clone)]
pub struct DomainStats {
    /// Current circuit state
    state: CircuitState,
    /// Number of consecutive failures
    consecutive_failures: u32,
    /// Total number of failures
    total_failures: u64,
    /// Total number of successes
    total_successes: u64,
    /// Successes in half-open state (for recovery detection)
    half_open_successes: u32,
    /// Last failure timestamp
    last_failure: Option<SystemTime>,
    /// Last success timestamp
    last_success: Option<SystemTime>,
    /// When the circuit was opened
    circuit_opened_at: Option<SystemTime>,
    /// Current backoff duration
    backoff_duration: Duration,
}

impl Default for DomainStats {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            total_failures: 0,
            total_successes: 0,
            half_open_successes: 0,
            last_failure: None,
            last_success: None,
            circuit_opened_at: None,
            backoff_duration: Duration::from_secs(60), // Initial backoff: 1 minute
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to open circuit
    pub failure_threshold: u32,
    /// Initial backoff duration when circuit opens
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Duration to wait in open state before trying half-open
    pub open_duration: Duration,
    /// Number of successful requests in half-open to close circuit
    pub half_open_success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            initial_backoff: Duration::from_secs(60),
            max_backoff: Duration::from_secs(3600),  // 1 hour
            open_duration: Duration::from_secs(300), // 5 minutes
            half_open_success_threshold: 3,
        }
    }
}

/// Circuit breaker for managing domain failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    domain_stats: Arc<RwLock<HashMap<String, DomainStats>>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            domain_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a domain is allowed to be accessed
    pub async fn is_allowed(&self, domain: &str) -> bool {
        let mut stats_map = self.domain_stats.write().await;
        let stats = stats_map.entry(domain.to_string()).or_default();

        match stats.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if it's time to try half-open
                if let Some(opened_at) = stats.circuit_opened_at {
                    if let Ok(elapsed) = SystemTime::now().duration_since(opened_at) {
                        if elapsed >= stats.backoff_duration {
                            info!("Circuit breaker for {} moving to half-open state after {:?} cooldown", 
                                  domain, stats.backoff_duration);
                            stats.state = CircuitState::HalfOpen;
                            stats.consecutive_failures = 0;
                            stats.half_open_successes = 0; // Reset half-open success counter
                            return true;
                        }
                    }
                }
                debug!(
                    "Circuit breaker blocking request to {} (cooldown: {:?} remaining)",
                    domain,
                    stats.backoff_duration.saturating_sub(
                        SystemTime::now()
                            .duration_since(stats.circuit_opened_at.unwrap_or(SystemTime::now()))
                            .unwrap_or_default()
                    )
                );
                false
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                debug!("Circuit breaker allowing half-open request to {}", domain);
                true
            }
        }
    }

    /// Record a successful request
    pub async fn record_success(&self, domain: &str) {
        let mut stats_map = self.domain_stats.write().await;
        let stats = stats_map.entry(domain.to_string()).or_default();

        stats.total_successes += 1;
        stats.last_success = Some(SystemTime::now());
        stats.consecutive_failures = 0;

        match stats.state {
            CircuitState::HalfOpen => {
                stats.half_open_successes += 1;
                // Check if we should close the circuit based on half-open successes
                if stats.half_open_successes >= self.config.half_open_success_threshold {
                    info!(
                        "Circuit breaker for {} closing after {} successful recovery attempts",
                        domain, stats.half_open_successes
                    );
                    stats.state = CircuitState::Closed;
                    stats.backoff_duration = self.config.initial_backoff;
                    stats.circuit_opened_at = None;
                    stats.half_open_successes = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                warn!("Unexpected success recorded for open circuit: {}", domain);
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self, domain: &str) {
        let mut stats_map = self.domain_stats.write().await;
        let stats = stats_map.entry(domain.to_string()).or_default();

        stats.total_failures += 1;
        stats.consecutive_failures += 1;
        stats.last_failure = Some(SystemTime::now());

        match stats.state {
            CircuitState::Closed => {
                if stats.consecutive_failures >= self.config.failure_threshold {
                    warn!(
                        "Circuit breaker opening for {} after {} consecutive failures",
                        domain, stats.consecutive_failures
                    );
                    stats.state = CircuitState::Open;
                    stats.circuit_opened_at = Some(SystemTime::now());
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open state, reopen circuit with increased backoff
                warn!(
                    "Circuit breaker reopening for {} after half-open failure (had {} successes)",
                    domain, stats.half_open_successes
                );
                stats.state = CircuitState::Open;
                stats.circuit_opened_at = Some(SystemTime::now());
                stats.half_open_successes = 0; // Reset the counter

                // Exponential backoff
                stats.backoff_duration =
                    std::cmp::min(stats.backoff_duration * 2, self.config.max_backoff);
            }
            CircuitState::Open => {
                // Already open, update backoff if needed
                stats.backoff_duration =
                    std::cmp::min(stats.backoff_duration * 2, self.config.max_backoff);
            }
        }
    }

    /// Get the current state of a domain
    pub async fn get_state(&self, domain: &str) -> CircuitState {
        let stats_map = self.domain_stats.read().await;
        stats_map
            .get(domain)
            .map(|s| s.state)
            .unwrap_or(CircuitState::Closed)
    }

    /// Get statistics for a domain
    pub async fn get_stats(&self, domain: &str) -> Option<DomainStats> {
        let stats_map = self.domain_stats.read().await;
        stats_map.get(domain).cloned()
    }

    /// Get all domain statistics
    pub async fn get_all_stats(&self) -> HashMap<String, DomainStats> {
        let stats_map = self.domain_stats.read().await;
        stats_map.clone()
    }

    /// Reset statistics for a domain
    pub async fn reset_domain(&self, domain: &str) {
        let mut stats_map = self.domain_stats.write().await;
        stats_map.remove(domain);
        info!("Circuit breaker reset for domain: {}", domain);
    }

    /// Reset all statistics
    pub async fn reset_all(&self) {
        let mut stats_map = self.domain_stats.write().await;
        stats_map.clear();
        info!("Circuit breaker reset for all domains");
    }

    /// Get domains that are currently blocked
    pub async fn get_blocked_domains(&self) -> Vec<String> {
        let stats_map = self.domain_stats.read().await;
        stats_map
            .iter()
            .filter(|(_, stats)| stats.state == CircuitState::Open)
            .map(|(domain, _)| domain.clone())
            .collect()
    }

    /// Get domains in half-open state
    pub async fn get_half_open_domains(&self) -> Vec<String> {
        let stats_map = self.domain_stats.read().await;
        stats_map
            .iter()
            .filter(|(_, stats)| stats.state == CircuitState::HalfOpen)
            .map(|(domain, _)| domain.clone())
            .collect()
    }

    /// Calculate failure rate for a domain
    pub async fn get_failure_rate(&self, domain: &str) -> Option<f64> {
        let stats_map = self.domain_stats.read().await;
        stats_map.get(domain).map(|stats| {
            let total = stats.total_failures + stats.total_successes;
            if total == 0 {
                0.0
            } else {
                stats.total_failures as f64 / total as f64
            }
        })
    }

    /// Check and update circuit states (useful for periodic maintenance)
    pub async fn check_and_update_states(&self) {
        let mut stats_map = self.domain_stats.write().await;
        let now = SystemTime::now();

        for (domain, stats) in stats_map.iter_mut() {
            if stats.state == CircuitState::Open {
                if let Some(opened_at) = stats.circuit_opened_at {
                    if let Ok(elapsed) = now.duration_since(opened_at) {
                        if elapsed >= stats.backoff_duration {
                            info!(
                                "Circuit breaker for {} ready for half-open transition",
                                domain
                            );
                            // Don't auto-transition here, wait for next request
                        }
                    }
                }
            }
        }
    }
}

/// Global circuit breaker instance
static GLOBAL_CIRCUIT_BREAKER: Lazy<CircuitBreaker> = Lazy::new(|| CircuitBreaker::new());

/// Check if a domain is allowed using the global circuit breaker
pub async fn is_domain_allowed(domain: &str) -> bool {
    GLOBAL_CIRCUIT_BREAKER.is_allowed(domain).await
}

/// Record a successful request using the global circuit breaker
pub async fn record_domain_success(domain: &str) {
    GLOBAL_CIRCUIT_BREAKER.record_success(domain).await;
}

/// Record a failed request using the global circuit breaker
pub async fn record_domain_failure(domain: &str) {
    GLOBAL_CIRCUIT_BREAKER.record_failure(domain).await;
}

/// Get the current state of a domain using the global circuit breaker
pub async fn get_domain_state(domain: &str) -> CircuitState {
    GLOBAL_CIRCUIT_BREAKER.get_state(domain).await
}

/// Get statistics for all domains
pub async fn get_all_domain_stats() -> HashMap<String, DomainStats> {
    GLOBAL_CIRCUIT_BREAKER.get_all_stats().await
}

/// Reset a specific domain's circuit breaker
pub async fn reset_domain_circuit(domain: &str) {
    GLOBAL_CIRCUIT_BREAKER.reset_domain(domain).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_states() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(1),
            open_duration: Duration::from_millis(200),
            half_open_success_threshold: 2,
        };

        let breaker = CircuitBreaker::with_config(config);
        let domain = "test.com";

        // Initially closed
        assert_eq!(breaker.get_state(domain).await, CircuitState::Closed);
        assert!(breaker.is_allowed(domain).await);

        // Record failures to open circuit
        breaker.record_failure(domain).await;
        assert!(breaker.is_allowed(domain).await); // Still closed after 1 failure

        breaker.record_failure(domain).await;
        assert_eq!(breaker.get_state(domain).await, CircuitState::Open);
        assert!(!breaker.is_allowed(domain).await); // Now open

        // Wait for half-open
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(breaker.is_allowed(domain).await); // Should be half-open now
        assert_eq!(breaker.get_state(domain).await, CircuitState::HalfOpen);

        // Success in half-open
        breaker.record_success(domain).await;
        breaker.record_success(domain).await;
        assert_eq!(breaker.get_state(domain).await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_failure_rate_calculation() {
        let breaker = CircuitBreaker::new();
        let domain = "test.com";

        breaker.record_success(domain).await;
        breaker.record_success(domain).await;
        breaker.record_failure(domain).await;

        let rate = breaker.get_failure_rate(domain).await.unwrap();
        assert!((rate - 0.333).abs() < 0.01); // ~33% failure rate
    }
}
