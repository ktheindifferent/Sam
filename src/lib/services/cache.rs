use anyhow::{Result, Context};
use deadpool_redis::{Pool, redis::AsyncCommands};
use deadpool_redis::redis::{aio::PubSub, Script};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{info, warn, error, debug};
use prometheus::{Counter, Histogram, IntGauge, register_counter, register_histogram, register_int_gauge};
use lazy_static::lazy_static;
use std::num::NonZeroUsize;
use lru::LruCache;
use tokio::sync::Mutex;
use std::future::Future;
use std::fmt::Debug;
use futures::StreamExt;

const DEFAULT_CACHE_TTL: u64 = 600; // 10 minutes
const DEFAULT_LOCK_TTL: u64 = 30; // 30 seconds
const DEFAULT_MEMORY_SIZE: usize = 1000;
const REFRESH_THRESHOLD: f64 = 0.8; // Refresh when 80% of TTL has passed

lazy_static! {
    static ref CACHE_HIT_COUNTER: Counter = register_counter!(
        "cache_hits_total",
        "Total number of cache hits"
    ).unwrap();
    
    static ref CACHE_MISS_COUNTER: Counter = register_counter!(
        "cache_misses_total",
        "Total number of cache misses"
    ).unwrap();
    
    static ref CACHE_EVICTION_COUNTER: Counter = register_counter!(
        "cache_evictions_total",
        "Total number of cache evictions"
    ).unwrap();
    
    static ref CACHE_LOAD_DURATION: Histogram = register_histogram!(
        "cache_load_duration_seconds",
        "Time taken to load data from source"
    ).unwrap();
    
    static ref MEMORY_CACHE_SIZE: IntGauge = register_int_gauge!(
        "memory_cache_size",
        "Current size of memory cache"
    ).unwrap();
    
    static ref REDIS_CACHE_OPS: Histogram = register_histogram!(
        "redis_cache_operation_duration_seconds",
        "Time taken for Redis cache operations"
    ).unwrap();
}

/// Returns the current time as seconds since UNIX_EPOCH, or 0 on clock error.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedItem<T> {
    pub value: T,
    pub expires_at: u64,
    pub created_at: u64,
    pub access_count: u64,
    pub last_accessed: u64,
}

impl<T> CachedItem<T> {
    fn new(value: T, ttl: u64) -> Self {
        let now = now_secs();

        Self {
            value,
            expires_at: now + ttl,
            created_at: now,
            access_count: 0,
            last_accessed: now,
        }
    }

    fn is_expired(&self) -> bool {
        now_secs() >= self.expires_at
    }

    fn should_refresh(&self, threshold: f64) -> bool {
        let now = now_secs();
        let age = now - self.created_at;
        let ttl = self.expires_at - self.created_at;
        age as f64 >= ttl as f64 * threshold
    }

    fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = now_secs();
    }
}

pub struct CacheConfig {
    pub memory_size: usize,
    pub default_ttl: u64,
    pub lock_ttl: u64,
    pub refresh_threshold: f64,
    pub enable_metrics: bool,
    pub enable_warming: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            memory_size: DEFAULT_MEMORY_SIZE,
            default_ttl: DEFAULT_CACHE_TTL,
            lock_ttl: DEFAULT_LOCK_TTL,
            refresh_threshold: REFRESH_THRESHOLD,
            enable_metrics: true,
            enable_warming: false,
        }
    }
}

pub struct HybridCache {
    redis_pool: Arc<Pool>,
    memory_cache: Arc<RwLock<LruCache<String, Vec<u8>>>>,
    locks: Arc<Mutex<HashMap<String, tokio::time::Instant>>>,
    config: CacheConfig,
    invalidation_tx: tokio::sync::broadcast::Sender<String>,
    invalidation_rx: Arc<Mutex<tokio::sync::broadcast::Receiver<String>>>,
}

impl HybridCache {
    pub async fn new(redis_pool: Pool, config: CacheConfig) -> Result<Self> {
        let memory_size = NonZeroUsize::new(config.memory_size)
            .context("Invalid memory cache size")?;
        
        let (tx, rx) = tokio::sync::broadcast::channel(1000);
        
        let cache = Self {
            redis_pool: Arc::new(redis_pool),
            memory_cache: Arc::new(RwLock::new(LruCache::new(memory_size))),
            locks: Arc::new(Mutex::new(HashMap::new())),
            config,
            invalidation_tx: tx,
            invalidation_rx: Arc::new(Mutex::new(rx)),
        };
        
        // Start background tasks
        cache.start_background_tasks();
        
        Ok(cache)
    }
    
    pub async fn get_or_load<T, F, E>(
        &self,
        key: &str,
        loader: F,
        ttl: Option<u64>,
    ) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone + Debug,
        F: Future<Output = std::result::Result<T, E>>,
        E: Into<anyhow::Error>,
    {
        let start = std::time::Instant::now();
        
        // 1. Check memory cache
        if let Some(item) = self.get_from_memory(key).await {
            if let Ok(cached_item) = serde_json::from_slice::<CachedItem<T>>(&item) {
                if !cached_item.is_expired() {
                    if self.config.enable_metrics {
                        CACHE_HIT_COUNTER.inc();
                    }
                    debug!("Memory cache hit for key: {}", key);
                    
                    // Check if we should refresh in background
                    if cached_item.should_refresh(self.config.refresh_threshold) {
                        self.refresh_in_background(key.to_string(), ttl);
                    }
                    
                    return Ok(cached_item.value);
                }
            }
        }
        
        // 2. Try to acquire distributed lock
        let lock_key = format!("lock:{}", key);
        let lock_acquired = self.acquire_lock(&lock_key).await?;
        
        if !lock_acquired {
            // Wait and retry from cache
            tokio::time::sleep(Duration::from_millis(100)).await;
            return self.get_without_load(key).await
                .ok_or_else(|| anyhow::anyhow!("Failed to get value after lock wait"));
        }
        
        // 3. Double-check caches after acquiring lock
        if let Some(value) = self.get_without_load::<T>(key).await {
            self.release_lock(&lock_key).await?;
            return Ok(value);
        }
        
        // 4. Load from source
        let load_start = std::time::Instant::now();
        let value = loader.await.map_err(Into::into)?;
        
        if self.config.enable_metrics {
            CACHE_LOAD_DURATION.observe(load_start.elapsed().as_secs_f64());
            CACHE_MISS_COUNTER.inc();
        }
        
        // 5. Store in both caches
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let cached_item = CachedItem::new(value.clone(), ttl);
        
        self.set_in_both_caches(key, &cached_item, ttl).await?;
        
        // 6. Release lock
        self.release_lock(&lock_key).await?;
        
        debug!("Cache miss and loaded for key: {} (took {:?})", key, start.elapsed());
        
        Ok(value)
    }
    
    async fn get_without_load<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        // Check memory first
        if let Some(item) = self.get_from_memory(key).await {
            if let Ok(cached_item) = serde_json::from_slice::<CachedItem<T>>(&item) {
                if !cached_item.is_expired() {
                    return Some(cached_item.value);
                }
            }
        }
        
        // Check Redis
        if let Ok(Some(item)) = self.get_from_redis(key).await {
            if let Ok(cached_item) = serde_json::from_slice::<CachedItem<T>>(&item) {
                if !cached_item.is_expired() {
                    // Update memory cache
                    let _ = self.set_in_memory(key, &item).await;
                    return Some(cached_item.value);
                }
            }
        }
        
        None
    }
    
    async fn get_from_memory(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.memory_cache.write().await;
        cache.get(key).cloned()
    }
    
    async fn get_from_redis(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let timer = REDIS_CACHE_OPS.start_timer();
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let result: Option<Vec<u8>> = conn.get(key).await
            .context("Failed to get from Redis")?;
        
        timer.observe_duration();
        Ok(result)
    }
    
    async fn set_in_memory(&self, key: &str, value: &[u8]) -> Result<()> {
        let mut cache = self.memory_cache.write().await;
        let evicted = cache.push(key.to_string(), value.to_vec());
        
        if evicted.is_some() && self.config.enable_metrics {
            CACHE_EVICTION_COUNTER.inc();
        }
        
        if self.config.enable_metrics {
            MEMORY_CACHE_SIZE.set(cache.len() as i64);
        }
        
        Ok(())
    }
    
    async fn set_in_redis(&self, key: &str, value: &[u8], ttl: u64) -> Result<()> {
        let timer = REDIS_CACHE_OPS.start_timer();
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        conn.set_ex::<_, _, ()>(key, value, ttl).await
            .context("Failed to set in Redis")?;
        
        timer.observe_duration();
        Ok(())
    }
    
    async fn set_in_both_caches<T>(&self, key: &str, item: &CachedItem<T>, ttl: u64) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_vec(item)
            .context("Failed to serialize cached item")?;
        
        // Set in Redis first
        self.set_in_redis(key, &serialized, ttl).await?;
        
        // Then in memory
        self.set_in_memory(key, &serialized).await?;
        
        Ok(())
    }
    
    async fn acquire_lock(&self, lock_key: &str) -> Result<bool> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let lock_value = nanoid::nanoid!();
        let script = Script::new(
            r#"
            if redis.call("get", KEYS[1]) == false then
                return redis.call("set", KEYS[1], ARGV[1], "NX", "EX", ARGV[2])
            else
                return nil
            end
            "#
        );
        
        let result: Option<String> = script
            .key(lock_key)
            .arg(&lock_value)
            .arg(self.config.lock_ttl)
            .invoke_async(&mut conn)
            .await
            .context("Failed to acquire lock")?;
        
        Ok(result.is_some())
    }
    
    async fn release_lock(&self, lock_key: &str) -> Result<()> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let _: () = conn.del(lock_key).await
            .context("Failed to release lock")?;
        
        Ok(())
    }
    
    pub async fn invalidate(&self, pattern: &str) -> Result<()> {
        info!("Invalidating cache for pattern: {}", pattern);
        
        // Invalidate memory cache
        {
            let mut cache = self.memory_cache.write().await;
            let keys_to_remove: Vec<String> = cache
                .iter()
                .filter_map(|(k, _)| {
                    if k.contains(pattern) {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            for key in keys_to_remove {
                cache.pop(&key);
            }
        }
        
        // Invalidate Redis cache
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let keys: Vec<String> = conn.keys(format!("*{}*", pattern)).await
            .context("Failed to get keys from Redis")?;
        
        if !keys.is_empty() {
            let _: () = conn.del(keys).await
                .context("Failed to delete keys from Redis")?;
        }
        
        // Publish invalidation event
        let _: () = conn.publish("cache_invalidation", pattern).await
            .context("Failed to publish invalidation event")?;
        
        Ok(())
    }
    
    pub async fn invalidate_key(&self, key: &str) -> Result<()> {
        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.pop(key);
        }
        
        // Remove from Redis
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let _: () = conn.del(key).await
            .context("Failed to delete key from Redis")?;
        
        // Publish invalidation event
        let _: () = conn.publish("cache_invalidation", key).await
            .context("Failed to publish invalidation event")?;
        
        info!("Invalidated cache key: {}", key);
        
        Ok(())
    }
    
    pub async fn warm_cache<T, F>(&self, keys: Vec<String>, loader: F) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone + Debug + Send + Sync + 'static,
        F: Fn(String) -> std::pin::Pin<Box<dyn Future<Output = Result<T>> + Send>> + Send + Sync + 'static,
    {
        info!("Warming cache with {} keys", keys.len());
        
        let loader = Arc::new(loader);
        let tasks = keys.into_iter().map(|key| {
            let cache = self.clone();
            let loader = loader.clone();
            
            tokio::spawn(async move {
                let key_clone = key.clone();
                let result = cache.get_or_load(
                    &key,
                    loader(key_clone),
                    Some(cache.config.default_ttl),
                ).await;
                
                match result {
                    Ok(_) => debug!("Warmed cache for key: {}", key),
                    Err(e) => warn!("Failed to warm cache for key {}: {}", key, e),
                }
            })
        });
        
        futures::future::join_all(tasks).await;
        
        info!("Cache warming completed");
        Ok(())
    }
    
    fn refresh_in_background(&self, key: String, ttl: Option<u64>) {
        let _cache = self.clone();
        
        tokio::spawn(async move {
            debug!("Starting background refresh for key: {}", key);
            
            // Note: This is a placeholder. In real implementation,
            // you'd need to store and reuse the original loader function
            // or have a registry of refresh functions
            
            // For now, just log that we would refresh
            debug!("Would refresh key: {} with TTL: {:?}", key, ttl);
        });
    }
    
    fn start_background_tasks(&self) {
        // Start invalidation listener
        let cache = self.clone();
        tokio::spawn(async move {
            cache.listen_for_invalidations().await;
        });
        
        // Start metrics reporter
        if self.config.enable_metrics {
            let cache = self.clone();
            tokio::spawn(async move {
                cache.report_metrics().await;
            });
        }
    }
    
    async fn listen_for_invalidations(&self) {
        loop {
            match self.setup_pubsub().await {
                Ok(mut pubsub) => {
                    loop {
                        match pubsub.on_message().next().await {
                            Some(msg) => {
                                if let Ok(pattern) = msg.get_payload::<String>() {
                                    debug!("Received invalidation for pattern: {}", pattern);
                                    
                                    // Invalidate local memory cache
                                    let mut cache = self.memory_cache.write().await;
                                    let keys_to_remove: Vec<String> = cache
                                        .iter()
                                        .filter_map(|(k, _)| {
                                            if k.contains(&pattern) {
                                                Some(k.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    
                                    for key in keys_to_remove {
                                        cache.pop(&key);
                                    }
                                }
                            }
                            None => {
                                warn!("Pub/sub connection closed, attempting to reconnect...");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to setup pub/sub: {}", e);
                }
            }
            
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    
    async fn setup_pubsub(&self) -> Result<PubSub> {
        // TODO: Implement proper Redis pubsub using deadpool_redis API
        // For now, return a placeholder
        Err(anyhow::anyhow!("PubSub not yet implemented with deadpool_redis"))
    }
    
    async fn report_metrics(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            let cache_size = {
                let cache = self.memory_cache.read().await;
                cache.len()
            };
            
            info!(
                "Cache metrics - Memory size: {}, Redis pool status: {:?}",
                cache_size,
                self.redis_pool.status()
            );
        }
    }
    
    pub async fn get_stats(&self) -> CacheStats {
        let memory_size = {
            let cache = self.memory_cache.read().await;
            cache.len()
        };
        
        let pool_status = self.redis_pool.status();
        
        CacheStats {
            memory_cache_size: memory_size,
            redis_pool_size: pool_status.size,
            redis_pool_available: pool_status.available,
            redis_pool_waiting: pool_status.waiting,
        }
    }
}

impl Clone for HybridCache {
    fn clone(&self) -> Self {
        Self {
            redis_pool: Arc::clone(&self.redis_pool),
            memory_cache: Arc::clone(&self.memory_cache),
            locks: Arc::clone(&self.locks),
            config: CacheConfig {
                memory_size: self.config.memory_size,
                default_ttl: self.config.default_ttl,
                lock_ttl: self.config.lock_ttl,
                refresh_threshold: self.config.refresh_threshold,
                enable_metrics: self.config.enable_metrics,
                enable_warming: self.config.enable_warming,
            },
            invalidation_tx: self.invalidation_tx.clone(),
            invalidation_rx: Arc::clone(&self.invalidation_rx),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub memory_cache_size: usize,
    pub redis_pool_size: usize,
    pub redis_pool_available: usize,
    pub redis_pool_waiting: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cached_item_expiration() {
        let item = CachedItem::new("test_value", 1);
        assert!(!item.is_expired());
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(item.is_expired());
    }
    
    #[tokio::test]
    async fn test_cached_item_refresh_threshold() {
        let item = CachedItem::new("test_value", 10);
        assert!(!item.should_refresh(0.8));
        
        tokio::time::sleep(Duration::from_secs(8)).await;
        assert!(item.should_refresh(0.8));
    }
}