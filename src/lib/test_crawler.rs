#[cfg(test)]
mod tests {
    use crate::services::crawler::url_patterns;
    use crate::services::crawler::CrawledContent;
    use crate::services::crawler::memory_optimized;
    
    #[test]
    fn test_url_normalization() {
        let result = url_patterns::normalize_url("https://example.com");
        assert_eq!(result, "https://example.com");
    }
    
    #[test]
    fn test_crawled_content() {
        let hash = CrawledContent::compute_hash("test content");
        assert!(!hash.is_empty());
    }
    
    #[test]
    fn test_memory_optimized() {
        let config = memory_optimized::MemoryConfig {
            bloom_filter_size: 1000,
            bloom_filter_fp_rate: 0.01,
            lru_cache_size: 100,
            max_queue_size: 1000,
            enable_redis_spillover: false,
        };
        let mut tracker = memory_optimized::OptimizedUrlTracker::new(config);
        assert!(tracker.visit_url("https://example.com"));
    }
}
