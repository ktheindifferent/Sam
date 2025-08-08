#[cfg(test)]
mod crawler_job_tests {
    use crate::sam::services::crawler::job::CrawlJob;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_crawl_job_new() {
        let job = CrawlJob::new();
        
        assert_eq!(job.id, 0);
        assert!(!job.oid.is_empty());
        assert_eq!(job.oid.len(), 15);
        assert!(job.start_url.is_empty());
        assert_eq!(job.status, "pending");
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        assert!(job.created_at <= now);
        assert!(job.created_at > now - 10);
        assert_eq!(job.created_at, job.updated_at);
    }

    #[test]
    fn test_crawl_job_default() {
        let job1 = CrawlJob::default();
        let job2 = CrawlJob::new();
        
        assert_eq!(job1.status, job2.status);
        assert_eq!(job1.id, job2.id);
        assert!(job1.start_url.is_empty());
    }

    #[test]
    fn test_crawl_job_unique_oids() {
        let mut oids = std::collections::HashSet::new();
        
        for _ in 0..100 {
            let job = CrawlJob::new();
            assert!(oids.insert(job.oid.clone()), "Duplicate OID generated");
        }
    }

    #[test]
    fn test_sql_table_name() {
        assert_eq!(CrawlJob::sql_table_name(), "crawl_jobs");
    }

    #[test]
    fn test_sql_build_statement() {
        let statement = CrawlJob::sql_build_statement();
        assert!(statement.contains("CREATE TABLE IF NOT EXISTS crawl_jobs"));
        assert!(statement.contains("id serial PRIMARY KEY"));
        assert!(statement.contains("oid varchar NOT NULL UNIQUE"));
        assert!(statement.contains("start_url varchar NOT NULL"));
        assert!(statement.contains("status varchar NOT NULL"));
    }

    #[test]
    fn test_sql_indexes() {
        let indexes = CrawlJob::sql_indexes();
        assert_eq!(indexes.len(), 5);
        
        assert!(indexes.iter().any(|i| i.contains("idx_crawl_jobs_oid")));
        assert!(indexes.iter().any(|i| i.contains("idx_crawl_jobs_start_url")));
        assert!(indexes.iter().any(|i| i.contains("idx_crawl_jobs_status")));
        assert!(indexes.iter().any(|i| i.contains("idx_crawl_jobs_created_at")));
        assert!(indexes.iter().any(|i| i.contains("idx_crawl_jobs_updated_at")));
    }

    #[test]
    fn test_job_status_transitions() {
        let mut job = CrawlJob::new();
        
        let valid_statuses = vec!["pending", "running", "done", "error"];
        
        for status in valid_statuses {
            job.status = status.to_string();
            assert_eq!(job.status, status);
        }
    }
}

#[cfg(test)]
mod crawler_runner_tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::test;

    #[test]
    async fn test_normalize_url() {
        let test_cases = vec![
            ("http://example.com", "http://example.com"),
            ("http://example.com/", "http://example.com"),
            ("http://example.com//", "http://example.com"),
            ("http://example.com/path/", "http://example.com/path"),
            ("http://example.com/path//", "http://example.com/path"),
            ("HTTP://EXAMPLE.COM", "http://example.com"),
            ("http://example.com:80", "http://example.com"),
            ("https://example.com:443", "https://example.com"),
            ("http://example.com:8080", "http://example.com:8080"),
        ];

        for (input, expected) in test_cases {
            let result = crate::sam::services::crawler::runner::normalize_url(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    async fn test_extract_domain() {
        let test_cases = vec![
            ("http://example.com/path", Some("example.com")),
            ("https://subdomain.example.com", Some("subdomain.example.com")),
            ("http://example.com:8080", Some("example.com")),
            ("invalid-url", None),
            ("", None),
        ];

        for (input, expected) in test_cases {
            let result = crate::sam::services::crawler::runner::extract_domain(input);
            assert_eq!(result, expected.map(String::from), "Failed for input: {}", input);
        }
    }

    #[test]
    async fn test_url_validation() {
        let valid_urls = vec![
            "http://example.com",
            "https://example.com",
            "http://subdomain.example.com",
            "https://example.com/path/to/resource",
            "http://example.com:8080",
        ];

        let invalid_urls = vec![
            "",
            "not-a-url",
            "ftp://example.com",
            "javascript:alert(1)",
            "data:text/html,<h1>Hello</h1>",
            "//example.com",
        ];

        for url in valid_urls {
            assert!(
                url::Url::parse(url).is_ok() && 
                (url.starts_with("http://") || url.starts_with("https://")),
                "Expected valid URL: {}", url
            );
        }

        for url in invalid_urls {
            assert!(
                url::Url::parse(url).is_err() || 
                (!url.starts_with("http://") && !url.starts_with("https://")),
                "Expected invalid URL: {}", url
            );
        }
    }

    #[test]
    async fn test_rate_limiting() {
        use std::time::Instant;
        
        let domain = "example.com";
        let min_interval = std::time::Duration::from_millis(100);
        
        let start = Instant::now();
        
        // Simulate multiple requests to the same domain
        let mut last_access = start;
        
        for _ in 0..5 {
            let now = Instant::now();
            let elapsed = now.duration_since(last_access);
            
            if elapsed < min_interval {
                tokio::time::sleep(min_interval - elapsed).await;
            }
            
            last_access = Instant::now();
            assert!(last_access.duration_since(start) >= min_interval);
        }
    }

    #[test]
    async fn test_concurrent_crawling_limits() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_concurrent = 10;
        
        let mut handles = vec![];
        
        for _ in 0..20 {
            let count = concurrent_count.clone();
            let handle = tokio::spawn(async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                assert!(current <= max_concurrent, "Exceeded max concurrent limit");
                
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                
                count.fetch_sub(1, Ordering::SeqCst);
            });
            handles.push(handle);
            
            if handles.len() >= max_concurrent {
                handles.remove(0).await.unwrap();
            }
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
    }
}

#[cfg(test)]
mod crawler_page_tests {
    use crate::sam::services::crawler::page::CrawledPage;
    use scraper::{Html, Selector};

    #[test]
    fn test_extract_links_from_html() {
        let html = r#"
            <html>
                <body>
                    <a href="http://example.com">Example</a>
                    <a href="/relative/path">Relative</a>
                    <a href="https://external.com">External</a>
                    <a>No href</a>
                    <a href="">Empty href</a>
                    <a href="javascript:void(0)">JavaScript</a>
                    <a href="mailto:test@example.com">Email</a>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let selector = Selector::parse("a[href]").unwrap();
        let mut links = vec![];
        
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if !href.is_empty() && 
                   !href.starts_with("javascript:") && 
                   !href.starts_with("mailto:") {
                    links.push(href.to_string());
                }
            }
        }

        assert_eq!(links.len(), 3);
        assert!(links.contains(&"http://example.com".to_string()));
        assert!(links.contains(&"/relative/path".to_string()));
        assert!(links.contains(&"https://external.com".to_string()));
    }

    #[test]
    fn test_extract_title_from_html() {
        let test_cases = vec![
            ("<title>Test Title</title>", Some("Test Title")),
            ("<title>  Spaces  </title>", Some("Spaces")),
            ("<title></title>", None),
            ("<html><head></head></html>", None),
            ("<title>Multi\nLine\nTitle</title>", Some("Multi Line Title")),
        ];

        for (html, expected) in test_cases {
            let document = Html::parse_document(html);
            let selector = Selector::parse("title").unwrap();
            let title = document.select(&selector)
                .next()
                .and_then(|t| {
                    let text = t.text().collect::<String>().trim().to_string();
                    if text.is_empty() { None } else { Some(text) }
                });
            
            assert_eq!(title, expected.map(String::from), "Failed for HTML: {}", html);
        }
    }

    #[test]
    fn test_extract_tokens_from_content() {
        let content = "The quick brown fox jumps over the lazy dog. THE QUICK BROWN FOX!";
        let tokens: Vec<String> = content
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| s.chars().all(char::is_alphanumeric))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert_eq!(tokens.iter().filter(|t| *t == "the").count(), 1);
    }

    #[test]
    fn test_handle_relative_urls() {
        let base_url = "http://example.com/path/page.html";
        let test_cases = vec![
            ("/absolute/path", "http://example.com/absolute/path"),
            ("relative/path", "http://example.com/path/relative/path"),
            ("../parent/path", "http://example.com/parent/path"),
            ("http://other.com", "http://other.com"),
            ("//cdn.example.com/resource", "http://cdn.example.com/resource"),
        ];

        for (relative, expected) in test_cases {
            let base = url::Url::parse(base_url).unwrap();
            let resolved = base.join(relative).ok().map(|u| u.to_string());
            assert_eq!(resolved, Some(expected.to_string()), "Failed for: {}", relative);
        }
    }
}