use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout};
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestartError {
    #[error("Lock acquisition failed: {0}")]
    LockError(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Restart failed: {0}")]
    RestartFailed(String),
}

use super::orchestrator::{ServiceName, ServiceStatus, ServiceHealth};

/// Restart strategy for a service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestartStrategy {
    /// Restart immediately
    Immediate,
    /// Restart after a delay
    Delayed(Duration),
    /// Restart at a scheduled time
    #[serde(skip)]
    Scheduled(Instant),
    /// Exponential backoff with base delay and maximum delay
    ExponentialBackoff {
        base_delay: Duration,
        max_delay: Duration,
        multiplier: f32,
    },
}

/// Circuit breaker state for service restarts
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed, service can be restarted
    Closed,
    /// Circuit is open, service cannot be restarted
    Open(Instant), // Time when circuit was opened
    /// Circuit is half-open, testing if service is stable
    HalfOpen,
}

/// Configuration for service restart behavior
#[derive(Debug, Clone)]
pub struct RestartConfig {
    pub strategy: RestartStrategy,
    pub max_attempts: u32,
    pub health_check_timeout: Duration,
    pub health_check_retries: u32,
    pub dependency_check: bool,
    pub circuit_breaker_enabled: bool,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout: Duration,
    pub notify_on_restart: bool,
    pub notify_on_failure: bool,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self {
            strategy: RestartStrategy::ExponentialBackoff {
                base_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                multiplier: 2.0,
            },
            max_attempts: 3,
            health_check_timeout: Duration::from_secs(30),
            health_check_retries: 3,
            dependency_check: true,
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(300),
            notify_on_restart: true,
            notify_on_failure: true,
        }
    }
}

/// Statistics for service restarts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestartMetrics {
    pub total_restarts: u64,
    pub successful_restarts: u64,
    pub failed_restarts: u64,
    #[serde(skip)]
    pub last_restart: Option<Instant>,
    #[serde(skip)]
    pub last_success: Option<Instant>,
    #[serde(skip)]
    pub last_failure: Option<Instant>,
    pub average_restart_time: Duration,
    pub consecutive_failures: u32,
    pub circuit_breaker_trips: u32,
}

/// Event types for restart notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartEvent {
    RestartInitiated {
        service: ServiceName,
        attempt: u32,
        reason: String,
    },
    RestartSucceeded {
        service: ServiceName,
        duration: Duration,
    },
    RestartFailed {
        service: ServiceName,
        attempt: u32,
        error: String,
    },
    CircuitBreakerTripped {
        service: ServiceName,
        failure_count: u32,
    },
    CircuitBreakerReset {
        service: ServiceName,
    },
    DependencyCheckFailed {
        service: ServiceName,
        missing_dependencies: Vec<ServiceName>,
    },
    HealthCheckFailed {
        service: ServiceName,
        error: String,
    },
}

/// Notification handler for restart events
#[async_trait]
pub trait RestartNotifier: Send + Sync {
    async fn notify(&self, event: RestartEvent) -> Result<()>;
}

/// Default notification handler that logs events
pub struct LogNotifier;

#[async_trait]
impl RestartNotifier for LogNotifier {
    async fn notify(&self, event: RestartEvent) -> Result<()> {
        match event {
            RestartEvent::RestartInitiated { service, attempt, reason } => {
                info!("Initiating restart for {:?} (attempt {}): {}", service, attempt, reason);
            }
            RestartEvent::RestartSucceeded { service, duration } => {
                info!("Successfully restarted {:?} in {:?}", service, duration);
            }
            RestartEvent::RestartFailed { service, attempt, error } => {
                error!("Failed to restart {:?} (attempt {}): {}", service, attempt, error);
            }
            RestartEvent::CircuitBreakerTripped { service, failure_count } => {
                error!("Circuit breaker tripped for {:?} after {} failures", service, failure_count);
            }
            RestartEvent::CircuitBreakerReset { service } => {
                info!("Circuit breaker reset for {:?}", service);
            }
            RestartEvent::DependencyCheckFailed { service, missing_dependencies } => {
                warn!("Dependency check failed for {:?}, missing: {:?}", service, missing_dependencies);
            }
            RestartEvent::HealthCheckFailed { service, error } => {
                warn!("Health check failed for {:?}: {}", service, error);
            }
        }
        Ok(())
    }
}

/// Service restart manager
#[derive(Clone)]
pub struct RestartManager {
    configs: Arc<RwLock<HashMap<ServiceName, RestartConfig>>>,
    metrics: Arc<RwLock<HashMap<ServiceName, RestartMetrics>>>,
    circuit_states: Arc<RwLock<HashMap<ServiceName, CircuitState>>>,
    restart_locks: Arc<Mutex<HashMap<ServiceName, Arc<Mutex<()>>>>>,
    notifiers: Arc<RwLock<Vec<Arc<dyn RestartNotifier>>>>,
}

impl RestartManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            circuit_states: Arc::new(RwLock::new(HashMap::new())),
            restart_locks: Arc::new(Mutex::new(HashMap::new())),
            notifiers: Arc::new(RwLock::new(vec![Arc::new(LogNotifier)])),
        }
    }

    /// Register a restart configuration for a service
    pub fn register_config(&self, service: ServiceName, config: RestartConfig) -> Result<()> {
        self.configs.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire configs lock: {}", e))?
            .insert(service.clone(), config);
        self.metrics.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire metrics lock: {}", e))?
            .insert(service.clone(), RestartMetrics::default());
        self.circuit_states.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire circuit states lock: {}", e))?
            .insert(service, CircuitState::Closed);
        Ok(())
    }

    /// Add a notification handler
    pub fn add_notifier(&self, notifier: Arc<dyn RestartNotifier>) -> Result<()> {
        self.notifiers.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire notifiers lock: {}", e))?
            .push(notifier);
        Ok(())
    }

    /// Send notification to all handlers
    pub async fn notify(&self, event: RestartEvent) {
        let notifiers = match self.notifiers.read() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire notifiers lock: {}", e);
                return;
            }
        };
        for notifier in notifiers {
            if let Err(e) = notifier.notify(event.clone()).await {
                error!("Failed to send restart notification: {}", e);
            }
        }
    }

    /// Calculate delay based on restart strategy and attempt number
    pub fn calculate_delay(&self, service: &ServiceName, attempt: u32) -> Duration {
        let configs = match self.configs.read() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire configs lock: {}", e);
                return Duration::from_secs(1);
            }
        };
        let config = configs.get(service).cloned().unwrap_or_default();
        
        match config.strategy {
            RestartStrategy::Immediate => Duration::from_secs(0),
            RestartStrategy::Delayed(delay) => delay,
            RestartStrategy::Scheduled(time) => {
                let now = Instant::now();
                if time > now {
                    time - now
                } else {
                    Duration::from_secs(0)
                }
            }
            RestartStrategy::ExponentialBackoff { base_delay, max_delay, multiplier } => {
                let delay_ms = (base_delay.as_millis() as f64 * (multiplier as f64).powi(attempt as i32)) as u64;
                let delay = Duration::from_millis(delay_ms);
                std::cmp::min(delay, max_delay)
            }
        }
    }

    /// Check if circuit breaker allows restart
    pub fn check_circuit_breaker(&self, service: &ServiceName) -> bool {
        let mut states = match self.circuit_states.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire circuit states lock: {}", e);
                return false;
            }
        };
        let configs = match self.configs.read() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire configs lock: {}", e);
                return false;
            }
        };
        let config = configs.get(service).cloned().unwrap_or_default();
        
        if !config.circuit_breaker_enabled {
            return true;
        }
        
        let state = states.get_mut(service).cloned().unwrap_or(CircuitState::Closed);
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open(opened_at) => {
                // Check if timeout has passed
                if Instant::now() > opened_at + config.circuit_breaker_timeout {
                    states.insert(service.clone(), CircuitState::HalfOpen);
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Update circuit breaker state based on restart result
    pub fn update_circuit_breaker(&self, service: &ServiceName, success: bool) {
        let mut states = self.circuit_states.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();
        let configs = self.configs.read().unwrap();
        let config = configs.get(service).cloned().unwrap_or_default();
        
        if !config.circuit_breaker_enabled {
            return;
        }
        
        let state = states.get(service).cloned().unwrap_or(CircuitState::Closed);
        let service_metrics = metrics.get_mut(service);
        
        match state {
            CircuitState::HalfOpen => {
                if success {
                    // Reset to closed on success
                    states.insert(service.clone(), CircuitState::Closed);
                    if let Some(m) = service_metrics {
                        m.consecutive_failures = 0;
                    }
                } else {
                    // Back to open on failure
                    states.insert(service.clone(), CircuitState::Open(Instant::now()));
                }
            }
            _ => {
                if !success {
                    if let Some(m) = service_metrics {
                        m.consecutive_failures += 1;
                        
                        // Trip circuit breaker if threshold reached
                        if m.consecutive_failures >= config.circuit_breaker_threshold {
                            states.insert(service.clone(), CircuitState::Open(Instant::now()));
                            m.circuit_breaker_trips += 1;
                        }
                    }
                } else if let Some(m) = service_metrics {
                    m.consecutive_failures = 0;
                }
            }
        }
    }

    /// Update restart metrics
    pub fn update_metrics(&self, service: &ServiceName, success: bool, duration: Duration) {
        let mut metrics = match self.metrics.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire metrics lock: {}", e);
                return;
            }
        };
        let m = metrics.entry(service.clone()).or_insert_with(RestartMetrics::default);
        
        m.total_restarts += 1;
        m.last_restart = Some(Instant::now());
        
        if success {
            m.successful_restarts += 1;
            m.last_success = Some(Instant::now());
            
            // Update average restart time
            let total_time = m.average_restart_time.as_millis() as u64 * (m.successful_restarts - 1);
            let new_average = (total_time + duration.as_millis() as u64) / m.successful_restarts;
            m.average_restart_time = Duration::from_millis(new_average);
        } else {
            m.failed_restarts += 1;
            m.last_failure = Some(Instant::now());
        }
    }

    /// Get restart metrics for a service
    pub fn get_metrics(&self, service: &ServiceName) -> Option<RestartMetrics> {
        match self.metrics.read() {
            Ok(guard) => guard.get(service).cloned(),
            Err(e) => {
                error!("Failed to acquire metrics lock: {}", e);
                None
            }
        }
    }

    /// Get all restart metrics
    pub fn get_all_metrics(&self) -> HashMap<ServiceName, RestartMetrics> {
        match self.metrics.read() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire metrics lock: {}", e);
                HashMap::new()
            }
        }
    }

    /// Reset metrics for a service
    pub fn reset_metrics(&self, service: &ServiceName) -> Result<()> {
        self.metrics.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire metrics lock: {}", e))?
            .insert(service.clone(), RestartMetrics::default());
        self.circuit_states.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire circuit states lock: {}", e))?
            .insert(service.clone(), CircuitState::Closed);
        Ok(())
    }
}

#[cfg(test)]
#[path = "restart_test.rs"]
mod restart_test;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_calculation() {
        let manager = RestartManager::new();
        let config = RestartConfig {
            strategy: RestartStrategy::ExponentialBackoff {
                base_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(30),
                multiplier: 2.0,
            },
            ..Default::default()
        };
        
        manager.register_config(ServiceName::Redis, config).expect("Failed to register config");
        
        // Test backoff progression
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 0), Duration::from_secs(1));
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 1), Duration::from_secs(2));
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 2), Duration::from_secs(4));
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 3), Duration::from_secs(8));
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 4), Duration::from_secs(16));
        assert_eq!(manager.calculate_delay(&ServiceName::Redis, 5), Duration::from_secs(30)); // Capped at max
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let manager = RestartManager::new();
        let config = RestartConfig {
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: 3,
            ..Default::default()
        };
        
        manager.register_config(ServiceName::PostgreSQL, config).expect("Failed to register config");
        
        // Initially closed
        assert!(manager.check_circuit_breaker(&ServiceName::PostgreSQL));
        
        // Simulate failures
        manager.update_circuit_breaker(&ServiceName::PostgreSQL, false);
        manager.update_circuit_breaker(&ServiceName::PostgreSQL, false);
        assert!(manager.check_circuit_breaker(&ServiceName::PostgreSQL)); // Still closed
        
        // Third failure should trip the breaker
        manager.update_circuit_breaker(&ServiceName::PostgreSQL, false);
        assert!(!manager.check_circuit_breaker(&ServiceName::PostgreSQL)); // Now open
    }

    #[test]
    fn test_metrics_tracking() {
        let manager = RestartManager::new();
        manager.register_config(ServiceName::Docker, RestartConfig::default()).expect("Failed to register config");
        
        // Simulate successful restart
        manager.update_metrics(&ServiceName::Docker, true, Duration::from_secs(5));
        
        let metrics = manager.get_metrics(&ServiceName::Docker).expect("Failed to get metrics");
        assert_eq!(metrics.total_restarts, 1);
        assert_eq!(metrics.successful_restarts, 1);
        assert_eq!(metrics.failed_restarts, 0);
        assert_eq!(metrics.average_restart_time, Duration::from_secs(5));
        
        // Simulate failed restart
        manager.update_metrics(&ServiceName::Docker, false, Duration::from_secs(0));
        
        let metrics = manager.get_metrics(&ServiceName::Docker).expect("Failed to get metrics after failure");
        assert_eq!(metrics.total_restarts, 2);
        assert_eq!(metrics.successful_restarts, 1);
        assert_eq!(metrics.failed_restarts, 1);
    }
}