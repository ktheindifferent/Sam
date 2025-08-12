use sam::sam::services::cache::{HybridCache, CacheConfig, CacheStats};
use sam::sam::services::redis;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_cache_basic_operations() {
    // Skip if Redis is not available
    let cache = match redis::create_cache().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    // Test basic get_or_load
    let key = "test_key_basic";
    let value = cache.get_or_load(key, async {
        Ok::<String, anyhow::Error>("test_value".to_string())
    }, Some(60)).await;
    
    assert!(value.is_ok());
    assert_eq!(value.unwrap(), "test_value");
    
    // Second call should hit cache
    let value2 = cache.get_or_load(key, async {
        Ok::<String, anyhow::Error>("different_value".to_string())
    }, Some(60)).await;
    
    assert!(value2.is_ok());
    assert_eq!(value2.unwrap(), "test_value"); // Should get cached value
    
    // Invalidate and try again
    cache.invalidate_key(key).await.unwrap();
    
    let value3 = cache.get_or_load(key, async {
        Ok::<String, anyhow::Error>("new_value".to_string())
    }, Some(60)).await;
    
    assert!(value3.is_ok());
    assert_eq!(value3.unwrap(), "new_value");
}

#[tokio::test]
async fn test_cache_expiration() {
    let cache = match redis::create_cache().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    let key = "test_key_expiration";
    
    // Set with 1 second TTL
    let _value = cache.get_or_load(key, async {
        Ok::<String, anyhow::Error>("expires_soon".to_string())
    }, Some(1)).await.unwrap();
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Should load new value
    let value2 = cache.get_or_load(key, async {
        Ok::<String, anyhow::Error>("new_after_expiry".to_string())
    }, Some(60)).await;
    
    assert!(value2.is_ok());
    assert_eq!(value2.unwrap(), "new_after_expiry");
}

#[tokio::test]
async fn test_cache_stampede_prevention() {
    let cache = match redis::create_cache().await {
        Ok(c) => Arc::new(c),
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    let key = "test_stampede_key";
    let barrier = Arc::new(Barrier::new(10));
    let counter = Arc::new(tokio::sync::Mutex::new(0));
    
    // Clear the key first
    cache.invalidate_key(key).await.unwrap();
    
    // Spawn 10 concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let barrier_clone = Arc::clone(&barrier);
        let counter_clone = Arc::clone(&counter);
        let key = key.to_string();
        
        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier_clone.wait().await;
            
            // All try to get the same key simultaneously
            let result = cache_clone.get_or_load(&key, async {
                // Increment counter when loader is called
                let mut count = counter_clone.lock().await;
                *count += 1;
                
                // Simulate slow operation
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                Ok::<String, anyhow::Error>(format!("value_{}", i))
            }, Some(60)).await;
            
            result
        });
        
        handles.push(handle);
    }
    
    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Check that loader was called only once
    let final_count = *counter.lock().await;
    assert_eq!(final_count, 1, "Loader should be called only once");
    
    // All should get the same value
    let first_value = results[0].as_ref().unwrap().as_ref().unwrap();
    for result in &results {
        assert_eq!(result.as_ref().unwrap().as_ref().unwrap(), first_value);
    }
}

#[tokio::test]
async fn test_cache_invalidation_patterns() {
    let cache = match redis::create_cache().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    // Set multiple keys with pattern
    for i in 0..5 {
        let key = format!("user:session:{}", i);
        cache.get_or_load(&key, async move {
            Ok::<String, anyhow::Error>(format!("session_data_{}", i))
        }, Some(60)).await.unwrap();
    }
    
    // Set some other keys
    for i in 0..3 {
        let key = format!("api:token:{}", i);
        cache.get_or_load(&key, async move {
            Ok::<String, anyhow::Error>(format!("token_data_{}", i))
        }, Some(60)).await.unwrap();
    }
    
    // Invalidate all user sessions
    cache.invalidate("user:session").await.unwrap();
    
    // User sessions should be gone
    let session_result = cache.get_or_load("user:session:0", async {
        Ok::<String, anyhow::Error>("new_session".to_string())
    }, Some(60)).await.unwrap();
    assert_eq!(session_result, "new_session");
    
    // API tokens should still exist
    let token_result = cache.get_or_load("api:token:0", async {
        Ok::<String, anyhow::Error>("new_token".to_string())
    }, Some(60)).await.unwrap();
    assert_eq!(token_result, "token_data_0");
}

#[tokio::test]
async fn test_cache_stats() {
    let cache = match redis::create_cache().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    // Add some items
    for i in 0..5 {
        let key = format!("stats_test_{}", i);
        cache.get_or_load(&key, async move {
            Ok::<String, anyhow::Error>(format!("value_{}", i))
        }, Some(60)).await.unwrap();
    }
    
    // Get stats
    let stats = cache.get_stats().await;
    
    // Memory cache should have items
    assert!(stats.memory_cache_size > 0);
    assert!(stats.memory_cache_size <= 5);
    
    // Redis pool should be available
    assert!(stats.redis_pool_available > 0);
}

#[tokio::test]
async fn test_cache_error_handling() {
    let cache = match redis::create_cache().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    let key = "error_test_key";
    
    // Test loader error handling
    let result = cache.get_or_load::<String, _, _>(key, async {
        Err::<String, anyhow::Error>(anyhow::anyhow!("Loader failed"))
    }, Some(60)).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Loader failed"));
}

#[tokio::test]
async fn test_cache_concurrent_different_keys() {
    let cache = match redis::create_cache().await {
        Ok(c) => Arc::new(c),
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    let mut handles = vec![];
    
    // Spawn concurrent requests for different keys
    for i in 0..20 {
        let cache_clone = Arc::clone(&cache);
        let key = format!("concurrent_key_{}", i);
        
        let handle = tokio::spawn(async move {
            cache_clone.get_or_load(&key, async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<String, anyhow::Error>(format!("value_{}", i))
            }, Some(60)).await
        });
        
        handles.push(handle);
    }
    
    // All should complete successfully
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok());
        let inner = result.as_ref().unwrap();
        assert!(inner.is_ok());
        assert_eq!(inner.as_ref().unwrap(), &format!("value_{}", i));
    }
}

#[tokio::test]
async fn test_memory_cache_lru_eviction() {
    // Create cache with small memory size
    let mut config = CacheConfig::default();
    config.memory_size = 3; // Very small for testing
    
    let cache = match redis::create_cache_with_config(config).await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping test - Redis not available");
            return;
        }
    };
    
    // Add items that exceed memory cache size
    for i in 0..5 {
        let key = format!("lru_test_{}", i);
        cache.get_or_load(&key, async move {
            Ok::<String, anyhow::Error>(format!("value_{}", i))
        }, Some(60)).await.unwrap();
    }
    
    // Stats should show max 3 items in memory
    let stats = cache.get_stats().await;
    assert!(stats.memory_cache_size <= 3);
}