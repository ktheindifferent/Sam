use std::sync::{Arc, Mutex, RwLock, LockResult, PoisonError, Condvar};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use std::panic::{self, UnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use tracing::{error, warn, info, debug};
use prometheus::{IntGauge, IntCounter, Histogram, register_int_gauge, register_int_counter, register_histogram};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use thiserror::Error;
use crossbeam_channel::{bounded, unbounded, Select};
use parking_lot::FairMutex;

#[derive(Error, Debug)]
pub enum ThreadManagerError {
    #[error("Failed to acquire lock: {0}")]
    LockError(String),
    #[error("Thread not found: {0}")]
    ThreadNotFound(String),
    #[error("Thread operation failed: {0}")]
    OperationFailed(String),
    #[error("Thread pool exhausted: {0}")]
    PoolExhausted(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Queue full: {0}")]
    QueueFull(String),
}

lazy_static! {
    static ref THREAD_MANAGER: Arc<RwLock<ThreadManager>> = Arc::new(RwLock::new(ThreadManager::new()));
    
    static ref THREAD_POOL_MANAGER: Arc<ThreadPoolManager> = Arc::new(ThreadPoolManager::new(
        ThreadPoolConfig::default()
    ));
    
    static ref ACTIVE_THREADS: IntGauge = register_int_gauge!(
        "thread_manager_active_threads",
        "Number of currently active managed threads"
    ).unwrap();
    
    static ref TOTAL_THREADS_CREATED: IntCounter = register_int_counter!(
        "thread_manager_total_threads_created",
        "Total number of threads created"
    ).unwrap();
    
    static ref PANIC_COUNT: IntCounter = register_int_counter!(
        "thread_manager_panic_count",
        "Total number of thread panics"
    ).unwrap();
    
    static ref RESTART_COUNT: IntCounter = register_int_counter!(
        "thread_manager_restart_count",
        "Total number of thread restarts"
    ).unwrap();
    
    static ref QUEUED_TASKS: IntGauge = register_int_gauge!(
        "thread_manager_queued_tasks",
        "Number of tasks waiting in queue"
    ).unwrap();
    
    static ref REJECTED_TASKS: IntCounter = register_int_counter!(
        "thread_manager_rejected_tasks",
        "Total number of rejected tasks due to backpressure"
    ).unwrap();
    
    static ref TASK_LATENCY: Histogram = register_histogram!(
        "thread_manager_task_latency_seconds",
        "Task execution latency in seconds"
    ).unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub id: String,
    pub name: String,
    pub status: ThreadStatus,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub restart_count: usize,
    pub panic_count: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreadStatus {
    Running,
    Stopped,
    Panicked,
    Restarting,
    Shutting,
}

#[derive(Debug, Clone)]
pub struct ThreadConfig {
    pub name: String,
    pub restart_on_panic: bool,
    pub max_restarts: usize,
    pub restart_delay_ms: u64,
    pub health_check_interval_ms: Option<u64>,
    pub enable_monitoring: bool,
    pub priority: ThreadPriority,
    pub max_memory_mb: Option<usize>,
    pub cpu_affinity: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        ThreadConfig {
            name: "unnamed".to_string(),
            restart_on_panic: true,
            max_restarts: 3,
            restart_delay_ms: 1000,
            health_check_interval_ms: Some(5000),
            enable_monitoring: true,
            priority: ThreadPriority::Normal,
            max_memory_mb: None,
            cpu_affinity: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    pub max_threads: usize,
    pub core_threads: usize,
    pub queue_size: usize,
    pub keep_alive_ms: u64,
    pub enable_backpressure: bool,
    pub backpressure_threshold: f64,
    pub enable_auto_scaling: bool,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub monitoring_interval_ms: u64,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        ThreadPoolConfig {
            max_threads: num_cpus * 4,
            core_threads: num_cpus,
            queue_size: 1000,
            keep_alive_ms: 60000,
            enable_backpressure: true,
            backpressure_threshold: 0.8,
            enable_auto_scaling: true,
            scale_up_threshold: 0.75,
            scale_down_threshold: 0.25,
            monitoring_interval_ms: 5000,
        }
    }
}

pub struct ThreadPoolManager {
    config: ThreadPoolConfig,
    worker_threads: Arc<Mutex<Vec<WorkerThread>>>,
    task_queue: Arc<(Mutex<VecDeque<Task>>, Condvar)>,
    active_count: Arc<AtomicUsize>,
    total_threads: Arc<AtomicUsize>,
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<ThreadPoolMetrics>,
    monitor_handle: Option<JoinHandle<()>>,
}

struct WorkerThread {
    id: String,
    handle: Option<JoinHandle<()>>,
    status: WorkerStatus,
    created_at: Instant,
    last_active: Instant,
    tasks_completed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WorkerStatus {
    Idle,
    Running,
    Terminating,
}

struct Task {
    id: String,
    priority: ThreadPriority,
    func: Box<dyn FnOnce() + Send + 'static>,
    created_at: Instant,
    max_duration: Option<Duration>,
}

struct ThreadPoolMetrics {
    tasks_submitted: AtomicUsize,
    tasks_completed: AtomicUsize,
    tasks_failed: AtomicUsize,
    tasks_rejected: AtomicUsize,
    avg_wait_time_ms: AtomicUsize,
    avg_execution_time_ms: AtomicUsize,
}

impl ThreadPoolManager {
    pub fn new(config: ThreadPoolConfig) -> Self {
        let manager = ThreadPoolManager {
            config: config.clone(),
            worker_threads: Arc::new(Mutex::new(Vec::with_capacity(config.max_threads))),
            task_queue: Arc::new((Mutex::new(VecDeque::with_capacity(config.queue_size)), Condvar::new())),
            active_count: Arc::new(AtomicUsize::new(0)),
            total_threads: Arc::new(AtomicUsize::new(0)),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(ThreadPoolMetrics {
                tasks_submitted: AtomicUsize::new(0),
                tasks_completed: AtomicUsize::new(0),
                tasks_failed: AtomicUsize::new(0),
                tasks_rejected: AtomicUsize::new(0),
                avg_wait_time_ms: AtomicUsize::new(0),
                avg_execution_time_ms: AtomicUsize::new(0),
            }),
            monitor_handle: None,
        };
        
        manager.initialize_core_threads();
        manager.start_monitor();
        manager
    }
    
    fn initialize_core_threads(&self) {
        for i in 0..self.config.core_threads {
            self.spawn_worker(format!("core-worker-{}", i));
        }
        info!("Initialized {} core worker threads", self.config.core_threads);
    }
    
    fn spawn_worker(&self, name: String) -> Result<(), ThreadManagerError> {
        let current_count = self.total_threads.load(Ordering::Relaxed);
        if current_count >= self.config.max_threads {
            return Err(ThreadManagerError::PoolExhausted(
                format!("Maximum thread limit {} reached", self.config.max_threads)
            ));
        }
        
        let worker_id = format!("{}_{}", name, nanoid::nanoid!());
        let task_queue = self.task_queue.clone();
        let active_count = self.active_count.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let metrics = self.metrics.clone();
        let keep_alive = Duration::from_millis(self.config.keep_alive_ms);
        
        let handle = thread::Builder::new()
            .name(worker_id.clone())
            .spawn(move || {
                info!("Worker thread {} started", worker_id);
                
                loop {
                    let (queue_lock, condvar) = &*task_queue;
                    let result = condvar.wait_timeout_while(
                        queue_lock.lock().unwrap(),
                        keep_alive,
                        |queue| queue.is_empty() && !shutdown_signal.load(Ordering::Relaxed)
                    );
                    
                    if shutdown_signal.load(Ordering::Relaxed) {
                        break;
                    }
                    
                    match result {
                        Ok((mut queue, timeout_result)) if !timeout_result.timed_out() => {
                            if let Some(task) = queue.pop_front() {
                                drop(queue);
                                
                                active_count.fetch_add(1, Ordering::Relaxed);
                                let wait_time = task.created_at.elapsed();
                                let exec_start = Instant::now();
                                
                                let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                                    (task.func)();
                                }));
                                
                                let exec_time = exec_start.elapsed();
                                active_count.fetch_sub(1, Ordering::Relaxed);
                                
                                match result {
                                    Ok(_) => {
                                        metrics.tasks_completed.fetch_add(1, Ordering::Relaxed);
                                        debug!("Task {} completed in {:?}", task.id, exec_time);
                                    }
                                    Err(e) => {
                                        metrics.tasks_failed.fetch_add(1, Ordering::Relaxed);
                                        error!("Task {} panicked: {:?}", task.id, e);
                                    }
                                }
                                
                                metrics.avg_wait_time_ms.store(
                                    wait_time.as_millis() as usize,
                                    Ordering::Relaxed
                                );
                                metrics.avg_execution_time_ms.store(
                                    exec_time.as_millis() as usize,
                                    Ordering::Relaxed
                                );
                            }
                        }
                        _ => {
                            break;
                        }
                    }
                }
                
                info!("Worker thread {} terminated", worker_id);
            })
            .map_err(|e| ThreadManagerError::OperationFailed(format!("Failed to spawn worker: {}", e)))?;
        
        let mut workers = self.worker_threads.lock().unwrap();
        workers.push(WorkerThread {
            id: worker_id,
            handle: Some(handle),
            status: WorkerStatus::Running,
            created_at: Instant::now(),
            last_active: Instant::now(),
            tasks_completed: 0,
        });
        
        self.total_threads.fetch_add(1, Ordering::Relaxed);
        TOTAL_THREADS_CREATED.inc();
        
        Ok(())
    }
    
    pub fn submit<F>(&self, name: String, priority: ThreadPriority, f: F) -> Result<String, ThreadManagerError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.should_apply_backpressure() {
            REJECTED_TASKS.inc();
            return Err(ThreadManagerError::QueueFull(
                "Task queue is full, applying backpressure".to_string()
            ));
        }
        
        let task_id = format!("{}_{}", name, nanoid::nanoid!());
        let task = Task {
            id: task_id.clone(),
            priority,
            func: Box::new(f),
            created_at: Instant::now(),
            max_duration: None,
        };
        
        let (queue_lock, condvar) = &*self.task_queue;
        let mut queue = queue_lock.lock().unwrap();
        
        if queue.len() >= self.config.queue_size {
            REJECTED_TASKS.inc();
            return Err(ThreadManagerError::QueueFull(
                format!("Queue size limit {} exceeded", self.config.queue_size)
            ));
        }
        
        let insert_pos = queue.iter().position(|t| t.priority < priority).unwrap_or(queue.len());
        queue.insert(insert_pos, task);
        
        QUEUED_TASKS.set(queue.len() as i64);
        self.metrics.tasks_submitted.fetch_add(1, Ordering::Relaxed);
        
        condvar.notify_one();
        
        if self.config.enable_auto_scaling {
            self.check_scaling();
        }
        
        Ok(task_id)
    }
    
    fn should_apply_backpressure(&self) -> bool {
        if !self.config.enable_backpressure {
            return false;
        }
        
        let (queue_lock, _) = &*self.task_queue;
        let queue = queue_lock.lock().unwrap();
        let queue_utilization = queue.len() as f64 / self.config.queue_size as f64;
        
        queue_utilization > self.config.backpressure_threshold
    }
    
    fn check_scaling(&self) {
        let active = self.active_count.load(Ordering::Relaxed);
        let total = self.total_threads.load(Ordering::Relaxed);
        let utilization = active as f64 / total.max(1) as f64;
        
        if utilization > self.config.scale_up_threshold && total < self.config.max_threads {
            if let Err(e) = self.spawn_worker(format!("scaled-worker-{}", total)) {
                warn!("Failed to scale up: {}", e);
            } else {
                info!("Scaled up to {} threads", total + 1);
            }
        } else if utilization < self.config.scale_down_threshold && total > self.config.core_threads {
            self.scale_down();
        }
    }
    
    fn scale_down(&self) {
        let mut workers = self.worker_threads.lock().unwrap();
        if let Some(pos) = workers.iter().position(|w| w.status == WorkerStatus::Idle) {
            if let Some(mut worker) = workers.get_mut(pos) {
                worker.status = WorkerStatus::Terminating;
                self.total_threads.fetch_sub(1, Ordering::Relaxed);
                info!("Scaled down, {} threads remaining", self.total_threads.load(Ordering::Relaxed));
            }
        }
    }
    
    fn start_monitor(&self) {
        let workers = self.worker_threads.clone();
        let metrics = self.metrics.clone();
        let shutdown = self.shutdown_signal.clone();
        let interval = Duration::from_millis(self.config.monitoring_interval_ms);
        
        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(interval);
                
                let workers_guard = workers.lock().unwrap();
                let active_workers = workers_guard.iter().filter(|w| w.status == WorkerStatus::Running).count();
                let idle_workers = workers_guard.iter().filter(|w| w.status == WorkerStatus::Idle).count();
                
                info!(
                    "ThreadPool stats: active={}, idle={}, submitted={}, completed={}, failed={}, rejected={}",
                    active_workers,
                    idle_workers,
                    metrics.tasks_submitted.load(Ordering::Relaxed),
                    metrics.tasks_completed.load(Ordering::Relaxed),
                    metrics.tasks_failed.load(Ordering::Relaxed),
                    metrics.tasks_rejected.load(Ordering::Relaxed)
                );
            }
        });
        
        unsafe {
            let self_mut = &mut *(self as *const ThreadPoolManager as *mut ThreadPoolManager);
            self_mut.monitor_handle = Some(handle);
        }
    }
    
    pub fn shutdown(&self) {
        info!("Shutting down thread pool");
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        let (_, condvar) = &*self.task_queue;
        condvar.notify_all();
        
        let mut workers = self.worker_threads.lock().unwrap();
        for worker in workers.iter_mut() {
            if let Some(handle) = worker.handle.take() {
                let _ = handle.join();
            }
        }
        workers.clear();
        
        info!("Thread pool shut down complete");
    }
}

pub struct ManagedThread {
    config: ThreadConfig,
    handle: Option<JoinHandle<()>>,
    status: Arc<RwLock<ThreadStatus>>,
    restart_count: Arc<AtomicUsize>,
    panic_count: Arc<AtomicUsize>,
    last_error: Arc<RwLock<Option<String>>>,
    created_at: DateTime<Utc>,
    last_heartbeat: Arc<RwLock<DateTime<Utc>>>,
    shutdown_signal: Arc<AtomicBool>,
    health_sender: Option<Sender<()>>,
}

impl ManagedThread {
    fn new(config: ThreadConfig) -> Self {
        ManagedThread {
            config,
            handle: None,
            status: Arc::new(RwLock::new(ThreadStatus::Stopped)),
            restart_count: Arc::new(AtomicUsize::new(0)),
            panic_count: Arc::new(AtomicUsize::new(0)),
            last_error: Arc::new(RwLock::new(None)),
            created_at: Utc::now(),
            last_heartbeat: Arc::new(RwLock::new(Utc::now())),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            health_sender: None,
        }
    }
    
    fn update_heartbeat(&self) {
        match self.last_heartbeat.write() {
            Ok(mut heartbeat) => *heartbeat = Utc::now(),
            Err(e) => error!("Failed to update heartbeat: {}", e),
        }
    }
    
    fn is_healthy(&self) -> bool {
        if let Some(interval_ms) = self.config.health_check_interval_ms {
            match self.last_heartbeat.read() {
                Ok(last_heartbeat) => {
                    let elapsed = Utc::now().signed_duration_since(*last_heartbeat);
                    elapsed.num_milliseconds() < (interval_ms * 2) as i64
                }
                Err(e) => {
                    error!("Failed to read heartbeat: {}", e);
                    false
                }
            }
        } else {
            true
        }
    }
}

pub struct ThreadManager {
    threads: Arc<Mutex<HashMap<String, Arc<Mutex<ManagedThread>>>>>,
    shutdown_signal: Arc<AtomicBool>,
    monitor_handle: Option<JoinHandle<()>>,
}

impl ThreadManager {
    fn new() -> Self {
        let manager = ThreadManager {
            threads: Arc::new(Mutex::new(HashMap::new())),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            monitor_handle: None,
        };
        
        manager.start_monitor();
        manager
    }
    
    fn start_monitor(&self) {
        let threads = self.threads.clone();
        let shutdown = self.shutdown_signal.clone();
        
        let handle = thread::spawn(move || {
            info!("Thread monitor started");
            
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(5));
                
                let threads_map = match threads.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        error!("Monitor failed to acquire threads lock: {}", e);
                        continue;
                    }
                };
                for (id, thread_arc) in threads_map.iter() {
                    let thread = match thread_arc.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            error!("Monitor failed to acquire thread lock for {}: {}", id, e);
                            continue;
                        }
                    };
                    
                    if !thread.is_healthy() {
                        warn!("Thread {} is unhealthy", id);
                    }
                    
                    if thread.config.enable_monitoring {
                        debug!(
                            "Thread {}: status={:?}, restarts={}, panics={}",
                            id,
                            thread.status.read().map(|s| format!("{:?}", *s)).unwrap_or_else(|_| "unknown".to_string()),
                            thread.restart_count.load(Ordering::Relaxed),
                            thread.panic_count.load(Ordering::Relaxed)
                        );
                    }
                }
                
                ACTIVE_THREADS.set(threads_map.len() as i64);
            }
            
            info!("Thread monitor stopped");
        });
        
        let mut self_mut = unsafe { 
            &mut *(self as *const ThreadManager as *mut ThreadManager)
        };
        self_mut.monitor_handle = Some(handle);
    }
    
    pub fn get_instance() -> Arc<RwLock<ThreadManager>> {
        THREAD_MANAGER.clone()
    }
    
    pub fn spawn_managed<F>(&mut self, config: ThreadConfig, f: F) -> String
    where
        F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
    {
        let thread_id = format!("{}_{}", config.name, nanoid::nanoid!());
        self.spawn_managed_with_id(thread_id.clone(), config, f);
        thread_id
    }
    
    pub fn spawn_managed_with_id<F>(&mut self, id: String, config: ThreadConfig, f: F)
    where
        F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
    {
        let mut managed_thread = ManagedThread::new(config.clone());
        let (health_sender, health_receiver) = if config.health_check_interval_ms.is_some() {
            let (tx, rx) = mpsc::channel();
            managed_thread.health_sender = Some(tx);
            (managed_thread.health_sender.clone(), Some(rx))
        } else {
            (None, None)
        };
        
        self.start_thread(&id, &mut managed_thread, f, health_receiver);
        
        if let Err(e) = self.threads.lock().map(|mut guard| guard.insert(
            id.clone(),
            Arc::new(Mutex::new(managed_thread))
        )) {
            error!("Failed to insert thread {}: {}", id, e);
        }
        
        TOTAL_THREADS_CREATED.inc();
        info!("Started managed thread: {}", id);
    }
    
    fn start_thread<F>(
        &self,
        id: &str,
        managed_thread: &mut ManagedThread,
        f: F,
        health_receiver: Option<Receiver<()>>,
    )
    where
        F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
    {
        let status = managed_thread.status.clone();
        let shutdown_signal = managed_thread.shutdown_signal.clone();
        let last_error = managed_thread.last_error.clone();
        let panic_count = managed_thread.panic_count.clone();
        let last_heartbeat = managed_thread.last_heartbeat.clone();
        let thread_name = managed_thread.config.name.clone();
        let thread_id = id.to_string();
        let health_sender = managed_thread.health_sender.clone();
        
        match status.write() {
            Ok(mut s) => *s = ThreadStatus::Running,
            Err(e) => error!("Failed to update thread status: {}", e),
        }
        
        let handle = thread::Builder::new()
            .name(thread_name.clone())
            .spawn(move || {
                let prev_hook = panic::take_hook();
                panic::set_hook(Box::new(move |panic_info| {
                    error!("Thread {} panicked: {:?}", thread_name, panic_info);
                    prev_hook(panic_info);
                }));
                
                let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    info!("Thread {} started", thread_id);
                    
                    if let Some(sender) = health_sender.as_ref() {
                        thread::spawn(move || {
                            loop {
                                thread::sleep(Duration::from_millis(1000));
                                if sender.send(()).is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    
                    f(shutdown_signal.clone(), health_receiver);
                    
                    info!("Thread {} completed normally", thread_id);
                }));
                
                match result {
                    Ok(_) => {
                        match status.write() {
                            Ok(mut s) => *s = ThreadStatus::Stopped,
                            Err(e) => error!("Failed to update status to stopped: {}", e),
                        }
                    }
                    Err(e) => {
                        panic_count.fetch_add(1, Ordering::Relaxed);
                        PANIC_COUNT.inc();
                        
                        let error_msg = if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else if let Some(s) = e.downcast_ref::<&str>() {
                            s.to_string()
                        } else {
                            "Unknown panic".to_string()
                        };
                        
                        match last_error.write() {
                            Ok(mut e) => *e = Some(error_msg.clone()),
                            Err(e) => error!("Failed to update last error: {}", e),
                        }
                        match status.write() {
                            Ok(mut s) => *s = ThreadStatus::Panicked,
                            Err(e) => error!("Failed to update status to panicked: {}", e),
                        }
                        
                        error!("Thread {} panicked: {}", thread_id, error_msg);
                    }
                }
            })
            .expect("Failed to spawn thread");
        
        managed_thread.handle = Some(handle);
    }
    
    pub fn restart_thread(&mut self, id: &str) -> Result<(), String> {
        let threads_map = self.threads.lock()
            .map_err(|e| format!("Failed to acquire threads lock: {}", e))?;
        
        if let Some(thread_arc) = threads_map.get(id) {
            let mut thread = thread_arc.lock()
                .map_err(|e| format!("Failed to acquire thread lock: {}", e))?;
            
            let current_restarts = thread.restart_count.load(Ordering::Relaxed);
            if current_restarts >= thread.config.max_restarts {
                return Err(format!("Thread {} exceeded max restarts", id));
            }
            
            thread.status.write()
                .map_err(|e| format!("Failed to update status: {}", e))
                .map(|mut s| *s = ThreadStatus::Restarting)?;
            thread.shutdown_signal.store(true, Ordering::Relaxed);
            
            if let Some(handle) = thread.handle.take() {
                let _ = handle.join();
            }
            
            thread::sleep(Duration::from_millis(thread.config.restart_delay_ms));
            
            thread.restart_count.fetch_add(1, Ordering::Relaxed);
            RESTART_COUNT.inc();
            
            thread.shutdown_signal.store(false, Ordering::Relaxed);
            
            info!("Restarting thread {}", id);
            Ok(())
        } else {
            Err(format!("Thread {} not found", id))
        }
    }
    
    pub fn stop_thread(&mut self, id: &str) -> Result<(), String> {
        let mut threads_map = self.threads.lock()
            .map_err(|e| format!("Failed to acquire threads lock: {}", e))?;
        
        if let Some(thread_arc) = threads_map.remove(id) {
            let mut thread = thread_arc.lock()
                .map_err(|e| format!("Failed to acquire thread lock: {}", e))?;
            
            thread.status.write()
                .map_err(|e| format!("Failed to update status: {}", e))
                .map(|mut s| *s = ThreadStatus::Shutting)?;
            thread.shutdown_signal.store(true, Ordering::Relaxed);
            
            if let Some(handle) = thread.handle.take() {
                let _ = handle.join();
            }
            
            info!("Stopped thread {}", id);
            Ok(())
        } else {
            Err(format!("Thread {} not found", id))
        }
    }
    
    pub fn get_thread_info(&self, id: &str) -> Option<ThreadInfo> {
        let threads_map = match self.threads.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire threads lock: {}", e);
                return None;
            }
        };
        
        threads_map.get(id).and_then(|thread_arc| {
            match thread_arc.lock() {
                Ok(thread) => {
                    let status = thread.status.read()
                        .map(|s| s.clone())
                        .unwrap_or(ThreadStatus::Stopped);
                    let last_heartbeat = thread.last_heartbeat.read()
                        .map(|h| *h)
                        .unwrap_or_else(|_| Utc::now());
                    let last_error = thread.last_error.read()
                        .map(|e| e.clone())
                        .unwrap_or(None);
                    
                    Some(ThreadInfo {
                        id: id.to_string(),
                        name: thread.config.name.clone(),
                        status,
                        created_at: thread.created_at,
                        last_heartbeat,
                        restart_count: thread.restart_count.load(Ordering::Relaxed),
                        panic_count: thread.panic_count.load(Ordering::Relaxed),
                        last_error,
                    })
                }
                Err(e) => {
                    error!("Failed to acquire thread lock for {}: {}", id, e);
                    None
                }
            }
        })
    }
    
    pub fn list_threads(&self) -> Vec<ThreadInfo> {
        let threads_map = match self.threads.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire threads lock: {}", e);
                return Vec::new();
            }
        };
        
        threads_map.iter().filter_map(|(id, thread_arc)| {
            match thread_arc.lock() {
                Ok(thread) => {
                    let status = thread.status.read()
                        .map(|s| s.clone())
                        .unwrap_or(ThreadStatus::Stopped);
                    let last_heartbeat = thread.last_heartbeat.read()
                        .map(|h| *h)
                        .unwrap_or_else(|_| Utc::now());
                    let last_error = thread.last_error.read()
                        .map(|e| e.clone())
                        .unwrap_or(None);
                    
                    Some(ThreadInfo {
                        id: id.clone(),
                        name: thread.config.name.clone(),
                        status,
                        created_at: thread.created_at,
                        last_heartbeat,
                        restart_count: thread.restart_count.load(Ordering::Relaxed),
                        panic_count: thread.panic_count.load(Ordering::Relaxed),
                        last_error,
                    })
                }
                Err(e) => {
                    error!("Failed to acquire thread lock for {}: {}", id, e);
                    None
                }
            }
        }).collect()
    }
    
    pub fn shutdown_all(&mut self) {
        info!("Shutting down all managed threads");
        
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        let threads_map = match self.threads.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire threads lock for shutdown: {}", e);
                return;
            }
        };
        for (id, thread_arc) in threads_map {
            let mut thread = match thread_arc.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("Failed to acquire thread lock for {}: {}", id, e);
                    continue;
                }
            };
            thread.shutdown_signal.store(true, Ordering::Relaxed);
            
            if let Some(handle) = thread.handle.take() {
                let _ = handle.join();
            }
            
            info!("Shut down thread {}", id);
        }
        
        match self.threads.lock() {
            Ok(mut guard) => guard.clear(),
            Err(e) => error!("Failed to clear threads map: {}", e),
        }
        
        if let Some(handle) = self.monitor_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn spawn<F>(name: &str, f: F) -> String
where
    F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
{
    let config = ThreadConfig {
        name: name.to_string(),
        ..Default::default()
    };
    
    let mut manager = match THREAD_MANAGER.write() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire manager write lock: {}", e);
            return format!("error_{}", nanoid::nanoid!());
        }
    };
    manager.spawn_managed(config, f)
}

pub fn spawn_with_config<F>(config: ThreadConfig, f: F) -> String
where
    F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
{
    let mut manager = match THREAD_MANAGER.write() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire manager write lock: {}", e);
            return format!("error_{}", nanoid::nanoid!());
        }
    };
    manager.spawn_managed(config, f)
}

pub fn spawn_loop<F>(name: &str, mut f: F) -> String
where
    F: FnMut() -> bool + Send + 'static + Clone + UnwindSafe,
{
    spawn(name, move |shutdown_signal, health_rx| {
        while !shutdown_signal.load(Ordering::Relaxed) {
            if !f() {
                break;
            }
            
            if let Some(rx) = &health_rx {
                let _ = rx.try_recv();
            }
        }
    })
}

pub fn spawn_interval<F>(name: &str, interval: Duration, mut f: F) -> String
where
    F: FnMut() + Send + 'static + Clone + UnwindSafe,
{
    spawn(name, move |shutdown_signal, health_rx| {
        while !shutdown_signal.load(Ordering::Relaxed) {
            f();
            
            let start = Instant::now();
            while start.elapsed() < interval {
                if shutdown_signal.load(Ordering::Relaxed) {
                    break;
                }
                
                if let Some(rx) = &health_rx {
                    let _ = rx.try_recv();
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        }
    })
}

pub fn submit_task<F>(name: &str, f: F) -> Result<String, ThreadManagerError>
where
    F: FnOnce() + Send + 'static,
{
    THREAD_POOL_MANAGER.submit(name.to_string(), ThreadPriority::Normal, f)
}

pub fn submit_task_with_priority<F>(name: &str, priority: ThreadPriority, f: F) -> Result<String, ThreadManagerError>
where
    F: FnOnce() + Send + 'static,
{
    THREAD_POOL_MANAGER.submit(name.to_string(), priority, f)
}

pub fn spawn_pooled<F>(name: &str, f: F) -> Result<String, ThreadManagerError>
where
    F: FnOnce() + Send + 'static,
{
    submit_task(name, f)
}

pub fn spawn_pooled_with_config<F>(config: ThreadConfig, f: F) -> Result<String, ThreadManagerError>
where
    F: FnOnce() + Send + 'static,
{
    THREAD_POOL_MANAGER.submit(config.name, config.priority, f)
}

pub fn get_pool_stats() -> ThreadPoolStats {
    let workers = THREAD_POOL_MANAGER.worker_threads.lock().unwrap();
    let active_workers = workers.iter().filter(|w| w.status == WorkerStatus::Running).count();
    let idle_workers = workers.iter().filter(|w| w.status == WorkerStatus::Idle).count();
    
    let (queue_lock, _) = &*THREAD_POOL_MANAGER.task_queue;
    let queue = queue_lock.lock().unwrap();
    let queued_tasks = queue.len();
    
    ThreadPoolStats {
        active_threads: active_workers,
        idle_threads: idle_workers,
        total_threads: THREAD_POOL_MANAGER.total_threads.load(Ordering::Relaxed),
        queued_tasks,
        tasks_submitted: THREAD_POOL_MANAGER.metrics.tasks_submitted.load(Ordering::Relaxed),
        tasks_completed: THREAD_POOL_MANAGER.metrics.tasks_completed.load(Ordering::Relaxed),
        tasks_failed: THREAD_POOL_MANAGER.metrics.tasks_failed.load(Ordering::Relaxed),
        tasks_rejected: THREAD_POOL_MANAGER.metrics.tasks_rejected.load(Ordering::Relaxed),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolStats {
    pub active_threads: usize,
    pub idle_threads: usize,
    pub total_threads: usize,
    pub queued_tasks: usize,
    pub tasks_submitted: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_rejected: usize,
}

pub fn configure_thread_pool(config: ThreadPoolConfig) -> Result<(), ThreadManagerError> {
    warn!("Thread pool reconfiguration is not supported after initialization");
    Ok(())
}

pub fn shutdown_thread_pool() {
    THREAD_POOL_MANAGER.shutdown();
}

pub fn get_thread_info(id: &str) -> Option<ThreadInfo> {
    match THREAD_MANAGER.read() {
        Ok(manager) => manager.get_thread_info(id),
        Err(e) => {
            error!("Failed to acquire manager read lock: {}", e);
            None
        }
    }
}

pub fn list_threads() -> Vec<ThreadInfo> {
    match THREAD_MANAGER.read() {
        Ok(manager) => manager.list_threads(),
        Err(e) => {
            error!("Failed to acquire manager read lock: {}", e);
            Vec::new()
        }
    }
}

pub fn stop_thread(id: &str) -> Result<(), String> {
    let mut manager = THREAD_MANAGER.write()
        .map_err(|e| format!("Failed to acquire manager write lock: {}", e))?;
    manager.stop_thread(id)
}

pub fn restart_thread(id: &str) -> Result<(), String> {
    let mut manager = THREAD_MANAGER.write()
        .map_err(|e| format!("Failed to acquire manager write lock: {}", e))?;
    manager.restart_thread(id)
}

pub fn shutdown_all() {
    match THREAD_MANAGER.write() {
        Ok(mut manager) => manager.shutdown_all(),
        Err(e) => error!("Failed to acquire manager write lock for shutdown: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    #[test]
    fn test_spawn_and_stop() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let thread_id = spawn("test_thread", move |shutdown, _| {
            while !shutdown.load(Ordering::Relaxed) {
                counter_clone.fetch_add(1, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(10));
            }
        });
        
        thread::sleep(Duration::from_millis(100));
        
        let result = stop_thread(&thread_id);
        assert!(result.is_ok());
        
        assert!(counter.load(Ordering::Relaxed) > 0);
    }
    
    #[test]
    fn test_panic_recovery() {
        let panic_thread_id = spawn("panic_thread", |_, _| {
            panic!("Test panic!");
        });
        
        thread::sleep(Duration::from_millis(100));
        
        let info = get_thread_info(&panic_thread_id);
        assert!(info.is_some());
        
        let thread_info = info.expect("Thread info should be available");
        assert_eq!(thread_info.status, ThreadStatus::Panicked);
        assert_eq!(thread_info.panic_count, 1);
    }
    
    #[test]
    fn test_list_threads() {
        let _id1 = spawn("thread1", |shutdown, _| {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
        });
        
        let _id2 = spawn("thread2", |shutdown, _| {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
        });
        
        let threads = list_threads();
        assert!(threads.len() >= 2);
        
        shutdown_all();
    }
}