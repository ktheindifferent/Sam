use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static::lazy_static! {
    /// Rate limiter for authentication attempts
    static ref AUTH_RATE_LIMITER: RwLock<HashMap<String, Vec<i64>>> = RwLock::new(HashMap::new());
}

/// Authentication utilities
pub struct Auth;

impl Auth {
    /// Hash a password using Argon2id
    pub fn hash_password(password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Password hashing failed: {}", e))?;
        Ok(password_hash.to_string())
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| format!("Invalid password hash: {}", e))?;
        let argon2 = Argon2::default();
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check rate limit for authentication attempts
    /// Returns true if the attempt is allowed, false if rate limited
    pub fn check_auth_rate_limit(identifier: &str) -> bool {
        let mut limiter = match AUTH_RATE_LIMITER.write() {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to acquire rate limiter lock: {}", e);
                return false; // Fail closed on lock error
            }
        };
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| {
                log::error!("System time error: {}", e);
                std::time::Duration::from_secs(0)
            })
            .as_secs() as i64;
        
        let attempts = limiter.entry(identifier.to_string()).or_insert_with(Vec::new);
        
        // Clean up old attempts (older than 15 minutes)
        attempts.retain(|&time| current_time - time < 900);
        
        // Progressive rate limiting based on number of attempts
        let (max_attempts, window_seconds) = match attempts.len() {
            0..=2 => (3, 60),    // 3 attempts per minute for first tries
            3..=5 => (2, 300),   // 2 attempts per 5 minutes after 3 failed attempts
            6..=9 => (1, 900),   // 1 attempt per 15 minutes after 6 failed attempts
            _ => (1, 3600),      // 1 attempt per hour after 10 failed attempts
        };
        
        // Count attempts in the current window
        let window_start = current_time - window_seconds;
        let recent_attempts = attempts.iter().filter(|&&time| time >= window_start).count();
        
        if recent_attempts >= max_attempts {
            false // Rate limit exceeded
        } else {
            // Add current attempt
            attempts.push(current_time);
            true
        }
    }

    /// Get the number of failed attempts for an identifier
    pub fn get_failed_attempts(identifier: &str) -> usize {
        let limiter = match AUTH_RATE_LIMITER.read() {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to acquire rate limiter lock: {}", e);
                return 0;
            }
        };
        limiter.get(identifier).map(|v| v.len()).unwrap_or(0)
    }

    /// Clear rate limit for an identifier (on successful login)
    pub fn clear_auth_rate_limit(identifier: &str) {
        let mut limiter = match AUTH_RATE_LIMITER.write() {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to acquire rate limiter lock: {}", e);
                return;
            }
        };
        limiter.remove(identifier);
    }

    /// Get wait time in seconds before next attempt is allowed
    pub fn get_wait_time(identifier: &str) -> Option<i64> {
        let limiter = match AUTH_RATE_LIMITER.read() {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to acquire rate limiter lock: {}", e);
                return None;
            }
        };
        if let Some(attempts) = limiter.get(identifier) {
            if attempts.is_empty() {
                return None;
            }

            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|e| {
                    log::error!("System time error: {}", e);
                    std::time::Duration::from_secs(0)
                })
                .as_secs() as i64;

            // Determine wait time based on number of attempts
            let wait_seconds = match attempts.len() {
                0..=2 => 60,
                3..=5 => 300,
                6..=9 => 900,
                _ => 3600,
            };

            if let Some(&last_attempt) = attempts.last() {
                let elapsed = current_time - last_attempt;
                if elapsed < wait_seconds {
                    return Some(wait_seconds - elapsed);
                }
            }
        }
        None
    }
}

/// CORS configuration
pub struct CorsConfig {
    allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        CorsConfig {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ],
        }
    }
}

impl CorsConfig {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        CorsConfig { allowed_origins }
    }

    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|allowed| allowed == origin)
    }

    pub fn get_cors_header(&self, origin: Option<&str>) -> Option<String> {
        if let Some(origin) = origin {
            if self.is_origin_allowed(origin) {
                return Some(origin.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "SecurePassword123!";
        let hash = Auth::hash_password(password).unwrap();
        
        // Hash should not be the same as the password
        assert_ne!(hash, password);
        
        // Should be able to verify the password
        assert!(Auth::verify_password(password, &hash).unwrap());
        
        // Wrong password should fail
        assert!(!Auth::verify_password("WrongPassword", &hash).unwrap());
    }

    #[test]
    fn test_rate_limiting() {
        let identifier = "test_user@example.com";
        
        // Clear any existing attempts
        Auth::clear_auth_rate_limit(identifier);
        
        // First 3 attempts should be allowed
        for _ in 0..3 {
            assert!(Auth::check_auth_rate_limit(identifier));
        }
        
        // 4th attempt within a minute should be blocked
        assert!(!Auth::check_auth_rate_limit(identifier));
        
        // Check that we have 4 attempts recorded (3 successful checks + 1 failed)
        assert_eq!(Auth::get_failed_attempts(identifier), 4);
        
        // Clear rate limit
        Auth::clear_auth_rate_limit(identifier);
        assert_eq!(Auth::get_failed_attempts(identifier), 0);
    }

    #[test]
    fn test_cors_config() {
        let cors = CorsConfig::default();
        
        // Allowed origins
        assert!(cors.is_origin_allowed("http://localhost:3000"));
        assert!(cors.is_origin_allowed("http://127.0.0.1:8080"));
        
        // Disallowed origins
        assert!(!cors.is_origin_allowed("http://evil.com"));
        assert!(!cors.is_origin_allowed("https://localhost:3000")); // Different scheme
        
        // Get CORS header
        assert_eq!(
            cors.get_cors_header(Some("http://localhost:3000")),
            Some("http://localhost:3000".to_string())
        );
        assert_eq!(cors.get_cors_header(Some("http://evil.com")), None);
    }
}