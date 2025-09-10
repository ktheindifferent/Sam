//! Memory-optimized structures for crawler URL management
//! 
//! This module provides efficient data structures for storing and managing
//! large numbers of URLs during crawling sessions without unbounded memory growth.

use anyhow::{Result, Context};
use bloomfilter::Bloom;
use lru::LruCache;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{debug, info, warn};

/// Memory-efficient URL tracker using Bloom filter for visited URLs
/// and LRU cache for recent URLs
pub struct OptimizedUrlTracker {
    /// Bloom filter for fast "probably visited" checks
    bloom: Arc<RwLock<Bloom<String>>>,
    
    /// LRU cache for recent URLs (to handle false positives)
    recent_cache: Arc<RwLock<LruCache<String, bool>>>,
    
    /// Configuration
    expected_urls: usize,
    false_positive_rate: f64,
    cache_size: NonZeroUsize,
}

impl OptimizedUrlTracker {
    /// Create a new optimized URL tracker
    pub fn new(expected_urls: usize, cache_size: usize) -> Self {
        let false_positive_rate = 0.01; // 1% false positive rate
        let bloom = Bloom::new_for_fp_rate(expected_urls, false_positive_rate);
        
        let cache_size = NonZeroUsize::new(cache_size.max(1000))
            .expect("Cache size must be non-zero");
        
        Self {
            bloom: Arc::new(RwLock::new(bloom)),
            recent_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            expected_urls,
            false_positive_rate,
            cache_size,
        }
    }
    
    /// Check if a URL has been visited
    pub async fn has_visited(&self, url: &str) -> bool {
        // First check the LRU cache
        {
            let mut cache = self.recent_cache.write().await;
            if let Some(&visited) = cache.get(url) {
                return visited;
            }
        }
        
        // Then check the Bloom filter
        let bloom = self.bloom.read().await;
        bloom.check(&url.to_string())
    }
    
    /// Mark a URL as visited
    pub async fn mark_visited(&self, url: String) {
        // Add to Bloom filter
        {
            let mut bloom = self.bloom.write().await;
            bloom.set(&url);
        }
        
        // Add to recent cache
        {
            let mut cache = self.recent_cache.write().await;
            cache.put(url, true);
        }
    }
    
    /// Get memory usage estimate in bytes
    pub async fn memory_usage(&self) -> usize {
        let bloom = self.bloom.read().await;
        let cache = self.recent_cache.read().await;
        
        // Bloom filter size + LRU cache estimate
        let bloom_size = (bloom.number_of_bits() / 8) as usize; // Convert bits to bytes
        let cache_size = cache.len() * 256; // Estimate 256 bytes per URL
        
        bloom_size + cache_size
    }
    
    /// Clear all tracked URLs
    pub async fn clear(&self) {
        let mut bloom = self.bloom.write().await;
        let mut cache = self.recent_cache.write().await;
        
        bloom.clear();
        cache.clear();
    }
}

/// Memory-bounded URL queue with spillover to disk
pub struct BoundedUrlQueue {
    /// In-memory queue (limited size)
    memory_queue: Arc<RwLock<VecDeque<(String, usize)>>>,
    
    /// Maximum items in memory
    max_memory_items: usize,
    
    /// Items spilled to Redis when memory limit exceeded
    redis_queue_key: String,
    
    /// Redis connection pool (optional, falls back to memory-only if unavailable)
    redis_pool: Option<deadpool_redis::Pool>,
}

impl BoundedUrlQueue {
    /// Create a new bounded URL queue
    pub async fn new(max_memory_items: usize, queue_name: &str) -> Result<Self> {
        let redis_queue_key = format!("sam:crawler:queue:{}", queue_name);
        
        // Try to connect to Redis for spillover
        let redis_pool = match crate::services::redis::connect().await {
            Ok(pool) => {
                info!("Connected to Redis for URL queue spillover");
                Some(pool)
            }
            Err(e) => {
                warn!("Failed to connect to Redis, using memory-only queue: {}", e);
                None
            }
        };
        
        Ok(Self {
            memory_queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_memory_items))),
            max_memory_items,
            redis_queue_key,
            redis_pool,
        })
    }
    
    /// Push a URL to the queue
    pub async fn push(&self, url: String, depth: usize) -> Result<()> {
        let mut queue = self.memory_queue.write().await;
        
        // If memory queue is full, spill to Redis
        if queue.len() >= self.max_memory_items {
            if let Some(pool) = &self.redis_pool {
                // Spill half of the queue to Redis
                let spill_count = queue.len() / 2;
                let mut items_to_spill = Vec::with_capacity(spill_count);
                
                for _ in 0..spill_count {
                    if let Some(item) = queue.pop_front() {
                        items_to_spill.push(item);
                    }
                }
                
                // Push to Redis
                if !items_to_spill.is_empty() {
                    let mut conn = pool.get().await
                        .context("Failed to get Redis connection for spillover")?;
                    
                    for (url, depth) in items_to_spill {
                        let item = format!("{}|{}", url, depth);
                        deadpool_redis::redis::cmd("RPUSH")
                            .arg(&self.redis_queue_key)
                            .arg(item)
                            .query_async::<()>(&mut conn)
                            .await
                            .context("Failed to push URL to Redis spillover")?;
                    }
                    
                    debug!("Spilled {} URLs to Redis", spill_count);
                }
            } else {
                // No Redis, just warn when queue is getting full
                if queue.len() == self.max_memory_items {
                    warn!("URL queue at maximum capacity ({}), new URLs may be dropped", self.max_memory_items);
                }
            }
        }
        
        queue.push_back((url, depth));
        Ok(())
    }
    
    /// Pop a URL from the queue
    pub async fn pop(&self) -> Result<Option<(String, usize)>> {
        let mut queue = self.memory_queue.write().await;
        
        // First try to get from memory queue
        if let Some(item) = queue.pop_front() {
            return Ok(Some(item));
        }
        
        // If memory queue is empty, try to load from Redis
        if let Some(pool) = &self.redis_pool {
            let mut conn = pool.get().await
                .context("Failed to get Redis connection")?;
            
            // Load a batch from Redis
            let batch_size = self.max_memory_items / 4; // Load 25% of capacity
            let items: Vec<String> = deadpool_redis::redis::cmd("LPOP")
                .arg(&self.redis_queue_key)
                .arg(batch_size)
                .query_async(&mut conn)
                .await
                .unwrap_or_default();
            
            // Parse and add to memory queue
            for item in items {
                if let Some((url, depth_str)) = item.rsplit_once('|') {
                    if let Ok(depth) = depth_str.parse::<usize>() {
                        queue.push_back((url.to_string(), depth));
                    }
                }
            }
            
            // Return the first item if we loaded any
            Ok(queue.pop_front())
        } else {
            Ok(None)
        }
    }
    
    /// Get the approximate size of the queue
    pub async fn len(&self) -> usize {
        let memory_len = self.memory_queue.read().await.len();
        
        // Add Redis queue length if available
        if let Some(pool) = &self.redis_pool {
            if let Ok(mut conn) = pool.get().await {
                if let Ok(redis_len) = deadpool_redis::redis::cmd("LLEN")
                    .arg(&self.redis_queue_key)
                    .query_async::<usize>(&mut conn)
                    .await
                {
                    return memory_len + redis_len;
                }
            }
        }
        
        memory_len
    }
    
    /// Check if the queue is empty
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
    
    /// Clear the queue
    pub async fn clear(&self) -> Result<()> {
        self.memory_queue.write().await.clear();
        
        // Clear Redis queue if available
        if let Some(pool) = &self.redis_pool {
            let mut conn = pool.get().await
                .context("Failed to get Redis connection")?;
            
            deadpool_redis::redis::cmd("DEL")
                .arg(&self.redis_queue_key)
                .query_async::<()>(&mut conn)
                .await
                .context("Failed to clear Redis queue")?;
        }
        
        Ok(())
    }
}

/// Configuration for memory-optimized crawling
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum URLs to keep in memory
    pub max_memory_urls: usize,
    
    /// Expected total URLs for Bloom filter sizing
    pub expected_total_urls: usize,
    
    /// LRU cache size for recent URLs
    pub recent_cache_size: usize,
    
    /// Maximum items in URL queue before spillover
    pub max_queue_items: usize,
    
    /// Batch size for database saves
    pub db_batch_size: usize,
    
    /// Maximum pages to keep in memory before saving
    pub max_pages_in_memory: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_urls: 10_000,
            expected_total_urls: 1_000_000,
            recent_cache_size: 5_000,
            max_queue_items: 50_000,
            db_batch_size: 100,
            max_pages_in_memory: 500,
        }
    }
}

impl MemoryConfig {
    /// Create a configuration for small memory footprint
    pub fn small() -> Self {
        Self {
            max_memory_urls: 1_000,
            expected_total_urls: 100_000,
            recent_cache_size: 500,
            max_queue_items: 5_000,
            db_batch_size: 50,
            max_pages_in_memory: 100,
        }
    }
    
    /// Create a configuration for large crawls
    pub fn large() -> Self {
        Self {
            max_memory_urls: 100_000,
            expected_total_urls: 10_000_000,
            recent_cache_size: 50_000,
            max_queue_items: 500_000,
            db_batch_size: 500,
            max_pages_in_memory: 5_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_url_tracker() {
        let tracker = OptimizedUrlTracker::new(1000, 100);
        
        // Test marking and checking URLs
        let url = "https://example.com";
        assert!(!tracker.has_visited(url).await);
        
        tracker.mark_visited(url.to_string()).await;
        assert!(tracker.has_visited(url).await);
        
        // Test memory usage
        let usage = tracker.memory_usage().await;
        assert!(usage > 0);
    }
    
    #[tokio::test]
    async fn test_bounded_queue() {
        let queue = BoundedUrlQueue::new(10, "test").await.unwrap();
        
        // Test push and pop
        queue.push("https://example.com".to_string(), 0).await.unwrap();
        
        let item = queue.pop().await.unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap().0, "https://example.com");
        
        // Test empty queue
        assert!(queue.is_empty().await);
        assert!(queue.pop().await.unwrap().is_none());
    }
}