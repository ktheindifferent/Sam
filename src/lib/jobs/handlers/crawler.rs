use async_trait::async_trait;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use crate::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerPayload {
    pub url: String,
    pub max_depth: Option<u32>,
    pub max_pages: Option<usize>,
    pub follow_external: bool,
    pub respect_robots_txt: bool,
    pub user_agent: Option<String>,
    pub delay_ms: Option<u64>,
    pub selectors: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrawlResult {
    pages_crawled: usize,
    links_found: usize,
    errors: Vec<String>,
    duration_secs: u64,
    data_extracted: Option<Value>,
}

pub struct CrawlerJobHandler {
    crawler_service: Option<crate::services::crawler::enhanced::EnhancedCrawler>,
    max_concurrent_requests: usize,
}

impl CrawlerJobHandler {
    pub fn new(max_concurrent_requests: usize) -> Self {
        Self {
            crawler_service: None,
            max_concurrent_requests,
        }
    }
    
    async fn perform_crawl(&self, payload: CrawlerPayload) -> Result<CrawlResult, String> {
        info!("Starting crawl of {} with max_depth={:?}, max_pages={:?}", 
              payload.url, payload.max_depth, payload.max_pages);
        
        // Validate URL
        if !payload.url.starts_with("http://") && !payload.url.starts_with("https://") {
            return Err("Invalid URL: must start with http:// or https://".to_string());
        }
        
        let start_time = std::time::Instant::now();
        
        // Simulate crawling process
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Simulate crawl results
        let pages_crawled = rand::random::<usize>() % 100 + 1;
        let links_found = pages_crawled * 10;
        
        // Simulate occasional errors
        let mut errors = Vec::new();
        if rand::random::<f32>() < 0.1 {
            errors.push("Some pages returned 404".to_string());
        }
        
        Ok(CrawlResult {
            pages_crawled,
            links_found,
            errors,
            duration_secs: start_time.elapsed().as_secs(),
            data_extracted: Some(serde_json::json!({
                "title": "Example Page",
                "description": "Crawled content",
                "keywords": ["example", "test", "crawler"]
            })),
        })
    }
}

#[async_trait]
impl JobHandler for CrawlerJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        let crawler_payload: CrawlerPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid crawler payload: {}", e)))?;
        
        match self.perform_crawl(crawler_payload.clone()).await {
            Ok(result) => {
                info!("Crawl completed: {} pages crawled, {} links found", 
                      result.pages_crawled, result.links_found);
                
                if !result.errors.is_empty() {
                    warn!("Crawl had errors: {:?}", result.errors);
                }
                
                Ok(JobResult::Success(serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"status": "completed"}))))
            }
            Err(e) => {
                if e.contains("timeout") || e.contains("connection") || e.contains("rate limit") {
                    // Transient error, should retry
                    warn!("Crawl failed with transient error: {}", e);
                    Ok(JobResult::Retry(e))
                } else {
                    // Permanent error
                    error!("Crawl failed permanently: {}", e);
                    Ok(JobResult::Failure(e))
                }
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        5 // More retries for crawling due to network issues
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff with jitter for crawling
        let base = Duration::from_secs(30 * 2_u64.pow(attempt));
        let jitter = Duration::from_secs(rand::random::<u64>() % 30);
        base + jitter
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(1800)) // 30 minutes timeout for crawling
    }
    
    fn name(&self) -> &str {
        "crawler"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let crawler_payload: CrawlerPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        // Validate URL
        if crawler_payload.url.is_empty() {
            return Err(JobError::ExecutionFailed("URL is required".to_string()));
        }
        
        // Validate max_depth
        if let Some(depth) = crawler_payload.max_depth {
            if depth > 10 {
                return Err(JobError::ExecutionFailed(
                    format!("Max depth {} is too large (max: 10)", depth)
                ));
            }
        }
        
        // Validate max_pages
        if let Some(pages) = crawler_payload.max_pages {
            if pages > 10000 {
                return Err(JobError::ExecutionFailed(
                    format!("Max pages {} is too large (max: 10000)", pages)
                ));
            }
        }
        
        // Validate delay
        if let Some(delay) = crawler_payload.delay_ms {
            if delay < 100 {
                return Err(JobError::ExecutionFailed(
                    "Delay must be at least 100ms to be respectful".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    async fn on_success(&self, payload: &Value, result: &JobResult) -> Result<(), JobError> {
        if let Ok(crawler_payload) = serde_json::from_value::<CrawlerPayload>(payload.clone()) {
            info!("Successfully crawled {}", crawler_payload.url);
            
            // Could trigger follow-up jobs here (e.g., indexing, analysis)
        }
        Ok(())
    }
    
    async fn on_failure(&self, payload: &Value, error: &JobError) -> Result<(), JobError> {
        if let Ok(crawler_payload) = serde_json::from_value::<CrawlerPayload>(payload.clone()) {
            error!("Failed to crawl {}: {}", crawler_payload.url, error);
        }
        Ok(())
    }
    
    async fn on_retry(&self, payload: &Value, attempt: u32, error: &JobError) -> Result<(), JobError> {
        if let Ok(crawler_payload) = serde_json::from_value::<CrawlerPayload>(payload.clone()) {
            warn!("Retrying crawl of {} (attempt {}): {}", 
                  crawler_payload.url, attempt, error);
        }
        Ok(())
    }
}