use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use deadpool_redis::{redis::cmd, Pool};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use super::types::{JobStatus, Priority};

const METRICS_KEY_PREFIX: &str = "jobs:metrics:";
const STATS_KEY: &str = "jobs:stats";

#[derive(Debug, Clone)]
pub struct JobMonitor {
    redis_pool: Pool,
    metrics: Arc<RwLock<Metrics>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl JobMonitor {
    pub async fn new(redis_pool: Pool) -> Result<Self> {
        Ok(Self {
            redis_pool,
            metrics: Arc::new(RwLock::new(Metrics::default())),
            shutdown_tx: None,
            handle: None,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);
        
        let redis_pool = self.redis_pool.clone();
        let metrics = self.metrics.clone();
        
        let handle = tokio::spawn(async move {
            info!("Job monitor starting");
            
            let mut metrics_interval = interval(Duration::from_secs(60));
            
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Job monitor shutting down");
                        break;
                    }
                    _ = metrics_interval.tick() => {
                        // Persist metrics to Redis
                        if let Err(e) = Self::persist_metrics(&redis_pool, &metrics).await {
                            error!("Failed to persist metrics: {}", e);
                        }
                        
                        // Generate alerts if needed
                        if let Err(e) = Self::check_alerts(&redis_pool, &metrics).await {
                            error!("Failed to check alerts: {}", e);
                        }
                    }
                }
            }
            
            info!("Job monitor stopped");
        });
        
        self.handle = Some(handle);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        if let Some(handle) = self.handle.take() {
            handle.await.context("Failed to join monitor task")?;
        }
        
        Ok(())
    }
    
    pub async fn record_success(&self, job_id: &str, processing_time: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_completed += 1;
        metrics.processing_times.push(processing_time.as_millis() as f64);
        
        // Keep only last 1000 processing times
        if metrics.processing_times.len() > 1000 {
            metrics.processing_times.remove(0);
        }
        
        debug!("Recorded success for job {} ({}ms)", job_id, processing_time.as_millis());
    }
    
    pub async fn record_failure(&self, job_id: &str, processing_time: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_failed += 1;
        metrics.processing_times.push(processing_time.as_millis() as f64);
        
        debug!("Recorded failure for job {} ({}ms)", job_id, processing_time.as_millis());
    }
    
    pub async fn record_retry(&self, job_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.total_retried += 1;
        
        debug!("Recorded retry for job {}", job_id);
    }
    
    pub async fn record_timeout(&self, job_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.total_timeouts += 1;
        
        debug!("Recorded timeout for job {}", job_id);
    }
    
    pub async fn get_stats(&self) -> Result<super::JobStats> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let metrics = self.metrics.read().await;
        
        // Get queue lengths
        let mut pending_jobs = 0u64;
        for priority in [Priority::Critical, Priority::High, Priority::Normal, Priority::Low] {
            let queue_name = priority.queue_name();
            let length: u64 = cmd("LLEN")
                .arg(&queue_name)
                .query_async(&mut conn)
                .await
                .unwrap_or(0);
            pending_jobs += length;
        }
        
        // Get running jobs count
        let running_jobs: u64 = cmd("SCARD")
            .arg("jobs:running")
            .query_async(&mut conn)
            .await
            .unwrap_or(0);
        
        // Get scheduled jobs count
        let scheduled_jobs: u64 = cmd("ZCARD")
            .arg("jobs:scheduled")
            .query_async(&mut conn)
            .await
            .unwrap_or(0);
        
        // Get dead letter queue size
        let dead_letter_jobs: u64 = cmd("LLEN")
            .arg("jobs:dead_letter")
            .query_async(&mut conn)
            .await
            .unwrap_or(0);
        
        // Calculate average processing time
        let avg_processing_time = if !metrics.processing_times.is_empty() {
            metrics.processing_times.iter().sum::<f64>() / metrics.processing_times.len() as f64
        } else {
            0.0
        };
        
        // Calculate success rate
        let total_processed = metrics.total_completed + metrics.total_failed;
        let success_rate = if total_processed > 0 {
            (metrics.total_completed as f64 / total_processed as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(super::JobStats {
            total_jobs: metrics.total_completed + metrics.total_failed + pending_jobs + running_jobs,
            pending_jobs: pending_jobs + scheduled_jobs,
            running_jobs,
            completed_jobs: metrics.total_completed,
            failed_jobs: metrics.total_failed,
            retry_jobs: metrics.total_retried,
            dead_letter_jobs,
            average_processing_time_ms: avg_processing_time,
            success_rate,
            worker_count: 0, // Will be set by WorkerPool
            active_workers: 0, // Will be set by WorkerPool
        })
    }
    
    async fn persist_metrics(redis_pool: &Pool, metrics: &Arc<RwLock<Metrics>>) -> Result<()> {
        let mut conn = redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let metrics = metrics.read().await;
        let timestamp = Utc::now().timestamp();
        
        // Store current metrics snapshot
        let snapshot = MetricsSnapshot {
            timestamp,
            total_completed: metrics.total_completed,
            total_failed: metrics.total_failed,
            total_retried: metrics.total_retried,
            total_timeouts: metrics.total_timeouts,
            avg_processing_time: if !metrics.processing_times.is_empty() {
                metrics.processing_times.iter().sum::<f64>() / metrics.processing_times.len() as f64
            } else {
                0.0
            },
        };
        
        let snapshot_json = serde_json::to_string(&snapshot)
            .context("Failed to serialize metrics snapshot")?;
        
        let key = format!("{}{}", METRICS_KEY_PREFIX, timestamp);
        
        // Store with 7 day expiration
        cmd("SETEX")
            .arg(&key)
            .arg(604800)
            .arg(&snapshot_json)
            .query_async(&mut conn)
            .await
            .context("Failed to persist metrics")?;
        
        // Update global stats
        cmd("HINCRBY")
            .arg(STATS_KEY)
            .arg("total_completed")
            .arg(metrics.total_completed)
            .query_async::<_, ()>(&mut conn)
            .await
            .ok();
        
        cmd("HINCRBY")
            .arg(STATS_KEY)
            .arg("total_failed")
            .arg(metrics.total_failed)
            .query_async::<_, ()>(&mut conn)
            .await
            .ok();
        
        debug!("Persisted metrics snapshot at {}", timestamp);
        Ok(())
    }
    
    async fn check_alerts(redis_pool: &Pool, metrics: &Arc<RwLock<Metrics>>) -> Result<()> {
        let metrics = metrics.read().await;
        
        // Check failure rate
        let total_recent = metrics.total_completed + metrics.total_failed;
        if total_recent > 100 {
            let failure_rate = (metrics.total_failed as f64 / total_recent as f64) * 100.0;
            
            if failure_rate > 10.0 {
                // High failure rate alert
                info!("ALERT: High job failure rate: {:.2}%", failure_rate);
                // Here you would send notifications, log to monitoring system, etc.
            }
        }
        
        // Check for excessive timeouts
        if metrics.total_timeouts > 10 {
            info!("ALERT: High number of job timeouts: {}", metrics.total_timeouts);
        }
        
        // Check queue depth (would need to query Redis for this)
        let mut conn = redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        for priority in [Priority::Critical, Priority::High] {
            let queue_name = priority.queue_name();
            let length: u64 = cmd("LLEN")
                .arg(&queue_name)
                .query_async(&mut conn)
                .await
                .unwrap_or(0);
            
            if length > 1000 {
                info!("ALERT: {} queue depth is high: {} jobs", priority, length);
            }
        }
        
        Ok(())
    }
    
    pub async fn get_job_history(&self, job_id: &str) -> Result<Vec<JobEvent>> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let history_key = format!("jobs:history:{}", job_id);
        let events: Vec<String> = cmd("LRANGE")
            .arg(&history_key)
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await
            .context("Failed to get job history")?;
        
        let mut history = Vec::new();
        for event_json in events {
            if let Ok(event) = serde_json::from_str::<JobEvent>(&event_json) {
                history.push(event);
            }
        }
        
        Ok(history)
    }
    
    pub async fn record_event(&self, job_id: &str, event_type: JobEventType, details: Option<String>) -> Result<()> {
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let event = JobEvent {
            timestamp: Utc::now(),
            event_type,
            details,
        };
        
        let event_json = serde_json::to_string(&event)
            .context("Failed to serialize job event")?;
        
        let history_key = format!("jobs:history:{}", job_id);
        
        cmd("LPUSH")
            .arg(&history_key)
            .arg(&event_json)
            .query_async(&mut conn)
            .await
            .context("Failed to record job event")?;
        
        // Keep only last 100 events
        cmd("LTRIM")
            .arg(&history_key)
            .arg(0)
            .arg(99)
            .query_async::<_, ()>(&mut conn)
            .await
            .ok();
        
        // Set expiration
        cmd("EXPIRE")
            .arg(&history_key)
            .arg(604800) // 7 days
            .query_async::<_, ()>(&mut conn)
            .await
            .ok();
        
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Metrics {
    total_completed: u64,
    total_failed: u64,
    total_retried: u64,
    total_timeouts: u64,
    processing_times: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricsSnapshot {
    timestamp: i64,
    total_completed: u64,
    total_failed: u64,
    total_retried: u64,
    total_timeouts: u64,
    avg_processing_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: JobEventType,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobEventType {
    Created,
    Enqueued,
    Started,
    Completed,
    Failed,
    Retried,
    TimedOut,
    Cancelled,
    MovedToDeadLetter,
}