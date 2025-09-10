//! Prometheus metrics for crawler monitoring
//! 
//! This module provides comprehensive metrics collection for the crawler
//! using Prometheus format for monitoring and observability.

use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramVec, TextEncoder, Encoder,
    register_int_counter, register_int_gauge, IntCounter, IntGauge,
};
use lazy_static::lazy_static;
use std::time::Duration;
use anyhow::Result;
use log::debug;

lazy_static! {
    /// Total number of URLs crawled
    static ref URLS_CRAWLED_TOTAL: CounterVec = register_counter_vec!(
        "crawler_urls_crawled_total",
        "Total number of URLs crawled",
        &["status", "domain"]
    ).expect("Failed to create urls_crawled_total metric");
    
    /// Current number of active crawl jobs
    static ref ACTIVE_JOBS: IntGauge = register_int_gauge!(
        "crawler_active_jobs",
        "Number of currently active crawl jobs"
    ).expect("Failed to create active_jobs metric");
    
    /// Current queue size
    static ref QUEUE_SIZE: GaugeVec = register_gauge_vec!(
        "crawler_queue_size",
        "Current size of various queues",
        &["queue_type"]
    ).expect("Failed to create queue_size metric");
    
    /// Response time histogram
    static ref RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "crawler_response_time_seconds",
        "Response time for crawled URLs",
        &["domain", "status_code"],
        vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).expect("Failed to create response_time metric");
    
    /// Content size histogram
    static ref CONTENT_SIZE: HistogramVec = register_histogram_vec!(
        "crawler_content_size_bytes",
        "Size of crawled content in bytes",
        &["content_type"],
        vec![1024.0, 10240.0, 102400.0, 1048576.0, 10485760.0] // 1KB, 10KB, 100KB, 1MB, 10MB
    ).expect("Failed to create content_size metric");
    
    /// Rate limit hits
    static ref RATE_LIMIT_HITS: CounterVec = register_counter_vec!(
        "crawler_rate_limit_hits_total",
        "Number of rate limit responses received",
        &["domain", "status_code"]
    ).expect("Failed to create rate_limit_hits metric");
    
    /// Robots.txt denials
    static ref ROBOTS_DENIALS: CounterVec = register_counter_vec!(
        "crawler_robots_denials_total",
        "Number of URLs denied by robots.txt",
        &["domain"]
    ).expect("Failed to create robots_denials metric");
    
    /// Database operations
    static ref DB_OPERATIONS: CounterVec = register_counter_vec!(
        "crawler_db_operations_total",
        "Database operations performed",
        &["operation", "status"]
    ).expect("Failed to create db_operations metric");
    
    /// Database operation duration
    static ref DB_DURATION: HistogramVec = register_histogram_vec!(
        "crawler_db_duration_seconds",
        "Duration of database operations",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).expect("Failed to create db_duration metric");
    
    /// Memory usage
    static ref MEMORY_USAGE: GaugeVec = register_gauge_vec!(
        "crawler_memory_usage_bytes",
        "Memory usage by component",
        &["component"]
    ).expect("Failed to create memory_usage metric");
    
    /// Deduplication statistics
    static ref DEDUP_RATIO: GaugeVec = register_gauge_vec!(
        "crawler_deduplication_ratio",
        "Content deduplication ratio",
        &["type"]
    ).expect("Failed to create dedup_ratio metric");
    
    /// Circuit breaker state
    static ref CIRCUIT_BREAKER_STATE: GaugeVec = register_gauge_vec!(
        "crawler_circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open, 2=half-open)",
        &["domain"]
    ).expect("Failed to create circuit_breaker_state metric");
    
    /// Total bytes downloaded
    static ref BYTES_DOWNLOADED: IntCounter = register_int_counter!(
        "crawler_bytes_downloaded_total",
        "Total bytes downloaded"
    ).expect("Failed to create bytes_downloaded metric");
    
    /// Total bytes compressed
    static ref BYTES_COMPRESSED: IntCounter = register_int_counter!(
        "crawler_bytes_compressed_total",
        "Total bytes after compression"
    ).expect("Failed to create bytes_compressed metric");
    
    /// Crawl depth distribution
    static ref CRAWL_DEPTH: HistogramVec = register_histogram_vec!(
        "crawler_depth_distribution",
        "Distribution of crawl depths reached",
        &["job_id"],
        vec![1.0, 2.0, 3.0, 5.0, 10.0, 20.0]
    ).expect("Failed to create crawl_depth metric");
    
    /// Error counts by type
    static ref ERROR_COUNTS: CounterVec = register_counter_vec!(
        "crawler_errors_total",
        "Total errors by type",
        &["error_type", "domain"]
    ).expect("Failed to create error_counts metric");
}

/// Record a crawled URL
pub fn record_url_crawled(domain: &str, status: &str) {
    URLS_CRAWLED_TOTAL
        .with_label_values(&[status, domain])
        .inc();
    debug!("Recorded crawl: domain={}, status={}", domain, status);
}

/// Record response time
pub fn record_response_time(domain: &str, status_code: u16, duration: Duration) {
    RESPONSE_TIME
        .with_label_values(&[domain, &status_code.to_string()])
        .observe(duration.as_secs_f64());
}

/// Record content size
pub fn record_content_size(content_type: &str, size: usize) {
    CONTENT_SIZE
        .with_label_values(&[content_type])
        .observe(size as f64);
    BYTES_DOWNLOADED.inc_by(size as u64);
}

/// Record compressed size
pub fn record_compressed_size(size: usize) {
    BYTES_COMPRESSED.inc_by(size as u64);
}

/// Update active jobs count
pub fn set_active_jobs(count: i64) {
    ACTIVE_JOBS.set(count);
}

/// Update queue size
pub fn set_queue_size(queue_type: &str, size: f64) {
    QUEUE_SIZE
        .with_label_values(&[queue_type])
        .set(size);
}

/// Record rate limit hit
pub fn record_rate_limit(domain: &str, status_code: u16) {
    RATE_LIMIT_HITS
        .with_label_values(&[domain, &status_code.to_string()])
        .inc();
}

/// Record robots.txt denial
pub fn record_robots_denial(domain: &str) {
    ROBOTS_DENIALS
        .with_label_values(&[domain])
        .inc();
}

/// Record database operation
pub fn record_db_operation(operation: &str, success: bool, duration: Duration) {
    let status = if success { "success" } else { "failure" };
    DB_OPERATIONS
        .with_label_values(&[operation, status])
        .inc();
    DB_DURATION
        .with_label_values(&[operation])
        .observe(duration.as_secs_f64());
}

/// Update memory usage
pub fn set_memory_usage(component: &str, bytes: f64) {
    MEMORY_USAGE
        .with_label_values(&[component])
        .set(bytes);
}

/// Update deduplication ratio
pub fn set_dedup_ratio(ratio_type: &str, ratio: f64) {
    DEDUP_RATIO
        .with_label_values(&[ratio_type])
        .set(ratio);
}

/// Update circuit breaker state
pub fn set_circuit_breaker_state(domain: &str, state: CircuitBreakerState) {
    let value = match state {
        CircuitBreakerState::Closed => 0.0,
        CircuitBreakerState::Open => 1.0,
        CircuitBreakerState::HalfOpen => 2.0,
    };
    CIRCUIT_BREAKER_STATE
        .with_label_values(&[domain])
        .set(value);
}

/// Record crawl depth
pub fn record_crawl_depth(job_id: &str, depth: f64) {
    CRAWL_DEPTH
        .with_label_values(&[job_id])
        .observe(depth);
}

/// Record error
pub fn record_error(error_type: &str, domain: &str) {
    ERROR_COUNTS
        .with_label_values(&[error_type, domain])
        .inc();
}

/// Circuit breaker states for metrics
#[derive(Debug, Clone, Copy)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Export metrics in Prometheus format
pub fn export_metrics() -> Result<String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

/// Crawler statistics summary
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawlerStats {
    pub urls_crawled: u64,
    pub active_jobs: i64,
    pub pending_queue_size: usize,
    pub retry_queue_size: usize,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub total_bytes_downloaded: u64,
    pub total_bytes_compressed: u64,
    pub compression_ratio: f64,
    pub deduplication_ratio: f64,
    pub error_count: u64,
}

/// Get current crawler statistics
pub async fn get_crawler_stats() -> Result<CrawlerStats> {
    // This would aggregate metrics from various sources
    // For now, return a placeholder
    Ok(CrawlerStats {
        urls_crawled: 0,
        active_jobs: ACTIVE_JOBS.get(),
        pending_queue_size: 0,
        retry_queue_size: 0,
        success_rate: 0.0,
        avg_response_time_ms: 0.0,
        total_bytes_downloaded: BYTES_DOWNLOADED.get(),
        total_bytes_compressed: BYTES_COMPRESSED.get(),
        compression_ratio: 0.0,
        deduplication_ratio: 0.0,
        error_count: 0,
    })
}

/// Initialize metrics system
pub fn init_metrics() {
    // Force lazy_static initialization
    let _ = &*URLS_CRAWLED_TOTAL;
    let _ = &*ACTIVE_JOBS;
    let _ = &*QUEUE_SIZE;
    let _ = &*RESPONSE_TIME;
    let _ = &*CONTENT_SIZE;
    let _ = &*RATE_LIMIT_HITS;
    let _ = &*ROBOTS_DENIALS;
    let _ = &*DB_OPERATIONS;
    let _ = &*DB_DURATION;
    let _ = &*MEMORY_USAGE;
    let _ = &*DEDUP_RATIO;
    let _ = &*CIRCUIT_BREAKER_STATE;
    let _ = &*BYTES_DOWNLOADED;
    let _ = &*BYTES_COMPRESSED;
    let _ = &*CRAWL_DEPTH;
    let _ = &*ERROR_COUNTS;
    
    debug!("Prometheus metrics initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_recording() {
        init_metrics();
        
        // Record some metrics
        record_url_crawled("example.com", "success");
        record_response_time("example.com", 200, Duration::from_millis(500));
        record_content_size("text/html", 10240);
        set_active_jobs(5);
        
        // Export and check metrics exist
        let metrics = export_metrics().unwrap();
        assert!(metrics.contains("crawler_urls_crawled_total"));
        assert!(metrics.contains("crawler_response_time_seconds"));
        assert!(metrics.contains("crawler_content_size_bytes"));
        assert!(metrics.contains("crawler_active_jobs"));
    }
}