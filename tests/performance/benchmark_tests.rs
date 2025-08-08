#[cfg(test)]
mod performance_benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
    use std::time::Duration;
    use tokio::runtime::Runtime;

    fn benchmark_crawl_job_creation(c: &mut Criterion) {
        c.bench_function("crawl_job_new", |b| {
            b.iter(|| {
                let job = crate::sam::services::crawler::CrawlJob::new();
                black_box(job);
            });
        });
    }

    fn benchmark_url_normalization(c: &mut Criterion) {
        let urls = vec![
            "http://example.com",
            "http://example.com/path/to/page",
            "https://subdomain.example.com/very/long/path/with/many/segments",
            "http://example.com:8080/path?query=param&another=value#fragment",
        ];

        let mut group = c.benchmark_group("url_normalization");
        for url in urls.iter() {
            group.bench_with_input(BenchmarkId::from_parameter(url), url, |b, url| {
                b.iter(|| {
                    let normalized = crate::sam::services::crawler::runner::normalize_url(url);
                    black_box(normalized);
                });
            });
        }
        group.finish();
    }

    fn benchmark_token_extraction(c: &mut Criterion) {
        let content_sizes = vec![
            ("small", "The quick brown fox jumps over the lazy dog."),
            ("medium", &"Lorem ipsum dolor sit amet. ".repeat(100)),
            ("large", &"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000)),
        ];

        let mut group = c.benchmark_group("token_extraction");
        for (size, content) in content_sizes.iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), content, |b, content| {
                b.iter(|| {
                    let tokens: Vec<String> = content
                        .split_whitespace()
                        .map(|s| s.to_lowercase())
                        .filter(|s| s.chars().all(char::is_alphanumeric))
                        .collect();
                    black_box(tokens);
                });
            });
        }
        group.finish();
    }

    fn benchmark_concurrent_crawls(c: &mut Criterion) {
        let rt = Runtime::new().unwrap();
        
        let mut group = c.benchmark_group("concurrent_crawls");
        group.measurement_time(Duration::from_secs(10));
        
        for num_concurrent in &[1, 5, 10, 20] {
            group.bench_with_input(
                BenchmarkId::from_parameter(num_concurrent),
                num_concurrent,
                |b, &num| {
                    b.to_async(&rt).iter(|| async move {
                        let mut handles = vec![];
                        for _ in 0..num {
                            handles.push(tokio::spawn(async {
                                // Simulate crawl work
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }));
                        }
                        futures::future::join_all(handles).await;
                    });
                },
            );
        }
        group.finish();
    }

    fn benchmark_dns_cache_lookup(c: &mut Criterion) {
        let rt = Runtime::new().unwrap();
        
        // Pre-populate cache
        rt.block_on(async {
            let mut cache = HashMap::new();
            for i in 0..1000 {
                cache.insert(format!("domain{}.com", i), i % 2 == 0);
            }
            *crate::sam::services::crawler::runner::DNS_LOOKUP_CACHE.lock().await = cache;
        });

        c.bench_function("dns_cache_lookup", |b| {
            b.to_async(&rt).iter(|| async {
                let domain = format!("domain{}.com", rand::random::<u32>() % 1000);
                let cache = crate::sam::services::crawler::runner::DNS_LOOKUP_CACHE.lock().await;
                let result = cache.get(&domain);
                black_box(result);
            });
        });
    }

    fn benchmark_html_parsing(c: &mut Criterion) {
        let html_samples = vec![
            ("simple", "<html><body>Hello World</body></html>"),
            ("medium", &format!("<html><body>{}</body></html>", 
                "<p>Paragraph</p>".repeat(100))),
            ("complex", &format!("<html><head><title>Test</title></head><body>{}{}</body></html>",
                "<div><p>Content</p></div>".repeat(100),
                "<a href='link'>Link</a>".repeat(50))),
        ];

        let mut group = c.benchmark_group("html_parsing");
        for (name, html) in html_samples.iter() {
            group.bench_with_input(BenchmarkId::from_parameter(name), html, |b, html| {
                b.iter(|| {
                    let document = scraper::Html::parse_document(html);
                    let title_selector = scraper::Selector::parse("title").unwrap();
                    let link_selector = scraper::Selector::parse("a[href]").unwrap();
                    
                    let _title = document.select(&title_selector).next();
                    let _links: Vec<_> = document.select(&link_selector).collect();
                    
                    black_box(document);
                });
            });
        }
        group.finish();
    }

    criterion_group!(
        benches,
        benchmark_crawl_job_creation,
        benchmark_url_normalization,
        benchmark_token_extraction,
        benchmark_concurrent_crawls,
        benchmark_dns_cache_lookup,
        benchmark_html_parsing
    );
}

#[cfg(test)]
mod memory_benchmarks {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingAllocator;

    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            }
            ret
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        }
    }

    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    #[test]
    fn test_memory_usage_crawl_job() {
        let before = ALLOCATED.load(Ordering::SeqCst);
        
        let mut jobs = Vec::new();
        for _ in 0..1000 {
            jobs.push(crate::sam::services::crawler::CrawlJob::new());
        }
        
        let after = ALLOCATED.load(Ordering::SeqCst);
        let per_job = (after - before) / 1000;
        
        println!("Memory per CrawlJob: {} bytes", per_job);
        assert!(per_job < 1024, "CrawlJob uses too much memory");
        
        drop(jobs);
        
        let cleaned = ALLOCATED.load(Ordering::SeqCst);
        assert!(cleaned <= before + 1024, "Memory leak detected");
    }

    #[test]
    fn test_memory_usage_crawled_page() {
        let before = ALLOCATED.load(Ordering::SeqCst);
        
        let mut pages = Vec::new();
        for i in 0..100 {
            let page = crate::sam::services::crawler::CrawledPage {
                url: format!("http://example{}.com", i),
                title: Some(format!("Title {}", i)),
                content: "x".repeat(1000),
                links: vec![format!("http://link{}.com", i)],
                status_code: 200,
                headers: std::collections::HashMap::new(),
                crawled_at: chrono::Utc::now(),
                tokens: vec!["token".to_string(); 10],
                error: None,
            };
            pages.push(page);
        }
        
        let after = ALLOCATED.load(Ordering::SeqCst);
        let total_size = after - before;
        
        println!("Total memory for 100 CrawledPages: {} bytes", total_size);
        assert!(total_size < 1024 * 1024, "CrawledPages use too much memory");
        
        drop(pages);
    }
}