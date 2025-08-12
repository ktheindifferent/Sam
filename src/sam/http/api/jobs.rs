use rouille::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json;
use log::{error, info};
use std::sync::Arc;
use crate::sam::jobs::{Job, JobSystem, JobStats, Priority, JobType};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub job_type: String,
    pub payload: serde_json::Value,
    pub priority: Option<String>,
    pub max_retries: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub scheduled_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub job: Option<Job>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JobListResponse {
    pub jobs: Vec<Job>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

pub async fn handle_jobs_api(request: &Request, job_system: Arc<JobSystem>) -> Response {
    let path = request.url();
    
    match request.method() {
        "GET" => {
            if path == "/api/jobs/stats" {
                handle_get_stats(job_system).await
            } else if path.starts_with("/api/jobs/") {
                let job_id = path.trim_start_matches("/api/jobs/");
                handle_get_job(job_id, job_system).await
            } else {
                handle_list_jobs(request, job_system).await
            }
        }
        "POST" => {
            if path == "/api/jobs" {
                handle_create_job(request, job_system).await
            } else if path.starts_with("/api/jobs/") && path.ends_with("/retry") {
                let job_id = path.trim_start_matches("/api/jobs/")
                    .trim_end_matches("/retry");
                handle_retry_job(job_id, job_system).await
            } else if path.starts_with("/api/jobs/") && path.ends_with("/cancel") {
                let job_id = path.trim_start_matches("/api/jobs/")
                    .trim_end_matches("/cancel");
                handle_cancel_job(job_id, job_system).await
            } else {
                Response::empty_404()
            }
        }
        "DELETE" => {
            if path.starts_with("/api/jobs/dead-letter/") {
                let job_id = path.trim_start_matches("/api/jobs/dead-letter/");
                handle_purge_dead_letter(job_id, job_system).await
            } else {
                Response::empty_404()
            }
        }
        _ => Response::empty_404()
    }
}

async fn handle_create_job(request: &Request, job_system: Arc<JobSystem>) -> Response {
    let body = match rouille::input::plain_text_body(request) {
        Ok(body) => body,
        Err(_) => {
            return Response::json(&serde_json::json!({
                "error": "Invalid request body"
            })).with_status_code(400);
        }
    };
    
    let create_request: CreateJobRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            return Response::json(&serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            })).with_status_code(400);
        }
    };
    
    // Create the job
    let mut job = Job::new(create_request.job_type, create_request.payload);
    
    // Set priority if provided
    if let Some(priority_str) = create_request.priority {
        let priority = match priority_str.as_str() {
            "critical" => Priority::Critical,
            "high" => Priority::High,
            "normal" => Priority::Normal,
            "low" => Priority::Low,
            _ => Priority::Normal,
        };
        job = job.with_priority(priority);
    }
    
    // Set max retries if provided
    if let Some(max_retries) = create_request.max_retries {
        job = job.with_max_retries(max_retries);
    }
    
    // Set timeout if provided
    if let Some(timeout_secs) = create_request.timeout_secs {
        job = job.with_timeout(timeout_secs);
    }
    
    // Set schedule if provided
    if let Some(scheduled_at_str) = create_request.scheduled_at {
        if let Ok(scheduled_at) = chrono::DateTime::parse_from_rfc3339(&scheduled_at_str) {
            job = job.with_schedule(scheduled_at.with_timezone(&chrono::Utc));
        }
    }
    
    // Enqueue the job
    match job_system.enqueue(job.clone()).await {
        Ok(job_id) => {
            info!("Created job {}", job_id);
            Response::json(&CreateJobResponse {
                job_id,
                status: "enqueued".to_string(),
            })
        }
        Err(e) => {
            error!("Failed to create job: {}", e);
            Response::json(&serde_json::json!({
                "error": format!("Failed to create job: {}", e)
            })).with_status_code(500)
        }
    }
}

async fn handle_get_job(job_id: &str, job_system: Arc<JobSystem>) -> Response {
    match job_system.queue.get_job(job_id).await {
        Ok(Some(job)) => {
            Response::json(&JobResponse {
                job: Some(job),
                error: None,
            })
        }
        Ok(None) => {
            Response::json(&JobResponse {
                job: None,
                error: Some("Job not found".to_string()),
            }).with_status_code(404)
        }
        Err(e) => {
            error!("Failed to get job {}: {}", job_id, e);
            Response::json(&JobResponse {
                job: None,
                error: Some(format!("Failed to get job: {}", e)),
            }).with_status_code(500)
        }
    }
}

async fn handle_list_jobs(request: &Request, job_system: Arc<JobSystem>) -> Response {
    // Parse query parameters for pagination
    let page = request.get_param("page")
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(1);
    
    let per_page = request.get_param("per_page")
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(50)
        .min(100); // Cap at 100 per page
    
    // For now, return empty list as we'd need to implement list functionality
    // In production, you'd query Redis for job lists
    Response::json(&JobListResponse {
        jobs: vec![],
        total: 0,
        page,
        per_page,
    })
}

async fn handle_get_stats(job_system: Arc<JobSystem>) -> Response {
    match job_system.get_stats().await {
        Ok(stats) => Response::json(&stats),
        Err(e) => {
            error!("Failed to get job stats: {}", e);
            Response::json(&serde_json::json!({
                "error": format!("Failed to get stats: {}", e)
            })).with_status_code(500)
        }
    }
}

async fn handle_retry_job(job_id: &str, job_system: Arc<JobSystem>) -> Response {
    match job_system.dead_letter.retry(job_id, &job_system.queue).await {
        Ok(true) => {
            info!("Retried job {} from dead letter queue", job_id);
            Response::json(&serde_json::json!({
                "status": "retried",
                "job_id": job_id
            }))
        }
        Ok(false) => {
            Response::json(&serde_json::json!({
                "error": "Job not found in dead letter queue"
            })).with_status_code(404)
        }
        Err(e) => {
            error!("Failed to retry job {}: {}", job_id, e);
            Response::json(&serde_json::json!({
                "error": format!("Failed to retry job: {}", e)
            })).with_status_code(500)
        }
    }
}

async fn handle_cancel_job(job_id: &str, job_system: Arc<JobSystem>) -> Response {
    match job_system.queue.cancel_job(job_id).await {
        Ok(true) => {
            info!("Cancelled job {}", job_id);
            Response::json(&serde_json::json!({
                "status": "cancelled",
                "job_id": job_id
            }))
        }
        Ok(false) => {
            Response::json(&serde_json::json!({
                "error": "Job not found or cannot be cancelled"
            })).with_status_code(404)
        }
        Err(e) => {
            error!("Failed to cancel job {}: {}", job_id, e);
            Response::json(&serde_json::json!({
                "error": format!("Failed to cancel job: {}", e)
            })).with_status_code(500)
        }
    }
}

async fn handle_purge_dead_letter(job_id: &str, job_system: Arc<JobSystem>) -> Response {
    match job_system.dead_letter.purge(job_id).await {
        Ok(true) => {
            info!("Purged job {} from dead letter queue", job_id);
            Response::json(&serde_json::json!({
                "status": "purged",
                "job_id": job_id
            }))
        }
        Ok(false) => {
            Response::json(&serde_json::json!({
                "error": "Job not found in dead letter queue"
            })).with_status_code(404)
        }
        Err(e) => {
            error!("Failed to purge job {}: {}", job_id, e);
            Response::json(&serde_json::json!({
                "error": format!("Failed to purge job: {}", e)
            })).with_status_code(500)
        }
    }
}