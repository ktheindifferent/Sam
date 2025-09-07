//! Integration tests for the crawler with mock HTTP servers

#[cfg(test)]
mod crawler_integration_tests {
    use sam::sam::services::crawler;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_crawl_with_mock_server() {
        // Start mock server
        let mock_server = MockServer::start().await;
        
        // Set up mock response for main page
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"
                    <!DOCTYPE html>
                    <html>
                        <head>
                            <title>Test Page</title>
                            <meta name="description" content="Test description">
                        </head>
                        <body>
                            <h1>Welcome</h1>
                            <a href="/page1">Page 1</a>
                            <a href="/page2">Page 2</a>
                            <a href="https://external.com">External Link</a>
                        </body>
                    </html>
                "#)
                .insert_header("content-type", "text/html"))
            .mount(&mock_server)
            .await;

        // Set up robots.txt
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nAllow: /\nCrawl-delay: 1"))
            .mount(&mock_server)
            .await;

        // Create HTTP client
        let client = Arc::new(reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap());

        let job_id = "test-job-001".to_string();
        let url = format!("{}/", mock_server.uri());

        // Perform crawl
        let result = crawler::crawl_url(job_id, url.clone(), client).await;
        
        assert!(result.is_ok(), "Crawl should succeed");
        let pages = result.unwrap();
        assert!(!pages.is_empty(), "Should have crawled at least one page");
        
        // Verify the first page
        let first_page = &pages[0];
        assert_eq!(first_page.url, url);
        assert!(first_page.title.contains("Test Page"));
    }

    #[tokio::test]
    async fn test_robots_txt_blocking() {
        let mock_server = MockServer::start().await;
        
        // Set up restrictive robots.txt
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nDisallow: /private/"))
            .mount(&mock_server)
            .await;

        // Check if private URL is allowed
        let private_url = format!("{}/private/data", mock_server.uri());
        let allowed = crawler::robots::is_url_allowed(&private_url, "*").await;
        
        assert!(!allowed, "Private URLs should be blocked by robots.txt");
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mock_server = MockServer::start().await;
        
        // Set up robots.txt with crawl delay
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nAllow: /\nCrawl-delay: 2"))
            .mount(&mock_server)
            .await;

        // Set up pages
        for i in 1..=3 {
            Mock::given(method("GET"))
                .and(path(format!("/page{}", i)))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_string(format!("<html><body>Page {}</body></html>", i)))
                .mount(&mock_server)
                .await;
        }

        let start = std::time::Instant::now();
        
        // Make multiple requests
        let limiter = crawler::rate_limiter::get_rate_limiter().await;
        let domain = mock_server.uri().replace("http://", "");
        
        for i in 1..=3 {
            limiter.wait_if_needed(&domain).await;
            // Simulate request
            limiter.record_request(&domain, 200, Duration::from_millis(100)).await;
        }

        let elapsed = start.elapsed();
        
        // Should have taken at least 4 seconds (2 delays of 2 seconds each)
        assert!(elapsed >= Duration::from_secs(4), 
                "Rate limiting should enforce crawl delay");
    }

    #[tokio::test]
    async fn test_circuit_breaker_on_failures() {
        let mock_server = MockServer::start().await;
        
        // Set up failing endpoint
        Mock::given(method("GET"))
            .and(path("/failing"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let domain = mock_server.uri().replace("http://", "").replace("/", "");
        
        // Record multiple failures
        for _ in 0..5 {
            crawler::circuit_breaker::record_domain_failure(&domain).await;
        }

        // Check if domain is blocked
        let allowed = crawler::circuit_breaker::is_domain_allowed(&domain).await;
        assert!(!allowed, "Domain should be blocked after multiple failures");
        
        // Get circuit state
        let state = crawler::circuit_breaker::get_domain_state(&domain).await;
        assert_eq!(state, crawler::circuit_breaker::CircuitState::Open);
    }

    #[tokio::test]
    async fn test_content_deduplication() {
        let content1 = "This is test content for deduplication";
        let content2 = "This is test content for deduplication"; // Same content
        let content3 = "This is different content";

        let hash1 = crawler::CrawledContent::compute_hash(content1);
        let hash2 = crawler::CrawledContent::compute_hash(content2);
        let hash3 = crawler::CrawledContent::compute_hash(content3);

        assert_eq!(hash1, hash2, "Same content should have same hash");
        assert_ne!(hash1, hash3, "Different content should have different hash");
    }

    #[tokio::test]
    async fn test_sitemap_parsing() {
        let mock_server = MockServer::start().await;
        
        // Set up sitemap.xml
        Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"<?xml version="1.0" encoding="UTF-8"?>
                    <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
                        <url>
                            <loc>https://example.com/page1</loc>
                            <lastmod>2024-01-01</lastmod>
                            <priority>0.8</priority>
                        </url>
                        <url>
                            <loc>https://example.com/page2</loc>
                            <lastmod>2024-01-02</lastmod>
                            <priority>0.6</priority>
                        </url>
                    </urlset>"#)
                .insert_header("content-type", "application/xml"))
            .mount(&mock_server)
            .await;

        let sitemap_url = format!("{}/sitemap.xml", mock_server.uri());
        let urls = crawler::sitemap::extract_urls_from_sitemaps(&[sitemap_url]).await;
        
        assert_eq!(urls.len(), 2, "Should extract 2 URLs from sitemap");
        assert!(urls.contains(&"https://example.com/page1".to_string()));
        assert!(urls.contains(&"https://example.com/page2".to_string()));
    }

    #[tokio::test]
    async fn test_feed_detection_and_parsing() {
        let mock_server = MockServer::start().await;
        
        // Set up HTML with feed links
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"
                    <html>
                        <head>
                            <link rel="alternate" type="application/rss+xml" href="/feed.rss">
                            <link rel="alternate" type="application/atom+xml" href="/feed.atom">
                        </head>
                        <body>Content</body>
                    </html>
                "#))
            .mount(&mock_server)
            .await;

        // Set up RSS feed
        Mock::given(method("GET"))
            .and(path("/feed.rss"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"<?xml version="1.0"?>
                    <rss version="2.0">
                        <channel>
                            <title>Test Feed</title>
                            <item>
                                <title>Article 1</title>
                                <link>https://example.com/article1</link>
                            </item>
                        </channel>
                    </rss>"#))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let html = reqwest::get(&base_url).await.unwrap().text().await.unwrap();
        
        let feeds = crawler::feed_parser::detect_feed_links(&html, &base_url);
        assert_eq!(feeds.len(), 2, "Should detect 2 feed links");
        
        // Parse RSS feed
        let rss_url = format!("{}/feed.rss", base_url);
        let rss_content = reqwest::get(&rss_url).await.unwrap().text().await.unwrap();
        let feed = crawler::feed_parser::FeedParser::parse(&rss_content, &rss_url).unwrap();
        
        assert_eq!(feed.title, Some("Test Feed".to_string()));
        assert_eq!(feed.items.len(), 1);
    }

    #[tokio::test]
    async fn test_user_agent_rotation() {
        let rotator = crawler::user_agents::UserAgentRotator::new(
            crawler::user_agents::RotationStrategy::RoundRobin
        );
        
        let mut agents = Vec::new();
        for _ in 0..3 {
            let agent = rotator.get_user_agent("https://example.com").await;
            agents.push(agent);
        }
        
        // Should have different agents
        assert!(agents[0] != agents[1] || agents[1] != agents[2],
                "Round-robin should rotate through different user agents");
    }

    #[tokio::test]
    async fn test_url_pattern_detection() {
        let detector = crawler::url_patterns::UrlPatternDetector::new();
        
        // Test calendar patterns
        assert!(detector.is_calendar_pattern("https://example.com/2024/01/15"));
        assert!(detector.is_calendar_pattern("https://example.com/calendar/2024-01"));
        assert!(!detector.is_calendar_pattern("https://example.com/about"));
        
        // Test pagination patterns
        assert!(detector.is_pagination_pattern("https://example.com/posts?page=5"));
        assert!(detector.is_pagination_pattern("https://example.com/results?offset=100"));
        assert!(!detector.is_pagination_pattern("https://example.com/contact"));
    }

    #[tokio::test]
    async fn test_javascript_rendering_detection() {
        let renderer = crawler::js_renderer::JsRenderer::new(
            crawler::js_renderer::JsRendererConfig::default()
        );
        
        // Test framework detection
        let react_html = r#"<div id="root"></div><script>React.createElement</script>"#;
        let frameworks = renderer.detect_frameworks(react_html).await;
        assert!(frameworks.contains(&"React".to_string()));
        
        let vue_html = r#"<div id="app" v-app></div>"#;
        let frameworks = renderer.detect_frameworks(vue_html).await;
        assert!(frameworks.contains(&"Vue".to_string()));
    }

    #[tokio::test]
    async fn test_content_type_handling() {
        let mock_server = MockServer::start().await;
        
        // Set up different content types
        Mock::given(method("GET"))
            .and(path("/document.pdf"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_bytes(vec![0x25, 0x50, 0x44, 0x46]) // PDF magic bytes
                .insert_header("content-type", "application/pdf"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/data.json"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"{"key": "value"}"#)
                .insert_header("content-type", "application/json"))
            .mount(&mock_server)
            .await;

        // Test content type detection
        assert_eq!(
            crawler::content_types::ContentType::from_mime("application/pdf"),
            crawler::content_types::ContentType::Pdf
        );
        
        assert_eq!(
            crawler::content_types::ContentType::from_mime("application/json"),
            crawler::content_types::ContentType::Json
        );
    }

    #[tokio::test]
    async fn test_memory_optimization() {
        let config = crawler::memory_optimized::MemoryConfig::default();
        let mut tracker = crawler::memory_optimized::OptimizedUrlTracker::new(config);
        
        // First visit should return true
        assert!(tracker.visit_url("https://example.com/page1"));
        
        // Second visit should return false (already visited)
        assert!(!tracker.visit_url("https://example.com/page1"));
        
        // New URL should return true
        assert!(tracker.visit_url("https://example.com/page2"));
    }
}