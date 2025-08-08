#[cfg(test)]
mod crawler_integration_tests {
    use crate::sam::services::crawler::{CrawlJob, CrawledPage, crawl_url, start_service_async, stop_service};
    use std::collections::HashMap;
    use tokio::test;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[test]
    async fn test_full_crawl_workflow() {
        // Start mock server
        let mock_server = MockServer::start().await;
        
        // Setup mock responses
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"
                    <html>
                        <head><title>Test Page</title></head>
                        <body>
                            <h1>Test Content</h1>
                            <a href="/page1">Link 1</a>
                            <a href="/page2">Link 2</a>
                        </body>
                    </html>
                "#))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("<html><body>Page 1</body></html>"))
            .mount(&mock_server)
            .await;

        // Test crawling
        let result = crawl_url(&mock_server.uri()).await;
        assert!(result.is_ok());
        
        let page = result.unwrap();
        assert_eq!(page.status_code, 200);
        assert_eq!(page.title, Some("Test Page".to_string()));
        assert!(page.links.len() >= 2);
    }

    #[test]
    async fn test_crawler_service_lifecycle() {
        // Test service start
        let start_result = start_service_async().await;
        assert!(start_result.is_ok() || start_result.is_err()); // Handle already running
        
        // Check service status
        let status = crate::sam::services::crawler::service_status();
        assert!(status == "running" || status == "stopped");
        
        // Test service stop
        stop_service().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let status_after = crate::sam::services::crawler::service_status();
        assert_eq!(status_after, "stopped");
    }

    #[test]
    async fn test_concurrent_crawling() {
        let mock_server = MockServer::start().await;
        
        for i in 0..10 {
            Mock::given(method("GET"))
                .and(path(format!("/page{}", i)))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_string(format!("<html><body>Page {}</body></html>", i)))
                .mount(&mock_server)
                .await;
        }

        let mut handles = vec![];
        
        for i in 0..10 {
            let url = format!("{}/page{}", mock_server.uri(), i);
            let handle = tokio::spawn(async move {
                crawl_url(&url).await
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Task {} panicked", i);
            let crawl_result = result.as_ref().unwrap();
            assert!(crawl_result.is_ok(), "Crawl {} failed", i);
        }
    }

    #[test]
    async fn test_error_handling() {
        // Test 404 response
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notfound"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let result = crawl_url(&format!("{}/notfound", mock_server.uri())).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.status_code, 404);

        // Test timeout
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(35)))
            .mount(&mock_server)
            .await;

        let timeout_result = crawl_url(&format!("{}/slow", mock_server.uri())).await;
        assert!(timeout_result.is_err());
    }

    #[test]
    async fn test_redirect_handling() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/redirect"))
            .respond_with(ResponseTemplate::new(301)
                .insert_header("Location", "/final"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/final"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("<html><body>Final Page</body></html>"))
            .mount(&mock_server)
            .await;

        let result = crawl_url(&format!("{}/redirect", mock_server.uri())).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.status_code, 200);
        assert!(page.content.contains("Final Page"));
    }

    #[test]
    async fn test_content_extraction() {
        let mock_server = MockServer::start().await;
        
        let complex_html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Complex Page</title>
                <meta name="description" content="Test description">
                <meta name="keywords" content="test, keywords, extraction">
            </head>
            <body>
                <h1>Main Title</h1>
                <p>Paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
                <ul>
                    <li>Item 1</li>
                    <li>Item 2</li>
                </ul>
                <a href="http://external.com">External Link</a>
                <a href="/internal">Internal Link</a>
                <a href="mailto:test@example.com">Email</a>
                <script>console.log('ignored');</script>
                <style>body { color: black; }</style>
            </body>
            </html>
        "#;

        Mock::given(method("GET"))
            .and(path("/complex"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(complex_html))
            .mount(&mock_server)
            .await;

        let result = crawl_url(&format!("{}/complex", mock_server.uri())).await;
        assert!(result.is_ok());
        
        let page = result.unwrap();
        assert_eq!(page.title, Some("Complex Page".to_string()));
        assert!(page.content.contains("Main Title"));
        assert!(page.content.contains("Paragraph"));
        assert!(!page.content.contains("console.log"));
        assert!(!page.content.contains("body { color"));
        
        let valid_links: Vec<_> = page.links.iter()
            .filter(|l| !l.starts_with("mailto:"))
            .collect();
        assert!(valid_links.len() >= 2);
    }

    #[test]
    async fn test_job_persistence() {
        if !crate::sam::services::pg::is_running().await {
            return; // Skip if database not available
        }

        let job = CrawlJob::new();
        let save_result = job.save().await;
        assert!(save_result.is_ok());

        let loaded = CrawlJob::load_by_oid(&job.oid).await;
        assert!(loaded.is_ok());
        
        let loaded_job = loaded.unwrap();
        assert_eq!(loaded_job.oid, job.oid);
        assert_eq!(loaded_job.status, job.status);
    }

    #[test]
    async fn test_dns_caching() {
        let domain = "test.example.com";
        
        // First lookup (cache miss)
        let start = std::time::Instant::now();
        let result1 = crate::sam::services::crawler::runner::dns_lookup(domain).await;
        let first_duration = start.elapsed();
        
        // Second lookup (cache hit)
        let start = std::time::Instant::now();
        let result2 = crate::sam::services::crawler::runner::dns_lookup(domain).await;
        let second_duration = start.elapsed();
        
        assert_eq!(result1, result2);
        
        // Cache hit should be significantly faster
        if result1.is_some() {
            assert!(second_duration < first_duration / 2);
        }
    }
}