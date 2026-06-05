use super::constants::*;
use super::errors::{CodingAgentError, CodingAgentResult, ErrorSeverity};
use log::{error, info, warn};
use std::fmt;

/// Trait for consistent error handling across the coding agent
pub trait ErrorHandler {
    /// Handle an error with appropriate logging and recovery
    fn handle_error(&self, error: &CodingAgentError, context: &str) -> ErrorAction;

    /// Log error with appropriate severity
    fn log_error(&self, error: &CodingAgentError, context: &str);

    /// Determine if operation should be retried
    fn should_retry(&self, error: &CodingAgentError, attempt: u32) -> bool;

    /// Get retry delay for the error
    fn get_retry_delay(&self, error: &CodingAgentError, attempt: u32) -> std::time::Duration;
}

/// Action to take after handling an error
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorAction {
    Retry { delay: std::time::Duration },
    Fallback { strategy: FallbackStrategy },
    Propagate,
    Ignore,
}

/// Fallback strategies for error recovery
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackStrategy {
    UseDefault,
    UseCache,
    UseDifferentProvider,
    SkipOperation,
}

/// Default error handler implementation
pub struct DefaultErrorHandler {
    max_retries: u32,
    base_retry_delay_ms: u64,
}

impl Default for DefaultErrorHandler {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_RETRY_ATTEMPTS,
            base_retry_delay_ms: DEFAULT_RETRY_DELAY_SECONDS * 1000,
        }
    }
}

impl ErrorHandler for DefaultErrorHandler {
    fn handle_error(&self, error: &CodingAgentError, context: &str) -> ErrorAction {
        self.log_error(error, context);

        match error.severity() {
            ErrorSeverity::Info => ErrorAction::Ignore,
            ErrorSeverity::Warning => ErrorAction::Ignore,
            ErrorSeverity::Error => {
                if error.is_retryable() {
                    ErrorAction::Retry {
                        delay: std::time::Duration::from_millis(self.base_retry_delay_ms),
                    }
                } else {
                    ErrorAction::Fallback {
                        strategy: FallbackStrategy::UseDefault,
                    }
                }
            }
            ErrorSeverity::Critical | ErrorSeverity::Fatal => ErrorAction::Propagate,
        }
    }

    fn log_error(&self, error: &CodingAgentError, context: &str) {
        let message = format!("[{}] {}: {}", context, error.severity(), error);

        match error.severity() {
            ErrorSeverity::Info => info!("{}", message),
            ErrorSeverity::Warning => warn!("{}", message),
            ErrorSeverity::Error | ErrorSeverity::Critical => error!("{}", message),
            ErrorSeverity::Fatal => error!("FATAL: {}", message),
        }
    }

    fn should_retry(&self, error: &CodingAgentError, attempt: u32) -> bool {
        error.is_retryable() && attempt < self.max_retries
    }

    fn get_retry_delay(&self, error: &CodingAgentError, attempt: u32) -> std::time::Duration {
        let base_delay = error
            .retry_delay_seconds(attempt)
            .unwrap_or(DEFAULT_RETRY_DELAY_SECONDS);

        // Exponential backoff with jitter
        let delay_ms = self.base_retry_delay_ms * 2u64.pow(attempt.min(5));
        let jitter = (delay_ms / 10) as i64; // 10% jitter
        let final_delay = delay_ms as i64 + (rand::random::<i64>() % jitter);

        std::time::Duration::from_millis(final_delay.max(0) as u64)
            .min(std::time::Duration::from_secs(MAX_RETRY_DELAY_SECONDS))
    }
}

/// Retry helper with exponential backoff
pub async fn retry_with_backoff<T, F, Fut>(
    operation: F,
    context: &str,
    handler: &impl ErrorHandler,
) -> CodingAgentResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = CodingAgentResult<T>>,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                attempt += 1;

                if !handler.should_retry(&error, attempt) {
                    handler.log_error(&error, context);
                    return Err(error);
                }

                let delay = handler.get_retry_delay(&error, attempt);
                info!(
                    "Retrying {} after {:?} (attempt {}/{})",
                    context, delay, attempt, DEFAULT_RETRY_ATTEMPTS
                );

                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Convert any error to CodingAgentError with context
pub fn wrap_error<E: std::error::Error>(error: E, context: &str) -> CodingAgentError {
    CodingAgentError::Other(anyhow::anyhow!("{}: {}", context, error))
}

/// Log and convert Result to Option
pub fn log_and_continue<T, E: fmt::Display>(result: Result<T, E>, context: &str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            warn!("{}: {}", context, e);
            None
        }
    }
}

/// Ensure a critical operation succeeds or panic
pub fn ensure_critical<T, E: fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(e) => {
            error!("Critical failure in {}: {}", context, e);
            panic!("Critical operation failed: {}", context);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Fatal > ErrorSeverity::Critical);
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
        assert!(ErrorSeverity::Error > ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning > ErrorSeverity::Info);
    }

    #[test]
    fn test_default_error_handler() {
        let handler = DefaultErrorHandler::default();
        let error = CodingAgentError::NetworkError {
            message: "Test error".to_string(),
            url: None,
        };

        assert!(handler.should_retry(&error, 0));
        assert!(handler.should_retry(&error, 1));
        assert!(!handler.should_retry(&error, DEFAULT_RETRY_ATTEMPTS));
    }

    #[test]
    fn test_error_action() {
        let handler = DefaultErrorHandler::default();

        let info_error = CodingAgentError::ValidationError {
            field: "test".to_string(),
            message: "info".to_string(),
        };

        // Info level errors should be ignored
        let action = handler.handle_error(&info_error, "test");
        assert_eq!(action, ErrorAction::Ignore);
    }
}
