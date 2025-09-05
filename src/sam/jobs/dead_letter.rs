use anyhow::{Result, Context};
use chrono::Utc;
use deadpool_redis::{redis::cmd, Pool};
use log::{error, info, warn};
use serde_json;
use super::types::Job;
use super::queue::JobQueue;

const DEAD_LETTER_KEY: &str = "jobs:dead_letter";
const DEAD_LETTER_SET_KEY: &str = "jobs:dead_letter:set";
const DEAD_LETTER_STATS_KEY: &str = "jobs:dead_letter:stats";

#[derive(Debug)]
pub struct DeadLetterQueue {
    redis_pool: Pool,
}

impl DeadLetterQueue {
    pub async fn new(redis_pool: Pool) -> Result<Self> {
        Ok(Self { redis_pool })
    }
    
    pub async fn add(&self, mut job: Job) -> Result<()> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Update job status
        job.status = super::types::JobStatus::DeadLetter;
        job.updated_at = Utc::now();
        
        let job_json = serde_json::to_string(&job)
            .context("Failed to serialize job")?;
        
        // Add to dead letter queue
        cmd("LPUSH")
            .arg(DEAD_LETTER_KEY)
            .arg(&job_json)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to add job to dead letter queue")?;
        
        // Add job ID to set for quick lookup
        cmd("SADD")
            .arg(DEAD_LETTER_SET_KEY)
            .arg(&job.id)
            .query_async::<i32>(&mut conn)
            .await
            .context("Failed to add job ID to dead letter set")?;
        
        // Update stats
        cmd("HINCRBY")
            .arg(DEAD_LETTER_STATS_KEY)
            .arg("total_jobs")
            .arg(1)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        cmd("HINCRBY")
            .arg(DEAD_LETTER_STATS_KEY)
            .arg(&job.job_type)
            .arg(1)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        warn!("Job {} moved to dead letter queue", job.id);
        Ok(())
    }
    
    pub async fn get(&self, limit: usize) -> Result<Vec<Job>> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let job_jsons: Vec<String> = cmd("LRANGE")
            .arg(DEAD_LETTER_KEY)
            .arg(0)
            .arg(limit as isize - 1)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .context("Failed to get jobs from dead letter queue")?;
        
        let mut jobs = Vec::new();
        for json in job_jsons {
            match serde_json::from_str::<Job>(&json) {
                Ok(job) => jobs.push(job),
                Err(e) => error!("Failed to deserialize dead letter job: {}", e),
            }
        }
        
        Ok(jobs)
    }
    
    pub async fn retry(&self, job_id: &str, queue: &JobQueue) -> Result<bool> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Check if job is in dead letter queue
        let is_member: bool = cmd("SISMEMBER")
            .arg(DEAD_LETTER_SET_KEY)
            .arg(job_id)
            .query_async::<bool>(&mut conn)
            .await
            .context("Failed to check if job is in dead letter queue")?;
        
        if !is_member {
            return Ok(false);
        }
        
        // Get all jobs from dead letter queue
        let job_jsons: Vec<String> = cmd("LRANGE")
            .arg(DEAD_LETTER_KEY)
            .arg(0)
            .arg(-1)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .context("Failed to get jobs from dead letter queue")?;
        
        // Find the specific job
        for (index, json) in job_jsons.iter().enumerate() {
            if let Ok(mut job) = serde_json::from_str::<Job>(json) {
                if job.id == job_id {
                    // Reset job for retry
                    job.status = super::types::JobStatus::Pending;
                    job.retry_count = 0;
                    job.error = None;
                    job.started_at = None;
                    job.completed_at = None;
                    job.updated_at = Utc::now();
                    
                    // Re-enqueue the job
                    queue.enqueue(job.clone()).await?;
                    
                    // Remove from dead letter queue
                    // Note: This is not atomic, but good enough for most cases
                    cmd("LREM")
                        .arg(DEAD_LETTER_KEY)
                        .arg(1)
                        .arg(json)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    // Remove from set
                    cmd("SREM")
                        .arg(DEAD_LETTER_SET_KEY)
                        .arg(job_id)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    // Update stats
                    cmd("HINCRBY")
                        .arg(DEAD_LETTER_STATS_KEY)
                        .arg("total_retried")
                        .arg(1)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    info!("Job {} retried from dead letter queue", job_id);
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    pub async fn retry_all(&self, queue: &JobQueue) -> Result<usize> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let job_jsons: Vec<String> = cmd("LRANGE")
            .arg(DEAD_LETTER_KEY)
            .arg(0)
            .arg(-1)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .context("Failed to get jobs from dead letter queue")?;
        
        let mut retried_count = 0;
        
        for json in job_jsons {
            if let Ok(mut job) = serde_json::from_str::<Job>(&json) {
                // Reset job for retry
                job.status = super::types::JobStatus::Pending;
                job.retry_count = 0;
                job.error = None;
                job.started_at = None;
                job.completed_at = None;
                job.updated_at = Utc::now();
                
                // Re-enqueue the job
                if queue.enqueue(job.clone()).await.is_ok() {
                    retried_count += 1;
                    
                    // Remove from set
                    cmd("SREM")
                        .arg(DEAD_LETTER_SET_KEY)
                        .arg(&job.id)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                }
            }
        }
        
        // Clear the dead letter queue
        cmd("DEL")
            .arg(DEAD_LETTER_KEY)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        // Update stats
        cmd("HINCRBY")
            .arg(DEAD_LETTER_STATS_KEY)
            .arg("total_retried")
            .arg(retried_count as i64)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        info!("Retried {} jobs from dead letter queue", retried_count);
        Ok(retried_count)
    }
    
    pub async fn purge(&self, job_id: &str) -> Result<bool> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Check if job is in dead letter queue
        let is_member: bool = cmd("SISMEMBER")
            .arg(DEAD_LETTER_SET_KEY)
            .arg(job_id)
            .query_async::<bool>(&mut conn)
            .await
            .context("Failed to check if job is in dead letter queue")?;
        
        if !is_member {
            return Ok(false);
        }
        
        // Get all jobs from dead letter queue
        let job_jsons: Vec<String> = cmd("LRANGE")
            .arg(DEAD_LETTER_KEY)
            .arg(0)
            .arg(-1)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .context("Failed to get jobs from dead letter queue")?;
        
        // Find and remove the specific job
        for json in job_jsons {
            if let Ok(job) = serde_json::from_str::<Job>(&json) {
                if job.id == job_id {
                    // Remove from queue
                    cmd("LREM")
                        .arg(DEAD_LETTER_KEY)
                        .arg(1)
                        .arg(&json)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    // Remove from set
                    cmd("SREM")
                        .arg(DEAD_LETTER_SET_KEY)
                        .arg(job_id)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    // Update stats
                    cmd("HINCRBY")
                        .arg(DEAD_LETTER_STATS_KEY)
                        .arg("total_purged")
                        .arg(1)
                        .query_async::<i32>(&mut conn)
                        .await
                        .ok();
                    
                    info!("Job {} purged from dead letter queue", job_id);
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    pub async fn purge_all(&self) -> Result<usize> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Get count before purging
        let count: usize = cmd("LLEN")
            .arg(DEAD_LETTER_KEY)
            .query_async::<usize>(&mut conn)
            .await
            .unwrap_or(0);
        
        // Clear the dead letter queue
        cmd("DEL")
            .arg(DEAD_LETTER_KEY)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        // Clear the set
        cmd("DEL")
            .arg(DEAD_LETTER_SET_KEY)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        // Update stats
        cmd("HINCRBY")
            .arg(DEAD_LETTER_STATS_KEY)
            .arg("total_purged")
            .arg(count as i64)
            .query_async::<i32>(&mut conn)
            .await
            .ok();
        
        info!("Purged {} jobs from dead letter queue", count);
        Ok(count)
    }
    
    pub async fn get_stats(&self) -> Result<std::collections::HashMap<String, i64>> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let stats: std::collections::HashMap<String, i64> = cmd("HGETALL")
            .arg(DEAD_LETTER_STATS_KEY)
            .query_async::<std::collections::HashMap<String, i64>>(&mut conn)
            .await
            .context("Failed to get dead letter stats")?;
        
        Ok(stats)
    }
    
    pub async fn size(&self) -> Result<usize> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let size: usize = cmd("LLEN")
            .arg(DEAD_LETTER_KEY)
            .query_async::<usize>(&mut conn)
            .await
            .context("Failed to get dead letter queue size")?;
        
        Ok(size)
    }
}