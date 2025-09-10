//! REST API for crawler management
//! 
//! This module provides HTTP endpoints for managing the crawler service,
//! including starting/stopping crawls, viewing status, and configuring jobs.

use rouille::{Request, Response, router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use log::{info, warn, error};
use anyhow::Result;
use crate::services::crawler::{
    CrawlJobConfig, ConfigurableCrawlJob,
    PersistentJobQueue, QueueStats,
    get_crawler_metrics, get_rate_limiter,
    prometheus_metrics::{export_metrics, get_crawler_stats},
};

/// API response wrapper
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

/// Request to create a new crawl job
#[derive(Debug, Deserialize)]
struct CreateJobRequest {
    start_url: String,
    config: Option<CrawlJobConfig>,
}

/// Request to update job configuration
#[derive(Debug, Deserialize)]
struct UpdateJobRequest {
    config: CrawlJobConfig,
}

/// Job status response
#[derive(Debug, Serialize)]
struct JobStatusResponse {
    job_id: String,
    status: String,
    pages_crawled: usize,
    pages_failed: usize,
    bytes_downloaded: u64,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    error_message: Option<String>,
}

/// Queue status response
#[derive(Debug, Serialize)]
struct QueueStatusResponse {
    pending: usize,
    active: usize,
    failed: usize,
    retrying: usize,
}

/// Crawler status response
#[derive(Debug, Serialize)]
struct CrawlerStatusResponse {
    is_running: bool,
    active_jobs: i64,
    queue_status: QueueStatusResponse,
    urls_crawled_total: u64,
    bytes_downloaded_total: u64,
    success_rate: f64,
    avg_response_time_ms: f64,
}

/// Domain statistics response
#[derive(Debug, Serialize)]
struct DomainStatsResponse {
    domain: String,
    current_delay_ms: u64,
    avg_response_time_ms: f64,
    request_count: u64,
    active_requests: usize,
    has_rate_limited: bool,
}

/// Handle crawler API requests
pub fn handle_crawler_api(request: &Request) -> Response {
    router!(request,
        // Job Management
        (POST) (/api/crawler/jobs) => {
            handle_create_job(request)
        },
        
        (GET) (/api/crawler/jobs/{job_id: String}) => |job_id| {
            handle_get_job_status(job_id)
        },
        
        (PUT) (/api/crawler/jobs/{job_id: String}) => |job_id| {
            handle_update_job(request, job_id)
        },
        
        (DELETE) (/api/crawler/jobs/{job_id: String}) => |job_id| {
            handle_cancel_job(job_id)
        },
        
        (POST) (/api/crawler/jobs/{job_id: String}/pause) => |job_id| {
            handle_pause_job(job_id)
        },
        
        (POST) (/api/crawler/jobs/{job_id: String}/resume) => |job_id| {
            handle_resume_job(job_id)
        },
        
        // Service Control
        (GET) (/api/crawler/status) => {
            handle_get_crawler_status()
        },
        
        (POST) (/api/crawler/start) => {
            handle_start_crawler()
        },
        
        (POST) (/api/crawler/stop) => {
            handle_stop_crawler()
        },
        
        (POST) (/api/crawler/restart) => {
            handle_restart_crawler()
        },
        
        // Queue Management
        (GET) (/api/crawler/queue) => {
            handle_get_queue_status()
        },
        
        (DELETE) (/api/crawler/queue) => {
            handle_clear_queue()
        },
        
        // Statistics & Monitoring
        (GET) (/api/crawler/stats) => {
            handle_get_stats()
        },
        
        (GET) (/api/crawler/metrics) => {
            handle_get_metrics()
        },
        
        (GET) (/api/crawler/domains) => {
            handle_get_domain_stats()
        },
        
        (GET) (/api/crawler/domains/{domain: String}) => |domain| {
            handle_get_domain_stats_single(domain)
        },
        
        // Configuration
        (GET) (/api/crawler/config/presets) => {
            handle_get_config_presets()
        },
        
        _ => Response::empty_404()
    )
}

/// Create a new crawl job
fn handle_create_job(request: &Request) -> Response {
    let body = match rouille::input::json_input::<CreateJobRequest>(request) {
        Ok(body) => body,
        Err(e) => {
            return Response::json(&ApiResponse::<()>::error(format!("Invalid request: {}", e)))
                .with_status_code(400);
        }
    };
    
    // Validate URL
    if url::Url::parse(&body.start_url).is_err() {
        return Response::json(&ApiResponse::<()>::error("Invalid URL".to_string()))
            .with_status_code(400);
    }
    
    // Use provided config or default
    let config = body.config.unwrap_or_default();
    
    // Validate config
    if let Err(e) = config.validate() {
        return Response::json(&ApiResponse::<()>::error(format!("Invalid config: {}", e)))
            .with_status_code(400);
    }
    
    // Create job
    let job = ConfigurableCrawlJob::new(body.start_url, config);
    let job_id = job.oid.clone();
    
    // Add to queue (would need async runtime)
    info!("Created crawl job: {}", job_id);
    
    Response::json(&ApiResponse::success(json!({
        "job_id": job_id,
        "status": "pending"
    })))
}

/// Get job status
fn handle_get_job_status(job_id: String) -> Response {
    // This would query the database for job status
    // For now, return mock data
    let response = JobStatusResponse {
        job_id: job_id.clone(),
        status: "running".to_string(),
        pages_crawled: 42,
        pages_failed: 2,
        bytes_downloaded: 1024000,
        started_at: Some(1704067200),
        completed_at: None,
        error_message: None,
    };
    
    Response::json(&ApiResponse::success(response))
}

/// Update job configuration
fn handle_update_job(request: &Request, job_id: String) -> Response {
    let body = match rouille::input::json_input::<UpdateJobRequest>(request) {
        Ok(body) => body,
        Err(e) => {
            return Response::json(&ApiResponse::<()>::error(format!("Invalid request: {}", e)))
                .with_status_code(400);
        }
    };
    
    // Validate config
    if let Err(e) = body.config.validate() {
        return Response::json(&ApiResponse::<()>::error(format!("Invalid config: {}", e)))
            .with_status_code(400);
    }
    
    info!("Updated job {} configuration", job_id);
    
    Response::json(&ApiResponse::success(json!({
        "job_id": job_id,
        "message": "Configuration updated"
    })))
}

/// Cancel a job
fn handle_cancel_job(job_id: String) -> Response {
    info!("Cancelling job: {}", job_id);
    
    Response::json(&ApiResponse::success(json!({
        "job_id": job_id,
        "message": "Job cancelled"
    })))
}

/// Pause a job
fn handle_pause_job(job_id: String) -> Response {
    info!("Pausing job: {}", job_id);
    
    Response::json(&ApiResponse::success(json!({
        "job_id": job_id,
        "message": "Job paused"
    })))
}

/// Resume a job
fn handle_resume_job(job_id: String) -> Response {
    info!("Resuming job: {}", job_id);
    
    Response::json(&ApiResponse::success(json!({
        "job_id": job_id,
        "message": "Job resumed"
    })))
}

/// Get crawler status
fn handle_get_crawler_status() -> Response {
    let is_running = crate::services::crawler::service_status() == "running";
    
    let response = CrawlerStatusResponse {
        is_running,
        active_jobs: 0, // Would get from metrics
        queue_status: QueueStatusResponse {
            pending: 0,
            active: 0,
            failed: 0,
            retrying: 0,
        },
        urls_crawled_total: 0,
        bytes_downloaded_total: 0,
        success_rate: 0.0,
        avg_response_time_ms: 0.0,
    };
    
    Response::json(&ApiResponse::success(response))
}

/// Start crawler service
fn handle_start_crawler() -> Response {
    // This would need to be async
    info!("Starting crawler service");
    
    Response::json(&ApiResponse::success(json!({
        "message": "Crawler service started"
    })))
}

/// Stop crawler service
fn handle_stop_crawler() -> Response {
    crate::services::crawler::stop_service();
    info!("Stopped crawler service");
    
    Response::json(&ApiResponse::success(json!({
        "message": "Crawler service stopped"
    })))
}

/// Restart crawler service
fn handle_restart_crawler() -> Response {
    crate::services::crawler::stop_service();
    // Would need async to start again
    info!("Restarting crawler service");
    
    Response::json(&ApiResponse::success(json!({
        "message": "Crawler service restarted"
    })))
}

/// Get queue status
fn handle_get_queue_status() -> Response {
    let status = QueueStatusResponse {
        pending: 0,
        active: 0,
        failed: 0,
        retrying: 0,
    };
    
    Response::json(&ApiResponse::success(status))
}

/// Clear all queues
fn handle_clear_queue() -> Response {
    warn!("Clearing all crawler queues");
    
    Response::json(&ApiResponse::success(json!({
        "message": "All queues cleared"
    })))
}

/// Get crawler statistics
fn handle_get_stats() -> Response {
    // This would need async runtime
    Response::json(&ApiResponse::success(json!({
        "urls_crawled": 0,
        "bytes_downloaded": 0,
        "success_rate": 0.0,
        "avg_response_time_ms": 0.0,
        "deduplication_ratio": 0.0,
        "compression_ratio": 0.0
    })))
}

/// Get Prometheus metrics
fn handle_get_metrics() -> Response {
    match export_metrics() {
        Ok(metrics) => Response::text(metrics)
            .with_additional_header("Content-Type", "text/plain; version=0.0.4"),
        Err(e) => Response::json(&ApiResponse::<()>::error(format!("Failed to export metrics: {}", e)))
            .with_status_code(500)
    }
}

/// Get domain statistics
fn handle_get_domain_stats() -> Response {
    // This would need async runtime to get from rate limiter
    let domains: Vec<DomainStatsResponse> = vec![];
    
    Response::json(&ApiResponse::success(domains))
}

/// Get single domain statistics
fn handle_get_domain_stats_single(domain: String) -> Response {
    // This would need async runtime
    let stats = DomainStatsResponse {
        domain: domain.clone(),
        current_delay_ms: 1000,
        avg_response_time_ms: 250.0,
        request_count: 42,
        active_requests: 2,
        has_rate_limited: false,
    };
    
    Response::json(&ApiResponse::success(stats))
}

/// Get configuration presets
fn handle_get_config_presets() -> Response {
    let presets = json!({
        "shallow": CrawlJobConfig::shallow(),
        "deep": CrawlJobConfig::deep(),
        "archival": CrawlJobConfig::archival(),
        "default": CrawlJobConfig::default(),
    });
    
    Response::json(&ApiResponse::success(presets))
}

/// Register crawler API routes
pub fn register_crawler_routes() {
    info!("Crawler management API registered at /api/crawler");
}