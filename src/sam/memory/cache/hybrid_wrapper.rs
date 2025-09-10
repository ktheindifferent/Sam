use anyhow::{Result, Context};
use crate::sam::services::cache::HybridCache;
use crate::sam::services::redis;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use once_cell::sync::OnceCell;

static CACHE_INSTANCE: OnceCell<Arc<HybridCache>> = OnceCell::new();

/// Get or create the global cache instance
pub async fn get_cache() -> Result<Arc<HybridCache>> {
    if let Some(cache) = CACHE_INSTANCE.get() {
        return Ok(Arc::clone(cache));
    }
    
    let cache = redis::create_cache().await
        .context("Failed to create cache instance")?;
    
    let arc_cache = Arc::new(cache);
    CACHE_INSTANCE.set(Arc::clone(&arc_cache))
        .map_err(|_| anyhow::anyhow!("Failed to set cache instance"))?;
    
    Ok(arc_cache)
}

/// Cache wrapper for web crawl results
pub struct WebCrawlCache {
    cache: Arc<HybridCache>,
}

impl WebCrawlCache {
    pub async fn new() -> Result<Self> {
        let cache = get_cache().await?;
        Ok(Self { cache })
    }
    
    pub async fn get_or_fetch<T, F>(&self, url: &str, fetcher: F) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone + std::fmt::Debug,
        F: std::future::Future<Output = Result<T>>,
    {
        let key = format!("web_crawl:{}", url);
        self.cache.get_or_load(&key, fetcher, Some(3600)).await
    }
    
    pub async fn invalidate_url(&self, url: &str) -> Result<()> {
        let key = format!("web_crawl:{}", url);
        self.cache.invalidate_key(&key).await
    }
}

/// Cache wrapper for Wikipedia summaries
pub struct WikipediaCache {
    cache: Arc<HybridCache>,
}

impl WikipediaCache {
    pub async fn new() -> Result<Self> {
        let cache = get_cache().await?;
        Ok(Self { cache })
    }
    
    pub async fn get_or_fetch(&self, article: &str, fetcher: impl std::future::Future<Output = Result<String>>) -> Result<String> {
        let key = format!("wikipedia:{}", article);
        self.cache.get_or_load(&key, fetcher, Some(86400)).await // 24 hours
    }
    
    pub async fn invalidate_article(&self, article: &str) -> Result<()> {
        let key = format!("wikipedia:{}", article);
        self.cache.invalidate_key(&key).await
    }
}

/// Cache wrapper for session data with shorter TTL
pub struct SessionCache {
    cache: Arc<HybridCache>,
}

impl SessionCache {
    pub async fn new() -> Result<Self> {
        let cache = get_cache().await?;
        Ok(Self { cache })
    }
    
    pub async fn get_or_create<T>(&self, session_id: &str, creator: impl std::future::Future<Output = Result<T>>) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone + std::fmt::Debug,
    {
        let key = format!("session:{}", session_id);
        self.cache.get_or_load(&key, creator, Some(1800)).await // 30 minutes
    }
    
    pub async fn invalidate_session(&self, session_id: &str) -> Result<()> {
        let key = format!("session:{}", session_id);
        self.cache.invalidate_key(&key).await
    }
    
    pub async fn invalidate_all_sessions(&self) -> Result<()> {
        self.cache.invalidate("session:").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_web_crawl_cache() {
        // This test requires Redis to be running
        let cache = match WebCrawlCache::new().await {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Skipping test - Redis not available");
                return;
            }
        };
        
        let result = cache.get_or_fetch("https://example.com", async {
            Ok("Test content".to_string())
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test content");
    }
}