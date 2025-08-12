#[cfg(test)]
mod job_queue_tests {
    use sam::jobs::*;
    use sam::services::redis;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio;

    // Test job handler
    struct TestJobHandler {
        should_fail: bool,
        should_retry: bool,
    }

    #[async_trait]
    impl JobHandler for TestJobHandler {
        async fn handle(&self, payload: serde_json::Value) -> Result<JobResult, JobError> {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            if self.should_fail {
                Ok(JobResult::Failure("Test failure".to_string()))
            } else if self.should_retry {
                Ok(JobResult::Retry("Test retry".to_string()))
            } else {
                Ok(JobResult::Success(json!({
                    "processed": true,
                    "payload": payload
                })))
            }
        }
        
        fn name(&self) -> &str {
            "test"
        }
        
        fn max_retries(&self) -> u32 {
            3
        }
    }

    async fn setup_redis() -> deadpool_redis::Pool {
        // Ensure Redis is running
        redis::start().await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        redis::connect().await
            .expect("Failed to connect to Redis for tests")
    }

    #[tokio::test]
    async fn test_job_creation_and_enqueue() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        let job = Job::new("test".to_string(), json!({"data": "test"}));
        let job_id = job_system.enqueue(job).await
            .expect("Failed to enqueue job");
        
        assert!(!job_id.is_empty());
    }

    #[tokio::test]
    async fn test_job_processing() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        // Register test handler
        let handler = Arc::new(TestJobHandler {
            should_fail: false,
            should_retry: false,
        });
        job_system.register_handler("test".to_string(), handler).await
            .expect("Failed to register handler");
        
        // Start the system
        job_system.start().await
            .expect("Failed to start job system");
        
        // Enqueue a job
        let job = Job::new("test".to_string(), json!({"data": "test"}));
        let job_id = job_system.enqueue(job).await
            .expect("Failed to enqueue job");
        
        // Wait for processing
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Check job status
        let processed_job = job_system.queue.get_job(&job_id).await
            .expect("Failed to get job")
            .expect("Job not found");
        
        assert_eq!(processed_job.status, JobStatus::Completed);
        assert!(processed_job.result.is_some());
        
        job_system.stop().await
            .expect("Failed to stop job system");
    }

    #[tokio::test]
    async fn test_job_retry() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        // Register handler that will retry
        let handler = Arc::new(TestJobHandler {
            should_fail: false,
            should_retry: true,
        });
        job_system.register_handler("test_retry".to_string(), handler).await
            .expect("Failed to register handler");
        
        // Start the system
        job_system.start().await
            .expect("Failed to start job system");
        
        // Enqueue a job with limited retries
        let job = Job::new("test_retry".to_string(), json!({"data": "test"}))
            .with_max_retries(2);
        let job_id = job_system.enqueue(job).await
            .expect("Failed to enqueue job");
        
        // Wait for initial processing
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Check job status - should be retrying
        let retrying_job = job_system.queue.get_job(&job_id).await
            .expect("Failed to get job")
            .expect("Job not found");
        
        assert_eq!(retrying_job.status, JobStatus::Retrying);
        assert!(retrying_job.retry_count > 0);
        
        job_system.stop().await
            .expect("Failed to stop job system");
    }

    #[tokio::test]
    async fn test_job_priority() {
        let pool = setup_redis().await;
        let queue = JobQueue::new(pool.clone()).await
            .expect("Failed to create job queue");
        
        // Enqueue jobs with different priorities
        let critical_job = Job::new("test".to_string(), json!({"priority": "critical"}))
            .with_priority(Priority::Critical);
        let normal_job = Job::new("test".to_string(), json!({"priority": "normal"}))
            .with_priority(Priority::Normal);
        let low_job = Job::new("test".to_string(), json!({"priority": "low"}))
            .with_priority(Priority::Low);
        
        // Enqueue in reverse priority order
        queue.enqueue(low_job.clone()).await.expect("Failed to enqueue low priority job");
        queue.enqueue(normal_job.clone()).await.expect("Failed to enqueue normal priority job");
        queue.enqueue(critical_job.clone()).await.expect("Failed to enqueue critical priority job");
        
        // Dequeue should get critical job first
        let priorities = vec![Priority::Critical, Priority::High, Priority::Normal, Priority::Low];
        let dequeued = queue.dequeue(priorities).await
            .expect("Failed to dequeue")
            .expect("No job dequeued");
        
        assert_eq!(dequeued.priority, Priority::Critical);
    }

    #[tokio::test]
    async fn test_dead_letter_queue() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        // Register handler that will fail
        let handler = Arc::new(TestJobHandler {
            should_fail: true,
            should_retry: false,
        });
        job_system.register_handler("test_fail".to_string(), handler).await
            .expect("Failed to register handler");
        
        // Start the system
        job_system.start().await
            .expect("Failed to start job system");
        
        // Enqueue a job with no retries
        let job = Job::new("test_fail".to_string(), json!({"data": "test"}))
            .with_max_retries(0);
        let job_id = job_system.enqueue(job).await
            .expect("Failed to enqueue job");
        
        // Wait for processing
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Check dead letter queue
        let dead_letter_size = job_system.dead_letter.size().await
            .expect("Failed to get dead letter queue size");
        
        assert!(dead_letter_size > 0);
        
        // Try to retry from dead letter
        let retried = job_system.dead_letter.retry(&job_id, &job_system.queue).await
            .expect("Failed to retry from dead letter");
        
        assert!(retried);
        
        job_system.stop().await
            .expect("Failed to stop job system");
    }

    #[tokio::test]
    async fn test_job_scheduling() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        // Schedule a job for 2 seconds in the future
        let scheduled_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let job = Job::new("test".to_string(), json!({"scheduled": true}))
            .with_schedule(scheduled_at);
        
        let job_id = job_system.schedule(job, scheduled_at).await
            .expect("Failed to schedule job");
        
        // Start the scheduler
        job_system.start().await
            .expect("Failed to start job system");
        
        // Immediately check - job should be scheduled, not running
        let scheduled_job = job_system.queue.get_job(&job_id).await
            .expect("Failed to get job")
            .expect("Job not found");
        
        assert!(scheduled_job.scheduled_at.is_some());
        
        // Wait for scheduled time
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Job should now be enqueued
        // In a real test, we'd check if it's been processed
        
        job_system.stop().await
            .expect("Failed to stop job system");
    }

    #[tokio::test]
    async fn test_job_stats() {
        let pool = setup_redis().await;
        let job_system = JobSystem::new(pool, 2).await
            .expect("Failed to create job system");
        
        // Register test handler
        let handler = Arc::new(TestJobHandler {
            should_fail: false,
            should_retry: false,
        });
        job_system.register_handler("test".to_string(), handler).await
            .expect("Failed to register handler");
        
        // Start the system
        job_system.start().await
            .expect("Failed to start job system");
        
        // Enqueue multiple jobs
        for i in 0..5 {
            let job = Job::new("test".to_string(), json!({"index": i}));
            job_system.enqueue(job).await
                .expect("Failed to enqueue job");
        }
        
        // Wait for processing
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Get stats
        let stats = job_system.get_stats().await
            .expect("Failed to get stats");
        
        assert!(stats.total_jobs > 0);
        assert_eq!(stats.completed_jobs, 5);
        
        job_system.stop().await
            .expect("Failed to stop job system");
    }

    #[tokio::test]
    async fn test_worker_pool_resize() {
        let pool = setup_redis().await;
        let mut worker_pool = WorkerPool::new(
            2,
            Arc::new(JobQueue::new(pool.clone()).await.unwrap()),
            Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            Arc::new(JobMonitor::new(pool.clone()).await.unwrap()),
            Arc::new(DeadLetterQueue::new(pool).await.unwrap()),
        ).await.expect("Failed to create worker pool");
        
        // Start with 2 workers
        worker_pool.start().await
            .expect("Failed to start worker pool");
        
        assert_eq!(worker_pool.size(), 2);
        
        // Resize to 4 workers
        worker_pool.resize(4).await
            .expect("Failed to resize worker pool");
        
        assert_eq!(worker_pool.size(), 4);
        
        // Resize down to 1 worker
        worker_pool.resize(1).await
            .expect("Failed to resize worker pool");
        
        assert_eq!(worker_pool.size(), 1);
        
        worker_pool.stop().await
            .expect("Failed to stop worker pool");
    }
}