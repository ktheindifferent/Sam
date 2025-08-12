#[cfg(test)]
mod websocket_security_tests {
    use sam::websocket::security::*;
    use std::net::IpAddr;
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let config = WebSocketSecurityConfig {
            max_messages_per_minute: 5,
            ..Default::default()
        };
        
        let limiter = WsRateLimiter::new(config);
        
        // First 5 messages should pass
        for i in 1..=5 {
            assert!(
                limiter.check_rate_limit("test_client").await.is_ok(),
                "Message {} should pass", i
            );
        }
        
        // 6th message should fail
        assert!(
            limiter.check_rate_limit("test_client").await.is_err(),
            "6th message should be rate limited"
        );
    }
    
    #[tokio::test]
    async fn test_message_validation() {
        let config = WebSocketSecurityConfig::default();
        let validator = MessageValidator::new(config);
        
        // Valid JSON message
        let valid_msg = r#"{"type": "ping", "timestamp": 1234567890}"#;
        assert!(validator.validate_message(valid_msg).is_ok());
        
        // Message too large
        let large_msg = "x".repeat(65 * 1024); // 65KB
        assert!(matches!(
            validator.validate_message(&large_msg),
            Err(WsSecurityError::MessageTooLarge { .. })
        ));
        
        // Injection attempt
        let injection_msg = r#"{"data": "<script>alert('xss')</script>"}"#;
        assert!(matches!(
            validator.validate_message(injection_msg),
            Err(WsSecurityError::InjectionAttempt(_))
        ));
        
        // Invalid JSON
        let invalid_json = "not json";
        assert!(matches!(
            validator.validate_message(invalid_json),
            Err(WsSecurityError::MessageValidationFailed(_))
        ));
    }
    
    #[tokio::test]
    async fn test_connection_tracking() {
        let config = WebSocketSecurityConfig {
            max_connections_per_ip: 3,
            ..Default::default()
        };
        
        let tracker = ConnectionTracker::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        
        // First 3 connections should succeed
        assert!(tracker.add_connection(ip, "client1".to_string()).await.is_ok());
        assert!(tracker.add_connection(ip, "client2".to_string()).await.is_ok());
        assert!(tracker.add_connection(ip, "client3".to_string()).await.is_ok());
        
        // 4th connection should fail
        assert!(matches!(
            tracker.add_connection(ip, "client4".to_string()).await,
            Err(WsSecurityError::TooManyConnections { .. })
        ));
        
        // Remove a connection
        tracker.remove_connection(ip, "client1").await;
        
        // Now a new connection should succeed
        assert!(tracker.add_connection(ip, "client4".to_string()).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_session_management() {
        let config = WebSocketSecurityConfig::default();
        let manager = SessionManager::new(config);
        
        // Create a session
        let session = manager.create_session("client1".to_string(), Some("user1".to_string())).await;
        assert_eq!(session.client_id, "client1");
        assert_eq!(session.user_id, Some("user1".to_string()));
        
        // Validate session
        let validated = manager.validate_session("client1").await;
        assert!(validated.is_ok());
        
        // Remove session
        manager.remove_session("client1").await;
        
        // Validation should fail now
        let validation_result = manager.validate_session("client1").await;
        assert!(matches!(validation_result, Err(WsSecurityError::SessionInvalid)));
    }
    
    #[tokio::test]
    async fn test_message_queue_with_backpressure() {
        let queue = MessageQueue::new(3);
        
        // Add messages with different priorities
        assert!(queue.enqueue("client1", "low_priority".to_string(), MessagePriority::Low).await.is_ok());
        assert!(queue.enqueue("client1", "high_priority".to_string(), MessagePriority::High).await.is_ok());
        assert!(queue.enqueue("client1", "normal_priority".to_string(), MessagePriority::Normal).await.is_ok());
        
        // Queue should be full
        assert!(matches!(
            queue.enqueue("client1", "overflow".to_string(), MessagePriority::Normal).await,
            Err(WsSecurityError::QueueFull)
        ));
        
        // High priority should be dequeued first
        assert_eq!(queue.dequeue("client1").await, Some("high_priority".to_string()));
        
        // Queue size should be 2
        assert_eq!(queue.get_queue_size("client1").await, 2);
        
        // Now we can enqueue again
        assert!(queue.enqueue("client1", "new_message".to_string(), MessagePriority::Critical).await.is_ok());
        
        // Critical priority should be dequeued first
        assert_eq!(queue.dequeue("client1").await, Some("new_message".to_string()));
    }
    
    #[tokio::test]
    async fn test_websocket_limits_integration() {
        let config = WebSocketSecurityConfig {
            max_messages_per_minute: 10,
            max_connections_per_ip: 2,
            ..Default::default()
        };
        
        let limits = WebSocketLimits::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        
        // Test connection validation
        let session1 = limits.validate_connection(ip, "client1".to_string()).await;
        assert!(session1.is_ok());
        
        let session2 = limits.validate_connection(ip, "client2".to_string()).await;
        assert!(session2.is_ok());
        
        // Third connection should fail
        let session3 = limits.validate_connection(ip, "client3".to_string()).await;
        assert!(session3.is_err());
        
        // Test message validation with rate limiting
        let valid_msg = r#"{"type": "test", "data": "hello"}"#;
        
        // Send 10 messages (should all pass)
        for _ in 0..10 {
            assert!(limits.validate_message("client1", valid_msg).await.is_ok());
        }
        
        // 11th message should be rate limited
        assert!(limits.validate_message("client1", valid_msg).await.is_err());
    }
    
    #[test]
    fn test_command_validation() {
        let config = WebSocketSecurityConfig::default();
        let validator = MessageValidator::new(config);
        
        // Test with sufficient permissions
        let permissions = vec!["command:restart_service".to_string()];
        assert!(validator.validate_command("restart_service", &permissions).is_ok());
        
        // Test with insufficient permissions
        let permissions = vec!["read".to_string()];
        assert!(matches!(
            validator.validate_command("restart_service", &permissions),
            Err(WsSecurityError::UnauthorizedAction(_))
        ));
        
        // Test non-restricted command
        let permissions = vec!["read".to_string()];
        assert!(validator.validate_command("get_stats", &permissions).is_ok());
    }
}