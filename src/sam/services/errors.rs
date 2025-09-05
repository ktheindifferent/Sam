use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CommonError {
    Io(std::io::Error),
    Network(String),
    Parse(String),
    Timeout(String),
    NotFound(String),
    PermissionDenied(String),
    InvalidInput(String),
    ServiceUnavailable(String),
    RateLimited(String),
    Configuration(String),
    Database(String),
    Serialization(String),
    Authentication(String),
    Authorization(String),
    Validation(String),
    External(Box<dyn Error + Send + Sync>),
    Other(String),
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommonError::Io(e) => write!(f, "IO error: {}", e),
            CommonError::Network(msg) => write!(f, "Network error: {}", msg),
            CommonError::Parse(msg) => write!(f, "Parse error: {}", msg),
            CommonError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            CommonError::NotFound(msg) => write!(f, "Not found: {}", msg),
            CommonError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            CommonError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CommonError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            CommonError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            CommonError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            CommonError::Database(msg) => write!(f, "Database error: {}", msg),
            CommonError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            CommonError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            CommonError::Authorization(msg) => write!(f, "Authorization error: {}", msg),
            CommonError::Validation(msg) => write!(f, "Validation error: {}", msg),
            CommonError::External(e) => write!(f, "External error: {}", e),
            CommonError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for CommonError {}

impl From<std::io::Error> for CommonError {
    fn from(err: std::io::Error) -> Self {
        CommonError::Io(err)
    }
}

impl From<serde_json::Error> for CommonError {
    fn from(err: serde_json::Error) -> Self {
        CommonError::Serialization(err.to_string())
    }
}

impl From<reqwest::Error> for CommonError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            CommonError::Timeout(err.to_string())
        } else if err.is_connect() {
            CommonError::Network(format!("Connection failed: {}", err))
        } else {
            CommonError::Network(err.to_string())
        }
    }
}

impl From<hound::Error> for CommonError {
    fn from(err: hound::Error) -> Self {
        CommonError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}

impl<T> From<std::sync::PoisonError<T>> for CommonError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        CommonError::External(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", err),
        )))
    }
}

impl From<std::time::SystemTimeError> for CommonError {
    fn from(err: std::time::SystemTimeError) -> Self {
        CommonError::External(Box::new(err))
    }
}

impl From<String> for CommonError {
    fn from(err: String) -> Self {
        CommonError::Other(err)
    }
}

impl From<&str> for CommonError {
    fn from(err: &str) -> Self {
        CommonError::Other(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CommonError>;

pub trait ErrorContext<T> {
    fn context(self, msg: &str) -> Result<T>;
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<CommonError>,
{
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| {
            let err: CommonError = e.into();
            CommonError::External(Box::new(ContextError {
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
            let err: CommonError = e.into();
            CommonError::External(Box::new(ContextError {
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