//! WebSocket error handling module
//! 
//! Provides comprehensive error types for WebSocket operations with proper
//! error propagation and graceful fallback mechanisms.

use std::fmt;
use std::net::AddrParseError;
use thiserror::Error;

/// WebSocket error types with proper error handling
#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("JSON serialization failed: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    
    #[error("Regex compilation failed: {0}")]
    RegexCompilation(String),
    
    #[error("System time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
    
    #[error("IP address parsing failed: {0}")]
    IpParsing(#[from] AddrParseError),
    
    #[error("WebSocket communication error: {0}")]
    WebSocketComm(String),
    
    #[error("Channel send error: {0}")]
    ChannelSend(String),
    
    #[error("Security error: {0}")]
    Security(#[from] WsSecurityError),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

/// WebSocket security error types (existing, moved here for consistency)
#[derive(Debug, Clone, Error)]
pub enum WsSecurityError {
    #[error("Message size {size} exceeds maximum {max_size}")]
    MessageTooLarge { size: usize, max_size: usize },
    
    #[error("Rate limit exceeded: {limit} messages per {window:?}")]
    RateLimitExceeded { 
        limit: u32, 
        window: std::time::Duration 
    },
    
    #[error("Too many connections from {ip}: limit {limit}")]
    TooManyConnections { ip: String, limit: usize },
    
    #[error("Session has expired")]
    SessionExpired,
    
    #[error("Invalid session")]
    SessionInvalid,
    
    #[error("Message validation failed: {0}")]
    MessageValidationFailed(String),
    
    #[error("Injection attempt detected: {0}")]
    InjectionAttempt(String),
    
    #[error("Unauthorized action: {0}")]
    UnauthorizedAction(String),
    
    #[error("Connection idle timeout")]
    ConnectionIdle,
    
    #[error("Message queue is full")]
    QueueFull,
    
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    
    #[error("Token has expired")]
    TokenExpired,
    
    #[error("Token is missing")]
    MissingToken,
}

impl serde::Serialize for WsSecurityError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("WsSecurityError", 2)?;
        state.serialize_field("error", &self.to_string())?;
        state.serialize_field("type", &format!("{:?}", self))?;
        state.end()
    }
}

/// Result type alias for WebSocket operations
pub type WebSocketResult<T> = Result<T, WebSocketError>;

/// Result type alias for security operations
pub type SecurityResult<T> = Result<T, WsSecurityError>;

/// Helper functions for error recovery
impl WebSocketError {
    /// Create a JSON serialization error with context
    pub fn json_error(context: &str, err: serde_json::Error) -> Self {
        WebSocketError::JsonSerialization(err)
    }
    
    /// Log error and return a default value
    pub fn log_and_default<T: Default>(self) -> T {
        log::error!("WebSocket error (using default): {}", self);
        T::default()
    }
    
    /// Log error and continue operation
    pub fn log_and_continue(self) {
        log::error!("WebSocket error (continuing): {}", self);
    }
}

/// Utility functions for safe operations
pub mod safe_ops {
    use super::*;
    use regex::Regex;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    /// Safely compile a regex pattern with fallback
    pub fn compile_regex(pattern: &str) -> Result<Regex, WebSocketError> {
        Regex::new(pattern)
            .map_err(|e| WebSocketError::RegexCompilation(format!("Failed to compile regex '{}': {}", pattern, e)))
    }
    
    /// Safely compile regex with default fallback
    pub fn compile_regex_or_default(pattern: &str, default_pattern: &str) -> Regex {
        match Regex::new(pattern) {
            Ok(regex) => regex,
            Err(e) => {
                log::error!("Failed to compile regex '{}': {}, using default pattern", pattern, e);
                Regex::new(default_pattern)
                    .expect("Default regex pattern should be valid")
            }
        }
    }
    
    /// Safely get current Unix timestamp
    pub fn unix_timestamp() -> Result<usize, WebSocketError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as usize)
            .map_err(WebSocketError::from)
    }
    
    /// Safely get Unix timestamp with fallback
    pub fn unix_timestamp_or_default() -> usize {
        unix_timestamp().unwrap_or_else(|e| {
            log::error!("Failed to get Unix timestamp: {}, using 0", e);
            0
        })
    }
    
    /// Safely serialize to JSON
    pub fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String, WebSocketError> {
        serde_json::to_string(value).map_err(WebSocketError::from)
    }
    
    /// Safely serialize to JSON with fallback
    pub fn serialize_json_or_default<T: serde::Serialize>(value: &T, default: &str) -> String {
        serde_json::to_string(value).unwrap_or_else(|e| {
            log::error!("Failed to serialize JSON: {}, using default", e);
            default.to_string()
        })
    }
    
    /// Safely parse IP address
    pub fn parse_ip(ip_str: &str) -> Result<std::net::IpAddr, WebSocketError> {
        ip_str.parse().map_err(WebSocketError::from)
    }
    
    /// Safely parse IP with fallback
    pub fn parse_ip_or_default(ip_str: &str) -> std::net::IpAddr {
        ip_str.parse().unwrap_or_else(|e| {
            log::error!("Failed to parse IP '{}': {}, using localhost", ip_str, e);
            "127.0.0.1".parse().expect("Localhost IP should be valid")
        })
    }
}