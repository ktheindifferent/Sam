pub mod dead_letter;
pub mod handler;
pub mod monitoring;
pub mod queue;
pub mod scheduler;
pub mod types;
pub mod worker;

pub use dead_letter::DeadLetterQueue;
pub use handler::JobHandler;
pub use monitoring::JobMonitor;
pub use queue::JobQueue;
pub use scheduler::JobScheduler;
pub use types::{Job, JobError, JobResult, JobStatus, JobType, Priority};
pub use worker::{Worker, WorkerPool};

// JobSystem and JobStats are defined below

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct JobSystem {
    pub queue: Arc<JobQueue>,
    pub worker_pool: Arc<WorkerPool>,
    pub scheduler: Arc<JobScheduler>,
    pub monitor: Arc<JobMonitor>,
    pub dead_letter: Arc<DeadLetterQueue>,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
}

impl std::fmt::Debug for JobSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobSystem")
            .field("queue", &"<JobQueue>")
            .field("worker_pool", &"<WorkerPool>")
            .field("scheduler", &"<JobScheduler>")
            .field("monitor", &"<JobMonitor>")
            .field("dead_letter", &"<DeadLetterQueue>")
            .field(
                "handlers",
                &format!(
                    "<{} handlers>",
                    self.handlers.try_read().map(|h| h.len()).unwrap_or(0)
                ),
            )
            .finish()
    }
}

impl JobSystem {
    pub async fn new(redis_pool: deadpool_redis::Pool, num_workers: usize) -> Result<Self> {
        let queue = Arc::new(JobQueue::new(redis_pool.clone()).await?);
        let scheduler = Arc::new(JobScheduler::new(redis_pool.clone()).await?);
        let monitor = Arc::new(JobMonitor::new(redis_pool.clone()).await?);
        let dead_letter = Arc::new(DeadLetterQueue::new(redis_pool.clone()).await?);
        let handlers = Arc::new(RwLock::new(HashMap::new()));

        let worker_pool = Arc::new(
            WorkerPool::new(
                num_workers,
                queue.clone(),
                handlers.clone(),
                monitor.clone(),
                dead_letter.clone(),
            )
            .await?,
        );

        Ok(Self {
            queue,
            worker_pool,
            scheduler,
            monitor,
            dead_letter,
            handlers,
        })
    }

    pub async fn register_handler(
        &self,
        job_type: String,
        handler: Arc<dyn JobHandler>,
    ) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        handlers.insert(job_type, handler);
        Ok(())
    }

    pub async fn enqueue(&self, job: Job) -> Result<String> {
        self.queue.enqueue(job).await
    }

    pub async fn schedule(&self, job: Job, at: DateTime<Utc>) -> Result<String> {
        self.scheduler.schedule(job, at).await
    }

    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        self.worker_pool.start().await?;
        self.monitor.start().await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.worker_pool.stop().await?;
        self.scheduler.stop().await?;
        self.monitor.stop().await?;
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<JobStats> {
        self.monitor.get_stats().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStats {
    pub total_jobs: u64,
    pub pending_jobs: u64,
    pub running_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub retry_jobs: u64,
    pub dead_letter_jobs: u64,
    pub average_processing_time_ms: f64,
    pub success_rate: f64,
    pub worker_count: usize,
    pub active_workers: usize,
}
