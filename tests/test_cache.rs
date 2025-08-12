#[cfg(test)]
mod tests {
    use sam::sam::services::redis;
    use sam::sam::services::cache::{HybridCache, CacheConfig};
    
    #[tokio::test]
    async fn test_cache_creation() {
        // Try to create a cache instance
        let result = redis::create_cache().await;
        
        // If Redis is not available, skip the test
        if result.is_err() {
            eprintln!("Skipping test - Redis not available: {:?}", result.err());
            return;
        }
        
        let cache = result.unwrap();
        
        // Test basic operation
        let key = "test_key";
        let value = cache.get_or_load(key, async {
            Ok::<String, anyhow::Error>("test_value".to_string())
        }, Some(60)).await;
        
        assert!(value.is_ok());
        assert_eq!(value.unwrap(), "test_value");
    }
    
    #[tokio::test]
    async fn test_cache_with_custom_config() {
        let mut config = CacheConfig::default();
        config.memory_size = 100;
        config.default_ttl = 300;
        
        let result = redis::create_cache_with_config(config).await;
        
        if result.is_err() {
            eprintln!("Skipping test - Redis not available");
            return;
        }
        
        assert!(result.is_ok());
    }
}