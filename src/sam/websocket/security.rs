use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::net::IpAddr;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use log::{warn, error, info, debug};
use chrono::{DateTime, Utc};
use regex::Regex;
use once_cell::sync::Lazy;

const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64KB
const MAX_MESSAGES_PER_MINUTE: u32 = 100;
const MAX_CONNECTIONS_PER_IP: usize = 5;
const SESSION_TIMEOUT_SECONDS: u64 = 3600; // 1 hour
const IDLE_TIMEOUT_SECONDS: u64 = 300; // 5 minutes
const MESSAGE_QUEUE_SIZE: usize = 1000;

// Pattern validation for messages
static INJECTION_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(<script|javascript:|onerror=|onload=|onclick=|\.\./|%2e%2e|%252e)").unwrap()
});

/// WebSocket security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebSocketSecurityConfig {
    pub max_message_size: usize,
    pub max_messages_per_minute: u32,
    pub max_connections_per_ip: usize,
    pub session_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub enable_message_validation: bool,
    pub enable_rate_limiting: bool,
    pub enable_connection_limits: bool,
    pub enable_session_validation: bool,
    pub message_queue_size: usize,
}

impl Default for WebSocketSecurityConfig {
    fn default() -> Self {
        WebSocketSecurityConfig {
            max_message_size: MAX_MESSAGE_SIZE,
            max_messages_per_minute: MAX_MESSAGES_PER_MINUTE,
            max_connections_per_ip: MAX_CONNECTIONS_PER_IP,
            session_timeout_seconds: SESSION_TIMEOUT_SECONDS,
            idle_timeout_seconds: IDLE_TIMEOUT_SECONDS,
            enable_message_validation: true,
            enable_rate_limiting: true,
            enable_connection_limits: true,
            enable_session_validation: true,
            message_queue_size: MESSAGE_QUEUE_SIZE,
        }
    }
}

/// WebSocket error types
#[derive(Debug, Clone, Serialize)]
pub enum WsSecurityError {
    MessageTooLarge { size: usize, max_size: usize },
    RateLimitExceeded { limit: u32, window: Duration },
    TooManyConnections { ip: String, limit: usize },
    SessionExpired,
    SessionInvalid,
    MessageValidationFailed(String),
    InjectionAttempt(String),
    UnauthorizedAction(String),
    ConnectionIdle,
    QueueFull,
}

impl std::fmt::Display for WsSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsSecurityError::MessageTooLarge { size, max_size } => 
                write!(f, "Message size {} exceeds maximum {}", size, max_size),
            WsSecurityError::RateLimitExceeded { limit, window } => 
                write!(f, "Rate limit exceeded: {} messages per {:?}", limit, window),
            WsSecurityError::TooManyConnections { ip, limit } => 
                write!(f, "Too many connections from {}: limit {}", ip, limit),
            WsSecurityError::SessionExpired => write!(f, "Session has expired"),
            WsSecurityError::SessionInvalid => write!(f, "Invalid session"),
            WsSecurityError::MessageValidationFailed(msg) => write!(f, "Message validation failed: {}", msg),
            WsSecurityError::InjectionAttempt(msg) => write!(f, "Injection attempt detected: {}", msg),
            WsSecurityError::UnauthorizedAction(msg) => write!(f, "Unauthorized action: {}", msg),
            WsSecurityError::ConnectionIdle => write!(f, "Connection idle timeout"),
            WsSecurityError::QueueFull => write!(f, "Message queue is full"),
        }
    }
}

impl std::error::Error for WsSecurityError {}

/// Rate limiter for WebSocket connections
#[derive(Debug)]
pub struct WsRateLimiter {
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
    config: WebSocketSecurityConfig,
}

#[derive(Debug, Clone)]
struct RateLimitBucket {
    count: u32,
    window_start: Instant,
    violations: u32,
    last_violation: Option<Instant>,
}

impl WsRateLimiter {
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        WsRateLimiter {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), WsSecurityError> {
        if !self.config.enable_rate_limiting {
            return Ok(());
        }

        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        let bucket = buckets.entry(client_id.to_string()).or_insert_with(|| {
            RateLimitBucket {
                count: 0,
                window_start: now,
                violations: 0,
                last_violation: None,
            }
        });

        // Reset window if expired
        if now.duration_since(bucket.window_start) >= window {
            bucket.count = 0;
            bucket.window_start = now;
        }

        // Apply exponential backoff for repeat violations
        if let Some(last_violation) = bucket.last_violation {
            let backoff_duration = Duration::from_secs(2u64.pow(bucket.violations.min(5)));
            if now.duration_since(last_violation) < backoff_duration {
                return Err(WsSecurityError::RateLimitExceeded {
                    limit: self.config.max_messages_per_minute,
                    window,
                });
            }
        }

        bucket.count += 1;

        if bucket.count > self.config.max_messages_per_minute {
            bucket.violations += 1;
            bucket.last_violation = Some(now);
            
            warn!("Rate limit exceeded for client {}: {} violations", client_id, bucket.violations);
            
            Err(WsSecurityError::RateLimitExceeded {
                limit: self.config.max_messages_per_minute,
                window,
            })
        } else {
            Ok(())
        }
    }

    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        
        buckets.retain(|id, bucket| {
            let age = now.duration_since(bucket.window_start);
            if age > Duration::from_secs(3600) {
                debug!("Removing old rate limit bucket for {}", id);
                false
            } else {
                true
            }
        });
    }
}

/// Connection tracker for IP-based limits
#[derive(Debug)]
pub struct ConnectionTracker {
    connections: Arc<RwLock<HashMap<IpAddr, Vec<ConnectionInfo>>>>,
    config: WebSocketSecurityConfig,
}

#[derive(Debug, Clone)]
struct ConnectionInfo {
    client_id: String,
    connected_at: Instant,
    last_activity: Instant,
}

impl ConnectionTracker {
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        ConnectionTracker {
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn add_connection(&self, ip: IpAddr, client_id: String) -> Result<(), WsSecurityError> {
        if !self.config.enable_connection_limits {
            return Ok(());
        }

        let mut connections = self.connections.write().await;
        let now = Instant::now();

        let ip_connections = connections.entry(ip).or_insert_with(Vec::new);
        
        // Clean up old connections
        ip_connections.retain(|conn| {
            now.duration_since(conn.connected_at) < Duration::from_secs(self.config.session_timeout_seconds)
        });

        if ip_connections.len() >= self.config.max_connections_per_ip {
            error!("Too many connections from IP {}: {} connections", ip, ip_connections.len());
            return Err(WsSecurityError::TooManyConnections {
                ip: ip.to_string(),
                limit: self.config.max_connections_per_ip,
            });
        }

        ip_connections.push(ConnectionInfo {
            client_id,
            connected_at: now,
            last_activity: now,
        });

        info!("New connection from IP {}: total connections {}", ip, ip_connections.len());
        Ok(())
    }

    pub async fn remove_connection(&self, ip: IpAddr, client_id: &str) {
        let mut connections = self.connections.write().await;
        
        if let Some(ip_connections) = connections.get_mut(&ip) {
            ip_connections.retain(|conn| conn.client_id != client_id);
            
            if ip_connections.is_empty() {
                connections.remove(&ip);
            }
            
            info!("Removed connection {} from IP {}", client_id, ip);
        }
    }

    pub async fn update_activity(&self, ip: IpAddr, client_id: &str) {
        let mut connections = self.connections.write().await;
        
        if let Some(ip_connections) = connections.get_mut(&ip) {
            for conn in ip_connections.iter_mut() {
                if conn.client_id == client_id {
                    conn.last_activity = Instant::now();
                    break;
                }
            }
        }
    }

    pub async fn check_idle_connections(&self) -> Vec<(IpAddr, String)> {
        let mut idle_connections = Vec::new();
        let connections = self.connections.read().await;
        let now = Instant::now();

        for (ip, ip_connections) in connections.iter() {
            for conn in ip_connections {
                if now.duration_since(conn.last_activity) > Duration::from_secs(self.config.idle_timeout_seconds) {
                    idle_connections.push((*ip, conn.client_id.clone()));
                }
            }
        }

        idle_connections
    }
}

/// Message validator for WebSocket messages
pub struct MessageValidator {
    config: WebSocketSecurityConfig,
}

impl MessageValidator {
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        MessageValidator { config }
    }

    pub fn validate_message(&self, msg: &str) -> Result<(), WsSecurityError> {
        if !self.config.enable_message_validation {
            return Ok(());
        }

        // Check message size
        if msg.len() > self.config.max_message_size {
            return Err(WsSecurityError::MessageTooLarge {
                size: msg.len(),
                max_size: self.config.max_message_size,
            });
        }

        // Check for injection attempts
        if INJECTION_PATTERNS.is_match(msg) {
            warn!("Potential injection attempt detected in message");
            return Err(WsSecurityError::InjectionAttempt(
                "Suspicious patterns detected in message".to_string()
            ));
        }

        // Validate JSON structure
        if let Err(e) = serde_json::from_str::<serde_json::Value>(msg) {
            return Err(WsSecurityError::MessageValidationFailed(
                format!("Invalid JSON: {}", e)
            ));
        }

        Ok(())
    }

    pub fn validate_command(&self, command: &str, user_permissions: &[String]) -> Result<(), WsSecurityError> {
        // Define command permissions
        let restricted_commands = vec!["restart_service", "stop_service", "modify_config"];
        
        if restricted_commands.contains(&command) {
            if !user_permissions.contains(&format!("command:{}", command)) {
                return Err(WsSecurityError::UnauthorizedAction(
                    format!("Insufficient permissions for command: {}", command)
                ));
            }
        }

        Ok(())
    }
}

/// Session manager for WebSocket connections
#[derive(Debug)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    config: WebSocketSecurityConfig,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub client_id: String,
    pub user_id: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_authenticated: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl SessionManager {
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        SessionManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn create_session(&self, client_id: String, user_id: Option<String>) -> SessionInfo {
        let now = Utc::now();
        let session = SessionInfo {
            client_id: client_id.clone(),
            user_id,
            permissions: vec!["read".to_string()], // Default permissions
            created_at: now,
            last_authenticated: now,
            last_activity: now,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(client_id, session.clone());

        info!("Created new session for client {}", session.client_id);
        session
    }

    pub async fn validate_session(&self, client_id: &str) -> Result<SessionInfo, WsSecurityError> {
        if !self.config.enable_session_validation {
            return Ok(SessionInfo {
                client_id: client_id.to_string(),
                user_id: None,
                permissions: vec!["read".to_string()],
                created_at: Utc::now(),
                last_authenticated: Utc::now(),
                last_activity: Utc::now(),
            });
        }

        let sessions = self.sessions.read().await;
        
        match sessions.get(client_id) {
            Some(session) => {
                let now = Utc::now();
                let session_age = now.signed_duration_since(session.last_authenticated);
                
                if session_age.num_seconds() > self.config.session_timeout_seconds as i64 {
                    warn!("Session expired for client {}", client_id);
                    return Err(WsSecurityError::SessionExpired);
                }
                
                Ok(session.clone())
            }
            None => {
                warn!("No session found for client {}", client_id);
                Err(WsSecurityError::SessionInvalid)
            }
        }
    }

    pub async fn update_activity(&self, client_id: &str) {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(client_id) {
            session.last_activity = Utc::now();
        }
    }

    pub async fn reauthenticate(&self, client_id: &str, token: &str) -> Result<(), WsSecurityError> {
        // Validate token (implement actual token validation)
        if !self.validate_token(token) {
            return Err(WsSecurityError::SessionInvalid);
        }

        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(client_id) {
            session.last_authenticated = Utc::now();
            info!("Reauthenticated session for client {}", client_id);
            Ok(())
        } else {
            Err(WsSecurityError::SessionInvalid)
        }
    }

    pub async fn remove_session(&self, client_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(client_id);
        info!("Removed session for client {}", client_id);
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        
        sessions.retain(|id, session| {
            let age = now.signed_duration_since(session.last_activity);
            if age.num_seconds() > self.config.session_timeout_seconds as i64 {
                info!("Removing expired session for {}", id);
                false
            } else {
                true
            }
        });
    }

    fn validate_token(&self, _token: &str) -> bool {
        // TODO: Implement actual token validation
        // This should verify JWT or session token
        true
    }
}

/// Message queue with backpressure
#[derive(Debug)]
pub struct MessageQueue {
    queues: Arc<RwLock<HashMap<String, Vec<QueuedMessage>>>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
struct QueuedMessage {
    content: String,
    timestamp: Instant,
    priority: MessagePriority,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

impl MessageQueue {
    pub fn new(max_size: usize) -> Self {
        MessageQueue {
            queues: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    pub async fn enqueue(&self, client_id: &str, message: String, priority: MessagePriority) -> Result<(), WsSecurityError> {
        let mut queues = self.queues.write().await;
        let queue = queues.entry(client_id.to_string()).or_insert_with(Vec::new);

        if queue.len() >= self.max_size {
            // Apply backpressure
            warn!("Message queue full for client {}, applying backpressure", client_id);
            return Err(WsSecurityError::QueueFull);
        }

        queue.push(QueuedMessage {
            content: message,
            timestamp: Instant::now(),
            priority,
        });

        // Sort by priority
        queue.sort_by_key(|m| std::cmp::Reverse(m.priority.clone()));

        Ok(())
    }

    pub async fn dequeue(&self, client_id: &str) -> Option<String> {
        let mut queues = self.queues.write().await;
        
        if let Some(queue) = queues.get_mut(client_id) {
            if !queue.is_empty() {
                return Some(queue.remove(0).content);
            }
        }
        
        None
    }

    pub async fn get_queue_size(&self, client_id: &str) -> usize {
        let queues = self.queues.read().await;
        queues.get(client_id).map(|q| q.len()).unwrap_or(0)
    }

    pub async fn clear_queue(&self, client_id: &str) {
        let mut queues = self.queues.write().await;
        queues.remove(client_id);
    }
}

/// Combined WebSocket security limits
pub struct WebSocketLimits {
    pub message_validator: MessageValidator,
    pub rate_limiter: WsRateLimiter,
    pub connection_tracker: ConnectionTracker,
    pub session_manager: SessionManager,
    pub message_queue: MessageQueue,
    config: WebSocketSecurityConfig,
}

impl WebSocketLimits {
    pub fn new(config: WebSocketSecurityConfig) -> Self {
        let message_queue_size = config.message_queue_size;
        
        WebSocketLimits {
            message_validator: MessageValidator::new(config.clone()),
            rate_limiter: WsRateLimiter::new(config.clone()),
            connection_tracker: ConnectionTracker::new(config.clone()),
            session_manager: SessionManager::new(config.clone()),
            message_queue: MessageQueue::new(message_queue_size),
            config,
        }
    }

    pub async fn validate_connection(&self, ip: IpAddr, client_id: String) -> Result<SessionInfo, WsSecurityError> {
        // Check connection limits
        self.connection_tracker.add_connection(ip, client_id.clone()).await?;
        
        // Create session
        let session = self.session_manager.create_session(client_id, None).await;
        
        Ok(session)
    }

    pub async fn validate_message(&self, client_id: &str, message: &str) -> Result<(), WsSecurityError> {
        // Rate limiting
        self.rate_limiter.check_rate_limit(client_id).await?;
        
        // Message validation
        self.message_validator.validate_message(message)?;
        
        // Session validation
        self.session_manager.validate_session(client_id).await?;
        
        // Update activity
        self.session_manager.update_activity(client_id).await;
        
        Ok(())
    }

    pub async fn cleanup(&self) {
        // Run periodic cleanup tasks
        self.rate_limiter.cleanup_old_buckets().await;
        self.session_manager.cleanup_expired_sessions().await;
        
        // Check for idle connections
        let idle_connections = self.connection_tracker.check_idle_connections().await;
        for (ip, client_id) in idle_connections {
            self.connection_tracker.remove_connection(ip, &client_id).await;
            self.session_manager.remove_session(&client_id).await;
            self.message_queue.clear_queue(&client_id).await;
            info!("Cleaned up idle connection: {}", client_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = WebSocketSecurityConfig {
            max_messages_per_minute: 3,
            ..Default::default()
        };
        
        let limiter = WsRateLimiter::new(config);
        
        // First 3 messages should pass
        for _ in 0..3 {
            assert!(limiter.check_rate_limit("test_client").await.is_ok());
        }
        
        // 4th message should fail
        assert!(limiter.check_rate_limit("test_client").await.is_err());
    }

    #[tokio::test]
    async fn test_message_validation() {
        let config = WebSocketSecurityConfig::default();
        let validator = MessageValidator::new(config);
        
        // Valid message
        let valid_msg = r#"{"type": "ping", "timestamp": 123456}"#;
        assert!(validator.validate_message(valid_msg).is_ok());
        
        // Too large message
        let large_msg = "x".repeat(MAX_MESSAGE_SIZE + 1);
        assert!(validator.validate_message(&large_msg).is_err());
        
        // Injection attempt
        let injection_msg = r#"{"script": "<script>alert('xss')</script>"}"#;
        assert!(validator.validate_message(injection_msg).is_err());
    }

    #[tokio::test]
    async fn test_connection_tracker() {
        let config = WebSocketSecurityConfig {
            max_connections_per_ip: 2,
            ..Default::default()
        };
        
        let tracker = ConnectionTracker::new(config);
        let ip = "127.0.0.1".parse().unwrap();
        
        // First 2 connections should succeed
        assert!(tracker.add_connection(ip, "client1".to_string()).await.is_ok());
        assert!(tracker.add_connection(ip, "client2".to_string()).await.is_ok());
        
        // 3rd connection should fail
        assert!(tracker.add_connection(ip, "client3".to_string()).await.is_err());
        
        // Remove one connection
        tracker.remove_connection(ip, "client1").await;
        
        // Now a new connection should succeed
        assert!(tracker.add_connection(ip, "client3".to_string()).await.is_ok());
    }

    #[tokio::test]
    async fn test_message_queue() {
        let queue = MessageQueue::new(3);
        
        // Enqueue messages
        assert!(queue.enqueue("client1", "msg1".to_string(), MessagePriority::Normal).await.is_ok());
        assert!(queue.enqueue("client1", "msg2".to_string(), MessagePriority::High).await.is_ok());
        assert!(queue.enqueue("client1", "msg3".to_string(), MessagePriority::Low).await.is_ok());
        
        // Queue should be full
        assert!(queue.enqueue("client1", "msg4".to_string(), MessagePriority::Normal).await.is_err());
        
        // Dequeue should return high priority first
        assert_eq!(queue.dequeue("client1").await, Some("msg2".to_string()));
        
        // Now we can enqueue again
        assert!(queue.enqueue("client1", "msg4".to_string(), MessagePriority::Normal).await.is_ok());
    }
}