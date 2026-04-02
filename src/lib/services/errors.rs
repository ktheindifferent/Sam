use std::error::Error;
use std::fmt;

/// Main error type for SAM application
/// Provides comprehensive error context and source tracking
#[derive(Debug)]
pub enum SamError {
    /// IO operations (file, network socket, etc.)
    Io(std::io::Error),
    /// Network communication errors
    Network(String),
    /// Data parsing errors
    Parse(String),
    /// Timeout errors
    Timeout(String),
    /// Resource not found
    NotFound(String),
    /// Permission/authorization denied
    PermissionDenied(String),
    /// Invalid input provided
    InvalidInput(String),
    /// Service unavailable
    ServiceUnavailable(String),
    /// Rate limited
    RateLimited(String),
    /// Configuration error
    Configuration(String),
    /// Database operation error
    Database(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Authentication error
    Authentication(String),
    /// Authorization error
    Authorization(String),
    /// Validation error
    Validation(String),
    /// Error from external source
    External(Box<dyn Error + Send + Sync>),
    /// Other unspecified errors
    Other(String),
}

// Alias for backward compatibility
pub type CommonError = SamError;

impl fmt::Display for SamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SamError::Io(e) => write!(f, "IO error: {}", e),
            SamError::Network(msg) => write!(f, "Network error: {}", msg),
            SamError::Parse(msg) => write!(f, "Parse error: {}", msg),
            SamError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            SamError::NotFound(msg) => write!(f, "Not found: {}", msg),
            SamError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            SamError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            SamError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            SamError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            SamError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            SamError::Database(msg) => write!(f, "Database error: {}", msg),
            SamError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            SamError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            SamError::Authorization(msg) => write!(f, "Authorization error: {}", msg),
            SamError::Validation(msg) => write!(f, "Validation error: {}", msg),
            SamError::External(e) => write!(f, "External error: {}", e),
            SamError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for SamError {}

impl From<std::io::Error> for SamError {
    fn from(err: std::io::Error) -> Self {
        SamError::Io(err)
    }
}

impl From<serde_json::Error> for SamError {
    fn from(err: serde_json::Error) -> Self {
        SamError::Serialization(err.to_string())
    }
}

impl From<reqwest::Error> for SamError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            SamError::Timeout(err.to_string())
        } else if err.is_connect() {
            SamError::Network(format!("Connection failed: {}", err))
        } else {
            SamError::Network(err.to_string())
        }
    }
}

impl From<hound::Error> for SamError {
    fn from(err: hound::Error) -> Self {
        SamError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}

impl<T> From<std::sync::PoisonError<T>> for SamError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        SamError::External(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", err),
        )))
    }
}

impl From<std::time::SystemTimeError> for SamError {
    fn from(err: std::time::SystemTimeError) -> Self {
        SamError::External(Box::new(err))
    }
}

impl From<String> for SamError {
    fn from(err: String) -> Self {
        SamError::Other(err)
    }
}

impl From<&str> for SamError {
    fn from(err: &str) -> Self {
        SamError::Other(err.to_string())
    }
}

#[cfg(feature = "nst")]
impl From<tch::TchError> for SamError {
    fn from(err: tch::TchError) -> Self {
        SamError::External(Box::new(err))
    }
}

/// Result type alias using SamError
pub type Result<T> = std::result::Result<T, SamError>;

/// Trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context message to error
    fn context(self, msg: &str) -> Result<T>;
    /// Add context message via closure
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<SamError>,
{
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| {
            let err: SamError = e.into();
            SamError::External(Box::new(ContextError {
                context: msg.to_string(),
                source: Box::new(err),
            }))
        })
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let err: SamError = e.into();
            SamError::External(Box::new(ContextError {
                context: f(),
                source: Box::new(err),
            }))
        })
    }
}

#[derive(Debug)]
struct ContextError {
    context: String,
    source: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.source)
    }
}

impl Error for ContextError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sam_error_display() {
        let err = SamError::Network("Connection timeout".to_string());
        assert_eq!(err.to_string(), "Network error: Connection timeout");
    }

    #[test]
    fn test_sam_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let sam_err = SamError::from(io_err);
        assert!(matches!(sam_err, SamError::Io(_)));
    }

    #[test]
    fn test_error_context() {
        let result: Result<i32> = Err(SamError::Other("test error".to_string()));
        let contextualized = result.context("while processing config");
        assert!(contextualized.is_err());
    }
}
