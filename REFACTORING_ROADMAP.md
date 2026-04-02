# SAM Refactoring Roadmap - 2026 Q2

**Status:** Ready for Implementation  
**Duration:** 2-3 sprints (6-9 weeks)  
**Priority:** MEDIUM (code quality & maintainability)  

---

## Overview

This document outlines a structured approach to refactor SAM's largest functions and improve overall code quality. Work is organized into phases that can run in parallel or sequence.

---

## Phase 1: Large Function Extraction (Days 1-3)

### Module 1.1: LIFX API Server Refactoring
**File:** `src/lib/services/lifx/lifx_api_server.rs`

#### Current State:
- `start()` function: 965 lines
- Handles: Socket init, client routing, command parsing, device control
- Testing: Limited (monolithic function)

#### Target State:
```
lifx_api_server.rs
├── server.rs (new)
│   └── fn initialize_server() [~100 lines]
│   └── fn setup_server_socket() [~50 lines]
│   └── fn bind_with_retry() [~40 lines]
├── handlers.rs (refactored)
│   └── fn handle_bulb_command() [~150 lines] - NEW
│   └── fn handle_effects() [already 395 lines]
├── effects/
│   └── mod.rs [~50 lines]
│   └── rainbow.rs [~80 lines]
│   └── breathe.rs [~70 lines]
│   └── pulse.rs [~60 lines]
│   └── color.rs [~50 lines]
└── worker/
    └── mod.rs [~150 lines]
    └── event_loop.rs [~150 lines]
    └── message_dispatcher.rs [~100 lines]
```

#### Implementation Steps:
1. Create `server.rs` module with `initialize_server()`
2. Extract effect handlers to separate modules with trait
3. Create `worker.rs` for event loop logic
4. Move device state management to dedicated struct
5. Update tests to target new modules

#### Testing Strategy:
- Unit tests for each handler
- Integration test for server startup
- Property tests for color parsing

#### Timeline: 8 hours

---

### Module 1.2: Crawler Runner Refactoring
**File:** `src/lib/services/crawler/runner.rs`

#### Current State:
- Total: 2534 lines
- `run_crawler_service()`: 900 lines (main loop)
- `crawl_url_inner()`: 573 lines (page processing)
- Mix of concerns: Job queue, HTTP, DNS, caching, metrics

#### Target State:
```
crawler/
├── runner.rs [~200 lines] - Main entry point
├── job_processor.rs [~250 lines] - Job queue handling
├── url_crawler.rs [~300 lines] - URL fetching (refactored crawl_url_inner)
├── content_extractor.rs [~200 lines] - Content parsing
├── cache_manager.rs [~150 lines] - DNS/HTTP cache ops
├── retry_handler.rs [~100 lines] - Retry logic
├── metrics_reporter.rs [~100 lines] - Status reporting
└── dns/
    ├── resolver.rs [~100 lines]
    └── cache.rs [~80 lines]
```

#### Breaking Down `crawl_url_inner()` (573 lines → 300):
```rust
// OLD (monolithic)
async fn crawl_url_inner(url: &str) -> Result<CrawledPage> {
    // 100 lines: Header setup
    // 150 lines: HTTP request + response handling
    // 200 lines: Content parsing & extraction
    // 73 lines: Link discovery
}

// NEW (separated)
async fn crawl_url_inner(url: &str) -> Result<CrawledPage> {
    let response = fetch_page(url).await?;      // → url_crawler.rs
    let content = extract_content(&response)?;  // → content_extractor.rs
    let links = discover_links(&content)?;      // → content_extractor.rs
    Ok(CrawledPage { ... })
}
```

#### Implementation Steps:
1. Extract `process_job_queue()` to `job_processor.rs`
2. Split `crawl_url_inner()` → `fetch_page()` + `extract_content()` + `discover_links()`
3. Move cache operations to `cache_manager.rs`
4. Extract retry logic to `retry_handler.rs`
5. Create `metrics_reporter.rs` for status updates
6. Move DNS cache to separate module

#### Testing Strategy:
- Mocked HTTP client for URL crawler tests
- Fixture-based content extraction tests
- Cache manager unit tests with Redis mock
- Integration test for full job pipeline

#### Timeline: 12 hours

---

### Module 1.3: WebSocket Background Tasks
**File:** `src/lib/websocket/mod.rs`

#### Current State:
- Total: 1974 lines
- `start_background_tasks()`: 207 lines
- Message handling mixed with state management

#### Target State:
```
websocket/
├── mod.rs [~300 lines] - Public API
├── handler.rs [~250 lines] - Connection handler
├── message_processor.rs [~180 lines] - Message parsing & dispatch
├── state_manager.rs [~200 lines] - Client state
└── tasks/
    ├── mod.rs [~50 lines]
    ├── heartbeat.rs [~60 lines]
    ├── broadcast.rs [~80 lines]
    └── cleanup.rs [~70 lines]
```

#### Implementation Steps:
1. Extract message dispatch to `message_processor.rs`
2. Create `state_manager.rs` for client state
3. Move background tasks to `tasks/` module
4. Implement trait-based task scheduling
5. Add task priority queue

#### Performance Improvements:
- Message batching support
- Reduced allocation in hot loop
- Better task scheduling

#### Timeline: 8 hours

---

## Phase 2: Error Handling Overhaul (Days 4-5)

### Target Files (Priority Order):
1. `src/lib/services/crawler/runner.rs` (50 panic points)
2. `src/lib/services/lifx/lifx_api_server.rs` (35 panic points)
3. `src/lib/http/api.rs` (45 panic points)
4. `src/lib/db/connection_pool.rs` (30 panic points)
5. `src/lib/websocket/mod.rs` (25 panic points)

### Strategy:

#### Step 1: Define Custom Error Types
```rust
// errors.rs (new module in each service)
#[derive(thiserror::Error, Debug)]
pub enum CrawlerError {
    #[error("HTTP request failed: {0}")]
    HttpFailed(#[from] reqwest::Error),
    
    #[error("DNS lookup failed for {domain}: {reason}")]
    DnsLookupFailed { domain: String, reason: String },
    
    #[error("Content parsing error: {0}")]
    ParseError(String),
    
    #[error("Rate limit exceeded for {domain}, retry after {duration:?}")]
    RateLimited { domain: String, duration: Duration },
    
    #[error("Cache error: {0}")]
    CacheError(String),
}

pub type Result<T> = std::result::Result<T, CrawlerError>;
```

#### Step 2: Replace unwrap/expect
```rust
// BEFORE
let pool = DeadpoolConfig::from_url(&url)
    .create_pool(Runtime::Tokio1)
    .expect("Failed to create Redis pool");

// AFTER
let pool = DeadpoolConfig::from_url(&url)
    .create_pool(Runtime::Tokio1)
    .map_err(|e| CrawlerError::CacheError(e.to_string()))?;
```

#### Step 3: Add Context/Retry
```rust
pub async fn crawl_with_retry(url: &str, max_retries: u32) -> Result<CrawledPage> {
    let mut last_error = CrawlerError::ParseError("Not started".into());
    
    for attempt in 0..max_retries {
        match crawl_url_inner(url).await {
            Ok(page) => return Ok(page),
            Err(e) => {
                match &e {
                    CrawlerError::RateLimited { duration, .. } => {
                        sleep(*duration).await;
                    }
                    _ => {
                        if attempt < max_retries - 1 {
                            sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                        }
                    }
                }
                last_error = e;
            }
        }
    }
    
    Err(last_error)
}
```

### Timeline: 16 hours (can parallelize per-service)

---

## Phase 3: Dependency Updates (Day 6)

### Phase 3a: Patch Updates (Low Risk)
```toml
# Cargo.toml changes
serde = "1.0.207"              # 1.0.133 → latest stable
serde_derive = "1.0.207"       # Keep in sync
chrono = "0.4.39"              # 0.4 → latest patch
rustls = "0.23.14"             # 0.23 → latest patch
rustls-pemfile = "2.0.1"       # Latest patch
ring = "0.17.8"                # Latest patch
```

**Verification:**
```bash
# 1. Update Cargo.lock
cargo update serde serde_derive chrono rustls

# 2. Check for compilation issues
cargo build --lib

# 3. Run tests
cargo test

# 4. Run clippy
cargo clippy --all-targets
```

### Phase 3b: Minor Version Updates (Testing Required)
```toml
tokio = "1.43"                 # 1.42 → 1.43
reqwest = "0.12.5"             # 0.12 → latest
git2 = "0.19.1"                # Latest 0.19.x
```

**Process:**
1. Update one dependency at a time
2. Run full test suite
3. Check for breaking changes in changelog
4. Document any behavior changes

### Phase 3c: Major Version Candidates (Deferred)
```toml
# For next sprint - requires code changes:
# postgres = "0.20"     - Check deadpool-postgres compatibility
# Consider: sqlx = "0.8" - Compile-time query safety
```

### Timeline: 4 hours

---

## Phase 4: Performance Optimization (Days 7-8)

### 4.1 WebSocket Message Batching
**File:** `src/lib/websocket/mod.rs:215`

```rust
// BEFORE: Process one message at a time
loop {
    if let Some(msg) = receiver.recv().await {
        let parsed = serde_json::from_str::<Message>(&msg)?;
        process(&parsed).await;
    }
}

// AFTER: Batch processing
let mut batch = Vec::with_capacity(16);
loop {
    while let Ok(msg) = receiver.try_recv() {
        batch.push(msg);
        if batch.len() >= 16 { break; }
    }
    
    if batch.is_empty() {
        if let Some(msg) = receiver.recv().await {
            batch.push(msg);
        } else {
            break; // Channel closed
        }
    }
    
    // Process entire batch
    for msg in batch.drain(..) {
        if let Ok(parsed) = serde_json::from_str::<Message>(&msg) {
            process(&parsed).await;
        }
    }
}
```

**Expected: +35% throughput for burst traffic**

### 4.2 Crawler Content Streaming
**File:** `src/lib/services/crawler/runner.rs` (content_extractor.rs)

```rust
// BEFORE: Load entire page into memory
let text = String::from_utf8(response.bytes().await?)?;

// AFTER: Stream and truncate
async fn extract_text_streaming(
    body: reqwest::body::Body,
) -> Result<String> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    let mut buffer = String::with_capacity(64 * 1024);
    let mut stream = body.into_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        
        if buffer.len() > MAX_SIZE {
            break; // Truncate large documents
        }
    }
    
    Ok(buffer)
}
```

**Expected: +50% throughput, -40% memory**

### 4.3 Database Query Optimization
**Files:** `src/lib/db/`, `src/lib/memory/config/mod.rs`

```sql
-- Add missing indexes
CREATE INDEX idx_crawled_pages_url ON crawled_pages(url);
CREATE INDEX idx_crawled_pages_domain ON crawled_pages(domain);
CREATE INDEX idx_crawled_pages_timestamp ON crawled_pages(crawl_timestamp DESC);
CREATE INDEX idx_observations_time ON observations(timestamp DESC);
```

**Code changes:**
```rust
// Use prepared statements consistently
const QUERY_BY_URL: &str = "SELECT * FROM crawled_pages WHERE url = $1 LIMIT 1";

pub async fn get_by_url(pool: &Pool, url: &str) -> Result<Option<CrawledPage>> {
    let stmt = pool.prepare_cached(QUERY_BY_URL).await?;
    let row = pool.query_opt(&stmt, &[&url]).await?;
    row.map(|r| CrawledPage::from_row(&r)).transpose()
}
```

**Expected: +40% query performance for common patterns**

### Timeline: 8 hours

---

## Phase 5: Consolidation & Documentation (Day 9)

### 5.1 Create Migration Guides
- Document before/after patterns
- Explain new module organization
- Provide examples for new developers

### 5.2 Update Code Comments
- Add module-level documentation
- Document new error types
- Explain optimization decisions

### 5.3 Performance Benchmarks
```bash
# Before refactoring
cargo bench
# Record baseline

# After refactoring
cargo bench
# Compare results
```

### Timeline: 4 hours

---

## Parallel Work Streams

### Stream A: Function Extraction (Days 1-3)
- ✅ LIFX API server
- ✅ Crawler runner
- ✅ WebSocket tasks
- **Owner:** Backend specialist

### Stream B: Error Handling (Days 4-5)
- ✅ Custom error types
- ✅ Replace unwrap/expect
- ✅ Add retry logic
- **Owner:** Platform engineer

### Stream C: Dependencies (Day 6)
- ✅ Patch updates
- ✅ Minor version bumps
- ✅ Test compatibility
- **Owner:** DevOps/Platform

### Stream D: Performance (Days 7-8)
- ✅ WebSocket optimization
- ✅ Streaming improvements
- ✅ Database optimization
- **Owner:** Perf specialist

### Stream E: Documentation (Day 9)
- ✅ Migration guides
- ✅ Code comments
- ✅ Benchmark results
- **Owner:** Tech lead

---

## Success Criteria

### Code Quality:
- [ ] All functions < 400 lines (Tier 1) or < 250 lines (others)
- [ ] 0 new unwrap/expect in production code paths
- [ ] 80%+ test coverage for refactored modules
- [ ] All TODOs tracked in issue system

### Performance:
- [ ] WebSocket throughput +30%
- [ ] Crawler throughput +40%
- [ ] Database queries -30% latency
- [ ] Memory footprint -20%

### Dependencies:
- [ ] All critical patches applied
- [ ] No high-severity CVEs
- [ ] Build time < 5 minutes (from clean)

### Documentation:
- [ ] All modules documented with examples
- [ ] Migration guide for developers
- [ ] Benchmark results published

---

## Timeline Summary

```
Day 1:  LIFX API Server refactoring        [8 hours]
Day 2:  Crawler refactoring                [12 hours]
Day 3:  WebSocket refactoring + setup      [8 hours]
        TOTAL Stream A: 28 hours

Day 4:  Error type definition              [6 hours]
Day 5:  Unwrap/expect replacement          [10 hours]
        TOTAL Stream B: 16 hours

Day 6:  Dependency updates & testing       [4 hours]
        TOTAL Stream C: 4 hours

Day 7:  WebSocket optimization            [4 hours]
Day 8:  Crawler + DB optimization         [4 hours]
        TOTAL Stream D: 8 hours

Day 9:  Migration guides & docs           [4 hours]
        TOTAL Stream E: 4 hours

GRAND TOTAL: 60 hours (7.5 days) for 1 developer
             OR 2 weeks with standard 4-hour work days
```

---

## Risk Mitigation

### Risk: Regression in existing functionality
**Mitigation:**
- Maintain comprehensive integration tests
- Use feature flags for gradual rollout
- Parallel testing (old vs new implementations)

### Risk: Performance regression
**Mitigation:**
- Benchmark before/after
- Keep baseline results
- Rollback plan if performance degrades

### Risk: Breaking API changes
**Mitigation:**
- Version new modules with v2 in name
- Maintain backward compatibility layers
- Document deprecation path

---

## Next Steps

1. **Approval:** Review this roadmap with team
2. **Planning:** Assign owners to each stream
3. **Implementation:** Start Phase 1 (Function Extraction)
4. **Tracking:** Use project board for progress
5. **Review:** Weekly sync on blockers

---

**Status:** Ready for execution ✅  
**Last Updated:** 2026-04-02 10:47 UTC  
**Prepared by:** Worker 2 (Code Quality Specialist)
