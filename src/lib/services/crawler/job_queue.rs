//! Persistent job queue implementation for the crawler using Redis
//! 
//! This module provides a distributed, persistent job queue that survives restarts
//! and supports multi-instance deployments with proper locking.

use super::job::CrawlJob;
use crate::services::redis;
use anyhow::{Result, Context};
use deadpool_redis::{redis::AsyncCommands, Pool};
use log::{error, info, warn, debug};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Redis key prefixes for crawler operations
const REDIS_JOB_QUEUE_KEY: &str = "sam:crawler:job_queue";
const REDIS_ACTIVE_JOBS_KEY: &str = "sam:crawler:active_jobs";
const REDIS_FAILED_JOBS_KEY: &str = "sam:crawler:failed_jobs";
const REDIS_LOCK_PREFIX: &str = "sam:crawler:lock:";
const REDIS_RETRY_QUEUE_KEY: &str = "sam:crawler:retry_queue";

/// Job status in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running { worker_id: String, started_at: u64 },
    Completed { completed_at: u64 },
    Failed { failed_at: u64, error: String, retry_count: u32 },
    Retrying { retry_at: u64, retry_count: u32 },
}

/// Extended job information for queue management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedJob {
    pub job: CrawlJob,
    pub status: JobStatus,
    pub priority: i32,
    pub max_retries: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

impl QueuedJob {
    pub fn new(job: CrawlJob) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        Self {
            job,
            status: JobStatus::Pending,
            priority: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Persistent job queue backed by Redis
pub struct PersistentJobQueue {
    pool: Pool,
    worker_id: String,
    local_cache: Arc<RwLock<VecDeque<QueuedJob>>>,
    max_queue_size: usize,
}

impl PersistentJobQueue {
    /// Create a new persistent job queue
    pub async fn new(max_queue_size: usize) -> Result<Self> {
        let pool = redis::connect().await
            .context("Failed to connect to Redis for job queue")?;
        
        let worker_id = format!("worker_{}", Uuid::new_v4());
        
        Ok(Self {
            pool,
            worker_id,
            local_cache: Arc::new(RwLock::new(VecDeque::new())),
            max_queue_size,
        })
    }
    
    /// Add a job to the queue
    pub async fn enqueue(&self, job: CrawlJob, priority: Option<i32>) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        let queued_job = QueuedJob::new(job)
            .with_priority(priority.unwrap_or(0));
        
        let serialized = serde_json::to_string(&queued_job)
            .context("Failed to serialize job")?;
        
        // Add to Redis sorted set with priority as score (higher priority = lower score)
        conn.zadd::<_, _, _, ()>(REDIS_JOB_QUEUE_KEY, serialized, -queued_job.priority)
            .await
            .context("Failed to add job to Redis queue")?;
        
        info!("Enqueued job {} with priority {}", queued_job.job.oid, queued_job.priority);
        
        Ok(())
    }
    
    /// Get the next job from the queue (blocking)
    pub async fn dequeue(&self) -> Result<Option<QueuedJob>> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Try to get the highest priority job (lowest score)
        let result: Option<Vec<String>> = conn.zpopmin(REDIS_JOB_QUEUE_KEY, 1)
            .await
            .context("Failed to pop job from Redis queue")?;
        
        if let Some(items) = result {
            if !items.is_empty() {
                let job_str = &items[0];
                let mut job: QueuedJob = serde_json::from_str(job_str)
                    .context("Failed to deserialize job")?;
                
                // Update job status to running
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                job.status = JobStatus::Running {
                    worker_id: self.worker_id.clone(),
                    started_at: now,
                };
                job.updated_at = now;
                
                // Add to active jobs set
                let serialized = serde_json::to_string(&job)
                    .context("Failed to serialize active job")?;
                conn.hset::<_, _, _, ()>(REDIS_ACTIVE_JOBS_KEY, &job.job.oid, serialized)
                    .await
                    .context("Failed to add job to active set")?;
                
                debug!("Dequeued job {} for processing", job.job.oid);
                return Ok(Some(job));
            }
        }
        
        Ok(None)
    }
    
    /// Mark a job as completed
    pub async fn complete_job(&self, job_oid: &str) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Remove from active jobs
        conn.hdel::<_, _, ()>(REDIS_ACTIVE_JOBS_KEY, job_oid)
            .await
            .context("Failed to remove job from active set")?;
        
        info!("Job {} completed successfully", job_oid);
        Ok(())
    }
    
    /// Mark a job as failed and schedule for retry if applicable
    pub async fn fail_job(&self, job_oid: &str, error: String) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Get the active job
        let job_str: Option<String> = conn.hget(REDIS_ACTIVE_JOBS_KEY, job_oid)
            .await
            .context("Failed to get active job")?;
        
        if let Some(job_str) = job_str {
            let mut job: QueuedJob = serde_json::from_str(&job_str)
                .context("Failed to deserialize active job")?;
            
            let retry_count = match &job.status {
                JobStatus::Running { .. } => 0,
                JobStatus::Retrying { retry_count, .. } => *retry_count,
                _ => 0,
            };
            
            if retry_count < job.max_retries {
                // Schedule for retry with exponential backoff
                let retry_delay = Duration::from_secs(2u64.pow(retry_count));
                let retry_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() + retry_delay.as_secs())
                    .unwrap_or(0);
                
                job.status = JobStatus::Retrying {
                    retry_at,
                    retry_count: retry_count + 1,
                };
                job.updated_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                let serialized = serde_json::to_string(&job)
                    .context("Failed to serialize retry job")?;
                
                // Add to retry queue with retry time as score
                conn.zadd::<_, _, _, ()>(REDIS_RETRY_QUEUE_KEY, serialized, retry_at as f64)
                    .await
                    .context("Failed to add job to retry queue")?;
                
                warn!("Job {} failed, scheduled for retry #{} at {}", 
                      job_oid, retry_count + 1, retry_at);
            } else {
                // Max retries exceeded, move to failed jobs
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                job.status = JobStatus::Failed {
                    failed_at: now,
                    error: error.clone(),
                    retry_count,
                };
                job.updated_at = now;
                
                let serialized = serde_json::to_string(&job)
                    .context("Failed to serialize failed job")?;
                
                conn.hset::<_, _, _, ()>(REDIS_FAILED_JOBS_KEY, job_oid, serialized)
                    .await
                    .context("Failed to add job to failed set")?;
                
                error!("Job {} permanently failed after {} retries: {}", 
                       job_oid, retry_count, error);
            }
            
            // Remove from active jobs
            conn.hdel::<_, _, ()>(REDIS_ACTIVE_JOBS_KEY, job_oid)
                .await
                .context("Failed to remove job from active set")?;
        }
        
        Ok(())
    }
    
    /// Process jobs that are ready for retry
    pub async fn process_retries(&self) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        // Get jobs that are ready for retry (score <= now)
        let ready_jobs: Vec<String> = conn.zrangebyscore(
            REDIS_RETRY_QUEUE_KEY,
            0f64,
            now as f64
        ).await.context("Failed to get retry jobs")?;
        
        for job_str in ready_jobs {
            let job: QueuedJob = serde_json::from_str(&job_str)
                .context("Failed to deserialize retry job")?;
            
            // Remove from retry queue
            conn.zrem::<_, _, ()>(REDIS_RETRY_QUEUE_KEY, &job_str)
                .await
                .context("Failed to remove job from retry queue")?;
            
            // Re-enqueue with original priority
            self.enqueue(job.job, Some(job.priority)).await?;
            
            debug!("Re-enqueued job for retry");
        }
        
        Ok(())
    }
    
    /// Recover orphaned jobs (jobs marked as running but worker died)
    pub async fn recover_orphaned_jobs(&self, timeout: Duration) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        let active_jobs: Vec<(String, String)> = conn.hgetall(REDIS_ACTIVE_JOBS_KEY)
            .await
            .context("Failed to get active jobs")?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        for (job_oid, job_str) in active_jobs {
            let job: QueuedJob = serde_json::from_str(&job_str)
                .context("Failed to deserialize active job")?;
            
            if let JobStatus::Running { started_at, .. } = job.status {
                if now - started_at > timeout.as_secs() {
                    // Job has been running too long, likely orphaned
                    warn!("Recovering orphaned job {}", job_oid);
                    
                    // Mark as failed so it can be retried
                    self.fail_job(&job_oid, "Job orphaned (worker timeout)".to_string()).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        let pending_count: usize = conn.zcard(REDIS_JOB_QUEUE_KEY)
            .await
            .context("Failed to get pending job count")?;
        
        let active_count: usize = conn.hlen(REDIS_ACTIVE_JOBS_KEY)
            .await
            .context("Failed to get active job count")?;
        
        let failed_count: usize = conn.hlen(REDIS_FAILED_JOBS_KEY)
            .await
            .context("Failed to get failed job count")?;
        
        let retry_count: usize = conn.zcard(REDIS_RETRY_QUEUE_KEY)
            .await
            .context("Failed to get retry job count")?;
        
        Ok(QueueStats {
            pending: pending_count,
            active: active_count,
            failed: failed_count,
            retrying: retry_count,
            worker_id: self.worker_id.clone(),
        })
    }
    
    /// Clear all jobs from the queue (use with caution!)
    pub async fn clear_all(&self) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection")?;
        
        conn.del::<_, ()>(vec![
            REDIS_JOB_QUEUE_KEY,
            REDIS_ACTIVE_JOBS_KEY,
            REDIS_FAILED_JOBS_KEY,
            REDIS_RETRY_QUEUE_KEY,
        ]).await.context("Failed to clear queues")?;
        
        warn!("All crawler job queues cleared!");
        Ok(())
    }
}

/// Statistics about the job queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: usize,
    pub active: usize,
    pub failed: usize,
    pub retrying: usize,
    pub worker_id: String,
}

/// Distributed lock for multi-instance coordination
pub struct DistributedLock {
    pool: Pool,
    lock_key: String,
    lock_value: String,
    ttl_seconds: u64,
}

impl DistributedLock {
    /// Create a new distributed lock
    pub async fn new(pool: Pool, resource: &str, ttl_seconds: u64) -> Self {
        let lock_key = format!("{}{}", REDIS_LOCK_PREFIX, resource);
        let lock_value = Uuid::new_v4().to_string();
        
        Self {
            pool,
            lock_key,
            lock_value,
            ttl_seconds,
        }
    }
    
    /// Try to acquire the lock
    pub async fn try_acquire(&self) -> Result<bool> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection for lock")?;
        
        // SET NX EX - set if not exists with expiration
        let result: bool = deadpool_redis::redis::cmd("SET")
            .arg(&self.lock_key)
            .arg(&self.lock_value)
            .arg("NX")
            .arg("EX")
            .arg(self.ttl_seconds)
            .query_async(&mut conn)
            .await
            .context("Failed to acquire lock")?;
        
        Ok(result)
    }
    
    /// Release the lock (only if we own it)
    pub async fn release(&self) -> Result<()> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection for lock release")?;
        
        // Lua script to ensure we only delete our own lock
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;
        
        let _: i32 = deadpool_redis::redis::Script::new(script)
            .key(&self.lock_key)
            .arg(&self.lock_value)
            .invoke_async(&mut conn)
            .await
            .context("Failed to release lock")?;
        
        Ok(())
    }
    
    /// Extend the lock TTL (if we own it)
    pub async fn extend(&self, additional_seconds: u64) -> Result<bool> {
        let mut conn = self.pool.get().await
            .context("Failed to get Redis connection for lock extension")?;
        
        // Check if we own the lock
        let current_value: Option<String> = conn.get(&self.lock_key)
            .await
            .context("Failed to check lock ownership")?;
        
        if current_value.as_ref() == Some(&self.lock_value) {
            // Extend the TTL
            let result: bool = conn.expire(&self.lock_key, additional_seconds as i64)
                .await
                .context("Failed to extend lock TTL")?;
            
            Ok(result)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_job_queue_operations() {
        // This test requires Redis to be running
        if !crate::services::redis::is_running().await {
            println!("Skipping test - Redis not running");
            return;
        }
        
        let queue = PersistentJobQueue::new(1000).await.unwrap();
        
        // Clear any existing jobs
        queue.clear_all().await.unwrap();
        
        // Create and enqueue a job
        let mut job = CrawlJob::new();
        job.start_url = "https://example.com".to_string();
        
        queue.enqueue(job.clone(), Some(10)).await.unwrap();
        
        // Dequeue the job
        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        
        let dequeued_job = dequeued.unwrap();
        assert_eq!(dequeued_job.job.start_url, job.start_url);
        assert_eq!(dequeued_job.priority, 10);
        
        // Complete the job
        queue.complete_job(&dequeued_job.job.oid).await.unwrap();
        
        // Verify stats
        let stats = queue.get_stats().await.unwrap();
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.active, 0);
    }
}