# Cache Implementation with Invalidation Strategy

## Overview

A comprehensive hybrid caching solution has been implemented to address critical caching issues including cache stampede vulnerability, lack of invalidation, distributed locking issues, memory leaks, and missing metrics.

## Implementation Summary

### 1. **Hybrid Cache Architecture** (`src/sam/services/cache.rs`)
- **Two-tier caching**: Memory (LRU) + Redis
- **Cache-aside pattern** with automatic fallback
- **Distributed locking** to prevent cache stampede
- **TTL-based expiration** with configurable durations
- **Background refresh** when items approach expiration (80% threshold)

### 2. **Key Features Implemented**

#### Cache Stampede Prevention
```rust
// Distributed lock prevents multiple clients from hitting DB simultaneously
let lock_acquired = self.acquire_lock(&lock_key).await?;
if !lock_acquired {
    // Wait and retry from cache
    tokio::time::sleep(Duration::from_millis(100)).await;
    return self.get_without_load(key).await;
}
```

#### Memory Cache with LRU Eviction
```rust
// LRU cache automatically evicts least recently used items
memory_cache: Arc::new(RwLock::new(LruCache::new(memory_size)))
```

#### Cache Invalidation with Pub/Sub
```rust
// Pattern-based invalidation
pub async fn invalidate(&self, pattern: &str) -> Result<()>
// Single key invalidation
pub async fn invalidate_key(&self, key: &str) -> Result<()>
// Pub/sub for distributed invalidation
self.listen_for_invalidations().await
```

#### Comprehensive Metrics
- Cache hit/miss counters
- Load duration histograms
- Memory cache size gauge
- Redis operation latency tracking
- Eviction counters

### 3. **Integration Points** (`src/sam/memory/cache/hybrid_wrapper.rs`)

Specialized cache wrappers for different use cases:
- **WebCrawlCache**: For web crawling results (1 hour TTL)
- **WikipediaCache**: For Wikipedia summaries (24 hour TTL)
- **SessionCache**: For session data (30 minute TTL)

### 4. **Configuration Options**

```rust
pub struct CacheConfig {
    pub memory_size: usize,        // LRU cache size (default: 1000)
    pub default_ttl: u64,          // Default TTL in seconds (default: 600)
    pub lock_ttl: u64,             // Lock TTL in seconds (default: 30)
    pub refresh_threshold: f64,    // Refresh when X% of TTL passed (default: 0.8)
    pub enable_metrics: bool,      // Enable Prometheus metrics (default: true)
    pub enable_warming: bool,      // Enable cache warming (default: false)
}
```

## Usage Examples

### Basic Usage
```rust
use sam::sam::services::redis;

// Create cache with default config
let cache = redis::create_cache().await?;

// Get or load data
let value = cache.get_or_load("my_key", async {
    // Expensive operation
    fetch_from_database().await
}, Some(3600)).await?;
```

### Pattern-based Invalidation
```rust
// Invalidate all session keys
cache.invalidate("session:").await?;

// Invalidate specific key
cache.invalidate_key("user:123").await?;
```

### Cache Warming
```rust
let keys = vec!["popular:1", "popular:2", "popular:3"];
cache.warm_cache(keys, |key| {
    Box::pin(async move {
        fetch_data_for_key(&key).await
    })
}).await?;
```

## Testing

Comprehensive test suite in `tests/cache_integration_tests.rs`:
- Basic operations
- Expiration handling
- Stampede prevention
- Pattern invalidation
- LRU eviction
- Concurrent access
- Error handling

## Performance Improvements

1. **Reduced Database Load**: Cache-aside pattern prevents unnecessary DB queries
2. **No Cache Stampede**: Distributed locking ensures only one loader per key
3. **Memory Efficiency**: LRU eviction prevents unbounded memory growth
4. **Optimized Access**: Two-tier cache minimizes network calls
5. **Background Refresh**: Proactive refresh reduces perceived latency

## Monitoring

The cache exposes Prometheus metrics:
- `cache_hits_total`: Total number of cache hits
- `cache_misses_total`: Total number of cache misses
- `cache_evictions_total`: Total number of evictions
- `cache_load_duration_seconds`: Time to load from source
- `memory_cache_size`: Current memory cache size
- `redis_cache_operation_duration_seconds`: Redis operation latency

## Migration Guide

To migrate existing code to use the new cache:

1. Replace direct Redis calls with cache operations:
```rust
// Old
let value = redis_get(key).await?;

// New
let cache = redis::create_cache().await?;
let value = cache.get_or_load(key, loader_fn, ttl).await?;
```

2. Use specialized wrappers for domain-specific caching:
```rust
let web_cache = WebCrawlCache::new().await?;
let content = web_cache.get_or_fetch(url, fetch_fn).await?;
```

3. Implement invalidation on updates:
```rust
// After updating data
cache.invalidate_key(&format!("user:{}", user_id)).await?;
```

## Dependencies

Added dependencies:
- `lru = "0.12"` - LRU cache implementation
- Existing: `deadpool-redis`, `prometheus`, `futures`

## Future Enhancements

1. **Circuit Breaker**: Add circuit breaker for source failures
2. **Adaptive TTL**: Adjust TTL based on access patterns
3. **Compression**: Compress large cached values
4. **Cache Tagging**: Tag-based invalidation for related keys
5. **Write-through Cache**: Support write-through pattern for critical data