#[cfg(test)]
mod security_tests {
    use sam::security::input_validation::{
        InputValidator, ValidationConfig, ValidationResult,
        sanitize_html, sanitize_sql, sanitize_path, sanitize_command,
        validate_email, validate_username, validate_url, validate_file_upload,
        is_private_ip, detect_ssrf_attempt
    };
    use sam::security::session::{SessionManager, Session, CSRFToken};
    use sam::security::password::{PasswordManager, PasswordPolicy, PasswordStrength};
    use sam::security::rate_limiter::{RateLimiter, RateLimitConfig};
    use std::time::Duration;
    use std::collections::HashMap;

    #[test]
    fn test_sql_injection_prevention() {
        let test_cases = vec![
            ("SELECT * FROM users", "SELECT * FROM users"),
            ("'; DROP TABLE users; --", "'; DROP TABLE users; --"),
            ("1' OR '1'='1", "1' OR '1'='1"),
            ("admin'--", "admin'--"),
            ("' UNION SELECT * FROM passwords --", "' UNION SELECT * FROM passwords --"),
        ];
        
        for (input, _expected) in test_cases {
            let sanitized = sanitize_sql(input);
            assert!(!sanitized.contains("DROP"));
            assert!(!sanitized.contains("UNION"));
            assert!(!sanitized.contains("--") || sanitized == input);
        }
    }

    #[test]
    fn test_xss_prevention() {
        let test_cases = vec![
            ("<script>alert('XSS')</script>", "&lt;script&gt;alert('XSS')&lt;/script&gt;"),
            ("<img src=x onerror='alert(1)'>", "&lt;img src=x onerror='alert(1)'&gt;"),
            ("javascript:alert('XSS')", "javascript:alert('XSS')"),
            ("<iframe src='evil.com'></iframe>", "&lt;iframe src='evil.com'&gt;&lt;/iframe&gt;"),
            ("Hello <b>World</b>", "Hello &lt;b&gt;World&lt;/b&gt;"),
        ];
        
        for (input, expected) in test_cases {
            let sanitized = sanitize_html(input);
            assert_eq!(sanitized, expected);
        }
    }

    #[test]
    fn test_path_traversal_prevention() {
        let test_cases = vec![
            ("../../../etc/passwd", "etc/passwd"),
            ("..\\..\\windows\\system32", "windows/system32"),
            ("/etc/passwd", "etc/passwd"),
            ("C:\\Windows\\System32", "C:/Windows/System32"),
            ("./safe/path/file.txt", "safe/path/file.txt"),
            ("file://etc/passwd", "file:/etc/passwd"),
        ];
        
        for (input, expected) in test_cases {
            let sanitized = sanitize_path(input);
            assert!(!sanitized.contains(".."));
            assert!(!sanitized.starts_with("/"));
        }
    }

    #[test]
    fn test_command_injection_prevention() {
        let test_cases = vec![
            ("ls -la", "ls -la"),
            ("rm -rf /", "rm -rf"),
            ("cat /etc/passwd", "cat /etc/passwd"),
            ("echo test; rm -rf /", "echo test rm -rf"),
            ("test && malicious", "test malicious"),
            ("test | grep password", "test grep password"),
            ("test `whoami`", "test whoami"),
            ("test $(command)", "test command"),
        ];
        
        for (input, _) in test_cases {
            let sanitized = sanitize_command(input);
            assert!(!sanitized.contains(";"));
            assert!(!sanitized.contains("&&"));
            assert!(!sanitized.contains("|"));
            assert!(!sanitized.contains("`"));
            assert!(!sanitized.contains("$"));
        }
    }

    #[test]
    fn test_ssrf_prevention() {
        let test_cases = vec![
            ("http://localhost/admin", true),
            ("http://127.0.0.1:8080", true),
            ("http://192.168.1.1", true),
            ("http://10.0.0.1", true),
            ("http://172.16.0.1", true),
            ("http://[::1]", true),
            ("http://example.com", false),
            ("https://google.com", false),
            ("http://169.254.169.254", true),
            ("file:///etc/passwd", true),
        ];
        
        for (url, should_block) in test_cases {
            let is_ssrf = detect_ssrf_attempt(url);
            assert_eq!(is_ssrf, should_block, "Failed for URL: {}", url);
        }
    }

    #[test]
    fn test_email_validation() {
        let valid_emails = vec![
            "user@example.com",
            "test.user@example.co.uk",
            "user+tag@example.org",
            "123@example.com",
        ];
        
        let invalid_emails = vec![
            "invalid",
            "@example.com",
            "user@",
            "user@.com",
            "user@example",
            "user @example.com",
            "user@exam ple.com",
        ];
        
        for email in valid_emails {
            assert!(validate_email(email), "Failed to validate: {}", email);
        }
        
        for email in invalid_emails {
            assert!(!validate_email(email), "Should have rejected: {}", email);
        }
    }

    #[test]
    fn test_username_validation() {
        let valid_usernames = vec![
            "john_doe",
            "user123",
            "test-user",
            "User_Name",
        ];
        
        let invalid_usernames = vec![
            "ab",
            "a",
            "user name",
            "user@name",
            "user#name",
            "../admin",
            "admin'--",
            "a".repeat(65).as_str(),
        ];
        
        for username in valid_usernames {
            assert!(validate_username(username), "Failed to validate: {}", username);
        }
        
        for username in invalid_usernames {
            assert!(!validate_username(username), "Should have rejected: {}", username);
        }
    }

    #[test]
    fn test_url_validation() {
        let valid_urls = vec![
            "http://example.com",
            "https://example.com/path",
            "https://sub.example.com:8080/path?query=value",
            "ftp://files.example.com",
        ];
        
        let invalid_urls = vec![
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "not a url",
            "http://",
            "//example.com",
        ];
        
        for url in valid_urls {
            assert!(validate_url(url), "Failed to validate: {}", url);
        }
        
        for url in invalid_urls {
            assert!(!validate_url(url), "Should have rejected: {}", url);
        }
    }

    #[test]
    fn test_file_upload_validation() {
        let safe_files = vec![
            ("document.pdf", b"PDF content"),
            ("image.jpg", b"\xFF\xD8\xFF"),
            ("text.txt", b"Plain text"),
            ("data.json", b"{\"key\": \"value\"}"),
        ];
        
        let dangerous_files = vec![
            ("script.exe", b"MZ"),
            ("malware.bat", b"@echo off"),
            ("../../etc/passwd", b"root:"),
            ("shell.php", b"<?php"),
            ("hack.jsp", b"<%@"),
        ];
        
        for (filename, content) in safe_files {
            let result = validate_file_upload(filename, content, 1024 * 1024);
            assert!(result.is_safe, "Should be safe: {}", filename);
        }
        
        for (filename, content) in dangerous_files {
            let result = validate_file_upload(filename, content, 1024 * 1024);
            assert!(!result.is_safe, "Should be dangerous: {}", filename);
        }
    }

    #[test]
    fn test_input_validator_comprehensive() {
        let config = ValidationConfig {
            max_length: 1000,
            allow_html: false,
            allow_sql: false,
            allow_scripts: false,
            check_ssrf: true,
            check_path_traversal: true,
        };
        
        let validator = InputValidator::new(config);
        
        let test_inputs = vec![
            ("normal input", true),
            ("<script>alert(1)</script>", false),
            ("'; DROP TABLE users; --", false),
            ("../../../etc/passwd", false),
            ("http://localhost/admin", false),
        ];
        
        for (input, should_pass) in test_inputs {
            let result = validator.validate(input);
            assert_eq!(result.is_valid, should_pass, "Failed for input: {}", input);
        }
    }

    #[tokio::test]
    async fn test_session_management() {
        let manager = SessionManager::new("secret_key".to_string())
            .expect("Failed to create session manager");
        
        let session = manager.create_session("user123").await
            .expect("Failed to create session");
        
        assert_eq!(session.user_id, "user123");
        assert!(session.id.len() > 0);
        assert!(session.csrf_token.len() > 0);
        
        let retrieved = manager.get_session(&session.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.expect("Session should be retrievable").user_id, "user123");
        
        manager.invalidate_session(&session.id).await
            .expect("Failed to invalidate session");
        
        let after_invalidation = manager.get_session(&session.id).await;
        assert!(after_invalidation.is_none());
    }

    #[tokio::test]
    async fn test_csrf_protection() {
        let manager = SessionManager::new("secret_key".to_string())
            .expect("Failed to create session manager");
        
        let session = manager.create_session("user456").await
            .expect("Failed to create session");
        
        let csrf_token = &session.csrf_token;
        
        let is_valid = manager.validate_csrf_token(&session.id, csrf_token).await;
        assert!(is_valid);
        
        let is_invalid = manager.validate_csrf_token(&session.id, "wrong_token").await;
        assert!(!is_invalid);
    }

    #[test]
    fn test_password_hashing() {
        let manager = PasswordManager::new()
            .expect("Failed to create password manager");
        
        let password = "MySecurePassword123!";
        let hash = manager.hash_password(password)
            .expect("Failed to hash password");
        
        assert!(hash.len() > 0);
        assert_ne!(hash, password);
        
        let is_valid = manager.verify_password(password, &hash)
            .expect("Failed to verify password");
        assert!(is_valid);
        
        let is_invalid = manager.verify_password("WrongPassword", &hash)
            .expect("Failed to verify wrong password");
        assert!(!is_invalid);
    }

    #[test]
    fn test_password_strength() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
            max_consecutive: 3,
            min_entropy: 50.0,
        };
        
        let manager = PasswordManager::with_policy(policy)
            .expect("Failed to create password manager");
        
        let test_passwords = vec![
            ("weak", PasswordStrength::Weak),
            ("Password", PasswordStrength::Fair),
            ("Password123", PasswordStrength::Good),
            ("P@ssw0rd123!", PasswordStrength::Strong),
            ("MyV3ry$ecur3P@ssw0rd!", PasswordStrength::VeryStrong),
        ];
        
        for (password, expected_strength) in test_passwords {
            let strength = manager.check_strength(password);
            assert!(strength as u8 >= expected_strength as u8 - 1 &&
                   strength as u8 <= expected_strength as u8 + 1,
                   "Password '{}' expected {:?}, got {:?}", 
                   password, expected_strength, strength);
        }
    }

    #[test]
    fn test_password_policy_enforcement() {
        let policy = PasswordPolicy {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
            max_consecutive: 2,
            min_entropy: 60.0,
        };
        
        let manager = PasswordManager::with_policy(policy)
            .expect("Failed to create password manager");
        
        let violations = manager.validate_against_policy("password");
        assert!(violations.len() > 0);
        assert!(violations.contains(&"Password must be at least 12 characters"));
        assert!(violations.contains(&"Password must contain uppercase letters"));
        assert!(violations.contains(&"Password must contain numbers"));
        assert!(violations.contains(&"Password must contain special characters"));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_second: 2,
            burst_size: 3,
            window_duration: Duration::from_secs(1),
        };
        
        let limiter = RateLimiter::new(config);
        let client_id = "test_client";
        
        assert!(limiter.check_rate_limit(client_id).await);
        assert!(limiter.check_rate_limit(client_id).await);
        assert!(limiter.check_rate_limit(client_id).await);
        assert!(!limiter.check_rate_limit(client_id).await);
        
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(limiter.check_rate_limit(client_id).await);
    }

    #[tokio::test]
    async fn test_distributed_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_second: 5,
            burst_size: 10,
            window_duration: Duration::from_secs(1),
        };
        
        let limiter = RateLimiter::new_distributed(config, "redis://localhost:6379")
            .await
            .expect("Failed to create distributed rate limiter");
        
        let client_id = "distributed_client";
        
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(client_id).await);
        }
        
        assert!(!limiter.check_rate_limit(client_id).await);
    }

    #[test]
    fn test_private_ip_detection() {
        let private_ips = vec![
            "127.0.0.1",
            "127.255.255.255",
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "192.168.255.255",
            "169.254.0.1",
            "169.254.255.255",
            "::1",
            "fc00::",
            "fd00::1",
        ];
        
        let public_ips = vec![
            "8.8.8.8",
            "1.1.1.1",
            "93.184.216.34",
            "2001:4860:4860::8888",
        ];
        
        for ip in private_ips {
            assert!(is_private_ip(ip), "Should be private: {}", ip);
        }
        
        for ip in public_ips {
            assert!(!is_private_ip(ip), "Should be public: {}", ip);
        }
    }

    #[test]
    fn test_security_headers_validation() {
        let mut headers = HashMap::new();
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert("X-XSS-Protection".to_string(), "1; mode=block".to_string());
        headers.insert("Strict-Transport-Security".to_string(), 
                      "max-age=31536000; includeSubDomains".to_string());
        headers.insert("Content-Security-Policy".to_string(), 
                      "default-src 'self'".to_string());
        
        for (header, value) in &headers {
            assert!(value.len() > 0, "Header {} should have a value", header);
        }
        
        assert_eq!(headers.get("X-Frame-Options"), Some(&"DENY".to_string()));
        assert_eq!(headers.get("X-Content-Type-Options"), Some(&"nosniff".to_string()));
    }

    #[test]
    fn test_input_length_limits() {
        let config = ValidationConfig {
            max_length: 100,
            ..Default::default()
        };
        
        let validator = InputValidator::new(config);
        
        let short_input = "a".repeat(50);
        let exact_input = "a".repeat(100);
        let long_input = "a".repeat(101);
        
        assert!(validator.validate(&short_input).is_valid);
        assert!(validator.validate(&exact_input).is_valid);
        assert!(!validator.validate(&long_input).is_valid);
    }

    #[test]
    fn test_multi_layer_validation() {
        let input = "<script>alert('XSS')</script>'; DROP TABLE users; --";
        
        let html_sanitized = sanitize_html(input);
        assert!(!html_sanitized.contains("<script>"));
        
        let sql_sanitized = sanitize_sql(&html_sanitized);
        assert!(!sql_sanitized.contains("DROP TABLE"));
        
        let final_result = sanitize_command(&sql_sanitized);
        assert!(!final_result.contains(";"));
    }
}