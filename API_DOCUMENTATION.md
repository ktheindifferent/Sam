# S.A.M. API Documentation

## Overview
S.A.M. (Smart Artificial Mind) provides comprehensive APIs for security, automation, and intelligence services. This documentation covers all available modules and their public interfaces.

## Table of Contents
- [Security Module](#security-module)
- [Password Manager](#password-manager)
- [Vulnerability Scanner](#vulnerability-scanner)
- [Enhanced Web Crawler](#enhanced-web-crawler)
- [Session Management](#session-management)
- [Rate Limiting](#rate-limiting)

---

## Security Module

### Input Validation

#### URL Validation
```rust
use sam::security::validate_url;

// Validate URL for SSRF protection
match validate_url("https://example.com") {
    Ok(url) => println!("Safe URL: {}", url),
    Err(e) => println!("Blocked: {}", e),
}
```

**Features:**
- SSRF protection (blocks private IPs, localhost)
- Scheme validation (only HTTP/HTTPS)
- Metadata endpoint blocking

#### SQL Input Sanitization
```rust
use sam::security::sanitize_sql_input;

match sanitize_sql_input("user input") {
    Ok(safe_input) => println!("Safe: {}", safe_input),
    Err(e) => println!("SQL injection detected: {}", e),
}
```

#### XSS Protection
```rust
use sam::security::{sanitize_html_input, contains_xss};

let safe_html = sanitize_html_input("<script>alert('xss')</script>");
if contains_xss(&input) {
    println!("XSS attempt detected");
}
```

#### Path Traversal Protection
```rust
use sam::security::{validate_file_path, contains_path_traversal};

match validate_file_path("../../../etc/passwd") {
    Ok(path) => println!("Safe path: {}", path),
    Err(e) => println!("Path traversal blocked: {}", e),
}
```

#### Rate Limiting
```rust
use sam::security::RateLimiter;
use std::time::Duration;

let limiter = RateLimiter::new(100, Duration::from_secs(60));
if limiter.check_rate_limit("user_id") {
    println!("Request allowed");
} else {
    println!("Rate limit exceeded");
}
```

---

## Password Manager

### Creating a Password Vault
```rust
use sam::services::password_manager::PasswordVault;

let mut vault = PasswordVault::new(
    "user123".to_string(),
    "My Personal Vault".to_string()
);
```

### Adding Passwords
```rust
let entry_id = vault.add_entry(
    "master_password",
    "Gmail Account".to_string(),
    "user@gmail.com".to_string(),
    "secure_password123".to_string(),
    Some("https://gmail.com".to_string()),
    Some("Personal email account".to_string()),
    vec!["email".to_string(), "personal".to_string()],
)?;
```

### Retrieving Passwords
```rust
let (entry, password) = vault.get_entry("master_password", &entry_id)?;
println!("Username: {}", entry.username);
println!("Password: {}", password);
```

### Password Generation
```rust
use sam::services::password_manager::generate_password;

let strong_password = generate_password(
    16,    // length
    true,  // uppercase
    true,  // lowercase
    true,  // numbers
    true   // symbols
);
```

### Password Strength Analysis
```rust
use sam::services::password_manager::analyze_password_strength;

let strength = analyze_password_strength("MyP@ssw0rd123!");
match strength {
    PasswordStrength::VeryStrong => println!("Excellent password!"),
    PasswordStrength::Strong => println!("Good password"),
    _ => println!("Consider strengthening your password"),
}
```

### Security Audit
```rust
let audit_report = vault.audit_passwords("master_password");
println!("Weak passwords: {}", audit_report.weak_passwords.len());
println!("Duplicate passwords: {}", audit_report.duplicate_passwords.len());
```

---

## Vulnerability Scanner

### Basic Network Scan
```rust
use sam::services::vulnerability_scanner::{VulnerabilityScanner, ScanConfig};

let config = ScanConfig {
    targets: vec!["192.168.1.0/24".to_string()],
    port_range: (1, 1000),
    vulnerability_check: true,
    ..Default::default()
};

let scanner = VulnerabilityScanner::new(config);
let results = scanner.scan_network().await?;

for result in results {
    println!("Host: {} - Open ports: {}", result.ip, result.open_ports.len());
    for vuln in result.vulnerabilities {
        println!("  [{}] {}", vuln.severity, vuln.title);
    }
}
```

### Generating Reports
```rust
let report = scanner.generate_report().await;
println!("Scanned {} hosts", report.total_hosts_scanned);
println!("Critical vulnerabilities: {}", report.critical_count);
println!("High severity: {}", report.high_count);
```

### Custom Scan Configuration
```rust
let config = ScanConfig {
    targets: vec!["example.com".to_string()],
    port_range: (80, 443),
    scan_type: ScanType::TcpConnect,
    timeout_ms: 2000,
    max_concurrent: 50,
    service_detection: true,
    os_detection: true,
    vulnerability_check: true,
};
```

---

## Enhanced Web Crawler

### Basic Enhanced Crawling
```rust
use sam::services::crawler::enhanced::EnhancedCrawler;

let crawler = EnhancedCrawler::new(
    true,  // scan_ports
    true   // generate_summaries
);

let result = crawler.crawl_enhanced("https://example.com").await?;

println!("Title: {:?}", result.title);
println!("Description: {:?}", result.description);
println!("Open ports: {:?}", result.open_ports);
println!("Security score: {}", result.security_headers.security_score);
```

### Analyzing Crawl Results
```rust
// Social media information
if let Some(og_title) = result.social_media.og_title {
    println!("Open Graph title: {}", og_title);
}

// Security headers analysis
if result.security_headers.has_csp {
    println!("Content Security Policy detected");
}
if result.security_headers.has_hsts {
    println!("HSTS enabled");
}

// Link analysis
for link in result.links {
    match link.link_type {
        LinkType::External => println!("External link: {}", link.url),
        LinkType::Internal => println!("Internal link: {}", link.url),
        LinkType::Social => println!("Social media: {}", link.url),
        _ => {}
    }
}
```

### Server Information
```rust
if let Some(server_info) = result.server_info {
    println!("Server: {:?}", server_info.server_header);
    println!("Response time: {}ms", server_info.response_time_ms);
    if let Some(ip) = server_info.ip_address {
        println!("IP Address: {}", ip);
    }
}
```

---

## Session Management

### Creating Session Manager
```rust
use sam::security::{SessionManager, Session};

let session_manager = SessionManager::new(
    "redis://localhost:6379",
    24  // session TTL in hours
).await?;
```

### Creating and Managing Sessions
```rust
// Create new session
let session = session_manager.create_session(
    "192.168.1.100".to_string(),
    "Mozilla/5.0...".to_string()
).await?;

// Authenticate session
let mut authenticated_session = session_manager.get_session(&session.id).await?;
if let Some(mut session) = authenticated_session {
    session.authenticate(
        "user123".to_string(),
        "john_doe".to_string(),
        Some("john@example.com".to_string())
    );
    session_manager.save_session(&session).await?;
}
```

### Session Validation
```rust
// Validate CSRF token
let is_valid = session_manager.validate_csrf_token(
    &session_id,
    &csrf_token
).await?;

// Get user sessions
let user_sessions = session_manager.get_user_sessions("user123").await?;
println!("Active sessions: {}", user_sessions.len());
```

### Session Cleanup
```rust
// Clean expired sessions
let cleaned = session_manager.cleanup_expired_sessions().await?;
println!("Cleaned {} expired sessions", cleaned);

// Invalidate all user sessions (logout all devices)
session_manager.invalidate_user_sessions("user123").await?;
```

---

## Rate Limiting

### HTTP Security Middleware
```rust
use sam::security::{HttpSecurityMiddleware, RateLimitConfig, DosProtectionConfig};

let rate_config = RateLimitConfig {
    max_requests: 100,
    window_seconds: 60,
    burst_size: 10,
    block_duration: 300,
    ..Default::default()
};

let dos_config = DosProtectionConfig {
    max_connections_per_ip: 10,
    max_body_size: 10 * 1024 * 1024, // 10MB
    request_timeout: 30,
    ..Default::default()
};

let middleware = HttpSecurityMiddleware::new(
    rate_config,
    dos_config,
    Some(redis_pool)
).await;
```

### Request Validation
```rust
use std::net::IpAddr;

let client_ip: IpAddr = "192.168.1.100".parse()?;

// Check rate limit
if !middleware.check_rate_limit(client_ip).await? {
    return Err("Rate limit exceeded");
}

// Check connection limit
if !middleware.check_connection_limit(client_ip).await? {
    return Err("Too many connections");
}

// Validate body size
if !middleware.validate_body_size(request_size) {
    return Err("Request too large");
}
```

### Security Headers
```rust
use sam::security::headers;

// Extract client IP from headers
let client_ip = headers::extract_client_ip(
    Some(remote_addr),
    request.headers().get("x-forwarded-for"),
    request.headers().get("x-real-ip")
);

// Add security headers to response
let security_headers = headers::add_security_headers();
for (name, value) in security_headers {
    response.headers_mut().insert(name, value.parse()?);
}
```

---

## Error Handling

All APIs use Rust's `Result` type for error handling:

```rust
use sam::services::ServiceError;

match some_api_call().await {
    Ok(result) => {
        // Handle success
        println!("Success: {:?}", result);
    }
    Err(ServiceError::HttpRequest(e)) => {
        println!("HTTP error: {}", e);
    }
    Err(ServiceError::Postgres(e)) => {
        println!("Database error: {}", e);
    }
    Err(e) => {
        println!("Other error: {}", e);
    }
}
```

---

## Configuration

### Environment Variables
- `DATABASE_URL`: PostgreSQL connection string
- `REDIS_URL`: Redis connection string
- `RUST_LOG`: Logging level (info, debug, warn, error)
- `SAM_HTTP_PORT`: HTTP server port (default: 8000)
- `SAM_CONFIG`: Path to configuration file

### Docker Environment
```bash
# Start with Docker Compose
docker-compose up -d

# With custom configuration
docker run -d \
  -e DATABASE_URL="postgresql://user:pass@host/db" \
  -e REDIS_URL="redis://host:6379" \
  -e RUST_LOG="info" \
  -p 8000:8000 \
  sam:latest
```

---

## Testing

### Unit Tests
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test security
cargo test password_manager
cargo test vulnerability_scanner
```

### Integration Tests
```bash
# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Security Considerations

1. **Input Validation**: Always validate user inputs using the security module
2. **Session Management**: Use secure session tokens and CSRF protection
3. **Rate Limiting**: Implement rate limiting for all public endpoints
4. **Password Security**: Store passwords using the encrypted vault system
5. **Network Security**: Use the vulnerability scanner to assess network security
6. **HTTPS**: Always use HTTPS in production environments

---

## Support

For issues, questions, or contributions:
- GitHub: [S.A.M. Repository](https://github.com/ktheindifferent/Sam)
- License: GPLv3
- Documentation: This file and inline code documentation

---

*Last Updated: 2025-08-08*
*Version: 0.0.4-dev*