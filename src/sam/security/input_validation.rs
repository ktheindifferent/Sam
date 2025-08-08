use regex::Regex;
use std::collections::HashSet;
use once_cell::sync::Lazy;
use url::Url;

// Common patterns for validation
static SQL_INJECTION_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute|script|javascript|eval|setTimeout|setInterval)").unwrap_or_else(|_| Regex::new(r".*").unwrap())
});

static XSS_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(<script|javascript:|onerror=|onload=|onclick=|<iframe|<embed|<object)").unwrap_or_else(|_| Regex::new(r".*").unwrap())
});

static PATH_TRAVERSAL_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\.\./|\.\.\%2[fF]|\.\.\\|\.\.\%5[cC])").unwrap_or_else(|_| Regex::new(r".*").unwrap())
});

static COMMAND_INJECTION_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[;&|`$(){}\\n\\r]").unwrap_or_else(|_| Regex::new(r".*").unwrap())
});

// Blocked URL schemes for SSRF protection
static BLOCKED_SCHEMES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("file");
    set.insert("gopher");
    set.insert("dict");
    set.insert("ftp");
    set.insert("sftp");
    set.insert("ldap");
    set.insert("tftp");
    set
});

// Private IP ranges for SSRF protection
static PRIVATE_IP_RANGES: Lazy<Vec<(std::net::Ipv4Addr, u8)>> = Lazy::new(|| {
    vec![
        (std::net::Ipv4Addr::new(10, 0, 0, 0), 8),      // 10.0.0.0/8
        (std::net::Ipv4Addr::new(172, 16, 0, 0), 12),   // 172.16.0.0/12
        (std::net::Ipv4Addr::new(192, 168, 0, 0), 16),  // 192.168.0.0/16
        (std::net::Ipv4Addr::new(127, 0, 0, 0), 8),     // 127.0.0.0/8 (loopback)
        (std::net::Ipv4Addr::new(169, 254, 0, 0), 16),  // 169.254.0.0/16 (link-local)
    ]
});

/// Validates a URL for safe crawling (SSRF protection)
pub fn validate_url(url_str: &str) -> Result<Url, String> {
    // Parse the URL
    let url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
    
    // Check scheme
    if BLOCKED_SCHEMES.contains(url.scheme()) {
        return Err(format!("Blocked URL scheme: {}", url.scheme()));
    }
    
    // Only allow http and https
    if !["http", "https"].contains(&url.scheme()) {
        return Err(format!("Only HTTP/HTTPS URLs are allowed"));
    }
    
    // Check for localhost and private IPs
    if let Some(host) = url.host_str() {
        // Block localhost variations
        if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
            return Err(format!("Access to localhost is not allowed"));
        }
        
        // Check for private IP ranges
        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            for (range_start, prefix_len) in PRIVATE_IP_RANGES.iter() {
                if is_in_subnet(&ip, range_start, *prefix_len) {
                    return Err(format!("Access to private IP ranges is not allowed"));
                }
            }
        }
        
        // Block metadata endpoints (AWS, GCP, Azure)
        if host == "169.254.169.254" || host.contains("metadata") {
            return Err(format!("Access to metadata endpoints is not allowed"));
        }
    }
    
    Ok(url)
}

/// Check if an IP is in a subnet
fn is_in_subnet(ip: &std::net::Ipv4Addr, subnet: &std::net::Ipv4Addr, prefix_len: u8) -> bool {
    let ip_int = u32::from_be_bytes(ip.octets());
    let subnet_int = u32::from_be_bytes(subnet.octets());
    let mask = !((1u32 << (32 - prefix_len)) - 1);
    (ip_int & mask) == (subnet_int & mask)
}

/// Sanitize user input to prevent SQL injection
pub fn sanitize_sql_input(input: &str) -> Result<String, String> {
    if SQL_INJECTION_PATTERNS.is_match(input) {
        return Err("Potential SQL injection detected".to_string());
    }
    
    // Additional check for SQL special characters
    let dangerous_chars = ['\'', '"', ';', '-', '/', '*', '='];
    for ch in dangerous_chars {
        if input.contains(ch) && !input.starts_with("https://") && !input.starts_with("http://") {
            return Err(format!("Potentially dangerous character '{}' detected", ch));
        }
    }
    
    Ok(input.to_string())
}

/// Sanitize HTML input to prevent XSS
pub fn sanitize_html_input(input: &str) -> String {
    // Basic HTML entity encoding
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

/// Check for XSS patterns
pub fn contains_xss(input: &str) -> bool {
    XSS_PATTERNS.is_match(input)
}

/// Check for path traversal attempts
pub fn contains_path_traversal(input: &str) -> bool {
    PATH_TRAVERSAL_PATTERNS.is_match(input)
}

/// Validate file path (prevent path traversal)
pub fn validate_file_path(path: &str) -> Result<String, String> {
    if contains_path_traversal(path) {
        return Err("Path traversal attempt detected".to_string());
    }
    
    // Don't allow absolute paths or paths starting with /
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("Absolute paths are not allowed".to_string());
    }
    
    // Don't allow special file references
    if path == "." || path == ".." || path.contains("..") {
        return Err("Invalid path reference".to_string());
    }
    
    Ok(path.to_string())
}

/// Validate command arguments (prevent command injection)
pub fn validate_command_args(args: &str) -> Result<String, String> {
    if COMMAND_INJECTION_PATTERNS.is_match(args) {
        return Err("Potential command injection detected".to_string());
    }
    
    Ok(args.to_string())
}

/// Validate email address
pub fn validate_email(email: &str) -> Result<String, String> {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .unwrap_or_else(|_| Regex::new(r".*").unwrap());
    
    if !email_regex.is_match(email) {
        return Err("Invalid email format".to_string());
    }
    
    // Additional length check
    if email.len() > 254 {
        return Err("Email address too long".to_string());
    }
    
    Ok(email.to_string())
}

/// Validate username (alphanumeric + underscore only)
pub fn validate_username(username: &str) -> Result<String, String> {
    let username_regex = Regex::new(r"^[a-zA-Z0-9_]{3,32}$")
        .unwrap_or_else(|_| Regex::new(r".*").unwrap());
    
    if !username_regex.is_match(username) {
        return Err("Username must be 3-32 characters and contain only letters, numbers, and underscores".to_string());
    }
    
    Ok(username.to_string())
}

/// Rate limiting check (simple token bucket implementation)
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    max_tokens: u32,
    refill_rate: Duration,
}

struct TokenBucket {
    tokens: u32,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_tokens: u32, refill_rate: Duration) -> Self {
        RateLimiter {
            buckets: Mutex::new(HashMap::new()),
            max_tokens,
            refill_rate,
        }
    }
    
    pub fn check_rate_limit(&self, key: &str) -> bool {
        let mut buckets = match self.buckets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        
        let now = Instant::now();
        let bucket = buckets.entry(key.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens,
            last_refill: now,
        });
        
        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() / self.refill_rate.as_secs_f64() * self.max_tokens as f64) as u32;
        
        if tokens_to_add > 0 {
            bucket.tokens = (bucket.tokens + tokens_to_add).min(self.max_tokens);
            bucket.last_refill = now;
        }
        
        // Check if we have tokens available
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_url() {
        // Valid URLs
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com/path").is_ok());
        
        // Invalid URLs
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("http://localhost/admin").is_err());
        assert!(validate_url("http://127.0.0.1/").is_err());
        assert!(validate_url("http://192.168.1.1/").is_err());
        assert!(validate_url("http://169.254.169.254/").is_err());
    }
    
    #[test]
    fn test_sql_injection() {
        assert!(sanitize_sql_input("normal input").is_ok());
        assert!(sanitize_sql_input("DROP TABLE users").is_err());
        assert!(sanitize_sql_input("' OR '1'='1").is_err());
        assert!(sanitize_sql_input("admin'; --").is_err());
    }
    
    #[test]
    fn test_xss_detection() {
        assert!(!contains_xss("normal text"));
        assert!(contains_xss("<script>alert('xss')</script>"));
        assert!(contains_xss("javascript:alert(1)"));
        assert!(contains_xss("<img onerror=alert(1)>"));
    }
    
    #[test]
    fn test_path_traversal() {
        assert!(validate_file_path("normal/path.txt").is_ok());
        assert!(validate_file_path("../../../etc/passwd").is_err());
        assert!(validate_file_path("/etc/passwd").is_err());
        assert!(validate_file_path("..").is_err());
    }
    
    #[test]
    fn test_email_validation() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("invalid.email").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("@example.com").is_err());
    }
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        
        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("test_user"));
        }
        
        // 6th request should be blocked
        assert!(!limiter.check_rate_limit("test_user"));
    }
}