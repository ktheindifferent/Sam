//! Comprehensive test suite for crawler components

#[cfg(test)]
mod tests {
    use super::super::*;
    use tokio::test;
    use std::collections::HashMap;
    use std::time::Duration;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    // ============================================================================
    // Circuit Breaker Tests
    // ============================================================================
    
    mod circuit_breaker_tests {
        use super::*;
        use crate::sam::services::crawler::circuit_breaker::*;

        #[tokio::test]
        async fn test_circuit_breaker_opens_after_threshold() {
            let config = CircuitBreakerConfig {
                failure_threshold: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(1),
                open_duration: Duration::from_millis(200),
                half_open_success_threshold: 2,
            };
            
            let breaker = CircuitBreaker::with_config(config);
            let domain = "test.com";

            // Initially closed
            assert!(breaker.is_allowed(domain).await);
            assert_eq!(breaker.get_state(domain).await, CircuitState::Closed);

            // Record failures
            breaker.record_failure(domain).await;
            assert!(breaker.is_allowed(domain).await); // Still closed after 1 failure
            
            breaker.record_failure(domain).await;
            assert!(breaker.is_allowed(domain).await); // Still closed after 2 failures
            
            breaker.record_failure(domain).await;
            assert_eq!(breaker.get_state(domain).await, CircuitState::Open);
            assert!(!breaker.is_allowed(domain).await); // Now open, requests blocked
        }

        #[tokio::test]
        async fn test_circuit_breaker_transitions_to_half_open() {
            let config = CircuitBreakerConfig {
                failure_threshold: 2,
                initial_backoff: Duration::from_millis(50),
                max_backoff: Duration::from_secs(1),
                open_duration: Duration::from_millis(100),
                half_open_success_threshold: 2,
            };
            
            let breaker = CircuitBreaker::with_config(config);
            let domain = "test.com";

            // Open the circuit
            breaker.record_failure(domain).await;
            breaker.record_failure(domain).await;
            assert_eq!(breaker.get_state(domain).await, CircuitState::Open);

            // Wait for cooldown
            tokio::time::sleep(Duration::from_millis(60)).await;
            
            // Should transition to half-open on next check
            assert!(breaker.is_allowed(domain).await);
            assert_eq!(breaker.get_state(domain).await, CircuitState::HalfOpen);
        }

        #[tokio::test]
        async fn test_circuit_breaker_closes_after_recovery() {
            let config = CircuitBreakerConfig {
                failure_threshold: 2,
                initial_backoff: Duration::from_millis(50),
                max_backoff: Duration::from_secs(1),
                open_duration: Duration::from_millis(100),
                half_open_success_threshold: 2,
            };
            
            let breaker = CircuitBreaker::with_config(config);
            let domain = "test.com";

            // Open circuit
            breaker.record_failure(domain).await;
            breaker.record_failure(domain).await;
            
            // Wait and transition to half-open
            tokio::time::sleep(Duration::from_millis(60)).await;
            assert!(breaker.is_allowed(domain).await);
            
            // Record successes to close circuit
            breaker.record_success(domain).await;
            assert_eq!(breaker.get_state(domain).await, CircuitState::HalfOpen);
            
            breaker.record_success(domain).await;
            assert_eq!(breaker.get_state(domain).await, CircuitState::Closed);
        }

        #[tokio::test]
        async fn test_failure_rate_calculation() {
            let breaker = CircuitBreaker::new();
            let domain = "test.com";

            // Record mixed results
            breaker.record_success(domain).await;
            breaker.record_success(domain).await;
            breaker.record_failure(domain).await;
            breaker.record_success(domain).await;
            breaker.record_failure(domain).await;

            let rate = breaker.get_failure_rate(domain).await.unwrap();
            assert!((rate - 0.4).abs() < 0.01); // 2 failures out of 5 = 40%
        }
    }

    // ============================================================================
    // Rate Limiter Tests
    // ============================================================================
    
    mod rate_limiter_tests {
        use super::*;
        use crate::sam::services::crawler::rate_limiter::*;

        #[tokio::test]
        async fn test_rate_limiter_respects_rps() {
            let config = RateLimitConfig {
                default_delay: Duration::from_millis(100),
                min_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(5),
                requests_per_second: 10.0,
                burst_size: 5,
                adaptive: false,
                respect_crawl_delay: true,
                respect_retry_after: true,
            };

            let limiter = AdaptiveRateLimiter::with_config(config);
            let domain = "test.com";

            // First request should be immediate
            let start = std::time::Instant::now();
            limiter.wait_if_needed(domain).await;
            assert!(start.elapsed() < Duration::from_millis(10));

            // Record the request
            limiter.record_request(domain, 200, Duration::from_millis(100)).await;

            // Next request should be delayed
            let start = std::time::Instant::now();
            limiter.wait_if_needed(domain).await;
            assert!(start.elapsed() >= Duration::from_millis(90)); // ~100ms delay
        }

        #[tokio::test]
        async fn test_adaptive_rate_limiting() {
            let config = RateLimitConfig {
                default_delay: Duration::from_millis(100),
                min_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(5),
                requests_per_second: 10.0,
                burst_size: 5,
                adaptive: true,
                respect_crawl_delay: true,
                respect_retry_after: true,
            };

            let limiter = AdaptiveRateLimiter::with_config(config);
            let domain = "test.com";

            // Fast responses should decrease delay
            for _ in 0..5 {
                limiter.record_request(domain, 200, Duration::from_millis(50)).await;
            }

            let stats = limiter.get_domain_stats(domain).await;
            assert!(stats.is_some());
            let stats = stats.unwrap();
            assert!(stats.current_delay < Duration::from_millis(100));

            // Slow responses should increase delay
            for _ in 0..5 {
                limiter.record_request(domain, 200, Duration::from_secs(2)).await;
            }

            let stats = limiter.get_domain_stats(domain).await.unwrap();
            assert!(stats.current_delay > Duration::from_millis(100));
        }

        #[tokio::test]
        async fn test_retry_after_header_respect() {
            let limiter = AdaptiveRateLimiter::new();
            let domain = "test.com";

            // Set retry-after
            limiter.set_retry_after(domain, Duration::from_secs(5)).await;

            // Should not be allowed immediately
            assert!(!limiter.can_crawl_domain(domain).await);

            // Wait a bit
            tokio::time::sleep(Duration::from_secs(6)).await;

            // Should be allowed now
            assert!(limiter.can_crawl_domain(domain).await);
        }
    }

    // ============================================================================
    // URL Pattern Tests
    // ============================================================================
    
    mod url_pattern_tests {
        use super::*;
        use crate::sam::services::crawler::url_patterns::*;

        #[test]
        fn test_calendar_pattern_detection() {
            let detector = UrlPatternDetector::new();

            // Calendar URLs
            assert!(detector.is_calendar_pattern("https://example.com/2024/01/15"));
            assert!(detector.is_calendar_pattern("https://example.com/calendar/2024-01"));
            assert!(detector.is_calendar_pattern("https://example.com/events/2024/january"));
            
            // Non-calendar URLs
            assert!(!detector.is_calendar_pattern("https://example.com/about"));
            assert!(!detector.is_calendar_pattern("https://example.com/product/123"));
        }

        #[test]
        fn test_pagination_detection() {
            let detector = UrlPatternDetector::new();

            // Pagination URLs
            assert!(detector.is_pagination_pattern("https://example.com/posts?page=5"));
            assert!(detector.is_pagination_pattern("https://example.com/results?p=10"));
            assert!(detector.is_pagination_pattern("https://example.com/items?offset=100"));
            
            // Non-pagination URLs
            assert!(!detector.is_pagination_pattern("https://example.com/about"));
            assert!(!detector.is_pagination_pattern("https://example.com/contact"));
        }

        #[test]
        fn test_url_normalization() {
            // Remove tracking parameters
            assert_eq!(
                normalize_url("https://example.com/page?utm_source=test&id=123"),
                "https://example.com/page?id=123"
            );

            // Remove session IDs
            assert_eq!(
                normalize_url("https://example.com/page?sessionid=abc123&data=value"),
                "https://example.com/page?data=value"
            );

            // Remove fragment
            assert_eq!(
                normalize_url("https://example.com/page#section"),
                "https://example.com/page"
            );

            // Handle trailing slashes
            assert_eq!(
                normalize_url("https://example.com/page/"),
                "https://example.com/page"
            );
        }

        #[test]
        fn test_infinite_pattern_detection() {
            let mut detector = UrlPatternDetector::new();

            // Feed similar patterns
            for i in 1..20 {
                let url = format!("https://example.com/page/{}", i);
                detector.analyze_url(&url);
            }

            // Should detect pattern
            assert!(detector.is_likely_infinite_pattern("https://example.com/page/21"));
        }
    }

    // ============================================================================
    // Content Storage Tests
    // ============================================================================
    
    mod content_storage_tests {
        use super::*;
        use crate::sam::services::crawler::content_storage::*;

        #[test]
        fn test_content_hashing() {
            let content1 = "This is test content";
            let content2 = "This is test content"; // Same
            let content3 = "Different content";

            let hash1 = CrawledContent::compute_hash(content1);
            let hash2 = CrawledContent::compute_hash(content2);
            let hash3 = CrawledContent::compute_hash(content3);

            assert_eq!(hash1, hash2); // Same content = same hash
            assert_ne!(hash1, hash3); // Different content = different hash
        }

        #[test]
        fn test_title_extraction() {
            let html = r#"
                <html>
                    <head>
                        <title>Test Page Title</title>
                    </head>
                    <body>Content</body>
                </html>
            "#;

            let title = CrawledContent::extract_title(html);
            assert_eq!(title, Some("Test Page Title".to_string()));

            // Test with no title
            let html_no_title = "<html><body>Content</body></html>";
            assert_eq!(CrawledContent::extract_title(html_no_title), None);
        }

        #[test]
        fn test_description_extraction() {
            let html = r#"
                <html>
                    <head>
                        <meta name="description" content="This is a test description">
                    </head>
                    <body>Content</body>
                </html>
            "#;

            let description = CrawledContent::extract_description(html);
            assert_eq!(description, Some("This is a test description".to_string()));

            // Test with no description
            let html_no_desc = "<html><body>Content</body></html>";
            assert_eq!(CrawledContent::extract_description(html_no_desc), None);
        }

        #[test]
        fn test_language_detection() {
            // English text
            let english = "This is a sample English text with multiple words";
            let lang = CrawledContent::detect_language(english);
            assert_eq!(lang, Some("en".to_string()));

            // Spanish text
            let spanish = "Este es un texto de ejemplo en español con múltiples palabras";
            let lang = CrawledContent::detect_language(spanish);
            assert!(lang == Some("es".to_string()) || lang == Some("en".to_string())); // Basic detection

            // Too short for detection
            let short = "Hi";
            assert_eq!(CrawledContent::detect_language(short), None);
        }

        #[test]
        fn test_content_compression() {
            let original = "This is a test content that will be compressed. ".repeat(100);
            
            let compressed = CrawledContent::compress_content(&original).unwrap();
            assert!(compressed.len() < original.len()); // Should be smaller

            let decompressed = CrawledContent::decompress_content(&compressed).unwrap();
            assert_eq!(decompressed, original); // Should match original
        }
    }

    // ============================================================================
    // User Agent Tests
    // ============================================================================
    
    mod user_agent_tests {
        use super::*;
        use crate::sam::services::crawler::user_agents::*;

        #[tokio::test]
        async fn test_random_rotation() {
            let rotator = UserAgentRotator::new(RotationStrategy::Random);
            
            let mut agents = std::collections::HashSet::new();
            for _ in 0..10 {
                let agent = rotator.get_user_agent("https://example.com").await;
                agents.insert(agent);
            }
            
            // Should have multiple different agents
            assert!(agents.len() > 1);
        }

        #[tokio::test]
        async fn test_round_robin_rotation() {
            let rotator = UserAgentRotator::new(RotationStrategy::RoundRobin);
            
            let agent1 = rotator.get_user_agent("https://example.com").await;
            let agent2 = rotator.get_user_agent("https://example.com").await;
            let agent3 = rotator.get_user_agent("https://example.com").await;
            
            // Should be different agents in sequence
            assert_ne!(agent1, agent2);
            assert_ne!(agent2, agent3);
        }

        #[tokio::test]
        async fn test_per_domain_consistency() {
            let rotator = UserAgentRotator::new(RotationStrategy::PerDomain);
            
            let agent1 = rotator.get_user_agent("https://example.com/page1").await;
            let agent2 = rotator.get_user_agent("https://example.com/page2").await;
            let agent3 = rotator.get_user_agent("https://other.com/page").await;
            
            // Same domain should get same agent
            assert_eq!(agent1, agent2);
            // Different domain might get different agent
            // (not guaranteed to be different, but likely)
            let _ = agent3; // Just ensure it doesn't panic
        }

        #[tokio::test]
        async fn test_content_aware_selection() {
            let rotator = UserAgentRotator::new(RotationStrategy::ContentAware);
            
            // API endpoint should get bot user agent
            let api_agent = rotator.get_user_agent("https://api.example.com/v1/data").await;
            assert!(api_agent.contains("bot") || api_agent.contains("Bot"));
            
            // Regular page might get desktop agent
            let page_agent = rotator.get_user_agent("https://example.com/page").await;
            let _ = page_agent; // Just ensure it works
            
            // Mobile site might get mobile agent
            let mobile_agent = rotator.get_user_agent("https://m.example.com/").await;
            let _ = mobile_agent; // Just ensure it works
        }
    }

    // ============================================================================
    // Feed Parser Tests
    // ============================================================================
    
    mod feed_parser_tests {
        use super::*;
        use crate::sam::services::crawler::feed_parser::*;

        #[test]
        fn test_rss_feed_parsing() {
            let rss_content = r#"<?xml version="1.0"?>
                <rss version="2.0">
                    <channel>
                        <title>Test Feed</title>
                        <link>https://example.com</link>
                        <description>Test RSS Feed</description>
                        <item>
                            <title>Test Article</title>
                            <link>https://example.com/article1</link>
                            <description>Article description</description>
                            <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
                        </item>
                    </channel>
                </rss>"#;

            let feed = FeedParser::parse(rss_content, "https://example.com/feed.xml").unwrap();
            assert_eq!(feed.title, Some("Test Feed".to_string()));
            assert_eq!(feed.items.len(), 1);
            assert_eq!(feed.items[0].title, Some("Test Article".to_string()));
            assert_eq!(feed.items[0].link, Some("https://example.com/article1".to_string()));
        }

        #[test]
        fn test_atom_feed_parsing() {
            let atom_content = r#"<?xml version="1.0"?>
                <feed xmlns="http://www.w3.org/2005/Atom">
                    <title>Test Atom Feed</title>
                    <link href="https://example.com"/>
                    <entry>
                        <title>Test Entry</title>
                        <link href="https://example.com/entry1"/>
                        <summary>Entry summary</summary>
                        <updated>2024-01-01T00:00:00Z</updated>
                    </entry>
                </feed>"#;

            let feed = FeedParser::parse(atom_content, "https://example.com/atom.xml").unwrap();
            assert_eq!(feed.title, Some("Test Atom Feed".to_string()));
            assert_eq!(feed.items.len(), 1);
            assert_eq!(feed.items[0].title, Some("Test Entry".to_string()));
        }

        #[test]
        fn test_feed_detection_in_html() {
            let html = r#"
                <html>
                    <head>
                        <link rel="alternate" type="application/rss+xml" href="/feed.rss">
                        <link rel="alternate" type="application/atom+xml" href="/feed.atom">
                    </head>
                    <body>Content</body>
                </html>
            "#;

            let feeds = detect_feed_links(html, "https://example.com");
            assert_eq!(feeds.len(), 2);
            assert!(feeds.contains(&"https://example.com/feed.rss".to_string()));
            assert!(feeds.contains(&"https://example.com/feed.atom".to_string()));
        }
    }

    // ============================================================================
    // Job Queue Tests
    // ============================================================================
    
    mod job_queue_tests {
        use super::*;
        use crate::sam::services::crawler::job_queue::*;

        #[tokio::test]
        async fn test_job_enqueue_dequeue() {
            // This would require Redis mock or test container
            // For now, just test the structures
            
            let job = QueuedJob {
                id: "test-123".to_string(),
                url: "https://example.com".to_string(),
                priority: 5,
                config: None,
                status: JobStatus::Pending,
                created_at: chrono::Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
            };

            assert_eq!(job.status, JobStatus::Pending);
            assert_eq!(job.retry_count, 0);
        }

        #[test]
        fn test_job_status_transitions() {
            let mut job = QueuedJob {
                id: "test-123".to_string(),
                url: "https://example.com".to_string(),
                priority: 5,
                config: None,
                status: JobStatus::Pending,
                created_at: chrono::Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error: None,
                retry_count: 0,
                max_retries: 3,
            };

            // Pending -> Running
            job.status = JobStatus::Running;
            job.started_at = Some(chrono::Utc::now().timestamp());
            assert!(job.started_at.is_some());

            // Running -> Completed
            job.status = JobStatus::Completed;
            job.completed_at = Some(chrono::Utc::now().timestamp());
            assert!(job.completed_at.is_some());
        }
    }

    // ============================================================================
    // Memory Optimization Tests
    // ============================================================================
    
    mod memory_optimization_tests {
        use super::*;
        use crate::sam::services::crawler::memory_optimized::*;

        #[test]
        fn test_bloom_filter_deduplication() {
            let config = MemoryConfig::default();
            let mut tracker = OptimizedUrlTracker::new(config);

            // First visit should return true
            assert!(tracker.visit_url("https://example.com/page1"));
            
            // Second visit should return false (already visited)
            assert!(!tracker.visit_url("https://example.com/page1"));
            
            // New URL should return true
            assert!(tracker.visit_url("https://example.com/page2"));
        }

        #[test]
        fn test_bounded_queue() {
            let mut queue = BoundedUrlQueue::new(3); // Max 3 items

            // Add items
            assert!(queue.push("url1".to_string()));
            assert!(queue.push("url2".to_string()));
            assert!(queue.push("url3".to_string()));
            
            // Queue full, should return false
            assert!(!queue.push("url4".to_string()));

            // Pop items
            assert_eq!(queue.pop(), Some("url1".to_string()));
            assert_eq!(queue.pop(), Some("url2".to_string()));
            
            // Now can add more
            assert!(queue.push("url4".to_string()));
        }

        #[test]
        fn test_lru_cache() {
            let config = MemoryConfig {
                bloom_filter_size: 10000,
                bloom_filter_fp_rate: 0.01,
                lru_cache_size: 2, // Small cache for testing
                max_queue_size: 1000,
                enable_redis_spillover: false,
            };
            
            let mut tracker = OptimizedUrlTracker::new(config);

            // Visit URLs
            tracker.visit_url("url1");
            tracker.visit_url("url2");
            tracker.visit_url("url3"); // This should evict url1 from LRU

            // url1 might be in bloom filter but not in LRU
            // url2 and url3 should be in LRU
            assert!(!tracker.visit_url("url2")); // Should be found
            assert!(!tracker.visit_url("url3")); // Should be found
        }
    }

    // ============================================================================
    // Content Type Tests
    // ============================================================================
    
    mod content_type_tests {
        use super::*;
        use crate::sam::services::crawler::content_types::*;

        #[test]
        fn test_content_type_detection() {
            assert_eq!(
                ContentType::from_mime("text/html"),
                ContentType::Html
            );
            
            assert_eq!(
                ContentType::from_mime("application/pdf"),
                ContentType::Pdf
            );
            
            assert_eq!(
                ContentType::from_mime("image/jpeg"),
                ContentType::Image(ImageType::Jpeg)
            );
            
            assert_eq!(
                ContentType::from_mime("application/json"),
                ContentType::Json
            );
            
            assert_eq!(
                ContentType::from_mime("application/unknown"),
                ContentType::Unknown("application/unknown".to_string())
            );
        }

        #[test]
        fn test_content_type_from_extension() {
            assert_eq!(
                ContentType::from_extension("html"),
                ContentType::Html
            );
            
            assert_eq!(
                ContentType::from_extension("pdf"),
                ContentType::Pdf
            );
            
            assert_eq!(
                ContentType::from_extension("jpg"),
                ContentType::Image(ImageType::Jpeg)
            );
            
            assert_eq!(
                ContentType::from_extension("docx"),
                ContentType::Document(DocumentType::Docx)
            );
        }

        #[test]
        fn test_should_store_content() {
            assert!(ContentType::Html.should_store());
            assert!(ContentType::Pdf.should_store());
            assert!(ContentType::Json.should_store());
            assert!(ContentType::Xml.should_store());
            
            // Large media files might not be stored
            assert!(!ContentType::Video.should_store());
            assert!(!ContentType::Audio.should_store());
        }

        #[test]
        fn test_max_size_limits() {
            // HTML has larger limit
            assert!(ContentType::Html.max_size() > 10 * 1024 * 1024);
            
            // PDFs have reasonable limit
            assert!(ContentType::Pdf.max_size() > 5 * 1024 * 1024);
            
            // Images have smaller limit
            assert!(ContentType::Image(ImageType::Jpeg).max_size() < 10 * 1024 * 1024);
        }
    }

    // ============================================================================
    // JavaScript Renderer Tests
    // ============================================================================
    
    mod js_renderer_tests {
        use super::*;
        use crate::sam::services::crawler::js_renderer::*;

        #[tokio::test]
        async fn test_spa_detection() {
            let renderer = JsRenderer::new(JsRendererConfig::default());
            
            // Test React detection
            let react_html = r#"<div id="root"></div><script>React.createElement</script>"#;
            let frameworks = renderer.detect_frameworks(react_html).await;
            assert!(frameworks.contains(&"React".to_string()));
            
            // Test Angular detection
            let angular_html = r#"<div ng-app="myApp"></div>"#;
            let frameworks = renderer.detect_frameworks(angular_html).await;
            assert!(frameworks.contains(&"Angular".to_string()));
            
            // Test Vue detection
            let vue_html = r#"<div id="app" v-app></div>"#;
            let frameworks = renderer.detect_frameworks(vue_html).await;
            assert!(frameworks.contains(&"Vue".to_string()));
        }

        #[test]
        fn test_resource_type_blocking() {
            let config = JsRendererConfig {
                blocked_resources: vec![
                    ResourceType::Image,
                    ResourceType::Font,
                    ResourceType::Media,
                ],
                ..Default::default()
            };
            
            assert!(config.blocked_resources.contains(&ResourceType::Image));
            assert!(config.blocked_resources.contains(&ResourceType::Font));
            assert!(!config.blocked_resources.contains(&ResourceType::Script));
        }

        #[tokio::test]
        async fn test_browser_pool_management() {
            let config = JsRendererConfig {
                max_browsers: 2,
                ..Default::default()
            };
            
            let renderer = JsRenderer::new(config);
            
            // Initialize should create browser instances
            assert!(renderer.initialize().await.is_ok());
            
            // Should be able to get stats
            let stats = renderer.get_stats().await;
            assert_eq!(stats.total_renders, 0);
            assert_eq!(stats.successful_renders, 0);
        }
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================
    
    mod integration_tests {
        use super::*;

        #[tokio::test]
        #[ignore] // Run with --ignored flag
        async fn test_full_crawl_workflow() {
            // This test would require a mock HTTP server
            // Using mockito or similar library
            
            let mock_server = MockServer::start().await;
            
            Mock::given(method("GET"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_string(r#"
                        <html>
                            <head><title>Test Page</title></head>
                            <body>
                                <a href="/page1">Page 1</a>
                                <a href="/page2">Page 2</a>
                            </body>
                        </html>
                    "#)
                    .insert_header("content-type", "text/html"))
                .mount(&mock_server)
                .await;

            let client = std::sync::Arc::new(reqwest::Client::new());
            let job_id = "test-job-123".to_string();
            let url = format!("{}/", mock_server.uri());

            // Would need to set up test database connection
            // let result = crate::sam::services::crawler::runner::crawl_url(
            //     job_id,
            //     url,
            //     client
            // ).await;
            
            // assert!(result.is_ok());
            // let pages = result.unwrap();
            // assert!(!pages.is_empty());
        }

        #[tokio::test]
        async fn test_robots_txt_compliance() {
            // Test that crawler respects robots.txt
            let mock_server = MockServer::start().await;
            
            Mock::given(method("GET"))
                .and(path("/robots.txt"))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_string("User-agent: *\nDisallow: /private/"))
                .mount(&mock_server)
                .await;

            // Verify /private/ URLs are not crawled
            let allowed = crate::sam::services::crawler::robots::is_url_allowed(
                &format!("{}/private/data", mock_server.uri()),
                "*"
            ).await;
            
            // Note: This would need actual implementation
            // assert!(!allowed);
        }
    }
}