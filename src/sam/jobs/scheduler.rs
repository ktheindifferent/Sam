use anyhow::{Result, Context};
use chrono::{DateTime, Utc, Datelike, Timelike};
use deadpool_redis::Pool;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::{interval, sleep};
use super::queue::JobQueue;
use super::types::{Job, Priority};

pub struct JobScheduler {
    redis_pool: Pool,
    queue: Arc<JobQueue>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl JobScheduler {
    pub async fn new(redis_pool: Pool) -> Result<Self> {
        let queue = Arc::new(JobQueue::new(redis_pool.clone()).await?);
        
        Ok(Self {
            redis_pool,
            queue,
            shutdown_tx: None,
            handle: None,
        })
    }
    
    pub async fn schedule(&self, mut job: Job, at: DateTime<Utc>) -> Result<String> {
        job.scheduled_at = Some(at);
        self.queue.enqueue(job).await
    }
    
    pub async fn schedule_recurring(
        &self,
        job_template: Job,
        schedule: CronSchedule,
    ) -> Result<String> {
        // Store the recurring job schedule in Redis
        let mut conn = self.redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        let recurring_id = nanoid::nanoid!();
        let recurring_key = format!("jobs:recurring:{}", recurring_id);
        
        let recurring_job = RecurringJob {
            id: recurring_id.clone(),
            template: job_template,
            schedule,
            last_run: None,
            next_run: schedule.next_run_after(Utc::now()),
            enabled: true,
        };
        
        let job_json = serde_json::to_string(&recurring_job)
            .context("Failed to serialize recurring job")?;
        
        deadpool_redis::redis::cmd("SET")
            .arg(&recurring_key)
            .arg(&job_json)
            .query_async(&mut conn)
            .await
            .context("Failed to store recurring job")?;
        
        info!("Scheduled recurring job {}", recurring_id);
        Ok(recurring_id)
    }
    
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);
        
        let queue = self.queue.clone();
        let redis_pool = self.redis_pool.clone();
        
        let handle = tokio::spawn(async move {
            info!("Job scheduler starting");
            
            let mut check_interval = interval(Duration::from_secs(5));
            
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Job scheduler shutting down");
                        break;
                    }
                    _ = check_interval.tick() => {
                        // Check for scheduled jobs that are due
                        match queue.get_scheduled_jobs_due().await {
                            Ok(job_ids) => {
                                for job_id in job_ids {
                                    match queue.get_job(&job_id).await {
                                        Ok(Some(mut job)) => {
                                            // Reset scheduled_at and enqueue the job
                                            job.scheduled_at = None;
                                            if let Err(e) = queue.enqueue(job).await {
                                                error!("Failed to enqueue scheduled job {}: {}", job_id, e);
                                            } else {
                                                info!("Enqueued scheduled job {}", job_id);
                                            }
                                        }
                                        Ok(None) => {
                                            warn!("Scheduled job {} not found", job_id);
                                        }
                                        Err(e) => {
                                            error!("Failed to get scheduled job {}: {}", job_id, e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to check scheduled jobs: {}", e);
                            }
                        }
                        
                        // Check for recurring jobs
                        if let Err(e) = Self::process_recurring_jobs(&redis_pool, &queue).await {
                            error!("Failed to process recurring jobs: {}", e);
                        }
                    }
                }
            }
            
            info!("Job scheduler stopped");
        });
        
        self.handle = Some(handle);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        if let Some(handle) = self.handle.take() {
            handle.await.context("Failed to join scheduler task")?;
        }
        
        Ok(())
    }
    
    async fn process_recurring_jobs(redis_pool: &Pool, queue: &JobQueue) -> Result<()> {
        let mut conn = redis_pool.get().await
            .context("Failed to get Redis connection")?;
        
        // Get all recurring job keys
        let pattern = "jobs:recurring:*";
        let keys: Vec<String> = deadpool_redis::redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .context("Failed to get recurring job keys")?;
        
        let now = Utc::now();
        
        for key in keys {
            let job_json: Option<String> = deadpool_redis::redis::cmd("GET")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .context("Failed to get recurring job")?;
            
            if let Some(json) = job_json {
                match serde_json::from_str::<RecurringJob>(&json) {
                    Ok(mut recurring_job) => {
                        if recurring_job.enabled && recurring_job.should_run_now(now) {
                            // Create a new job from the template
                            let mut job = recurring_job.template.clone();
                            job.id = nanoid::nanoid!();
                            job.created_at = now;
                            job.updated_at = now;
                            
                            // Enqueue the job
                            match queue.enqueue(job).await {
                                Ok(job_id) => {
                                    info!("Enqueued recurring job {} (instance {})", 
                                          recurring_job.id, job_id);
                                    
                                    // Update last run and next run times
                                    recurring_job.last_run = Some(now);
                                    recurring_job.next_run = recurring_job.schedule.next_run_after(now);
                                    
                                    let updated_json = serde_json::to_string(&recurring_job)
                                        .context("Failed to serialize recurring job")?;
                                    
                                    deadpool_redis::redis::cmd("SET")
                                        .arg(&key)
                                        .arg(&updated_json)
                                        .query_async::<_, ()>(&mut conn)
                                        .await
                                        .ok();
                                }
                                Err(e) => {
                                    error!("Failed to enqueue recurring job {}: {}", 
                                           recurring_job.id, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize recurring job from {}: {}", key, e);
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecurringJob {
    pub id: String,
    pub template: Job,
    pub schedule: CronSchedule,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub enabled: bool,
}

impl RecurringJob {
    pub fn should_run_now(&self, now: DateTime<Utc>) -> bool {
        match self.next_run {
            Some(next) => now >= next,
            None => false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CronSchedule {
    Interval(Duration),
    Daily { hour: u32, minute: u32 },
    Weekly { day: Weekday, hour: u32, minute: u32 },
    Monthly { day: u32, hour: u32, minute: u32 },
    Cron(String), // Standard cron expression
}

impl CronSchedule {
    pub fn next_run_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            CronSchedule::Interval(duration) => {
                let duration_chrono = chrono::Duration::from_std(*duration).ok()?;
                Some(after + duration_chrono)
            }
            CronSchedule::Daily { hour, minute } => {
                let mut next = after
                    .with_hour(*hour)?
                    .with_minute(*minute)?
                    .with_second(0)?
                    .with_nanosecond(0)?;
                
                if next <= after {
                    next = next + chrono::Duration::days(1);
                }
                
                Some(next)
            }
            CronSchedule::Weekly { day, hour, minute } => {
                let mut next = after
                    .with_hour(*hour)?
                    .with_minute(*minute)?
                    .with_second(0)?
                    .with_nanosecond(0)?;
                
                // Find the next occurrence of the specified weekday
                while next.weekday() != day.to_chrono() || next <= after {
                    next = next + chrono::Duration::days(1);
                }
                
                Some(next)
            }
            CronSchedule::Monthly { day, hour, minute } => {
                let mut next = after
                    .with_day(*day)?
                    .with_hour(*hour)?
                    .with_minute(*minute)?
                    .with_second(0)?
                    .with_nanosecond(0)?;
                
                if next <= after {
                    // Move to next month
                    let month = next.month();
                    let year = next.year();
                    
                    if month == 12 {
                        next = next
                            .with_year(year + 1)?
                            .with_month(1)?;
                    } else {
                        next = next.with_month(month + 1)?;
                    }
                }
                
                Some(next)
            }
            CronSchedule::Cron(_expr) => {
                // For now, we'll skip cron expression parsing
                // In production, you'd use a crate like `cron` or `crono`
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub fn to_chrono(&self) -> chrono::Weekday {
        match self {
            Weekday::Monday => chrono::Weekday::Mon,
            Weekday::Tuesday => chrono::Weekday::Tue,
            Weekday::Wednesday => chrono::Weekday::Wed,
            Weekday::Thursday => chrono::Weekday::Thu,
            Weekday::Friday => chrono::Weekday::Fri,
            Weekday::Saturday => chrono::Weekday::Sat,
            Weekday::Sunday => chrono::Weekday::Sun,
        }
    }
}