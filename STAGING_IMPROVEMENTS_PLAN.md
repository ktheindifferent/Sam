# SAM Code Quality - Staging Improvements
**Status:** Ready for Integration  
**Priority:** MEDIUM  
**Effort:** Phased over 2-3 sprints

---

## Summary

This document outlines staged improvements identified during Worker 2 code quality analysis. Improvements are organized by urgency and can be implemented incrementally without blocking features.

---

## Stage 1: Immediate Code Quality Fixes (Ready Now)

### 1.1 Error Handling Template
**Target:** `src/lib/errors.rs` (new file)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SamError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SamError>;

// Add to lib/mod.rs
pub mod errors;
pub use errors::{SamError, Result};
```

**Implementation Time:** 30 minutes

---

### 1.2 Replace Top 20 Panic Points in `services/`

**High-Impact Files:**
1. `src/lib/services/crawler/runner.rs` - 5 replacements
2. `src/lib/services/lifx/lifx_api_server.rs` - 5 replacements  
3. `src/lib/http/api.rs` - 5 replacements
4. `src/lib/db/connection_pool.rs` - 5 replacements

**Pattern:**
```rust
// BEFORE
let value = data.get("key").expect("missing key");

// AFTER
let value = data.get("key")
    .ok_or_else(|| SamError::Config("missing key".into()))?;
```

**Implementation Time:** 2 hours

---

### 1.3 Add Request Timeouts

**Target:** `src/lib/http/api.rs`

```rust
// Current
let response = client.get(&url).send().await?;

// Improved
let response = client
    .get(&url)
    .timeout(Duration::from_secs(30))  // Add timeout
    .send()
    .await
    .map_err(|e| SamError::Http(e))?;
```

**Implementation Time:** 1 hour

---

### 1.4 Document TODOs as Issues

**Action:** Convert 80+ TODO comments to GitHub issues

```bash
# Extract TODOs to CSV for tracking
grep -r "TODO\|FIXME\|HACK" src/ | \
  awk -F: '{print $1":"$2" - "$3}' | \
  sort > TODO_ISSUES.txt

# Create issues from this list
# Example issue title: "TODO: Enforce max sessions per user [src/lib/security/session.rs:52]"
```

**Implementation Time:** 30 minutes

---

## Stage 2: Function Refactoring (Week 1-2)

### 2.1 LIFX API Server Modularization

**Target:** Extract `start()` (965 lines → ~300 lines)

```
src/lib/services/lifx/
├── mod.rs              # Public API
├── server.rs (new)     # Server initialization [100 lines]
├── handlers.rs         # Device control handlers [150 lines]
├── effects/            # Effect implementations (new module)
│   ├── mod.rs
│   ├── rainbow.rs
│   ├── breathe.rs
│   └── pulse.rs
└── worker.rs (new)     # Event loop [150 lines]
```

**Testing:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_server_initialization() { }
    
    #[test]
    fn test_bulb_discovery() { }
    
    #[test]
    fn test_color_parsing() { }
    
    #[tokio::test]
    async fn test_effect_execution() { }
}
```

**Implementation Time:** 8 hours

---

### 2.2 Crawler Runner Refactoring

**Target:** Split 2534 lines into focused modules

```
src/lib/services/crawler/
├── mod.rs              # Public API
├── runner.rs           # Main entry point [200 lines]
├── job_processor.rs    # Queue management [250 lines]
├── url_crawler.rs      # URL fetching [300 lines]
├── content.rs          # Content extraction [250 lines]
├── cache/              # Cache operations (new)
│   ├── mod.rs
│   ├── redis.rs
│   └── file.rs
└── retry.rs            # Retry logic [100 lines]
```

**Key Extractions:**
```rust
// Move this to url_crawler.rs
async fn crawl_url_inner(url: &str) -> Result<CrawledPage> { ... }

// Move this to content.rs
fn extract_text(html: &str) -> String { ... }
async fn lookup_domain(domain: &str) -> Result<DomainInfo> { ... }

// Move to cache/mod.rs
async fn load_dns_cache(should_use_redis: bool) -> () { ... }
async fn save_dns_cache() { ... }
```

**Implementation Time:** 12 hours

---

### 2.3 WebSocket Task Reorganization

**Target:** Split 1974 lines into focused modules

```
src/lib/websocket/
├── mod.rs              # Public API [300 lines]
├── handler.rs          # Connection handler [250 lines]
├── message.rs          # Message processing [180 lines]
├── state.rs            # Client state [200 lines]
└── tasks/              # Background tasks (new)
    ├── mod.rs
    ├── heartbeat.rs
    ├── broadcast.rs
    └── cleanup.rs
```

**Implementation Time:** 8 hours

---

## Stage 3: Performance Optimization (Week 2-3)

### 3.1 WebSocket Message Batching

**File:** `src/lib/websocket/message.rs`

**Improvement:** Process messages in batches instead of one-at-a-time

```rust
pub struct MessageBatcher {
    batch: Vec<Message>,
    capacity: usize,
    flush_timeout: Duration,
}

impl MessageBatcher {
    pub fn new(capacity: usize) -> Self {
        Self {
            batch: Vec::with_capacity(capacity),
            capacity,
            flush_timeout: Duration::from_millis(50),
        }
    }
    
    pub async fn process_and_batch<F>(&mut self, msg: Message, handler: F) -> Result<()>
    where
        F: Fn(Vec<Message>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
    {
        self.batch.push(msg);
        if self.batch.len() >= self.capacity {
            let batch = std::mem::take(&mut self.batch);
            self.batch = Vec::with_capacity(self.capacity);
            handler(batch).await?;
        }
        Ok(())
    }
}
```

**Expected Gain:** +35% throughput

**Implementation Time:** 4 hours

---

### 3.2 Crawler Content Streaming

**File:** `src/lib/services/crawler/content.rs`

**Improvement:** Stream large documents instead of loading entirely

```rust
pub async fn extract_text_streaming(
    body: reqwest::body::Body,
    max_size: usize,
) -> Result<String> {
    use futures::stream::StreamExt;
    
    let mut buffer = String::with_capacity(64 * 1024);
    let mut stream = body.into_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| SamError::Http(e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        
        if buffer.len() > max_size {
            warn!("Document size exceeded limit, truncating");
            buffer.truncate(max_size);
            break;
        }
    }
    
    Ok(buffer)
}
```

**Expected Gain:** +50% throughput, -40% memory

**Implementation Time:** 3 hours

---

### 3.3 Database Query Optimization

**File:** `src/lib/db/migrations/v004_add_indexes.rs` (new)

```sql
-- Add missing indexes
CREATE INDEX IF NOT EXISTS idx_crawled_pages_url 
    ON crawled_pages(url);

CREATE INDEX IF NOT EXISTS idx_crawled_pages_domain 
    ON crawled_pages(domain);

CREATE INDEX IF NOT EXISTS idx_crawled_pages_timestamp 
    ON crawled_pages(crawl_timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_observations_time 
    ON observations(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_observations_human 
    ON observations(human_id);

-- Analyze tables for query planning
ANALYZE crawled_pages;
ANALYZE observations;
ANALYZE sessions;
```

**Expected Gain:** +40% query performance

**Implementation Time:** 2 hours (plus DB deployment)

---

## Stage 4: Dependency Management (Week 3)

### 4.1 Dependency Audit Script

**Create:** `scripts/check_deps.sh`

```bash
#!/bin/bash
set -e

echo "🔍 Checking dependency vulnerabilities..."
cargo audit 2>/dev/null || echo "⚠️  cargo-audit not installed"

echo "🔍 Checking outdated packages..."
cargo outdated 2>/dev/null || echo "⚠️  cargo-outdated not installed"

echo "🔍 Checking for unused dependencies..."
cargo tree --duplicates 2>/dev/null | head -20

echo "✅ Dependency check complete"
```

**Implementation Time:** 30 minutes

---

### 4.2 Staged Dependency Updates

**Phase 1 - Patch Updates (No breaking changes):**
```toml
serde = "1.0.207"              # ← 1.0.133
serde_derive = "1.0.207"       # ← 1.0.130
chrono = "0.4.39"              # ← 0.4 (latest)
rustls = "0.23.14"             # ← 0.23 (latest)
```

**Phase 2 - Minor Updates (After testing):**
```toml
tokio = "1.43"                 # ← 1.42 (safe)
reqwest = "0.12.5"             # ← 0.12 (safe)
git2 = "0.19.1"                # ← 0.19 (safe)
```

**Implementation Time:** 4 hours (phased)

---

## Stage 5: Documentation & Testing (Week 3-4)

### 5.1 Module Documentation

**Template for each refactored module:**

```rust
//! # Module Name
//!
//! Brief description of module purpose
//!
//! ## Features
//! - Feature 1
//! - Feature 2
//!
//! ## Example
//! ```no_run
//! use sam::module::function;
//!
//! async fn example() -> Result<()> {
//!     let result = function(args).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//! This module returns [SamError] on failure. Common error cases:
//! - ...
```

**Implementation Time:** 4 hours

---

### 5.2 Test Coverage Expansion

**Add tests for refactored modules:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Mock};
    
    #[tokio::test]
    async fn test_module_initialization() {
        let module = Module::new().unwrap();
        assert!(module.is_healthy());
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let result = function_that_fails().await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parsing() {
        let input = "test data";
        let output = parse(input).unwrap();
        assert_eq!(output.field, expected);
    }
}
```

**Target Coverage:** 80% for new/refactored modules

**Implementation Time:** 8 hours

---

## Implementation Checklist

### Phase 1: Ready Now ✅
- [ ] Create error.rs with SamError type
- [ ] Replace top 20 unwrap/expect calls
- [ ] Add timeouts to HTTP clients
- [ ] Extract TODOs to issue tracker
- [ ] **Est. Time:** 4 hours

### Phase 2: Week 1-2 ⏳
- [ ] Refactor LIFX API server (8h)
- [ ] Refactor crawler runner (12h)
- [ ] Refactor WebSocket tasks (8h)
- [ ] Add comprehensive tests (4h)
- [ ] **Est. Time:** 32 hours

### Phase 3: Week 2-3 ⏳
- [ ] WebSocket message batching (4h)
- [ ] Crawler content streaming (3h)
- [ ] Database indexes migration (2h)
- [ ] Performance benchmark (2h)
- [ ] **Est. Time:** 11 hours

### Phase 4: Week 3 ⏳
- [ ] Update dependencies (Phase 1) (2h)
- [ ] Test compatibility (2h)
- [ ] Update dependencies (Phase 2) (1h)
- [ ] Final testing (1h)
- [ ] **Est. Time:** 6 hours

### Phase 5: Week 4 ⏳
- [ ] Module documentation (4h)
- [ ] API documentation (2h)
- [ ] Migration guide (2h)
- [ ] Benchmark results (1h)
- [ ] **Est. Time:** 9 hours

---

## Total Effort

```
Phase 1: 4 hours   (Immediate)
Phase 2: 32 hours  (Week 1-2)
Phase 3: 11 hours  (Week 2-3)
Phase 4: 6 hours   (Week 3)
Phase 5: 9 hours   (Week 4)
─────────────────
TOTAL:   62 hours  (~2 weeks full-time or 4-5 weeks part-time)
```

---

## Success Metrics

### Code Quality:
- ✅ All functions < 400 lines (exceptions documented)
- ✅ 0 new panic points in new code
- ✅ 80%+ test coverage for refactored code
- ✅ All TODOs tracked in issues

### Performance:
- ✅ WebSocket: +30% throughput
- ✅ Crawler: +40% throughput  
- ✅ Database: -30% latency
- ✅ Memory: -20% footprint

### Maintainability:
- ✅ Clear module responsibilities
- ✅ Comprehensive documentation
- ✅ Migration guides for developers
- ✅ Performance benchmarks

---

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Regression in features | HIGH | Comprehensive integration tests, rollback plan |
| Performance regression | HIGH | Benchmark before/after, gradual deployment |
| Breaking API changes | MEDIUM | Feature flags, backward compatibility layer |
| Build time increase | LOW | Profile build, optimize link-time optimization |
| Team knowledge gap | MEDIUM | Code reviews, documentation, pair programming |

---

## Next Steps

1. **Approval:** Get buy-in from team/leads
2. **Scheduling:** Plan phases into sprint calendar
3. **Resources:** Assign owners to parallel streams
4. **Kickoff:** Start with Phase 1 (quick wins)
5. **Tracking:** Use project board + weekly syncs

---

**Status:** Ready for Implementation ✅  
**Prepared by:** Worker 2 (Features & Refactoring)  
**Date:** 2026-04-02 10:47 UTC
