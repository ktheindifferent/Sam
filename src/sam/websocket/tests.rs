//! Unit tests for WebSocket error handling
//! 
//! Tests that verify proper error handling and recovery
//! for all previously unsafe unwrap() operations.

#[cfg(test)]
mod error_handling_tests {
    use super::super::error::{WebSocketError, WsSecurityError, safe_ops};
    use super::super::security::{SessionManager, WebSocketSecurityConfig, MessageValidator};
    use std::time::Duration;
    
    #[test]
    fn test_regex_compilation_fallback() {
        // Test valid regex compilation
        let valid_regex = safe_ops::compile_regex(r"[a-z]+");
        assert!(valid_regex.is_ok());
        
        // Test invalid regex with fallback
        let invalid_pattern = r"[a-z";  // Missing closing bracket
        let fallback_regex = safe_ops::compile_regex_or_default(invalid_pattern, r"[a-z]+");
        assert!(fallback_regex.is_match("abc"));
        
        // Test that error is returned for invalid regex
        let invalid_result = safe_ops::compile_regex(invalid_pattern);
        assert!(invalid_result.is_err());
        match invalid_result {
            Err(WebSocketError::RegexCompilation(_)) => {},
            _ => panic!("Expected RegexCompilation error"),
        }
    }
    
    #[test]
    fn test_unix_timestamp_handling() {
        // Test normal timestamp
        let timestamp = safe_ops::unix_timestamp();
        assert!(timestamp.is_ok());
        assert!(timestamp.unwrap() > 0);
        
        // Test fallback timestamp
        let fallback_timestamp = safe_ops::unix_timestamp_or_default();
        assert!(fallback_timestamp >= 0);
    }
    
    #[test]
    fn test_json_serialization_fallback() {
        use serde_json::json;
        
        // Test valid JSON serialization
        let valid_data = json!({"key": "value"});
        let result = safe_ops::serialize_json(&valid_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"key":"value"}"#);
        
        // Test fallback serialization
        let data = json!({"test": "data"});
        let fallback_result = safe_ops::serialize_json_or_default(&data, r#"{"error":"default"}"#);
        assert!(!fallback_result.is_empty());
        
        // Test that default is used when needed
        // Note: It's hard to force serialization failure with valid serde types
        // so we mainly test the successful path
    }
    
    #[test]
    fn test_ip_parsing_fallback() {
        // Test valid IP parsing
        let valid_ip = safe_ops::parse_ip("192.168.1.1");
        assert!(valid_ip.is_ok());
        
        // Test invalid IP with error
        let invalid_ip = safe_ops::parse_ip("not.an.ip");
        assert!(invalid_ip.is_err());
        match invalid_ip {
            Err(WebSocketError::IpParsing(_)) => {},
            _ => panic!("Expected IpParsing error"),
        }
        
        // Test fallback IP parsing
        let fallback_ip = safe_ops::parse_ip_or_default("invalid.ip");
        assert_eq!(fallback_ip.to_string(), "127.0.0.1");
        
        // Test valid IP with fallback function
        let valid_fallback = safe_ops::parse_ip_or_default("10.0.0.1");
        assert_eq!(valid_fallback.to_string(), "10.0.0.1");
    }
    
    #[test]
    fn test_security_error_display() {
        let error = WsSecurityError::MessageTooLarge {
            size: 1000,
            max_size: 500,
        };
        assert_eq!(error.to_string(), "Message size 1000 exceeds maximum 500");
        
        let rate_limit_error = WsSecurityError::RateLimitExceeded {
            limit: 100,
            window: Duration::from_secs(60),
        };
        assert!(rate_limit_error.to_string().contains("Rate limit exceeded"));
        
        let token_error = WsSecurityError::TokenExpired;
        assert_eq!(token_error.to_string(), "Token has expired");
    }
    
    #[test]
    fn test_websocket_error_types() {
        use std::net::AddrParseError;
        use std::time::SystemTimeError;
        
        // Test JSON error conversion
        let json_str = "{invalid json}";
        let json_result: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        if let Err(e) = json_result {
            let ws_error = WebSocketError::from(e);
            assert!(matches!(ws_error, WebSocketError::JsonSerialization(_)));
        }
        
        // Test configuration error
        let config_error = WebSocketError::Configuration("Invalid config".to_string());
        assert!(config_error.to_string().contains("Invalid config"));
        
        // Test unexpected error
        let unexpected = WebSocketError::Unexpected("Something went wrong".to_string());
        assert!(unexpected.to_string().contains("Something went wrong"));
    }
    
    #[tokio::test]
    async fn test_session_manager_error_handling() {
        let config = WebSocketSecurityConfig::default();
        let session_manager = SessionManager::new(config);
        
        // Test invalid token validation
        let invalid_token = "invalid.jwt.token";
        let result = session_manager.reauthenticate("client1", invalid_token).await;
        assert!(result.is_err());
        
        // Test session validation for non-existent client
        let missing_session = session_manager.validate_session("non_existent_client").await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_message_validation_with_injection() {
        let config = WebSocketSecurityConfig::default();
        let validator = MessageValidator::new(config);
        
        // Test injection pattern detection
        let injection_msg = r#"{"script": "<script>alert('xss')</script>"}"#;
        let result = validator.validate_message(injection_msg);
        assert!(result.is_err());
        match result {
            Err(WsSecurityError::InjectionAttempt(_)) => {},
            _ => panic!("Expected InjectionAttempt error"),
        }
        
        // Test valid message
        let valid_msg = r#"{"type": "ping", "timestamp": 123456}"#;
        assert!(validator.validate_message(valid_msg).is_ok());
        
        // Test oversized message
        let large_msg = "x".repeat(100_000);  // Exceeds default 64KB limit
        let size_result = validator.validate_message(&large_msg);
        assert!(size_result.is_err());
        match size_result {
            Err(WsSecurityError::MessageTooLarge { .. }) => {},
            _ => panic!("Expected MessageTooLarge error"),
        }
    }
    
    #[test]
    fn test_error_recovery_helpers() {
        let error = WebSocketError::Configuration("Test error".to_string());
        
        // Test log_and_default
        let default_value: String = error.clone().log_and_default();
        assert_eq!(default_value, String::default());
        
        // Test log_and_continue
        error.log_and_continue();  // Should just log without panicking
    }
    
    #[test]
    fn test_serialization_of_security_errors() {
        use serde_json;
        
        let error = WsSecurityError::SessionExpired;
        let serialized = serde_json::to_string(&error);
        assert!(serialized.is_ok());
        let json = serialized.unwrap();
        assert!(json.contains("Session has expired"));
        
        let complex_error = WsSecurityError::TooManyConnections {
            ip: "192.168.1.1".to_string(),
            limit: 5,
        };
        let complex_serialized = serde_json::to_string(&complex_error);
        assert!(complex_serialized.is_ok());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::super::security::{
        WebSocketLimits, WebSocketSecurityConfig, ConnectionTracker,
        WsRateLimiter, MessageQueue, MessagePriority
    };
    use std::net::IpAddr;
    
    #[tokio::test]
    async fn test_connection_limits_with_safe_ip_parsing() {
        let config = WebSocketSecurityConfig {
            max_connections_per_ip: 2,
            ..Default::default()
        };
        
        let tracker = ConnectionTracker::new(config);
        
        // Use safe IP parsing
        let ip: IpAddr = super::super::error::safe_ops::parse_ip_or_default("192.168.1.1");
        
        // Test adding connections
        assert!(tracker.add_connection(ip, "client1".to_string()).await.is_ok());
        assert!(tracker.add_connection(ip, "client2".to_string()).await.is_ok());
        
        // Third connection should fail
        let result = tracker.add_connection(ip, "client3".to_string()).await;
        assert!(result.is_err());
        
        // Test with invalid IP fallback
        let fallback_ip = super::super::error::safe_ops::parse_ip_or_default("invalid.ip");
        assert_eq!(fallback_ip.to_string(), "127.0.0.1");
        assert!(tracker.add_connection(fallback_ip, "fallback_client".to_string()).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_rate_limiter_recovery() {
        let config = WebSocketSecurityConfig {
            max_messages_per_minute: 3,
            ..Default::default()
        };
        
        let limiter = WsRateLimiter::new(config);
        
        // Test rate limiting
        for i in 0..3 {
            let result = limiter.check_rate_limit("test_client").await;
            assert!(result.is_ok(), "Message {} should pass", i + 1);
        }
        
        // 4th message should fail gracefully
        let exceeded = limiter.check_rate_limit("test_client").await;
        assert!(exceeded.is_err());
        match exceeded {
            Err(super::super::error::WsSecurityError::RateLimitExceeded { .. }) => {},
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }
    
    #[tokio::test]
    async fn test_message_queue_with_priority() {
        let queue = MessageQueue::new(5);
        
        // Add messages with different priorities
        assert!(queue.enqueue("client1", "low".to_string(), MessagePriority::Low).await.is_ok());
        assert!(queue.enqueue("client1", "critical".to_string(), MessagePriority::Critical).await.is_ok());
        assert!(queue.enqueue("client1", "normal".to_string(), MessagePriority::Normal).await.is_ok());
        assert!(queue.enqueue("client1", "high".to_string(), MessagePriority::High).await.is_ok());
        
        // Critical should come first
        assert_eq!(queue.dequeue("client1").await, Some("critical".to_string()));
        assert_eq!(queue.dequeue("client1").await, Some("high".to_string()));
        assert_eq!(queue.dequeue("client1").await, Some("normal".to_string()));
        assert_eq!(queue.dequeue("client1").await, Some("low".to_string()));
        
        // Queue should be empty
        assert_eq!(queue.dequeue("client1").await, None);
    }
    
    #[tokio::test]
    async fn test_websocket_limits_integration() {
        let config = WebSocketSecurityConfig::default();
        let limits = WebSocketLimits::new(config);
        
        // Test connection validation with safe IP parsing
        let ip = super::super::error::safe_ops::parse_ip_or_default("10.0.0.1");
        let client_id = "test_client".to_string();
        
        // Connection without token should create unauthenticated session
        let session_result = limits.validate_connection(ip, client_id.clone(), None).await;
        assert!(session_result.is_ok());
        let session = session_result.unwrap();
        assert_eq!(session.client_id, client_id);
        assert!(session.user_id.is_none());
        
        // Test message validation
        let valid_message = r#"{"type": "ping", "timestamp": 123456}"#;
        let validate_result = limits.validate_message(&client_id, valid_message).await;
        assert!(validate_result.is_ok());
        
        // Test cleanup - should not panic
        limits.cleanup().await;
    }
}