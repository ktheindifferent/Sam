use sam::websocket::security::*;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_full_authentication_flow() {
    // Setup
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "192.168.1.100".parse().unwrap();
    
    // Step 1: Initial connection without token (unauthenticated)
    let initial_session = limits.validate_connection(ip, "client001".to_string(), None).await
        .expect("Should allow initial connection");
    assert_eq!(initial_session.user_id, None, "Initial session should be unauthenticated");
    assert_eq!(initial_session.permissions, vec!["read"], "Should have minimal permissions");
    
    // Step 2: Attempt to perform restricted action (should fail)
    let validator = MessageValidator::new(WebSocketSecurityConfig::default());
    let restricted_result = validator.validate_command("restart_service", &initial_session.permissions);
    assert!(restricted_result.is_err(), "Restricted commands should fail without auth");
    
    // Step 3: Generate authentication token
    let auth_token = limits.session_manager
        .generate_token("client001", "user123", vec![
            "read".to_string(),
            "write".to_string(),
            "command:restart_service".to_string()
        ])
        .expect("Should generate auth token");
    
    // Step 4: Reauthenticate with token
    limits.session_manager.reauthenticate("client001", &auth_token).await
        .expect("Reauthentication should succeed");
    
    // Step 5: Validate session after authentication
    let auth_session = limits.session_manager.validate_session("client001").await
        .expect("Should have valid session");
    assert_eq!(auth_session.user_id, Some("user123".to_string()));
    assert!(auth_session.permissions.contains(&"command:restart_service".to_string()));
    
    // Step 6: Attempt restricted action again (should succeed)
    let restricted_result = validator.validate_command("restart_service", &auth_session.permissions);
    assert!(restricted_result.is_ok(), "Restricted commands should work after auth");
    
    // Step 7: Send messages and verify rate limiting
    for i in 0..3 {
        let message = format!(r#"{{"type": "message", "content": "Test message {}", "seq": {}}}"#, i, i);
        limits.validate_message("client001", &message).await
            .expect(&format!("Message {} should be accepted", i));
    }
    
    // Step 8: Cleanup
    limits.connection_tracker.remove_connection(ip, "client001").await;
    limits.session_manager.remove_session("client001").await;
}

#[tokio::test]
async fn test_multiple_clients_authentication() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    
    // Simulate multiple clients connecting
    let clients = vec![
        ("client_a", "10.0.0.1", "user_alice", vec!["read", "write"]),
        ("client_b", "10.0.0.2", "user_bob", vec!["read"]),
        ("client_c", "10.0.0.3", "user_charlie", vec!["read", "write", "admin"]),
    ];
    
    let mut tokens = vec![];
    
    // Generate tokens for all clients
    for (client_id, _, user_id, permissions) in &clients {
        let token = limits.session_manager
            .generate_token(client_id, user_id, permissions.iter().map(|s| s.to_string()).collect())
            .expect("Should generate token");
        tokens.push(token);
    }
    
    // Connect all clients
    for ((client_id, ip_str, _, _), token) in clients.iter().zip(tokens.iter()) {
        let ip: IpAddr = ip_str.parse().unwrap();
        let session = limits.validate_connection(ip, client_id.to_string(), Some(token)).await
            .expect("Connection should succeed");
        
        // Verify session details
        assert!(session.user_id.is_some());
    }
    
    // Verify all sessions are active
    for (client_id, _, _, _) in &clients {
        let session = limits.session_manager.validate_session(client_id).await
            .expect("Session should be valid");
        assert!(session.user_id.is_some());
    }
    
    // Cleanup
    for (client_id, ip_str, _, _) in &clients {
        let ip: IpAddr = ip_str.parse().unwrap();
        limits.connection_tracker.remove_connection(ip, client_id).await;
        limits.session_manager.remove_session(client_id).await;
    }
}

#[tokio::test]
async fn test_token_refresh_flow() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig {
        secret: "test_secret".to_string(),
        issuer: "test".to_string(),
        audience: "test".to_string(),
        token_lifetime_seconds: 2, // Very short for testing
    };
    let limits = WebSocketLimits::new(config.clone());
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Initial connection with short-lived token
    let initial_token = manager.generate_token("client123", "user456", vec!["read".to_string()])
        .expect("Should generate token");
    
    // Connect with initial token
    let session = manager.authenticate_with_token(&initial_token).await
        .expect("Initial authentication should succeed");
    assert_eq!(session.user_id, Some("user456".to_string()));
    
    // Wait for token to expire
    sleep(Duration::from_secs(3)).await;
    
    // Try to use expired token (should fail)
    let expired_result = manager.authenticate_with_token(&initial_token).await;
    assert!(expired_result.is_err(), "Expired token should fail");
    
    // Generate refresh token
    let refresh_token = manager.generate_token("client123", "user456", vec!["read".to_string()])
        .expect("Should generate refresh token");
    
    // Reauthenticate with new token
    let refreshed_session = manager.authenticate_with_token(&refresh_token).await
        .expect("Refresh authentication should succeed");
    assert_eq!(refreshed_session.user_id, Some("user456".to_string()));
}

#[tokio::test]
async fn test_connection_limits_with_authentication() {
    let mut config = WebSocketSecurityConfig::default();
    config.max_connections_per_ip = 2;
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "192.168.1.50".parse().unwrap();
    
    // Generate tokens for multiple clients from same IP
    let token1 = limits.session_manager.generate_token("client1", "user1", vec!["read".to_string()])
        .expect("Should generate token");
    let token2 = limits.session_manager.generate_token("client2", "user1", vec!["read".to_string()])
        .expect("Should generate token");
    let token3 = limits.session_manager.generate_token("client3", "user1", vec!["read".to_string()])
        .expect("Should generate token");
    
    // First two connections should succeed
    let conn1 = limits.validate_connection(ip, "client1".to_string(), Some(&token1)).await;
    assert!(conn1.is_ok(), "First connection should succeed");
    
    let conn2 = limits.validate_connection(ip, "client2".to_string(), Some(&token2)).await;
    assert!(conn2.is_ok(), "Second connection should succeed");
    
    // Third connection should fail due to connection limit
    let conn3 = limits.validate_connection(ip, "client3".to_string(), Some(&token3)).await;
    assert!(conn3.is_err(), "Third connection should fail due to limit");
    
    match conn3.unwrap_err() {
        WsSecurityError::TooManyConnections { .. } => (),
        other => panic!("Expected TooManyConnections error, got: {:?}", other),
    }
    
    // Remove one connection
    limits.connection_tracker.remove_connection(ip, "client1").await;
    
    // Now third connection should succeed
    let conn3_retry = limits.validate_connection(ip, "client3".to_string(), Some(&token3)).await;
    assert!(conn3_retry.is_ok(), "Third connection should succeed after removing one");
    
    // Cleanup
    limits.connection_tracker.remove_connection(ip, "client2").await;
    limits.connection_tracker.remove_connection(ip, "client3").await;
}

#[tokio::test]
async fn test_message_validation_with_authentication() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Create authenticated session
    let token = limits.session_manager
        .generate_token("client123", "user456", vec!["read".to_string(), "write".to_string()])
        .expect("Should generate token");
    
    limits.validate_connection(ip, "client123".to_string(), Some(&token)).await
        .expect("Connection should succeed");
    
    // Test valid message
    let valid_message = r#"{"type": "chat", "content": "Hello, World!"}"#;
    let result = limits.validate_message("client123", valid_message).await;
    assert!(result.is_ok(), "Valid message should pass validation");
    
    // Test injection attempt
    let injection_message = r#"{"type": "chat", "content": "<script>alert('xss')</script>"}"#;
    let result = limits.validate_message("client123", injection_message).await;
    assert!(result.is_err(), "Injection attempt should be blocked");
    
    // Test oversized message
    let large_message = format!(r#"{{"type": "data", "content": "{}"}}"#, "x".repeat(MAX_MESSAGE_SIZE + 1));
    let result = limits.validate_message("client123", &large_message).await;
    assert!(result.is_err(), "Oversized message should be rejected");
    
    // Cleanup
    limits.connection_tracker.remove_connection(ip, "client123").await;
    limits.session_manager.remove_session("client123").await;
}

#[tokio::test]
async fn test_idle_connection_cleanup() {
    let mut config = WebSocketSecurityConfig::default();
    config.idle_timeout_seconds = 1; // Very short for testing
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Create authenticated connection
    let token = limits.session_manager
        .generate_token("idle_client", "user123", vec!["read".to_string()])
        .expect("Should generate token");
    
    limits.validate_connection(ip, "idle_client".to_string(), Some(&token)).await
        .expect("Connection should succeed");
    
    // Wait for idle timeout
    sleep(Duration::from_secs(2)).await;
    
    // Run cleanup
    limits.cleanup().await;
    
    // Verify session was removed
    let session_result = limits.session_manager.validate_session("idle_client").await;
    assert!(session_result.is_err(), "Idle session should be removed");
}

#[tokio::test]
async fn test_concurrent_message_validation() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Create authenticated session
    let token = limits.session_manager
        .generate_token("concurrent_client", "user789", vec!["read".to_string()])
        .expect("Should generate token");
    
    limits.validate_connection(ip, "concurrent_client".to_string(), Some(&token)).await
        .expect("Connection should succeed");
    
    // Send multiple messages concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let limits_ref = &limits;
        let message = format!(r#"{{"type": "ping", "seq": {}}}"#, i);
        
        let handle = tokio::spawn(async move {
            limits_ref.validate_message("concurrent_client", &message).await
        });
        handles.push(handle);
    }
    
    // Wait for all validations
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }
    
    // At least some messages should succeed (rate limiting may block some)
    assert!(success_count > 0, "Some messages should pass validation");
    
    // Cleanup
    limits.connection_tracker.remove_connection(ip, "concurrent_client").await;
    limits.session_manager.remove_session("concurrent_client").await;
}

#[tokio::test]
async fn test_security_bypass_prevention() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Attempt 1: Try to bypass with empty token
    let empty_result = limits.validate_connection(ip, "bypass1".to_string(), Some("")).await;
    assert!(empty_result.is_err(), "Empty token should not bypass authentication");
    
    // Attempt 2: Try to bypass with whitespace token
    let whitespace_result = limits.validate_connection(ip, "bypass2".to_string(), Some("   ")).await;
    assert!(whitespace_result.is_err(), "Whitespace token should not bypass authentication");
    
    // Attempt 3: Try to reuse token for different client
    let token = limits.session_manager
        .generate_token("original_client", "user123", vec!["read".to_string()])
        .expect("Should generate token");
    
    // First use should succeed
    limits.validate_connection(ip, "original_client".to_string(), Some(&token)).await
        .expect("Original client should connect");
    
    // Try to use same token for different client (through reauthentication)
    let reuse_result = limits.session_manager.reauthenticate("different_client", &token).await;
    assert!(reuse_result.is_err(), "Token reuse for different client should fail");
    
    // Cleanup
    limits.connection_tracker.remove_connection(ip, "original_client").await;
}