use thiserror::Error;
use std::path::PathBuf;

/// Comprehensive error types for the coding agent
#[derive(Debug, Error)]
pub enum CodingAgentError {
    #[error("Provider unavailable: {provider}")]
    ProviderUnavailable {
        provider: String,
        reason: Option<String>,
    },

    #[error("Provider not found: {provider}")]
    ProviderNotFound {
        provider: String
    },

    #[error("No provider configured")]
    NoProviderConfigured,

    #[error("Command execution failed: {command}")]
    CommandExecutionFailed {
        command: String,
        output: String,
        exit_code: Option<i32>,
    },

    #[error("Command not allowed: {command} (reason: {reason})")]
    CommandNotAllowed {
        command: String,
        reason: String,
    },

    #[error("Working directory error: {path:?}")]
    WorkingDirectoryError {
        path: PathBuf,
        reason: String,
    },

    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        context: Option<String>,
    },

    #[error("Context error: {message}")]
    ContextError {
        message: String
    },

    #[error("Template error: {template}")]
    TemplateError {
        template: String,
        reason: String,
    },

    #[error("Project analysis error: {message}")]
    ProjectAnalysisError {
        message: String
    },

    #[error("Git operation failed: {operation}")]
    GitError {
        operation: String,
        reason: String,
    },

    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded {
        resource: String,
        limit: String,
        current: String,
    },

    #[error("Timeout: operation took longer than {timeout_seconds} seconds")]
    Timeout {
        operation: String,
        timeout_seconds: u64,
    },

    #[error("Model error: {model}")]
    ModelError {
        model: String,
        reason: String,
    },

    #[error("Configuration error: {message}")]
    ConfigError {
        message: String
    },

    #[error("IO error: {message}")]
    IoError {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Network error: {message}")]
    NetworkError {
        message: String,
        url: Option<String>,
    },

    #[error("Serialization error: {message}")]
    SerializationError {
        message: String
    },

    #[error("Circuit breaker open for provider: {provider}")]
    CircuitBreakerOpen {
        provider: String,
        retry_after_seconds: Option<u64>,
    },

    #[error("Retry limit exceeded after {attempts} attempts")]
    RetryLimitExceeded {
        attempts: u32,
        last_error: String,
    },

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Not found: {resource} with id {id}")]
    NotFound {
        resource: String,
        id: String,
    },

    #[error("Validation error: {field} - {message}")]
    ValidationError {
        field: String,
        message: String,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type alias for coding agent operations
pub type CodingAgentResult<T> = Result<T, CodingAgentError>;

/// Error severity levels for better error handling decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorSeverity {
    /// Can be safely ignored or logged
    Warning,
    /// Should be handled but not critical
    Error,
    /// Critical error that should stop execution
    Critical,
    /// Fatal error that may require system restart
    Fatal,
}

impl CodingAgentError {
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::ParseError { .. } => ErrorSeverity::Warning,
            Self::ProviderUnavailable { .. } => ErrorSeverity::Error,
            Self::CommandNotAllowed { .. } => ErrorSeverity::Error,
            Self::ResourceLimitExceeded { .. } => ErrorSeverity::Critical,
            Self::NoProviderConfigured => ErrorSeverity::Critical,
            Self::WorkingDirectoryError { .. } => ErrorSeverity::Error,
            Self::CommandExecutionFailed { .. } => ErrorSeverity::Error,
            _ => ErrorSeverity::Error,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ProviderUnavailable { .. }
                | Self::NetworkError { .. }
                | Self::Timeout { .. }
                | Self::CircuitBreakerOpen { .. }
        )
    }

    /// Get suggested retry delay in seconds
    pub fn retry_delay_seconds(&self) -> Option<u64> {
        match self {
            Self::CircuitBreakerOpen { retry_after_seconds, .. } => *retry_after_seconds,
            Self::ProviderUnavailable { .. } => Some(5),
            Self::NetworkError { .. } => Some(2),
            Self::Timeout { .. } => Some(10),
            _ => None,
        }
    }

    /// Convert to user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::ProviderUnavailable { provider, .. } => {
                format!("The {} AI provider is currently unavailable. Trying alternatives...", provider)
            }
            Self::CommandNotAllowed { command, .. } => {
                format!("Command '{}' is not allowed for safety reasons", command)
            }
            Self::ResourceLimitExceeded { resource, limit, .. } => {
                format!("{} limit exceeded (max: {})", resource, limit)
            }
            Self::Timeout { operation, timeout_seconds } => {
                format!("{} timed out after {} seconds", operation, timeout_seconds)
            }
            _ => self.to_string(),
        }
    }
}

// Conversion implementations for common error types
impl From<std::io::Error> for CodingAgentError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            message: err.to_string(),
            path: None,
        }
    }
}

impl From<serde_json::Error> for CodingAgentError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for CodingAgentError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError {
            message: err.to_string(),
            url: err.url().map(|u| u.to_string()),
        }
    }
}