//! Performance benchmarks for the crawler components

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use libsam::services::crawler;
use std::time::Duration;
use tokio::runtime::Runtime;

fn benchmark_url_normalization(c: &mut Criterion) {
    let urls = vec![
        "https://example.com/page?utm_source=test&id=123&sessionid=abc",
        "https://example.com/page#section",
        "https://example.com/page/",
        "https://example.com/very/long/path/with/many/segments/and/parameters?a=1&b=2&c=3",
    ];

    c.bench_function("url_normalization", |b| {
        b.iter(|| {
            for url in &urls {
                crawler::url_patterns::normalize_url(black_box(url));
            }
        })
    });
}

fn benchmark_content_hashing(c: &mut Criterion) {
    let long_content = "Very long content that repeats. ".repeat(100);
    let extremely_long_content = "Extremely long content with lots of text. ".repeat(1000);
    
    let contents = vec![
        "Short content",
        "Medium length content that is a bit longer than the short one but not too long",
        &long_content,
        &extremely_long_content,
    ];

    let mut group = c.benchmark_group("content_hashing");
    for (_i, content) in contents.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(content.len()),
            content,
            |b, content| {
                b.iter(|| crawler::CrawledContent::compute_hash(black_box(content)))
            },
        );
    }
    group.finish();
}

fn benchmark_bloom_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter");
    
    for size in &[1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
            let config = crawler::memory_optimized::MemoryConfig {
                max_urls: size,
                cleanup_threshold: 0.8,
            };
                
                let mut tracker = crawler::memory_optimized::OptimizedUrlTracker::new(config);
                
                b.iter(|| {
                    for i in 0..100 {
                        let url = format!("https://example.com/page{}", i);
                        tracker.visit_url(black_box(&url));
                    }
                })
            },
        );
    }
    group.finish();
}

fn benchmark_pattern_detection(c: &mut Criterion) {
    let detector = crawler::url_patterns::UrlPatternDetector::new();
    
    let test_urls = vec![
        "https://example.com/2024/01/15",
        "https://example.com/posts?page=5",
        "https://example.com/about",
        "https://example.com/calendar/2024-01",
        "https://example.com/results?offset=100",
    ];

    c.bench_function("pattern_detection", |b| {
        b.iter(|| {
            for url in &test_urls {
                detector.is_calendar_pattern(black_box(url));
                detector.is_pagination_pattern(black_box(url));
            }
        })
    });
}

fn benchmark_html_parsing(c: &mut Criterion) {
    let complex_html = format!(r#"<html><body>{}</body></html>"#, 
                               "<a href='/link'>Link</a>".repeat(100));
    let html_samples = vec![
        r#"<html><head><title>Simple</title></head><body>Content</body></html>"#,
        r#"<html>
            <head>
                <title>Complex Page</title>
                <meta name="description" content="Description">
            </head>
            <body>
                <a href="/link1">Link 1</a>
                <a href="/link2">Link 2</a>
                <a href="/link3">Link 3</a>
            </body>
        </html>"#,
        &complex_html,
    ];

    let mut group = c.benchmark_group("html_parsing");
    for (i, html) in html_samples.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(i),
            html,
            |b, html| {
                b.iter(|| {
                    crawler::CrawledContent::extract_title(black_box(html));
                    crawler::CrawledContent::extract_description(black_box(html));
                })
            },
        );
    }
    group.finish();
}

fn benchmark_compression(c: &mut Criterion) {
    let medium_text = "Medium text content. ".repeat(50);
    let large_text = "Large text content with lots of repetition. ".repeat(500);
    let texts = vec![
        ("small", "Small text content"),
        ("medium", &medium_text),
        ("large", &large_text),
    ];

    let mut group = c.benchmark_group("compression");
    for (name, text) in texts {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            text,
            |b, text| {
                b.iter(|| {
                    let compressed = crawler::CrawledContent::compress_content(black_box(text));
                    crawler::CrawledContent::decompress_content(black_box(&compressed))
                        .unwrap();
                })
            },
        );
    }
    group.finish();
}

fn benchmark_circuit_breaker(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("circuit_breaker_operations", |b| {
        b.to_async(&rt).iter(|| async {
            let config = crawler::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                recovery_timeout_seconds: 30,
            };
            
            let breaker = crawler::circuit_breaker::CircuitBreaker::with_config(config);
            
            // Simulate operations
            for i in 0..10 {
                let domain = format!("domain{}.com", i);
                breaker.is_allowed(black_box(&domain)).await;
                if i % 3 == 0 {
                    breaker.record_failure(black_box(&domain)).await;
                } else {
                    breaker.record_success(black_box(&domain)).await;
                }
            }
        })
    });
}

fn benchmark_rate_limiter(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("rate_limiter_operations", |b| {
        b.to_async(&rt).iter(|| async {
            let config = crawler::rate_limiter::RateLimitConfig {
                requests_per_second: 1000,
                burst_size: 10,
                adaptive_threshold: 0.8,
            };
            
            let limiter = crawler::rate_limiter::AdaptiveRateLimiter::with_config(config);
            
            for i in 0..10 {
                let domain = format!("domain{}.com", i % 3);
                limiter.can_crawl_domain(black_box(&domain)).await;
                limiter.record_request(
                    black_box(&domain),
                    Duration::from_millis(10)
                ).await;
            }
        })
    });
}

fn benchmark_user_agent_rotation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("user_agent_rotation");
    
    let strategies = vec![
        ("random", crawler::user_agents::RotationStrategy::Random),
        ("round_robin", crawler::user_agents::RotationStrategy::RoundRobin),
        ("per_domain", crawler::user_agents::RotationStrategy::PerDomain),
    ];

    for (name, strategy) in strategies {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &strategy,
            |b, strategy| {
                b.to_async(&rt).iter(|| async {
                    let rotator = crawler::user_agents::UserAgentRotator::new(strategy.clone());
                    
                    for i in 0..20 {
                        let url = format!("https://example{}.com/page", i % 5);
                        rotator.get_user_agent(black_box(&url));
                    }
                })
            },
        );
    }
    group.finish();
}

fn benchmark_framework_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let html_samples = vec![
        ("react", r#"<div id="root"></div><script>React.createElement</script>"#),
        ("angular", r#"<div ng-app="myApp"></div>"#),
        ("vue", r#"<div id="app" v-app></div>"#),
        ("plain", r#"<html><body>Plain HTML</body></html>"#),
    ];

    c.bench_function("framework_detection", |b| {
        b.to_async(&rt).iter(|| async {
            let renderer = crawler::js_renderer::JsRenderer::new(
                crawler::js_renderer::JsRendererConfig::default()
            );
            
            for (_, html) in &html_samples {
                renderer.detect_frameworks(black_box(html)).await;
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_url_normalization,
    benchmark_content_hashing,
    benchmark_bloom_filter,
    benchmark_pattern_detection,
    benchmark_html_parsing,
    benchmark_compression,
    benchmark_circuit_breaker,
    benchmark_rate_limiter,
    benchmark_user_agent_rotation,
    benchmark_framework_detection,
);

criterion_main!(benches);