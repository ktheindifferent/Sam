//! Improved error handling with context and recovery strategies

use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

pub mod context;
pub mod recovery;
pub mod reporting;

pub use context::*;
pub use recovery::*;
pub use reporting::*;

/// Unified error type for the coding agent with rich context
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("{kind}: {message}")]
    Core {
        kind: ErrorKind,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Analysis error: {0}")]
    Analysis(#[from] AnalysisError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),

    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Error kinds for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    InvalidInput,
    Unauthorized,
    Forbidden,
    Timeout,
    RateLimited,
    ServiceUnavailable,
    Internal,
    Conflict,
    PreconditionFailed,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Not Found"),
            Self::InvalidInput => write!(f, "Invalid Input"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::Forbidden => write!(f, "Forbidden"),
            Self::Timeout => write!(f, "Timeout"),
            Self::RateLimited => write!(f, "Rate Limited"),
            Self::ServiceUnavailable => write!(f, "Service Unavailable"),
            Self::Internal => write!(f, "Internal Error"),
            Self::Conflict => write!(f, "Conflict"),
            Self::PreconditionFailed => write!(f, "Precondition Failed"),
        }
    }
}

/// Provider-specific errors
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider {name} unavailable: {reason}")]
    Unavailable { name: String, reason: String },

    #[error("Provider {name} not found")]
    NotFound { name: String },

    #[error("No provider configured")]
    NoProvider,

    #[error("Provider {name} rate limited: retry after {retry_after_seconds}s")]
    RateLimited {
        name: String,
        retry_after_seconds: u64,
    },

    #[error("Provider {name} authentication failed: {reason}")]
    AuthenticationFailed { name: String, reason: String },

    #[error("Provider {name} returned invalid response: {reason}")]
    InvalidResponse { name: String, reason: String },
}

/// Execution-specific errors
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Command execution failed: {command} (exit code: {exit_code:?})")]
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },

    #[error("Command not allowed: {command} (reason: {reason})")]
    CommandNotAllowed { command: String, reason: String },

    #[error("Working directory error: {path:?} - {reason}")]
    WorkingDirectoryError { path: PathBuf, reason: String },

    #[error("Execution timeout after {seconds}s: {operation}")]
    Timeout { operation: String, seconds: u64 },

    #[error("Invalid execution state: {message}")]
    InvalidState { message: String },
}

/// Analysis-specific errors
#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Language {language} not supported")]
    UnsupportedLanguage { language: String },

    #[error("File too large: {size_mb}MB (max: {max_mb}MB)")]
    FileTooLarge { size_mb: f64, max_mb: f64 },

    #[error("Analysis incomplete: {reason}")]
    Incomplete { reason: String },
}

/// Configuration-specific errors
#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Invalid configuration: {field} - {message}")]
    Invalid { field: String, message: String },

    #[error("Missing required configuration: {field}")]
    Missing { field: String },

    #[error("Configuration file error: {path:?} - {reason}")]
    FileError { path: PathBuf, reason: String },

    #[error("Environment variable {var} not set")]
    EnvironmentNotSet { var: String },
}

/// Resource-specific errors
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("Resource limit exceeded: {resource} (limit: {limit}, requested: {requested})")]
    LimitExceeded {
        resource: String,
        limit: String,
        requested: String,
    },

    #[error("Resource unavailable: {resource}")]
    Unavailable { resource: String },

    #[error("Resource locked: {resource}")]
    Locked { resource: String },

    #[error("Insufficient resources: {message}")]
    Insufficient { message: String },
}

/// Result type alias for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Extension trait for adding context to errors
pub trait ErrorExt<T> {
    /// Add context to the error
    fn context<C>(self, context: C) -> AgentResult<T>
    where
        C: fmt::Display + Send + Sync + 'static;

    /// Add context with a closure
    fn with_context<C, F>(self, f: F) -> AgentResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> ErrorExt<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> AgentResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| AgentError::Core {
            kind: ErrorKind::Internal,
            message: context.to_string(),
            source: Some(Box::new(e)),
            context: ErrorContext::default(),
        })
    }

    fn with_context<C, F>(self, f: F) -> AgentResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| AgentError::Core {
            kind: ErrorKind::Internal,
            message: f().to_string(),
            source: Some(Box::new(e)),
            context: ErrorContext::default(),
        })
    }
}

/// Convert from old error type to new
impl From<crate::services::coding::agent::errors::CodingAgentError> for AgentError {
    fn from(old: crate::services::coding::agent::errors::CodingAgentError) -> Self {
        use crate::services::coding::agent::errors::CodingAgentError as Old;

        match old {
            Old::ProviderUnavailable { provider, reason } => ProviderError::Unavailable {
                name: provider,
                reason: reason.unwrap_or_default(),
            }
            .into(),
            Old::ProviderNotFound { provider } => ProviderError::NotFound { name: provider }.into(),
            Old::NoProviderConfigured => ProviderError::NoProvider.into(),
            Old::CommandExecutionFailed {
                command,
                output,
                exit_code,
            } => ExecutionError::CommandFailed {
                command,
                exit_code,
                stdout: output.clone(),
                stderr: output,
            }
            .into(),
            Old::CommandNotAllowed { command, reason } => {
                ExecutionError::CommandNotAllowed { command, reason }.into()
            }
            Old::WorkingDirectoryError { path, reason } => {
                ExecutionError::WorkingDirectoryError { path, reason }.into()
            }
            Old::ParseError { message, context } => AnalysisError::ParseError {
                line: 0,
                column: 0,
                message: format!("{} (context: {:?})", message, context),
            }
            .into(),
            Old::Timeout {
                operation,
                timeout_seconds,
            } => ExecutionError::Timeout {
                operation,
                seconds: timeout_seconds,
            }
            .into(),
            Old::ConfigError { message } => ConfigurationError::Invalid {
                field: "unknown".to_string(),
                message,
            }
            .into(),
            Old::ResourceLimitExceeded {
                resource,
                limit,
                current,
            } => ResourceError::LimitExceeded {
                resource,
                limit,
                requested: current,
            }
            .into(),
            _ => AgentError::Other(anyhow::anyhow!("{}", old)),
        }
    }
}
