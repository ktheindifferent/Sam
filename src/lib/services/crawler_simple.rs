//! Simplified crawler module for benchmarks
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug, Clone)]
pub struct CrawledContent {
    pub url: String,
    pub content: String,
    pub status_code: Option<u16>,
    pub timestamp: u64,
}

impl CrawledContent {
    pub fn new(url: String, content: String) -> Self {
        Self {
            url,
            content,
            status_code: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn compute_hash(content: &str) -> String {
        format!("{:x}", content.len())
    }

    pub fn extract_title(html: &str) -> Option<String> {
        // Simple title extraction without regex
        if let Some(start) = html.find("<title>") {
            if let Some(end) = html[start + 7..].find("</title>") {
                let title = &html[start + 7..start + 7 + end];
                return Some(title.trim().to_string());
            }
        }
        None
    }

    pub fn extract_description(html: &str) -> Option<String> {
        // Simple meta description extraction
        let lower = html.to_lowercase();
        if let Some(start) = lower.find("<meta name=\"description\" content=\"") {
            let content_start = start + 34;
            if let Some(end) = lower[content_start..].find("\"") {
                let desc = &html[content_start..content_start + end];
                return Some(desc.trim().to_string());
            }
        }
        None
    }

    pub fn compress_content(content: &str) -> Vec<u8> {
        // Simple compression simulation (just convert to bytes)
        content.as_bytes().to_vec()
    }

    pub fn decompress_content(compressed: &[u8]) -> Result<String, String> {
        // Simple decompression simulation
        String::from_utf8(compressed.to_vec()).map_err(|e| e.to_string())
    }
}

pub mod url_patterns {
    use std::collections::HashMap;
    
    pub fn normalize_url(url: &str) -> String {
        // Basic URL normalization
        let url = url.trim();
        if url.starts_with("http://") {
            url.replacen("http://", "https://", 1)
        } else if !url.starts_with("https://") {
            format!("https://{}", url)
        } else {
            url.to_string()
        }
    }

    pub struct UrlPatternDetector {
        patterns: HashMap<String, String>,
    }

    impl UrlPatternDetector {
        pub fn new() -> Self {
            Self {
                patterns: HashMap::new(),
            }
        }

        pub fn detect_pattern(&self, url: &str) -> String {
            if url.contains("/api/") {
                "API".to_string()
            } else if url.contains("/blog/") {
                "Blog".to_string()
            } else if url.contains("/product/") {
                "Product".to_string()
            } else {
                "General".to_string()
            }
        }

        pub fn is_calendar_pattern(&self, url: &str) -> bool {
            url.contains("/calendar/") || url.contains("/date/") || url.contains("/2023/") || url.contains("/2024/")
        }

        pub fn is_pagination_pattern(&self, url: &str) -> bool {
            url.contains("page=") || url.contains("/page/") || url.contains("offset=") || url.contains("limit=")
        }
    }
}

pub mod memory_optimized {
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    pub struct MemoryConfig {
        pub max_urls: usize,
        pub cleanup_threshold: f32,
    }

    impl Default for MemoryConfig {
        fn default() -> Self {
            Self {
                max_urls: 10000,
                cleanup_threshold: 0.8,
            }
        }
    }

    pub struct OptimizedUrlTracker {
        urls: HashMap<String, u64>,
        config: MemoryConfig,
    }

    impl OptimizedUrlTracker {
        pub fn new(config: MemoryConfig) -> Self {
            Self {
                urls: HashMap::new(),
                config,
            }
        }

        pub fn track_url(&mut self, url: &str) -> bool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if self.urls.len() >= self.config.max_urls {
                self.cleanup();
            }
            
            self.urls.insert(url.to_string(), now);
            true
        }
        
        pub fn visit_url(&mut self, url: &str) -> bool {
            self.track_url(url)
        }

        fn cleanup(&mut self) {
            let threshold = (self.config.max_urls as f32 * self.config.cleanup_threshold) as usize;
            if self.urls.len() > threshold {
                // Keep newest entries
                let mut entries: Vec<_> = self.urls.iter().map(|(k, v)| (k.clone(), *v)).collect();
                entries.sort_by(|a, b| b.1.cmp(&a.1));
                self.urls.clear();
                for (url, timestamp) in entries.into_iter().take(threshold) {
                    self.urls.insert(url, timestamp);
                }
            }
        }
    }
}

pub mod circuit_breaker {
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    #[derive(Debug, Clone)]
    pub struct CircuitBreakerConfig {
        pub failure_threshold: u32,
        pub timeout_seconds: u64,
        pub recovery_timeout_seconds: u64,
    }

    impl Default for CircuitBreakerConfig {
        fn default() -> Self {
            Self {
                failure_threshold: 5,
                timeout_seconds: 60,
                recovery_timeout_seconds: 30,
            }
        }
    }

    #[derive(Debug)]
    pub enum CircuitState {
        Closed,
        Open,
        HalfOpen,
    }

    pub struct CircuitBreaker {
        config: CircuitBreakerConfig,
        state: CircuitState,
        failure_count: AtomicU32,
        last_failure_time: AtomicU64,
    }

    impl CircuitBreaker {
        pub fn with_config(config: CircuitBreakerConfig) -> Self {
            Self {
                config,
                state: CircuitState::Closed,
                failure_count: AtomicU32::new(0),
                last_failure_time: AtomicU64::new(0),
            }
        }

        pub fn call<T, E>(&self, _operation: impl Fn() -> Result<T, E>) -> Result<T, String> {
            // Simplified implementation
            Err("Circuit breaker simulation".to_string())
        }

        pub async fn is_allowed(&self, _domain: &str) -> bool {
            true
        }

        pub async fn record_failure(&self, _domain: &str) {
            // No-op for benchmarking
        }

        pub async fn record_success(&self, _domain: &str) {
            // No-op for benchmarking
        }
    }
}

pub mod rate_limiter {
    use std::time::Duration;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone)]
    pub struct RateLimitConfig {
        pub requests_per_second: u32,
        pub burst_size: u32,
        pub adaptive_threshold: f32,
    }

    impl Default for RateLimitConfig {
        fn default() -> Self {
            Self {
                requests_per_second: 10,
                burst_size: 20,
                adaptive_threshold: 0.8,
            }
        }
    }

    pub struct AdaptiveRateLimiter {
        config: RateLimitConfig,
        current_tokens: AtomicU64,
    }

    impl AdaptiveRateLimiter {
        pub fn with_config(config: RateLimitConfig) -> Self {
            let burst_size = config.burst_size as u64;
            Self {
                config,
                current_tokens: AtomicU64::new(burst_size),
            }
        }

        pub fn acquire(&self) -> bool {
            let current = self.current_tokens.load(Ordering::Relaxed);
            if current > 0 {
                self.current_tokens.store(current - 1, Ordering::Relaxed);
                true
            } else {
                false
            }
        }

        pub fn wait_time(&self) -> Duration {
            Duration::from_millis(1000 / self.config.requests_per_second as u64)
        }

        pub async fn can_crawl_domain(&self, _domain: &str) -> bool {
            self.acquire()
        }

        pub async fn record_request(&self, _domain: &str, _response_time: Duration) {
            // No-op for benchmarking
        }
    }
}

pub mod user_agents {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    pub enum RotationStrategy {
        Random,
        RoundRobin,
        PerDomain,
    }

    pub struct UserAgentRotator {
        strategy: RotationStrategy,
        agents: Vec<String>,
        current_index: AtomicUsize,
        domain_agents: HashMap<String, usize>,
    }

    impl UserAgentRotator {
        pub fn new(strategy: RotationStrategy) -> Self {
            let agents = vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36".to_string(),
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".to_string(),
            ];

            Self {
                strategy,
                agents,
                current_index: AtomicUsize::new(0),
                domain_agents: HashMap::new(),
            }
        }

        pub fn get_user_agent(&self, _url: &str) -> String {
            match self.strategy {
                RotationStrategy::Random => {
                    // Simple random without external crate
                    let index = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as usize % self.agents.len();
                    self.agents[index].clone()
                }
                RotationStrategy::RoundRobin => {
                    let index = self.current_index.fetch_add(1, Ordering::Relaxed);
                    self.agents[index % self.agents.len()].clone()
                }
                RotationStrategy::PerDomain => {
                    // Simplified: just use round robin for now
                    let index = self.current_index.fetch_add(1, Ordering::Relaxed);
                    self.agents[index % self.agents.len()].clone()
                }
            }
        }
    }
}

pub mod js_renderer {
    #[derive(Debug, Clone)]
    pub struct JsRendererConfig {
        pub timeout_ms: u64,
        pub wait_for_selectors: Vec<String>,
    }

    impl Default for JsRendererConfig {
        fn default() -> Self {
            Self {
                timeout_ms: 30000,
                wait_for_selectors: Vec::new(),
            }
        }
    }

    pub struct JsRenderer {
        config: JsRendererConfig,
    }

    impl JsRenderer {
        pub fn new(config: JsRendererConfig) -> Self {
            Self { config }
        }

        pub async fn render(&self, url: &str) -> Result<String, String> {
            // Simplified: just return a mock rendered page
            Ok(format!("<html><head><title>Rendered: {}</title></head><body>Rendered content for {}</body></html>", url, url))
        }

        pub async fn detect_frameworks(&self, _html: &str) -> Vec<String> {
            // Mock framework detection
            vec!["React".to_string(), "Vue".to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic() {
        let hash = CrawledContent::compute_hash("test");
        assert!(!hash.is_empty());
        
        let normalized = url_patterns::normalize_url("test");
        assert_eq!(normalized, "https://test");
    }
    
    #[test]
    fn test_content_methods() {
        let content = CrawledContent::new("https://example.com".to_string(), "test content".to_string());
        assert_eq!(content.url, "https://example.com");
        assert_eq!(content.content, "test content");
        
        // Test title extraction
        let html = "<html><head><title>Test Page</title></head><body>Content</body></html>";
        let title = CrawledContent::extract_title(html);
        assert_eq!(title, Some("Test Page".to_string()));
        
        // Test compression/decompression
        let compressed = CrawledContent::compress_content("test");
        let decompressed = CrawledContent::decompress_content(&compressed);
        assert_eq!(decompressed, Ok("test".to_string()));
    }
}
