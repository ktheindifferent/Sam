use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub priority: Priority,
    pub max_retries: u32,
    pub retry_count: u32,
    pub retry_delay_secs: u64,
    pub timeout_secs: Option<u64>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: JobStatus,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

impl Job {
    pub fn new(job_type: String, payload: serde_json::Value) -> Self {
        let id = nanoid::nanoid!();
        let now = Utc::now();

        Self {
            id,
            job_type,
            payload,
            priority: Priority::Normal,
            max_retries: 3,
            retry_count: 0,
            retry_delay_secs: 60,
            timeout_secs: Some(300), // 5 minutes default
            scheduled_at: None,
            created_at: now,
            updated_at: now,
            status: JobStatus::Pending,
            error: None,
            result: None,
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_retry_delay(mut self, retry_delay_secs: u64) -> Self {
        self.retry_delay_secs = retry_delay_secs;
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    pub fn with_schedule(mut self, scheduled_at: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(scheduled_at);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn calculate_retry_delay(&self) -> std::time::Duration {
        // Exponential backoff with jitter
        let base_delay = self.retry_delay_secs as f64;
        let exponential_delay = base_delay * 2_f64.powi(self.retry_count as i32);
        let jitter = rand::random::<f64>() * 10.0; // Add 0-10 seconds of jitter
        let total_delay = exponential_delay + jitter;

        // Cap at 1 hour
        let capped_delay = total_delay.min(3600.0);

        std::time::Duration::from_secs_f64(capped_delay)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Normal => write!(f, "normal"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl Priority {
    pub fn queue_name(&self) -> String {
        format!("jobs:queue:{}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Retrying,
    DeadLetter,
    Cancelled,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Scheduled => write!(f, "scheduled"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Retrying => write!(f, "retrying"),
            JobStatus::DeadLetter => write!(f, "dead_letter"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResult {
    Success(serde_json::Value),
    Failure(String),
    Retry(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    Email,
    Backup,
    Crawler,
    MediaProcessing,
    Cleanup,
    DataSync,
    Notification,
    Report,
    Custom(String),
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Email => write!(f, "email"),
            JobType::Backup => write!(f, "backup"),
            JobType::Crawler => write!(f, "crawler"),
            JobType::MediaProcessing => write!(f, "media_processing"),
            JobType::Cleanup => write!(f, "cleanup"),
            JobType::DataSync => write!(f, "data_sync"),
            JobType::Notification => write!(f, "notification"),
            JobType::Report => write!(f, "report"),
            JobType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Job timeout after {0} seconds")]
    Timeout(u64),

    #[error("Job handler not found for type: {0}")]
    HandlerNotFound(String),

    #[error("Job serialization error: {0}")]
    SerializationError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Job cancelled")]
    Cancelled,

    #[error("Invalid job state: {0}")]
    InvalidState(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<serde_json::Error> for JobError {
    fn from(err: serde_json::Error) -> Self {
        JobError::SerializationError(err.to_string())
    }
}

impl From<deadpool_redis::redis::RedisError> for JobError {
    fn from(err: deadpool_redis::redis::RedisError) -> Self {
        JobError::RedisError(err.to_string())
    }
}

impl From<anyhow::Error> for JobError {
    fn from(err: anyhow::Error) -> Self {
        JobError::Other(err.to_string())
    }
}

use std::collections::HashMap;
