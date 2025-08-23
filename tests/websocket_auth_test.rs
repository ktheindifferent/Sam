use sam::websocket::security::*;
use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use jsonwebtoken::{encode, Algorithm, Header, EncodingKey};
use serde_json::json;

#[tokio::test]
async fn test_jwt_token_validation_success() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig::default();
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    
    // Generate a valid token
    let token = manager.generate_token("client123", "user456", vec!["read".to_string(), "write".to_string()])
        .expect("Should generate token");
    
    // Validate the token
    let result = manager.authenticate_with_token(&token).await;
    assert!(result.is_ok(), "Valid token should authenticate successfully");
    
    let session = result.unwrap();
    assert_eq!(session.client_id, "client123");
    assert_eq!(session.user_id, Some("user456".to_string()));
    assert_eq!(session.permissions, vec!["read".to_string(), "write".to_string()]);
}

#[tokio::test]
async fn test_jwt_token_expired() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig {
        secret: "test_secret_key".to_string(),
        issuer: "test_issuer".to_string(),
        audience: "test_audience".to_string(),
        token_lifetime_seconds: 1, // Very short lifetime
    };
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    
    // Create an expired token manually
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    let expired_claims = JwtClaims {
        sub: "user123".to_string(),
        exp: now - 3600, // Expired 1 hour ago
        iat: now - 7200, // Issued 2 hours ago
        nbf: Some(now - 7200),
        client_id: "client123".to_string(),
        permissions: vec!["read".to_string()],
        session_id: "session123".to_string(),
    };
    
    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret("test_secret_key".as_bytes());
    let expired_token = encode(&header, &expired_claims, &key).unwrap();
    
    // Try to authenticate with expired token
    let result = manager.authenticate_with_token(&expired_token).await;
    assert!(result.is_err(), "Expired token should fail authentication");
    
    match result.unwrap_err() {
        WsSecurityError::TokenExpired => (),
        other => panic!("Expected TokenExpired error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_jwt_token_invalid_signature() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig {
        secret: "correct_secret".to_string(),
        issuer: "test_issuer".to_string(),
        audience: "test_audience".to_string(),
        token_lifetime_seconds: 3600,
    };
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    
    // Create a token with wrong secret
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    let claims = JwtClaims {
        sub: "user123".to_string(),
        exp: now + 3600,
        iat: now,
        nbf: Some(now),
        client_id: "client123".to_string(),
        permissions: vec!["read".to_string()],
        session_id: "session123".to_string(),
    };
    
    let header = Header::new(Algorithm::HS256);
    let wrong_key = EncodingKey::from_secret("wrong_secret".as_bytes());
    let invalid_token = encode(&header, &claims, &wrong_key).unwrap();
    
    // Try to authenticate with invalid signature
    let result = manager.authenticate_with_token(&invalid_token).await;
    assert!(result.is_err(), "Token with invalid signature should fail");
    
    match result.unwrap_err() {
        WsSecurityError::InvalidToken(msg) if msg.contains("signature") => (),
        other => panic!("Expected InvalidToken error with signature message, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_jwt_token_malformed() {
    let config = WebSocketSecurityConfig::default();
    let manager = SessionManager::new(config);
    
    // Test various malformed tokens
    let malformed_tokens = vec![
        "not_a_jwt_token",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", // Missing payload and signature
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid_base64", // Invalid base64 in payload
        "", // Empty token
        ".....", // Just dots
    ];
    
    for token in malformed_tokens {
        let result = manager.authenticate_with_token(token).await;
        assert!(result.is_err(), "Malformed token '{}' should fail", token);
        
        match result.unwrap_err() {
            WsSecurityError::InvalidToken(_) => (),
            other => panic!("Expected InvalidToken error for '{}', got: {:?}", token, other),
        }
    }
}

#[tokio::test]
async fn test_jwt_token_wrong_issuer() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig {
        secret: "test_secret".to_string(),
        issuer: "correct_issuer".to_string(),
        audience: "test_audience".to_string(),
        token_lifetime_seconds: 3600,
    };
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    
    // Create a token with wrong issuer
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    let mut claims = json!({
        "sub": "user123",
        "exp": now + 3600,
        "iat": now,
        "nbf": now,
        "client_id": "client123",
        "permissions": ["read"],
        "session_id": "session123",
        "iss": "wrong_issuer", // Wrong issuer
        "aud": "test_audience"
    });
    
    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret("test_secret".as_bytes());
    let invalid_token = encode(&header, &claims, &key).unwrap();
    
    // Try to authenticate with wrong issuer
    let result = manager.authenticate_with_token(&invalid_token).await;
    assert!(result.is_err(), "Token with wrong issuer should fail");
}

#[tokio::test]
async fn test_websocket_connection_with_valid_token() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config.clone());
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Generate a valid token
    let token = limits.session_manager.generate_token("client123", "user456", vec!["read".to_string()])
        .expect("Should generate token");
    
    // Validate connection with token
    let result = limits.validate_connection(ip, "client123".to_string(), Some(&token)).await;
    assert!(result.is_ok(), "Connection with valid token should succeed");
    
    let session = result.unwrap();
    assert_eq!(session.user_id, Some("user456".to_string()));
    assert_eq!(session.permissions, vec!["read".to_string()]);
}

#[tokio::test]
async fn test_websocket_connection_without_token() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config.clone());
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Validate connection without token
    let result = limits.validate_connection(ip, "client123".to_string(), None).await;
    assert!(result.is_ok(), "Connection without token should create unauthenticated session");
    
    let session = result.unwrap();
    assert_eq!(session.user_id, None);
    assert_eq!(session.permissions, vec!["read".to_string()]); // Default permissions only
}

#[tokio::test]
async fn test_websocket_connection_with_invalid_token() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config.clone());
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    let invalid_token = "invalid.jwt.token";
    
    // Validate connection with invalid token
    let result = limits.validate_connection(ip, "client123".to_string(), Some(invalid_token)).await;
    assert!(result.is_err(), "Connection with invalid token should fail");
    
    match result.unwrap_err() {
        WsSecurityError::InvalidToken(_) => (),
        other => panic!("Expected InvalidToken error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_reauthentication_with_valid_token() {
    let config = WebSocketSecurityConfig::default();
    let manager = SessionManager::new(config);
    
    // Create initial session
    let session = manager.create_session("client123".to_string(), None).await;
    assert_eq!(session.user_id, None);
    
    // Generate a valid token
    let token = manager.generate_token("client123", "user456", vec!["read".to_string(), "write".to_string()])
        .expect("Should generate token");
    
    // Reauthenticate with token
    let result = manager.reauthenticate("client123", &token).await;
    assert!(result.is_ok(), "Reauthentication with valid token should succeed");
    
    // Validate session was updated
    let updated_session = manager.validate_session("client123").await.unwrap();
    assert_eq!(updated_session.user_id, Some("user456".to_string()));
    assert_eq!(updated_session.permissions, vec!["read".to_string(), "write".to_string()]);
}

#[tokio::test]
async fn test_reauthentication_client_id_mismatch() {
    let config = WebSocketSecurityConfig::default();
    let manager = SessionManager::new(config);
    
    // Generate a token for client123
    let token = manager.generate_token("client123", "user456", vec!["read".to_string()])
        .expect("Should generate token");
    
    // Try to reauthenticate with different client_id
    let result = manager.reauthenticate("different_client", &token).await;
    assert!(result.is_err(), "Reauthentication with mismatched client_id should fail");
    
    match result.unwrap_err() {
        WsSecurityError::InvalidToken(msg) if msg.contains("Client ID mismatch") => (),
        other => panic!("Expected InvalidToken with client ID mismatch, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_token_not_before_validation() {
    let config = WebSocketSecurityConfig::default();
    let jwt_config = JwtConfig {
        secret: "test_secret".to_string(),
        issuer: "test_issuer".to_string(),
        audience: "test_audience".to_string(),
        token_lifetime_seconds: 3600,
    };
    let manager = SessionManager::with_jwt_config(config, jwt_config);
    
    // Create a token that's not valid yet
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    let future_claims = JwtClaims {
        sub: "user123".to_string(),
        exp: now + 7200,
        iat: now,
        nbf: Some(now + 3600), // Not valid for another hour
        client_id: "client123".to_string(),
        permissions: vec!["read".to_string()],
        session_id: "session123".to_string(),
    };
    
    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret("test_secret".as_bytes());
    let future_token = encode(&header, &future_claims, &key).unwrap();
    
    // Try to authenticate with future token
    let result = manager.authenticate_with_token(&future_token).await;
    assert!(result.is_err(), "Token not yet valid should fail");
    
    match result.unwrap_err() {
        WsSecurityError::InvalidToken(msg) if msg.contains("not yet valid") => (),
        other => panic!("Expected InvalidToken with 'not yet valid', got: {:?}", other),
    }
}

#[tokio::test]
async fn test_concurrent_authentication_attempts() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Generate multiple valid tokens
    let mut handles = vec![];
    
    for i in 0..5 {
        let limits_clone = &limits;
        let token = limits_clone.session_manager
            .generate_token(&format!("client{}", i), &format!("user{}", i), vec!["read".to_string()])
            .expect("Should generate token");
        
        let handle = tokio::spawn(async move {
            limits_clone.validate_connection(ip, format!("client{}", i), Some(&token)).await
        });
        handles.push(handle);
    }
    
    // Wait for all authentication attempts
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent authentication should succeed");
    }
}

#[tokio::test]
async fn test_session_cleanup_after_failed_auth() {
    let config = WebSocketSecurityConfig::default();
    let limits = WebSocketLimits::new(config.clone());
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    
    // Try to connect with invalid token
    let invalid_token = "invalid.token";
    let result = limits.validate_connection(ip, "client123".to_string(), Some(invalid_token)).await;
    assert!(result.is_err());
    
    // Verify connection was removed from tracker
    let tracker_connections = limits.connection_tracker.connections.read().await;
    assert!(!tracker_connections.contains_key(&ip), "Failed auth should remove connection from tracker");
}

#[tokio::test]
async fn test_permission_validation_with_token() {
    let config = WebSocketSecurityConfig::default();
    let manager = SessionManager::new(config);
    let validator = MessageValidator::new(WebSocketSecurityConfig::default());
    
    // Generate token with specific permissions
    let token = manager.generate_token("client123", "user456", vec!["command:restart_service".to_string()])
        .expect("Should generate token");
    
    // Authenticate
    let session = manager.authenticate_with_token(&token).await.unwrap();
    
    // Validate command with permissions
    let result = validator.validate_command("restart_service", &session.permissions);
    assert!(result.is_ok(), "Command should be allowed with proper permission");
    
    // Try unauthorized command
    let result = validator.validate_command("stop_service", &session.permissions);
    assert!(result.is_err(), "Command should be denied without permission");
}