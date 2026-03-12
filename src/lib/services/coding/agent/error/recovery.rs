//! Error recovery strategies

use std::time::Duration;
use async_trait::async_trait;
use log::{info, warn, error};

use super::AgentError;

/// Recovery strategy for handling errors
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    /// Attempt to recover from an error
    async fn recover(&self, error: &AgentError) -> RecoveryAction;

    /// Check if recovery should be attempted
    fn should_recover(&self, error: &AgentError) -> bool;

    /// Get recovery metadata
    fn metadata(&self) -> RecoveryMetadata;
}

/// Action to take for recovery
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry {
        delay: Duration,
        max_attempts: u32,
    },
    /// Fallback to alternative
    Fallback {
        alternative: String,
    },
    /// Skip and continue
    Skip,
    /// Abort execution
    Abort,
    /// Manual intervention required
    Manual {
        instructions: String,
    },
    /// Circuit breaker open
    CircuitBreak {
        duration: Duration,
    },
}

#[derive(Debug, Clone)]
pub struct RecoveryMetadata {
    pub strategy_name: String,
    pub max_retries: u32,
    pub backoff_multiplier: f64,
    pub circuit_breaker_threshold: u32,
}

/// Default recovery strategy with exponential backoff
pub struct DefaultRecoveryStrategy {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}

impl Default for DefaultRecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

#[async_trait]
impl RecoveryStrategy for DefaultRecoveryStrategy {
    async fn recover(&self, error: &AgentError) -> RecoveryAction {
        match error {
            AgentError::Provider(e) => self.recover_provider_error(e),
            AgentError::Execution(e) => self.recover_execution_error(e),
            AgentError::Resource(e) => self.recover_resource_error(e),
            AgentError::Configuration(_) => RecoveryAction::Abort,
            _ => RecoveryAction::Retry {
                delay: self.initial_delay,
                max_attempts: self.max_retries,
            },
        }
    }

    fn should_recover(&self, error: &AgentError) -> bool {
        !matches!(
            error,
            AgentError::Configuration(_) | AgentError::Analysis(_)
        )
    }

    fn metadata(&self) -> RecoveryMetadata {
        RecoveryMetadata {
            strategy_name: "DefaultRecoveryStrategy".to_string(),
            max_retries: self.max_retries,
            backoff_multiplier: self.backoff_multiplier,
            circuit_breaker_threshold: 5,
        }
    }
}

impl DefaultRecoveryStrategy {
    fn recover_provider_error(&self, error: &super::ProviderError) -> RecoveryAction {
        use super::ProviderError::*;

        match error {
            RateLimited {
                retry_after_seconds,
                ..
            } => RecoveryAction::Retry {
                delay: Duration::from_secs(*retry_after_seconds),
                max_attempts: 1,
            },
            Unavailable { .. } => RecoveryAction::Fallback {
                alternative: "alternative_provider".to_string(),
            },
            AuthenticationFailed { .. } => RecoveryAction::Manual {
                instructions: "Please check your API credentials".to_string(),
            },
            _ => RecoveryAction::Retry {
                delay: self.initial_delay,
                max_attempts: self.max_retries,
            },
        }
    }

    fn recover_execution_error(&self, error: &super::ExecutionError) -> RecoveryAction {
        use super::ExecutionError::*;

        match error {
            CommandNotAllowed { .. } => RecoveryAction::Abort,
            Timeout { .. } => RecoveryAction::Retry {
                delay: Duration::from_secs(1),
                max_attempts: 2,
            },
            CommandFailed { exit_code, .. } => {
                if exit_code.map_or(false, |code| code == 137) {
                    // Out of memory
                    RecoveryAction::Manual {
                        instructions: "Process killed due to memory limit".to_string(),
                    }
                } else {
                    RecoveryAction::Skip
                }
            }
            _ => RecoveryAction::Skip,
        }
    }

    fn recover_resource_error(&self, error: &super::ResourceError) -> RecoveryAction {
        use super::ResourceError::*;

        match error {
            LimitExceeded { .. } => RecoveryAction::CircuitBreak {
                duration: Duration::from_secs(60),
            },
            Locked { .. } => RecoveryAction::Retry {
                delay: Duration::from_millis(500),
                max_attempts: 5,
            },
            _ => RecoveryAction::Abort,
        }
    }
}

/// Recovery coordinator that manages multiple strategies
pub struct RecoveryCoordinator {
    strategies: Vec<Box<dyn RecoveryStrategy>>,
    attempt_counts: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
}

impl RecoveryCoordinator {
    pub fn new() -> Self {
        Self {
            strategies: vec![Box::new(DefaultRecoveryStrategy::default())],
            attempt_counts: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn RecoveryStrategy>) {
        self.strategies.push(strategy);
    }

    pub async fn handle_error(&self, error: AgentError, operation_id: &str) -> RecoveryResult {
        // Track attempt count
        let mut counts = self.attempt_counts.write().await;
        let attempt_count = counts.entry(operation_id.to_string()).or_insert(0);
        *attempt_count += 1;

        // Find appropriate strategy
        for strategy in &self.strategies {
            if strategy.should_recover(&error) {
                let action = strategy.recover(&error).await;

                info!(
                    "Recovery strategy {} suggested action {:?} for error: {}",
                    strategy.metadata().strategy_name,
                    action,
                    error
                );

                return RecoveryResult {
                    action,
                    attempt: *attempt_count,
                    error: Some(error),
                };
            }
        }

        // No recovery possible
        error!("No recovery strategy available for error: {}", error);
        RecoveryResult {
            action: RecoveryAction::Abort,
            attempt: *attempt_count,
            error: Some(error),
        }
    }

    pub async fn reset_attempts(&self, operation_id: &str) {
        let mut counts = self.attempt_counts.write().await;
        counts.remove(operation_id);
    }
}

#[derive(Debug)]
pub struct RecoveryResult {
    pub action: RecoveryAction,
    pub attempt: u32,
    pub error: Option<AgentError>,
}

/// Retry executor with recovery strategies
pub struct RetryExecutor {
    coordinator: RecoveryCoordinator,
}

impl RetryExecutor {
    pub fn new() -> Self {
        Self {
            coordinator: RecoveryCoordinator::new(),
        }
    }

    /// Execute with automatic recovery
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation_id: &str,
        operation: F,
    ) -> Result<T, AgentError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, AgentError>>,
    {
        let mut delay = Duration::from_millis(100);

        loop {
            match operation().await {
                Ok(result) => {
                    self.coordinator.reset_attempts(operation_id).await;
                    return Ok(result);
                }
                Err(error) => {
                    let recovery = self.coordinator.handle_error(error, operation_id).await;

                    match recovery.action {
                        RecoveryAction::Retry {
                            delay: retry_delay,
                            max_attempts,
                        } => {
                            if recovery.attempt >= max_attempts {
                                return Err(recovery.error.unwrap());
                            }

                            warn!(
                                "Retrying operation {} (attempt {}/{}) after {:?}",
                                operation_id, recovery.attempt, max_attempts, retry_delay
                            );

                            tokio::time::sleep(retry_delay).await;
                            delay = retry_delay * 2; // Exponential backoff
                        }
                        RecoveryAction::Skip => {
                            warn!("Skipping operation {} due to error", operation_id);
                            return Err(recovery.error.unwrap());
                        }
                        RecoveryAction::Abort => {
                            error!("Aborting operation {} due to unrecoverable error", operation_id);
                            return Err(recovery.error.unwrap());
                        }
                        RecoveryAction::Fallback { alternative } => {
                            warn!("Falling back to alternative: {}", alternative);
                            // In real implementation, would execute fallback
                            return Err(recovery.error.unwrap());
                        }
                        RecoveryAction::CircuitBreak { duration } => {
                            error!(
                                "Circuit breaker open for operation {}, waiting {:?}",
                                operation_id, duration
                            );
                            tokio::time::sleep(duration).await;
                            return Err(recovery.error.unwrap());
                        }
                        RecoveryAction::Manual { instructions } => {
                            error!(
                                "Manual intervention required for operation {}: {}",
                                operation_id, instructions
                            );
                            return Err(recovery.error.unwrap());
                        }
                    }
                }
            }
        }
    }
}