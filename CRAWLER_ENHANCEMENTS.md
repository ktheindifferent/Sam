# Web Crawler Service Enhancements

## Overview
This document outlines the comprehensive enhancements made to the SAM web crawler service to improve its production readiness, compliance, and reliability.

## 🚀 Key Enhancements Implemented

### 1. Robots.txt Compliance ✅
**Module:** `src/sam/services/crawler/robots.rs`

- **Features:**
  - Full robots.txt parsing and compliance checking
  - Caching of robots.txt rules for efficiency
  - Support for user-agent specific rules
  - Crawl delay extraction and enforcement
  - Sitemap discovery from robots.txt
  
- **Usage:**
  ```rust
  // Check if URL is allowed
  if !robots::is_url_allowed(&url).await {
      // URL blocked by robots.txt
  }
  
  // Get crawl delay for domain
  if let Some(delay) = robots::get_crawl_delay(&domain).await {
      tokio::time::sleep(delay).await;
  }
  ```

### 2. XML Sitemap Support ✅
**Module:** `src/sam/services/crawler/sitemap.rs`

- **Features:**
  - Parse standard sitemap.xml format
  - Support for sitemap index files
  - Handle compressed sitemaps (gzip)
  - Extract URLs with priority and change frequency
  - Recursive sitemap discovery
  
- **Usage:**
  ```rust
  // Extract all URLs from sitemaps
  let urls = sitemap::extract_urls_from_sitemaps(&domain).await;
  
  // Fetch specific sitemap
  let entries = sitemap::fetch_sitemap(&sitemap_url).await?;
  ```

### 3. Circuit Breaker Pattern ✅
**Module:** `src/sam/services/crawler/circuit_breaker.rs`

- **Features:**
  - Automatic circuit breaking for failing domains
  - Three states: Closed, Open, Half-Open
  - Exponential backoff for retries
  - Configurable failure thresholds
  - Automatic recovery detection
  
- **Circuit States:**
  - **Closed:** Normal operation, requests allowed
  - **Open:** Domain blocked due to failures
  - **Half-Open:** Testing recovery with limited requests
  
- **Usage:**
  ```rust
  // Check if domain is allowed
  if !circuit_breaker::is_domain_allowed(&domain).await {
      // Domain blocked by circuit breaker
  }
  
  // Record success/failure
  circuit_breaker::record_domain_success(&domain).await;
  circuit_breaker::record_domain_failure(&domain).await;
  ```

### 4. Comprehensive Metrics & Monitoring ✅
**Module:** `src/sam/services/crawler/metrics.rs`

- **Metrics Tracked:**
  - Total URLs crawled/discovered
  - Success/failure rates
  - Data downloaded (bytes)
  - Response times (average, per domain)
  - Crawl rate (URLs/second)
  - Robots.txt blocks
  - Circuit breaker blocks
  - HTTP status code distribution
  - Content type distribution
  
- **Progress Tracking:**
  - Job-level progress monitoring
  - Estimated time remaining
  - Current depth tracking
  - Real-time status updates
  
- **Usage:**
  ```rust
  // Get metrics report
  let report = metrics::generate_metrics_report().await;
  
  // Record crawl success
  metrics::record_crawl_success(
      &domain, &url, bytes, response_time, 
      status_code, content_type
  ).await;
  ```

### 5. Enhanced Error Handling ✅
- Proper URL validation before crawling
- Certificate validation enabled (was disabled)
- Graceful handling of timeouts and network errors
- Retry logic with exponential backoff
- Domain-specific error tracking

### 6. User-Agent Configuration ✅
- Centralized User-Agent string: `SAM-Crawler/0.0.2 (+https://github.com/OSF/sam)`
- Properly identifies the crawler to websites
- Helps avoid being blocked as an anonymous bot

### 7. Missing Function Fixes ✅
- Added synchronous `start_service()` function
- Fixed export inconsistency in module definition
- Added helper functions for metrics and status

## 📊 Performance Improvements

### Before Enhancements
- No robots.txt compliance
- No circuit breaking for failures
- Limited error recovery
- No metrics or monitoring
- Anonymous crawling (no User-Agent)
- Certificate validation disabled

### After Enhancements
- Full robots.txt compliance
- Intelligent circuit breaking
- Comprehensive error handling
- Real-time metrics and monitoring
- Proper crawler identification
- Secure HTTPS validation

## 🔧 Configuration

### Circuit Breaker Settings
```rust
CircuitBreakerConfig {
    failure_threshold: 5,           // Failures before opening
    initial_backoff: 60s,           // Initial backoff duration
    max_backoff: 3600s,            // Maximum backoff (1 hour)
    open_duration: 300s,           // Time before trying half-open
    half_open_success_threshold: 3, // Successes to close circuit
}
```

### Crawler Limits
- Max URLs per sitemap: 10,000
- Max sitemap depth: 3 levels
- Max concurrency: 2x CPU cores (capped at 32)
- Request timeout: 30 seconds
- Max redirects: 5

## 🛡️ Security Enhancements

1. **Certificate Validation:** SSL certificates are now properly validated
2. **Robots.txt Compliance:** Respects website crawling policies
3. **Rate Limiting:** Prevents overwhelming target servers
4. **User-Agent Header:** Properly identifies the crawler

## 📈 Monitoring & Observability

### Available Metrics Endpoints
```rust
// Get comprehensive metrics report
let report = crawler::get_metrics_report().await;

// Get circuit breaker status for all domains
let status = crawler::get_circuit_breaker_status().await;

// Check service status
let status = crawler::service_status();
```

### Metrics Report Example
```
=== Crawler Metrics Report ===
Runtime: 2.45 hours
URLs Crawled: 15,234
URLs Discovered: 45,123
Data Downloaded: 234.56 MB
Success Rate: 94.32%
Average Response Time: 234.5 ms
Current Crawl Rate: 12.3 URLs/sec
Robots Blocked: 123
Circuit Breaker Blocked: 45
Active Domains: 234
```

## 🧪 Testing

The enhancements include comprehensive unit tests for:
- Robots.txt parsing
- Sitemap XML parsing
- Circuit breaker state transitions
- Metrics collection and calculation

Run tests with:
```bash
cargo test crawler
```

## 🔄 Backward Compatibility

All enhancements maintain full backward compatibility:
- Existing APIs remain unchanged
- New features are opt-in through configuration
- Database schema remains compatible
- No breaking changes to public interfaces

## 📝 Usage Examples

### Basic Crawler with All Enhancements
```rust
use sam::services::crawler;

// Start the crawler service
crawler::start_service_async().await;

// The crawler now automatically:
// - Checks robots.txt before crawling
// - Respects crawl delays
// - Discovers and processes sitemaps
// - Tracks metrics for all operations
// - Applies circuit breaking to failing domains
// - Uses proper User-Agent headers
// - Validates SSL certificates

// Get metrics report
let report = crawler::get_metrics_report().await;
println!("{}", report);

// Check circuit breaker status
let cb_status = crawler::get_circuit_breaker_status().await;
for (domain, stats) in cb_status {
    println!("{}: {:?}", domain, stats);
}
```

### Manual URL Crawling with Compliance
```rust
// Check if URL is allowed before crawling
if crawler::is_url_allowed(&url).await {
    // Check circuit breaker
    if crawler::is_domain_allowed(&domain).await {
        // Crawl the URL
        let pages = crawler::crawl_url(job_id, url).await?;
        
        // Metrics are automatically recorded
    }
}
```

## 🚦 Production Readiness Checklist

✅ Robots.txt compliance  
✅ User-Agent identification  
✅ SSL certificate validation  
✅ Circuit breaker for failures  
✅ Comprehensive metrics  
✅ Sitemap discovery  
✅ Error handling & recovery  
✅ Rate limiting  
✅ Progress tracking  
✅ Backward compatibility  

## 📚 Further Improvements (Future)

While the current enhancements significantly improve the crawler, potential future improvements include:

1. **Distributed Crawling:** Support for multiple crawler instances
2. **Advanced Scheduling:** Priority queues and smart scheduling
3. **Content Deduplication:** Avoid re-crawling identical content
4. **JavaScript Rendering:** Support for dynamic content
5. **API Rate Limit Detection:** Automatic detection and adaptation
6. **Machine Learning:** Intelligent crawl path optimization
7. **Webhook Notifications:** Real-time crawl event notifications
8. **GraphQL API:** Modern API for crawler management

## 🤝 Contributing

When adding new features to the crawler:
1. Maintain backward compatibility
2. Add comprehensive tests
3. Update metrics collection
4. Document new functionality
5. Consider circuit breaker integration

## 📄 License

These enhancements are part of the SAM project and follow the same GPLv3 license.