//! # Crawler Metrics Module
//!
//! This module provides comprehensive metrics and monitoring for the web crawler service.
//! It tracks performance, success rates, and provides insights into crawler behavior.
//!
//! ## Features
//! - Real-time performance metrics
//! - Success/failure rate tracking
//! - Domain-specific statistics
//! - Resource usage monitoring
//! - Progress reporting

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;
use log::{debug, info};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;

/// Comprehensive metrics for the crawler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerMetrics {
    /// Total URLs crawled
    pub total_urls_crawled: u64,
    /// Total URLs discovered
    pub total_urls_discovered: u64,
    /// Total bytes downloaded
    pub total_bytes_downloaded: u64,
    /// Total crawl time in seconds
    pub total_crawl_time_secs: f64,
    /// Number of successful crawls
    pub successful_crawls: u64,
    /// Number of failed crawls
    pub failed_crawls: u64,
    /// Number of robots.txt blocked URLs
    pub robots_blocked: u64,
    /// Number of circuit breaker blocked URLs
    pub circuit_breaker_blocked: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Current crawl rate (URLs per second)
    pub current_crawl_rate: f64,
    /// Start time of the crawler
    pub start_time: SystemTime,
    /// Last update time
    pub last_update: SystemTime,
    /// Domain-specific metrics
    pub domain_metrics: HashMap<String, DomainMetrics>,
    /// HTTP status code distribution
    pub status_code_distribution: HashMap<u16, u64>,
    /// Content type distribution
    pub content_type_distribution: HashMap<String, u64>,
}

/// Metrics for a specific domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMetrics {
    /// URLs crawled from this domain
    pub urls_crawled: u64,
    /// URLs discovered from this domain
    pub urls_discovered: u64,
    /// Bytes downloaded from this domain
    pub bytes_downloaded: u64,
    /// Successful crawls
    pub successes: u64,
    /// Failed crawls
    pub failures: u64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Last crawl time
    pub last_crawl: SystemTime,
    /// Robots.txt compliance
    pub robots_compliant: bool,
    /// Sitemap found
    pub sitemap_found: bool,
}

/// Progress information for current crawl job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlProgress {
    /// Job ID
    pub job_id: String,
    /// Total URLs to crawl
    pub total_urls: u64,
    /// URLs completed
    pub completed_urls: u64,
    /// Current depth
    pub current_depth: u32,
    /// Maximum depth
    pub max_depth: u32,
    /// Estimated time remaining (seconds)
    pub estimated_time_remaining: Option<f64>,
    /// Progress percentage
    pub progress_percentage: f64,
    /// Current status
    pub status: String,
}

/// Real-time performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Moving average of response times
    response_times: Vec<Duration>,
    /// Window size for moving average
    window_size: usize,
    /// Last rate calculation time
    last_rate_calc: Instant,
    /// URLs crawled since last rate calculation
    urls_since_last_calc: u64,
}

impl Default for CrawlerMetrics {
    fn default() -> Self {
        Self {
            total_urls_crawled: 0,
            total_urls_discovered: 0,
            total_bytes_downloaded: 0,
            total_crawl_time_secs: 0.0,
            successful_crawls: 0,
            failed_crawls: 0,
            robots_blocked: 0,
            circuit_breaker_blocked: 0,
            avg_response_time_ms: 0.0,
            current_crawl_rate: 0.0,
            start_time: SystemTime::now(),
            last_update: SystemTime::now(),
            domain_metrics: HashMap::new(),
            status_code_distribution: HashMap::new(),
            content_type_distribution: HashMap::new(),
        }
    }
}

impl Default for DomainMetrics {
    fn default() -> Self {
        Self {
            urls_crawled: 0,
            urls_discovered: 0,
            bytes_downloaded: 0,
            successes: 0,
            failures: 0,
            avg_response_time_ms: 0.0,
            last_crawl: SystemTime::now(),
            robots_compliant: true,
            sitemap_found: false,
        }
    }
}

impl PerformanceMetrics {
    fn new(window_size: usize) -> Self {
        Self {
            response_times: Vec::with_capacity(window_size),
            window_size,
            last_rate_calc: Instant::now(),
            urls_since_last_calc: 0,
        }
    }

    fn add_response_time(&mut self, duration: Duration) {
        if self.response_times.len() >= self.window_size {
            self.response_times.remove(0);
        }
        self.response_times.push(duration);
    }

    fn get_avg_response_time(&self) -> Duration {
        if self.response_times.is_empty() {
            return Duration::from_secs(0);
        }
        let sum: Duration = self.response_times.iter().sum();
        sum / self.response_times.len() as u32
    }

    fn calculate_rate(&mut self) -> f64 {
        let elapsed = self.last_rate_calc.elapsed();
        if elapsed.as_secs() > 0 {
            let rate = self.urls_since_last_calc as f64 / elapsed.as_secs_f64();
            self.last_rate_calc = Instant::now();
            self.urls_since_last_calc = 0;
            rate
        } else {
            0.0
        }
    }
}

/// Metrics collector for the crawler
pub struct MetricsCollector {
    metrics: Arc<RwLock<CrawlerMetrics>>,
    performance: Arc<RwLock<PerformanceMetrics>>,
    progress: Arc<RwLock<HashMap<String, CrawlProgress>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(CrawlerMetrics::default())),
            performance: Arc::new(RwLock::new(PerformanceMetrics::new(100))),
            progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a successful crawl
    pub async fn record_success(
        &self,
        domain: &str,
        url: &str,
        bytes: u64,
        response_time: Duration,
        status_code: u16,
        content_type: Option<String>,
    ) {
        let mut metrics = self.metrics.write().await;
        let mut performance = self.performance.write().await;

        // Update global metrics
        metrics.total_urls_crawled += 1;
        metrics.successful_crawls += 1;
        metrics.total_bytes_downloaded += bytes;
        metrics.last_update = SystemTime::now();
        
        // Update status code distribution
        *metrics.status_code_distribution.entry(status_code).or_insert(0) += 1;
        
        // Update content type distribution
        if let Some(ct) = content_type {
            *metrics.content_type_distribution.entry(ct).or_insert(0) += 1;
        }

        // Update domain metrics
        let domain_name = domain.to_string();
        {
            let domain_metric = metrics.domain_metrics.entry(domain_name.clone()).or_default();
            domain_metric.urls_crawled += 1;
            domain_metric.successes += 1;
            domain_metric.bytes_downloaded += bytes;
            domain_metric.last_crawl = SystemTime::now();
        }
        
        // Update performance metrics
        performance.add_response_time(response_time);
        performance.urls_since_last_calc += 1;
        
        // Update average response times
        let avg_response = performance.get_avg_response_time();
        metrics.avg_response_time_ms = avg_response.as_millis() as f64;
        
        // Update domain average response time separately
        let domain_metric = metrics.domain_metrics.get_mut(&domain_name).unwrap();
        domain_metric.avg_response_time_ms = 
            (domain_metric.avg_response_time_ms * (domain_metric.urls_crawled - 1) as f64 
             + response_time.as_millis() as f64) / domain_metric.urls_crawled as f64;
        
        // Update crawl rate
        if performance.last_rate_calc.elapsed() > Duration::from_secs(5) {
            metrics.current_crawl_rate = performance.calculate_rate();
        }
        
        debug!("Recorded successful crawl of {} ({}ms, {} bytes)", url, response_time.as_millis(), bytes);
    }

    /// Record a failed crawl
    pub async fn record_failure(&self, domain: &str, url: &str, error: &str) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_urls_crawled += 1;
        metrics.failed_crawls += 1;
        metrics.last_update = SystemTime::now();
        
        let domain_metric = metrics.domain_metrics.entry(domain.to_string()).or_default();
        domain_metric.urls_crawled += 1;
        domain_metric.failures += 1;
        domain_metric.last_crawl = SystemTime::now();
        
        debug!("Recorded failed crawl of {}: {}", url, error);
    }

    /// Record a robots.txt block
    pub async fn record_robots_block(&self, domain: &str, url: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.robots_blocked += 1;
        
        let domain_metric = metrics.domain_metrics.entry(domain.to_string()).or_default();
        domain_metric.robots_compliant = true;
        
        debug!("Recorded robots.txt block for {}", url);
    }

    /// Record a circuit breaker block
    pub async fn record_circuit_breaker_block(&self, domain: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.circuit_breaker_blocked += 1;
        
        debug!("Recorded circuit breaker block for {}", domain);
    }

    /// Record discovered URLs
    pub async fn record_urls_discovered(&self, domain: &str, count: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_urls_discovered += count;
        
        let domain_metric = metrics.domain_metrics.entry(domain.to_string()).or_default();
        domain_metric.urls_discovered += count;
    }

    /// Record sitemap discovery
    pub async fn record_sitemap_found(&self, domain: &str) {
        let mut metrics = self.metrics.write().await;
        let domain_metric = metrics.domain_metrics.entry(domain.to_string()).or_default();
        domain_metric.sitemap_found = true;
        
        info!("Sitemap found for domain: {}", domain);
    }

    /// Update crawl progress
    pub async fn update_progress(
        &self,
        job_id: String,
        total_urls: u64,
        completed_urls: u64,
        current_depth: u32,
        max_depth: u32,
        status: String,
    ) {
        let mut progress_map = self.progress.write().await;
        
        let progress_percentage = if total_urls > 0 {
            (completed_urls as f64 / total_urls as f64) * 100.0
        } else {
            0.0
        };
        
        // Estimate time remaining based on current rate
        let estimated_time = if completed_urls > 0 {
            let metrics = self.metrics.read().await;
            if metrics.current_crawl_rate > 0.0 {
                let remaining = total_urls - completed_urls;
                Some(remaining as f64 / metrics.current_crawl_rate)
            } else {
                None
            }
        } else {
            None
        };
        
        let progress = CrawlProgress {
            job_id: job_id.clone(),
            total_urls,
            completed_urls,
            current_depth,
            max_depth,
            estimated_time_remaining: estimated_time,
            progress_percentage,
            status,
        };
        
        progress_map.insert(job_id, progress);
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> CrawlerMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get progress for a specific job
    pub async fn get_progress(&self, job_id: &str) -> Option<CrawlProgress> {
        let progress_map = self.progress.read().await;
        progress_map.get(job_id).cloned()
    }

    /// Get all job progress
    pub async fn get_all_progress(&self) -> HashMap<String, CrawlProgress> {
        let progress_map = self.progress.read().await;
        progress_map.clone()
    }

    /// Calculate success rate
    pub async fn get_success_rate(&self) -> f64 {
        let metrics = self.metrics.read().await;
        let total = metrics.successful_crawls + metrics.failed_crawls;
        if total == 0 {
            0.0
        } else {
            metrics.successful_crawls as f64 / total as f64
        }
    }

    /// Get top domains by crawled URLs
    pub async fn get_top_domains(&self, limit: usize) -> Vec<(String, u64)> {
        let metrics = self.metrics.read().await;
        let mut domains: Vec<_> = metrics.domain_metrics
            .iter()
            .map(|(domain, m)| (domain.clone(), m.urls_crawled))
            .collect();
        
        domains.sort_by(|a, b| b.1.cmp(&a.1));
        domains.truncate(limit);
        domains
    }

    /// Generate a summary report
    pub async fn generate_report(&self) -> String {
        let metrics = self.metrics.read().await;
        let success_rate = self.get_success_rate().await;
        let elapsed = SystemTime::now()
            .duration_since(metrics.start_time)
            .unwrap_or_default();
        
        format!(
            "=== Crawler Metrics Report ===\n\
             Runtime: {:.2} hours\n\
             URLs Crawled: {}\n\
             URLs Discovered: {}\n\
             Data Downloaded: {:.2} MB\n\
             Success Rate: {:.2}%\n\
             Average Response Time: {:.2} ms\n\
             Current Crawl Rate: {:.2} URLs/sec\n\
             Robots Blocked: {}\n\
             Circuit Breaker Blocked: {}\n\
             Active Domains: {}\n",
            elapsed.as_secs_f64() / 3600.0,
            metrics.total_urls_crawled,
            metrics.total_urls_discovered,
            metrics.total_bytes_downloaded as f64 / 1_048_576.0,
            success_rate * 100.0,
            metrics.avg_response_time_ms,
            metrics.current_crawl_rate,
            metrics.robots_blocked,
            metrics.circuit_breaker_blocked,
            metrics.domain_metrics.len()
        )
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        let mut metrics = self.metrics.write().await;
        let mut performance = self.performance.write().await;
        let mut progress = self.progress.write().await;
        
        *metrics = CrawlerMetrics::default();
        *performance = PerformanceMetrics::new(100);
        progress.clear();
        
        info!("Crawler metrics reset");
    }
}

/// Global metrics collector instance
static GLOBAL_METRICS: Lazy<MetricsCollector> = Lazy::new(|| {
    MetricsCollector::new()
});

/// Record a successful crawl using the global metrics collector
pub async fn record_crawl_success(
    domain: &str,
    url: &str,
    bytes: u64,
    response_time: Duration,
    status_code: u16,
    content_type: Option<String>,
) {
    GLOBAL_METRICS.record_success(domain, url, bytes, response_time, status_code, content_type).await;
}

/// Record a failed crawl using the global metrics collector
pub async fn record_crawl_failure(domain: &str, url: &str, error: &str) {
    GLOBAL_METRICS.record_failure(domain, url, error).await;
}

/// Get current metrics from the global collector
pub async fn get_crawler_metrics() -> CrawlerMetrics {
    GLOBAL_METRICS.get_metrics().await
}

/// Generate a metrics report from the global collector
pub async fn generate_metrics_report() -> String {
    GLOBAL_METRICS.generate_report().await
}

/// Standalone function to record robots.txt blocks
pub async fn record_robots_block(domain: &str, url: &str) {
    GLOBAL_METRICS.record_robots_block(domain, url).await;
}

/// Standalone function to record circuit breaker blocks
pub async fn record_circuit_breaker_block(domain: &str) {
    GLOBAL_METRICS.record_circuit_breaker_block(domain).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collection() {
        let collector = MetricsCollector::new();
        
        // Record some successful crawls
        collector.record_success(
            "example.com",
            "http://example.com/page1",
            1024,
            Duration::from_millis(100),
            200,
            Some("text/html".to_string()),
        ).await;
        
        collector.record_success(
            "example.com",
            "http://example.com/page2",
            2048,
            Duration::from_millis(150),
            200,
            Some("text/html".to_string()),
        ).await;
        
        // Record a failure
        collector.record_failure(
            "example.com",
            "http://example.com/page3",
            "Connection timeout",
        ).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.total_urls_crawled, 3);
        assert_eq!(metrics.successful_crawls, 2);
        assert_eq!(metrics.failed_crawls, 1);
        assert_eq!(metrics.total_bytes_downloaded, 3072);
        
        let success_rate = collector.get_success_rate().await;
        assert!((success_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_progress_tracking() {
        let collector = MetricsCollector::new();
        
        collector.update_progress(
            "job1".to_string(),
            100,
            25,
            2,
            5,
            "running".to_string(),
        ).await;
        
        let progress = collector.get_progress("job1").await.unwrap();
        assert_eq!(progress.completed_urls, 25);
        assert_eq!(progress.progress_percentage, 25.0);
        assert_eq!(progress.current_depth, 2);
    }
}