//! DNS Cache Module
//! 
//! Provides DNS resolution and caching functionality for the crawler.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn, debug};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};

/// DNS cache entry with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsCacheEntry {
    pub ips: Vec<IpAddr>,
    pub cached_at: i64,
    pub ttl: u64,
}

impl DnsCacheEntry {
    fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.cached_at) > self.ttl as i64
    }
}

/// DNS cache with Redis/file persistence
pub struct DnsCache {
    cache: Arc<RwLock<HashMap<String, DnsCacheEntry>>>,
    resolver: Arc<TokioAsyncResolver>,
    last_persist: Arc<RwLock<Instant>>,
    redis_pool: Option<Arc<deadpool_redis::Pool>>,
}

impl DnsCache {
    /// Create a new DNS cache
    pub async fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        let mut cache = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolver: Arc::new(resolver),
            last_persist: Arc::new(RwLock::new(Instant::now())),
            redis_pool: Self::create_redis_pool().await,
        };

        // Load existing cache
        if let Err(e) = cache.load().await {
            warn!("Failed to load DNS cache: {}", e);
        }

        cache
    }

    /// Create Redis connection pool if available
    async fn create_redis_pool() -> Option<Arc<deadpool_redis::Pool>> {
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            let config = deadpool_redis::Config::from_url(&redis_url);
            match config.create_pool(Some(deadpool_redis::Runtime::Tokio1)) {
                Ok(pool) => {
                    info!("DNS cache using Redis backend");
                    Some(Arc::new(pool))
                }
                Err(e) => {
                    warn!("Failed to create Redis pool for DNS cache: {}", e);
                    None
                }
            }
        } else {
            info!("DNS cache using file backend");
            None
        }
    }

    /// Lookup a domain, using cache if available
    pub async fn lookup(&self, domain: &str) -> Result<Vec<IpAddr>, DnsLookupError> {
        // Normalize domain
        let domain = domain.to_lowercase();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&domain) {
                if !entry.is_expired() {
                    debug!("DNS cache hit for {}", domain);
                    return Ok(entry.ips.clone());
                }
            }
        }

        debug!("DNS cache miss for {}, performing lookup", domain);

        // Perform DNS lookup
        let ips = self.resolve_domain(&domain).await?;

        // Cache the result
        let entry = DnsCacheEntry {
            ips: ips.clone(),
            cached_at: chrono::Utc::now().timestamp(),
            ttl: 3600, // 1 hour TTL
        };

        {
            let mut cache = self.cache.write().await;
            cache.insert(domain.clone(), entry);
        }

        // Persist if needed
        self.persist_if_needed().await;

        Ok(ips)
    }

    /// Perform actual DNS resolution
    async fn resolve_domain(&self, domain: &str) -> Result<Vec<IpAddr>, DnsLookupError> {
        match self.resolver.lookup_ip(domain).await {
            Ok(lookup) => {
                let ips: Vec<IpAddr> = lookup.iter().collect();
                if ips.is_empty() {
                    Err(DnsLookupError::NoRecords(domain.to_string()))
                } else {
                    Ok(ips)
                }
            }
            Err(e) => Err(DnsLookupError::ResolutionFailed(e.to_string()))
        }
    }

    /// Check if domain exists (has DNS records)
    pub async fn domain_exists(&self, domain: &str) -> bool {
        self.lookup(domain).await.is_ok()
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> DnsCacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|e| e.is_expired()).count();

        DnsCacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
        }
    }

    /// Clear expired entries
    pub async fn cleanup(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
        debug!("DNS cache cleanup completed");
    }

    /// Persist cache if enough time has passed
    async fn persist_if_needed(&self) {
        let mut last_persist = self.last_persist.write().await;
        
        if last_persist.elapsed() > Duration::from_secs(300) {
            self.persist().await;
            *last_persist = Instant::now();
        }
    }

    /// Force persist cache to storage
    pub async fn persist(&self) {
        if let Err(e) = self.save().await {
            warn!("Failed to persist DNS cache: {}", e);
        }
    }

    /// Save cache to Redis or file
    async fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cache = self.cache.read().await;
        
        if let Some(pool) = &self.redis_pool {
            // Save to Redis
            self.save_to_redis(pool, &*cache).await?;
        } else {
            // Save to file
            self.save_to_file(&*cache).await?;
        }

        info!("DNS cache persisted ({} entries)", cache.len());
        Ok(())
    }

    /// Save cache to Redis
    async fn save_to_redis(
        &self,
        pool: &deadpool_redis::Pool,
        cache: &HashMap<String, DnsCacheEntry>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use deadpool_redis::redis::AsyncCommands;
        
        let mut conn = pool.get().await?;
        let cache_json = serde_json::to_string(cache)?;
        conn.set_ex("sam:dns_cache", cache_json, 3600).await?;
        
        Ok(())
    }

    /// Save cache to file
    async fn save_to_file(
        &self,
        cache: &HashMap<String, DnsCacheEntry>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = "/tmp/sam_crawler";
        tokio::fs::create_dir_all(cache_dir).await?;
        
        let cache_file = format!("{}/dns_cache.json", cache_dir);
        let cache_json = serde_json::to_string_pretty(cache)?;
        tokio::fs::write(cache_file, cache_json).await?;
        
        Ok(())
    }

    /// Load cache from storage
    async fn load(&self) -> Result<(), Box<dyn std::error::Error>> {
        let loaded_cache = if let Some(pool) = &self.redis_pool {
            self.load_from_redis(pool).await?
        } else {
            self.load_from_file().await?
        };

        if !loaded_cache.is_empty() {
            let mut cache = self.cache.write().await;
            *cache = loaded_cache;
            info!("DNS cache loaded ({} entries)", cache.len());
        }

        Ok(())
    }

    /// Load cache from Redis
    async fn load_from_redis(
        &self,
        pool: &deadpool_redis::Pool,
    ) -> Result<HashMap<String, DnsCacheEntry>, Box<dyn std::error::Error>> {
        use deadpool_redis::redis::AsyncCommands;
        
        let mut conn = pool.get().await?;
        let cache_json: Option<String> = conn.get("sam:dns_cache").await?;
        
        if let Some(json) = cache_json {
            Ok(serde_json::from_str(&json)?)
        } else {
            Ok(HashMap::new())
        }
    }

    /// Load cache from file
    async fn load_from_file(&self) -> Result<HashMap<String, DnsCacheEntry>, Box<dyn std::error::Error>> {
        let cache_file = "/tmp/sam_crawler/dns_cache.json";
        
        if !tokio::fs::metadata(cache_file).await.is_ok() {
            return Ok(HashMap::new());
        }

        let cache_json = tokio::fs::read_to_string(cache_file).await?;
        Ok(serde_json::from_str(&cache_json)?)
    }

    /// Batch lookup multiple domains
    pub async fn batch_lookup(&self, domains: Vec<String>) -> HashMap<String, Result<Vec<IpAddr>, DnsLookupError>> {
        let mut results = HashMap::new();
        
        for domain in domains {
            let result = self.lookup(&domain).await;
            results.insert(domain, result);
        }
        
        results
    }

    /// Prefetch domains into cache
    pub async fn prefetch(&self, domains: Vec<String>) {
        for domain in domains {
            if let Err(e) = self.lookup(&domain).await {
                debug!("Failed to prefetch {}: {:?}", domain, e);
            }
        }
    }
}

/// DNS cache statistics
#[derive(Debug, Clone)]
pub struct DnsCacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

/// DNS lookup errors
#[derive(Debug)]
pub enum DnsLookupError {
    NoRecords(String),
    ResolutionFailed(String),
    Timeout,
}

impl std::fmt::Display for DnsLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRecords(domain) => write!(f, "No DNS records found for {}", domain),
            Self::ResolutionFailed(err) => write!(f, "DNS resolution failed: {}", err),
            Self::Timeout => write!(f, "DNS lookup timed out"),
        }
    }
}

impl std::error::Error for DnsLookupError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_cache_lookup() {
        let cache = DnsCache::new().await;
        
        // Test valid domain
        let result = cache.lookup("google.com").await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
        
        // Second lookup should hit cache
        let result2 = cache.lookup("google.com").await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_dns_cache_invalid_domain() {
        let cache = DnsCache::new().await;
        
        let result = cache.lookup("invalid-domain-that-does-not-exist-12345.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DnsCache::new().await;
        
        // Perform some lookups
        let _ = cache.lookup("google.com").await;
        let _ = cache.lookup("github.com").await;
        
        let stats = cache.get_stats().await;
        assert!(stats.total_entries >= 2);
        assert_eq!(stats.expired_entries, 0);
    }
}