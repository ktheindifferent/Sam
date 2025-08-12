use anyhow::{Result, Context};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{timeout, sleep};
use super::handler::JobHandler;
use super::queue::JobQueue;
use super::types::{Job, JobResult, JobError, Priority};
use super::monitoring::JobMonitor;
use super::dead_letter::DeadLetterQueue;

pub struct Worker {
    id: String,
    queue: Arc<JobQueue>,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
    monitor: Arc<JobMonitor>,
    dead_letter: Arc<DeadLetterQueue>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Worker {
    pub fn new(
        id: String,
        queue: Arc<JobQueue>,
        handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
        monitor: Arc<JobMonitor>,
        dead_letter: Arc<DeadLetterQueue>,
    ) -> Self {
        Self {
            id,
            queue,
            handlers,
            monitor,
            dead_letter,
            shutdown_tx: None,
            handle: None,
        }
    }
    
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);
        
        let worker_id = self.id.clone();
        let queue = self.queue.clone();
        let handlers = self.handlers.clone();
        let monitor = self.monitor.clone();
        let dead_letter = self.dead_letter.clone();
        
        let handle = tokio::spawn(async move {
            info!("Worker {} starting", worker_id);
            
            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    info!("Worker {} shutting down", worker_id);
                    break;
                }
                
                // Try to get a job from the queue
                let priorities = vec![
                    Priority::Critical,
                    Priority::High,
                    Priority::Normal,
                    Priority::Low,
                ];
                
                match queue.dequeue(priorities).await {
                    Ok(Some(job)) => {
                        info!("Worker {} processing job {}", worker_id, job.id);
                        
                        // Process the job
                        let handlers_lock = handlers.read().await;
                        let handler = handlers_lock.get(&job.job_type);
                        
                        match handler {
                            Some(h) => {
                                let handler = h.clone();
                                drop(handlers_lock); // Release the lock
                                
                                let start_time = std::time::Instant::now();
                                
                                // Execute with timeout
                                let timeout_duration = job.timeout_secs
                                    .map(Duration::from_secs)
                                    .unwrap_or(Duration::from_secs(300));
                                
                                let result = timeout(
                                    timeout_duration,
                                    handler.handle(job.payload.clone())
                                ).await;
                                
                                let processing_time = start_time.elapsed();
                                
                                match result {
                                    Ok(Ok(JobResult::Success(value))) => {
                                        // Job succeeded
                                        if let Err(e) = queue.complete_job(job.clone(), value.clone()).await {
                                            error!("Failed to mark job {} as complete: {}", job.id, e);
                                        }
                                        
                                        if let Err(e) = handler.on_success(&job.payload, &JobResult::Success(value)).await {
                                            error!("Job {} on_success hook failed: {}", job.id, e);
                                        }
                                        
                                        monitor.record_success(&job.id, processing_time).await;
                                    }
                                    Ok(Ok(JobResult::Failure(error))) => {
                                        // Job failed permanently
                                        error!("Job {} failed: {}", job.id, error);
                                        
                                        if let Err(e) = queue.fail_job(job.clone(), error.clone()).await {
                                            error!("Failed to mark job {} as failed: {}", job.id, e);
                                        }
                                        
                                        if let Err(e) = dead_letter.add(job.clone()).await {
                                            error!("Failed to add job {} to dead letter queue: {}", job.id, e);
                                        }
                                        
                                        if let Err(e) = handler.on_failure(&job.payload, &JobError::ExecutionFailed(error)).await {
                                            error!("Job {} on_failure hook failed: {}", job.id, e);
                                        }
                                        
                                        monitor.record_failure(&job.id, processing_time).await;
                                    }
                                    Ok(Ok(JobResult::Retry(error))) | Ok(Err(e)) => {
                                        // Job should be retried
                                        let error_msg = match result {
                                            Ok(Ok(JobResult::Retry(e))) => e,
                                            Ok(Err(e)) => e.to_string(),
                                            _ => "Unknown error".to_string(),
                                        };
                                        
                                        warn!("Job {} failed, will retry: {}", job.id, error_msg);
                                        
                                        if job.should_retry() {
                                            if let Err(e) = queue.retry_job(job.clone(), error_msg.clone()).await {
                                                error!("Failed to retry job {}: {}", job.id, e);
                                            }
                                            
                                            if let Err(e) = handler.on_retry(&job.payload, job.retry_count, &JobError::ExecutionFailed(error_msg)).await {
                                                error!("Job {} on_retry hook failed: {}", job.id, e);
                                            }
                                            
                                            monitor.record_retry(&job.id).await;
                                        } else {
                                            // Max retries exceeded
                                            error!("Job {} exceeded max retries", job.id);
                                            
                                            if let Err(e) = queue.fail_job(job.clone(), error_msg).await {
                                                error!("Failed to mark job {} as failed: {}", job.id, e);
                                            }
                                            
                                            if let Err(e) = dead_letter.add(job.clone()).await {
                                                error!("Failed to add job {} to dead letter queue: {}", job.id, e);
                                            }
                                            
                                            monitor.record_failure(&job.id, processing_time).await;
                                        }
                                    }
                                    Err(_) => {
                                        // Job timed out
                                        error!("Job {} timed out after {} seconds", job.id, timeout_duration.as_secs());
                                        
                                        if job.should_retry() {
                                            if let Err(e) = queue.retry_job(job.clone(), format!("Timeout after {} seconds", timeout_duration.as_secs())).await {
                                                error!("Failed to retry job {}: {}", job.id, e);
                                            }
                                            monitor.record_retry(&job.id).await;
                                        } else {
                                            if let Err(e) = queue.fail_job(job.clone(), format!("Timeout after {} seconds", timeout_duration.as_secs())).await {
                                                error!("Failed to mark job {} as failed: {}", job.id, e);
                                            }
                                            
                                            if let Err(e) = dead_letter.add(job.clone()).await {
                                                error!("Failed to add job {} to dead letter queue: {}", job.id, e);
                                            }
                                            
                                            monitor.record_timeout(&job.id).await;
                                        }
                                    }
                                }
                            }
                            None => {
                                error!("No handler found for job type: {}", job.job_type);
                                
                                if let Err(e) = queue.fail_job(job.clone(), format!("No handler for job type: {}", job.job_type)).await {
                                    error!("Failed to mark job {} as failed: {}", job.id, e);
                                }
                                
                                if let Err(e) = dead_letter.add(job).await {
                                    error!("Failed to add job to dead letter queue: {}", e);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // No jobs available, wait a bit
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("Worker {} failed to dequeue job: {}", worker_id, e);
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            
            info!("Worker {} stopped", worker_id);
        });
        
        self.handle = Some(handle);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        if let Some(handle) = self.handle.take() {
            handle.await.context("Failed to join worker task")?;
        }
        
        Ok(())
    }
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    queue: Arc<JobQueue>,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
    monitor: Arc<JobMonitor>,
    dead_letter: Arc<DeadLetterQueue>,
    num_workers: usize,
}

impl WorkerPool {
    pub async fn new(
        num_workers: usize,
        queue: Arc<JobQueue>,
        handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
        monitor: Arc<JobMonitor>,
        dead_letter: Arc<DeadLetterQueue>,
    ) -> Result<Self> {
        let mut workers = Vec::with_capacity(num_workers);
        
        for i in 0..num_workers {
            let worker = Worker::new(
                format!("worker-{}", i),
                queue.clone(),
                handlers.clone(),
                monitor.clone(),
                dead_letter.clone(),
            );
            workers.push(worker);
        }
        
        Ok(Self {
            workers,
            queue,
            handlers,
            monitor,
            dead_letter,
            num_workers,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting worker pool with {} workers", self.num_workers);
        
        for worker in &mut self.workers {
            worker.start().await?;
        }
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping worker pool");
        
        for worker in &mut self.workers {
            worker.stop().await?;
        }
        
        Ok(())
    }
    
    pub async fn resize(&mut self, new_size: usize) -> Result<()> {
        let current_size = self.workers.len();
        
        if new_size > current_size {
            // Add workers
            for i in current_size..new_size {
                let mut worker = Worker::new(
                    format!("worker-{}", i),
                    self.queue.clone(),
                    self.handlers.clone(),
                    self.monitor.clone(),
                    self.dead_letter.clone(),
                );
                worker.start().await?;
                self.workers.push(worker);
            }
            info!("Worker pool expanded from {} to {} workers", current_size, new_size);
        } else if new_size < current_size {
            // Remove workers
            while self.workers.len() > new_size {
                if let Some(mut worker) = self.workers.pop() {
                    worker.stop().await?;
                }
            }
            info!("Worker pool reduced from {} to {} workers", current_size, new_size);
        }
        
        self.num_workers = new_size;
        Ok(())
    }
    
    pub fn size(&self) -> usize {
        self.workers.len()
    }
}