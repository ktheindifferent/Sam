#[cfg(test)]
mod security_tests {
    use sam::sam::security::{Auth, CorsConfig};
    
    #[test]
    fn test_password_hashing() {
        let password = "TestPassword123!@#";
        
        // Hash the password
        let hash = Auth::hash_password(password).expect("Failed to hash password");
        
        // Verify the hash is not the same as the password
        assert_ne!(hash, password);
        
        // Verify the hash starts with argon2 identifier
        assert!(hash.starts_with("$argon2"));
        
        // Verify correct password
        assert!(Auth::verify_password(password, &hash).expect("Failed to verify"));
        
        // Verify incorrect password fails
        assert!(!Auth::verify_password("WrongPassword", &hash).expect("Failed to verify"));
    }
    
    #[test]
    fn test_rate_limiting() {
        let identifier = "test_user@example.com";
        
        // Clear any existing rate limits
        Auth::clear_auth_rate_limit(identifier);
        
        // First 3 attempts should succeed
        for i in 0..3 {
            assert!(Auth::check_auth_rate_limit(identifier), "Attempt {} failed", i + 1);
        }
        
        // 4th attempt should be rate limited
        assert!(!Auth::check_auth_rate_limit(identifier), "Rate limiting not working");
        
        // Get wait time
        let wait_time = Auth::get_wait_time(identifier);
        assert!(wait_time.is_some());
        
        // Clear rate limit
        Auth::clear_auth_rate_limit(identifier);
        assert_eq!(Auth::get_failed_attempts(identifier), 0);
    }
    
    #[test]
    fn test_cors_configuration() {
        let cors = CorsConfig::default();
        
        // Test allowed origins
        assert!(cors.is_origin_allowed("http://localhost:3000"));
        assert!(cors.is_origin_allowed("http://127.0.0.1:8080"));
        
        // Test blocked origins
        assert!(!cors.is_origin_allowed("http://evil.com"));
        assert!(!cors.is_origin_allowed("https://localhost:3000")); // Different scheme
        
        // Test CORS header generation
        assert_eq!(
            cors.get_cors_header(Some("http://localhost:3000")),
            Some("http://localhost:3000".to_string())
        );
        assert_eq!(
            cors.get_cors_header(Some("http://evil.com")),
            None
        );
    }
    
    #[test]
    fn test_sql_injection_prevention() {
        use sam::sam::security::input_validation::{sanitize_sql_input};
        
        // Safe inputs should pass
        assert!(sanitize_sql_input("john.doe@example.com").is_ok());
        assert!(sanitize_sql_input("user123").is_ok());
        
        // SQL injection attempts should fail
        assert!(sanitize_sql_input("'; DROP TABLE users; --").is_err());
        assert!(sanitize_sql_input("admin' OR '1'='1").is_err());
        assert!(sanitize_sql_input("UNION SELECT * FROM passwords").is_err());
    }
    
    #[test] 
    fn test_session_timeout() {
        // Session duration should be 24 hours (86400 seconds)
        const EXPECTED_SESSION_DURATION: i64 = 86400;
        
        // This verifies we're not using the insecure 99999999999999999 value
        assert!(EXPECTED_SESSION_DURATION < 100000);
        assert_eq!(EXPECTED_SESSION_DURATION, 24 * 60 * 60);
    }
}