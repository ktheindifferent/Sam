use super::types::{Job, JobStatus, Priority};
use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_redis::{redis::cmd, Pool};
use log::{debug, info, warn};
use serde_json;

const JOBS_KEY_PREFIX: &str = "jobs:";
const SCHEDULED_JOBS_KEY: &str = "jobs:scheduled";
const RUNNING_JOBS_KEY: &str = "jobs:running";
const STATS_KEY: &str = "jobs:stats";

#[derive(Debug)]
pub struct JobQueue {
    redis_pool: Pool,
}

impl JobQueue {
    pub async fn new(redis_pool: Pool) -> Result<Self> {
        Ok(Self { redis_pool })
    }

    pub async fn enqueue(&self, mut job: Job) -> Result<String> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        job.status = JobStatus::Pending;
        job.updated_at = Utc::now();

        let job_json = serde_json::to_string(&job).context("Failed to serialize job")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job.id);

        // Store job data
        cmd("SET")
            .arg(&job_key)
            .arg(&job_json)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to store job")?;

        // Set expiration (7 days)
        cmd("EXPIRE")
            .arg(&job_key)
            .arg(604800)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to set job expiration")?;

        if let Some(scheduled_at) = job.scheduled_at {
            // Add to scheduled set with timestamp as score
            cmd("ZADD")
                .arg(SCHEDULED_JOBS_KEY)
                .arg(scheduled_at.timestamp())
                .arg(&job.id)
                .query_async::<i32>(&mut conn)
                .await
                .context("Failed to add job to scheduled set")?;

            info!("Job {} scheduled for {}", job.id, scheduled_at);
        } else {
            // Add to priority queue
            let queue_name = job.priority.queue_name();
            cmd("LPUSH")
                .arg(&queue_name)
                .arg(&job.id)
                .query_async::<i32>(&mut conn)
                .await
                .context("Failed to add job to queue")?;

            info!("Job {} enqueued with priority {}", job.id, job.priority);
        }

        // Update stats
        self.increment_stat("total_enqueued").await?;

        Ok(job.id)
    }

    pub async fn dequeue(&self, priorities: Vec<Priority>) -> Result<Option<Job>> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        // Try to get a job from each priority queue in order
        for priority in priorities {
            let queue_name = priority.queue_name();

            // Pop job ID from queue
            let job_id: Option<String> = cmd("RPOP")
                .arg(&queue_name)
                .query_async::<Option<String>>(&mut conn)
                .await
                .context("Failed to pop job from queue")?;

            if let Some(id) = job_id {
                // Get job data
                let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, id);
                let job_json: Option<String> = cmd("GET")
                    .arg(&job_key)
                    .query_async::<Option<String>>(&mut conn)
                    .await
                    .context("Failed to get job data")?;

                if let Some(json) = job_json {
                    let mut job: Job =
                        serde_json::from_str(&json).context("Failed to deserialize job")?;

                    // Update job status
                    job.status = JobStatus::Running;
                    job.started_at = Some(Utc::now());
                    job.updated_at = Utc::now();

                    let updated_json =
                        serde_json::to_string(&job).context("Failed to serialize updated job")?;

                    // Update job in Redis
                    cmd("SET")
                        .arg(&job_key)
                        .arg(&updated_json)
                        .query_async::<()>(&mut conn)
                        .await
                        .context("Failed to update job")?;

                    // Add to running jobs set
                    cmd("SADD")
                        .arg(RUNNING_JOBS_KEY)
                        .arg(&job.id)
                        .query_async::<i32>(&mut conn)
                        .await
                        .context("Failed to add job to running set")?;

                    debug!("Dequeued job {} with priority {}", job.id, priority);
                    return Ok(Some(job));
                }
            }
        }

        Ok(None)
    }

    pub async fn complete_job(&self, mut job: Job, result: serde_json::Value) -> Result<()> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        job.status = JobStatus::Completed;
        job.completed_at = Some(Utc::now());
        job.updated_at = Utc::now();
        job.result = Some(result);

        let job_json = serde_json::to_string(&job).context("Failed to serialize job")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job.id);

        // Update job
        cmd("SET")
            .arg(&job_key)
            .arg(&job_json)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to update completed job")?;

        // Remove from running set
        cmd("SREM")
            .arg(RUNNING_JOBS_KEY)
            .arg(&job.id)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to remove job from running set")?;

        // Update stats
        self.increment_stat("total_completed").await?;

        info!("Job {} completed successfully", job.id);
        Ok(())
    }

    pub async fn fail_job(&self, mut job: Job, error: String) -> Result<()> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        job.status = JobStatus::Failed;
        job.completed_at = Some(Utc::now());
        job.updated_at = Utc::now();
        job.error = Some(error.clone());

        let job_json = serde_json::to_string(&job).context("Failed to serialize job")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job.id);

        // Update job
        cmd("SET")
            .arg(&job_key)
            .arg(&job_json)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to update failed job")?;

        // Remove from running set
        cmd("SREM")
            .arg(RUNNING_JOBS_KEY)
            .arg(&job.id)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to remove job from running set")?;

        // Update stats
        self.increment_stat("total_failed").await?;

        warn!("Job {} failed: {}", job.id, error);
        Ok(())
    }

    pub async fn retry_job(&self, mut job: Job, error: String) -> Result<()> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        job.retry_count += 1;
        job.status = JobStatus::Retrying;
        job.updated_at = Utc::now();
        job.error = Some(error);

        // Calculate retry delay
        let retry_delay = job.calculate_retry_delay();
        let retry_at = Utc::now()
            + chrono::Duration::from_std(retry_delay).context("Failed to convert duration")?;

        job.scheduled_at = Some(retry_at);

        let job_json = serde_json::to_string(&job).context("Failed to serialize job")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job.id);

        // Update job
        cmd("SET")
            .arg(&job_key)
            .arg(&job_json)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to update retrying job")?;

        // Remove from running set
        cmd("SREM")
            .arg(RUNNING_JOBS_KEY)
            .arg(&job.id)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to remove job from running set")?;

        // Add to scheduled set for retry
        cmd("ZADD")
            .arg(SCHEDULED_JOBS_KEY)
            .arg(retry_at.timestamp())
            .arg(&job.id)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to schedule job retry")?;

        // Update stats
        self.increment_stat("total_retries").await?;

        info!(
            "Job {} scheduled for retry at {} (attempt {})",
            job.id, retry_at, job.retry_count
        );
        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job_id);
        let job_json: Option<String> = cmd("GET")
            .arg(&job_key)
            .query_async::<Option<String>>(&mut conn)
            .await
            .context("Failed to get job")?;

        match job_json {
            Some(json) => {
                let job: Job = serde_json::from_str(&json).context("Failed to deserialize job")?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    pub async fn cancel_job(&self, job_id: &str) -> Result<bool> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        let job_key = format!("{}job:{}", JOBS_KEY_PREFIX, job_id);

        // Get the job first
        let job_json: Option<String> = cmd("GET")
            .arg(&job_key)
            .query_async::<Option<String>>(&mut conn)
            .await
            .context("Failed to get job")?;

        if let Some(json) = job_json {
            let mut job: Job = serde_json::from_str(&json).context("Failed to deserialize job")?;

            // Only cancel if job is pending or scheduled
            if job.status == JobStatus::Pending || job.status == JobStatus::Scheduled {
                job.status = JobStatus::Cancelled;
                job.updated_at = Utc::now();

                let updated_json =
                    serde_json::to_string(&job).context("Failed to serialize job")?;

                // Update job
                cmd("SET")
                    .arg(&job_key)
                    .arg(&updated_json)
                    .query_async::<()>(&mut conn)
                    .await
                    .context("Failed to update cancelled job")?;

                // Remove from queues
                for priority in [
                    Priority::Critical,
                    Priority::High,
                    Priority::Normal,
                    Priority::Low,
                ] {
                    let queue_name = priority.queue_name();
                    cmd("LREM")
                        .arg(&queue_name)
                        .arg(0)
                        .arg(job_id)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                }

                // Remove from scheduled set
                cmd("ZREM")
                    .arg(SCHEDULED_JOBS_KEY)
                    .arg(job_id)
                    .query_async::<i32>(&mut conn)
                    .await
                    .ok();

                info!("Job {} cancelled", job_id);
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn get_queue_length(&self, priority: Priority) -> Result<usize> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        let queue_name = priority.queue_name();
        let length: usize = cmd("LLEN")
            .arg(&queue_name)
            .query_async::<usize>(&mut conn)
            .await
            .context("Failed to get queue length")?;

        Ok(length)
    }

    pub async fn get_scheduled_jobs_due(&self) -> Result<Vec<String>> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        let now = Utc::now().timestamp();

        // Get all jobs scheduled before now
        let job_ids: Vec<String> = cmd("ZRANGEBYSCORE")
            .arg(SCHEDULED_JOBS_KEY)
            .arg("-inf")
            .arg(now)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .context("Failed to get scheduled jobs")?;

        // Remove them from the scheduled set
        if !job_ids.is_empty() {
            for job_id in &job_ids {
                cmd("ZREM")
                    .arg(SCHEDULED_JOBS_KEY)
                    .arg(job_id)
                    .query_async::<i32>(&mut conn)
                    .await
                    .ok();
            }
        }

        Ok(job_ids)
    }

    async fn increment_stat(&self, stat_name: &str) -> Result<()> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        cmd("HINCRBY")
            .arg(STATS_KEY)
            .arg(stat_name)
            .arg(1)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to increment stat")?;

        Ok(())
    }

    pub async fn get_stats(&self) -> Result<std::collections::HashMap<String, i64>> {
        let mut conn = self
            .redis_pool
            .get()
            .await
            .context("Failed to get Redis connection")?;

        let stats: std::collections::HashMap<String, i64> = cmd("HGETALL")
            .arg(STATS_KEY)
            .query_async::<std::collections::HashMap<String, i64>>(&mut conn)
            .await
            .context("Failed to get stats")?;

        Ok(stats)
    }
}
