# Crawler TODO List

## 🚀 High Priority

### Performance & Reliability
- [x] Add persistent job queue that survives restarts (currently jobs are lost on restart) ✅
- [x] Implement distributed locking for multi-instance deployments ✅
- [x] Add retry mechanism for failed URLs with exponential backoff ✅
- [x] Optimize memory usage for large crawl sessions (currently stores all URLs in memory) ✅
- [x] Add connection pooling for database operations ✅

### Rate Limiting & Politeness
- [x] Implement per-domain crawl delay based on robots.txt Crawl-delay directive ✅
- [x] Add adaptive rate limiting based on server response times ✅
- [x] Respect retry-after headers from 429/503 responses ✅
- [ ] Add user-agent rotation for better crawl success
- [x] Implement domain-specific concurrency limits ✅

### Data Storage & Processing
- [x] Store crawled page content (currently only stores metadata) ✅
- [x] Add full-text search capability for crawled content ✅
- [x] Implement content deduplication (hash-based) ✅
- [ ] Add support for different content types (PDF, images, etc.)
- [ ] Create data export functionality (JSON, CSV, etc.)

## 🔧 Medium Priority

### Crawl Intelligence
- [x] Add sitemap.xml parser for better URL discovery ✅
- [x] Implement RSS/Atom feed detection and parsing ✅
- [ ] Add JavaScript rendering support for SPA sites (using headless browser)
- [x] Detect and handle infinite URL patterns (e.g., calendars) ✅
- [x] Add language detection and filtering ✅

### Monitoring & Observability
- [x] Add Prometheus metrics for crawl statistics ✅
- [x] Implement crawl session reporting (URLs/sec, success rate, etc.) ✅
- [x] Add webhook notifications for crawl completion ✅
- [ ] Create dashboard for real-time crawl monitoring
- [x] Add detailed error categorization and reporting ✅

### Configuration & Control
- [x] Make max_depth configurable per job (currently hardcoded to 10) ✅
- [x] Add domain whitelist/blacklist functionality ✅
- [x] Implement crawl scheduling (cron-like) ✅
- [x] Add pause/resume functionality for running crawls ✅
- [x] Create REST API for crawler management ✅

## 📝 Low Priority

### Features
- [ ] Add support for authenticated crawling (cookies, OAuth)
- [ ] Implement focused/topical crawling with ML classification
- [ ] Add link graph analysis and visualization
- [ ] Support for crawling API endpoints (not just HTML)
- [ ] Add archive.org integration for historical snapshots

### Code Quality
- [ ] Add comprehensive unit tests for crawler components
- [ ] Implement integration tests with mock servers
- [ ] Add performance benchmarks
- [ ] Create documentation for crawler architecture
- [ ] Add configuration file support (YAML/TOML)

## 🐛 Bug Fixes

- [x] Fix database timeout issues in CapRover environment ✅
- [x] Handle redirects more gracefully (follow 301/302 properly) ✅
- [x] Fix memory leak in long-running crawl sessions ✅
- [x] Improve error handling for malformed URLs ✅
- [ ] Fix circuit breaker not resetting properly after cooldown

## 💡 Ideas for Future

- [ ] Implement distributed crawling across multiple nodes
- [ ] Add machine learning for crawl prioritization
- [ ] Create browser extension for manual URL submission
- [ ] Add support for Tor/proxy crawling
- [ ] Implement change detection for recrawling
- [ ] Add screenshot capture for visual archiving
- [ ] Create crawl replay functionality for testing

## 📊 Current Status

**Working:**
- Basic crawling with depth control ✅
- Robots.txt compliance ✅
- Circuit breaker for rate limiting ✅
- DNS caching ✅
- Concurrent crawling ✅
- URL discovery from HTML ✅

**Issues:**
- Database operations timeout in CapRover
- Jobs don't persist across restarts
- No content storage (only metadata)
- Memory usage grows unbounded
- No crawl progress persistence

## 🎯 Next Steps

1. ~~**Immediate:** Fix database timeout issues~~ ✅ COMPLETED
2. ~~**Short-term:** Add persistent job queue with Redis~~ ✅ COMPLETED
3. **NEW Priority:** Implement per-domain crawl delays and adaptive rate limiting
4. **Medium-term:** Implement content storage and full-text search
5. **Long-term:** Add distributed crawling support across multiple nodes

## 📝 Recent Improvements (2025-09-07)

### Session 1 - Infrastructure & Reliability:
1. **Persistent Job Queue** (`job_queue.rs`)
   - Redis-backed queue that survives restarts
   - Priority-based job scheduling
   - Automatic retry with exponential backoff
   - Orphaned job recovery
   - Queue statistics and monitoring

2. **Distributed Locking** (`job_queue.rs`)
   - Redis-based distributed locks for multi-instance coordination
   - Lock expiration and extension support
   - Safe lock release with ownership verification

3. **Memory Optimization** (`memory_optimized.rs`)
   - Bloom filter for efficient visited URL tracking
   - LRU cache for recent URLs
   - Bounded queue with Redis spillover
   - Configurable memory limits
   - Memory usage monitoring

4. **CapRover Database Fixes** (`crawler.rs`)
   - Extended timeouts for CapRover environment
   - Exponential backoff retry logic
   - Environment-specific connection pooling
   - Improved error handling and logging

### Session 2 - Rate Limiting & Content Storage:
5. **Adaptive Rate Limiting** (`rate_limiter.rs`)
   - Per-domain crawl delays respecting robots.txt
   - Dynamic adjustment based on server response times
   - Retry-After header support for 429/503 responses
   - Concurrent request limiting per domain
   - Global RPS limiting across all domains
   - Automatic cleanup of old domain statistics

6. **Enhanced Content Storage** (`content_storage.rs`)
   - Full page content storage with compression
   - SHA-256 hash-based content deduplication
   - Full-text search with PostgreSQL GIN indexes
   - Title and meta description extraction
   - Basic language detection
   - Deduplication statistics tracking
   - Compressed HTML storage (gzip)

7. **Existing Features Enhanced**
   - Sitemap.xml parsing already implemented and working
   - Robots.txt compliance with crawl-delay support
   - Circuit breaker pattern for resilient connections

### Session 3 - Monitoring & Configuration:
8. **Prometheus Metrics** (`prometheus_metrics.rs`)
   - Comprehensive metrics collection for all crawler operations
   - Response time, content size, and crawl depth histograms
   - Rate limit and robots.txt denial tracking
   - Database operation metrics
   - Memory usage and deduplication statistics
   - Circuit breaker state monitoring
   - Export endpoint for Prometheus scraping

9. **Configurable Job System** (`job_config.rs`)
   - Per-job max_depth configuration (no longer hardcoded)
   - Domain whitelist/blacklist support
   - URL pattern inclusion/exclusion with regex
   - Language and content type filtering
   - Custom headers and user agent configuration
   - Job priority and tagging system
   - Cron-like scheduling for recurring crawls
   - Webhook notifications on completion
   - Preset configurations (shallow, deep, focused, archival)

10. **REST API for Management** (`crawler_management.rs`)
    - Full CRUD operations for crawl jobs
    - Pause/resume functionality for running crawls
    - Service start/stop/restart endpoints
    - Queue management and clearing
    - Real-time statistics and metrics export
    - Domain-specific rate limit statistics
    - Configuration preset endpoints
    - Comprehensive error handling and validation

### Session 4 - Advanced Features & Bug Fixes:
11. **RSS/Atom Feed Support** (`feed_parser.rs`)
    - Automatic feed detection in HTML pages
    - RSS 2.0 and Atom 1.0 parsing
    - Feed URL discovery from common paths
    - Item extraction with metadata (title, description, date)
    - Enclosure and category support
    - Feed validation and testing

12. **Infinite URL Pattern Detection** (`url_patterns.rs`)
    - Calendar pattern detection (dates, months, years)
    - Pagination limit enforcement
    - Session and tracking parameter filtering
    - URL normalization and deduplication
    - Pattern frequency analysis
    - Configurable thresholds per pattern type
    - URL cleaning and canonicalization

13. **Webhook Notifications** (`webhooks.rs`)
    - Event-based notifications (start, complete, fail, pause, resume)
    - Configurable webhook endpoints
    - HMAC signature for security
    - Retry logic with exponential backoff
    - Milestone notifications
    - Custom headers and metadata
    - Webhook validation and testing

14. **Bug Fixes & Improvements**
    - Proper redirect handling (301/302)
    - Memory leak prevention with bounded queues
    - Malformed URL error handling
    - Language detection in content storage
    - Configuration file support structure
    - Enhanced error categorization

---

*Last updated: 2025-09-07*