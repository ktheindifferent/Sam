use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::panic::{self, UnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender, Receiver};
use tracing::{error, warn, info, debug};
use prometheus::{IntGauge, IntCounter, register_int_gauge, register_int_counter};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

lazy_static! {
    static ref THREAD_MANAGER: Arc<RwLock<ThreadManager>> = Arc::new(RwLock::new(ThreadManager::new()));
    
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
        }
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
        let mut heartbeat = self.last_heartbeat.write().unwrap();
        *heartbeat = Utc::now();
    }
    
    fn is_healthy(&self) -> bool {
        if let Some(interval_ms) = self.config.health_check_interval_ms {
            let last_heartbeat = self.last_heartbeat.read().unwrap();
            let elapsed = Utc::now().signed_duration_since(*last_heartbeat);
            elapsed.num_milliseconds() < (interval_ms * 2) as i64
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
                
                let threads_map = threads.lock().unwrap();
                for (id, thread_arc) in threads_map.iter() {
                    let thread = thread_arc.lock().unwrap();
                    
                    if !thread.is_healthy() {
                        warn!("Thread {} is unhealthy", id);
                    }
                    
                    if thread.config.enable_monitoring {
                        debug!(
                            "Thread {}: status={:?}, restarts={}, panics={}",
                            id,
                            *thread.status.read().unwrap(),
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
            (Some(managed_thread.health_sender.as_ref().unwrap().clone()), Some(rx))
        } else {
            (None, None)
        };
        
        self.start_thread(&id, &mut managed_thread, f, health_receiver);
        
        self.threads.lock().unwrap().insert(
            id.clone(),
            Arc::new(Mutex::new(managed_thread))
        );
        
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
        
        *status.write().unwrap() = ThreadStatus::Running;
        
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
                        *status.write().unwrap() = ThreadStatus::Stopped;
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
                        
                        *last_error.write().unwrap() = Some(error_msg.clone());
                        *status.write().unwrap() = ThreadStatus::Panicked;
                        
                        error!("Thread {} panicked: {}", thread_id, error_msg);
                    }
                }
            })
            .expect("Failed to spawn thread");
        
        managed_thread.handle = Some(handle);
    }
    
    pub fn restart_thread(&mut self, id: &str) -> Result<(), String> {
        let threads_map = self.threads.lock().unwrap();
        
        if let Some(thread_arc) = threads_map.get(id) {
            let mut thread = thread_arc.lock().unwrap();
            
            let current_restarts = thread.restart_count.load(Ordering::Relaxed);
            if current_restarts >= thread.config.max_restarts {
                return Err(format!("Thread {} exceeded max restarts", id));
            }
            
            *thread.status.write().unwrap() = ThreadStatus::Restarting;
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
        let mut threads_map = self.threads.lock().unwrap();
        
        if let Some(thread_arc) = threads_map.remove(id) {
            let mut thread = thread_arc.lock().unwrap();
            
            *thread.status.write().unwrap() = ThreadStatus::Shutting;
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
        let threads_map = self.threads.lock().unwrap();
        
        threads_map.get(id).map(|thread_arc| {
            let thread = thread_arc.lock().unwrap();
            ThreadInfo {
                id: id.to_string(),
                name: thread.config.name.clone(),
                status: thread.status.read().unwrap().clone(),
                created_at: thread.created_at,
                last_heartbeat: *thread.last_heartbeat.read().unwrap(),
                restart_count: thread.restart_count.load(Ordering::Relaxed),
                panic_count: thread.panic_count.load(Ordering::Relaxed),
                last_error: thread.last_error.read().unwrap().clone(),
            }
        })
    }
    
    pub fn list_threads(&self) -> Vec<ThreadInfo> {
        let threads_map = self.threads.lock().unwrap();
        
        threads_map.iter().map(|(id, thread_arc)| {
            let thread = thread_arc.lock().unwrap();
            ThreadInfo {
                id: id.clone(),
                name: thread.config.name.clone(),
                status: thread.status.read().unwrap().clone(),
                created_at: thread.created_at,
                last_heartbeat: *thread.last_heartbeat.read().unwrap(),
                restart_count: thread.restart_count.load(Ordering::Relaxed),
                panic_count: thread.panic_count.load(Ordering::Relaxed),
                last_error: thread.last_error.read().unwrap().clone(),
            }
        }).collect()
    }
    
    pub fn shutdown_all(&mut self) {
        info!("Shutting down all managed threads");
        
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        let threads_map = self.threads.lock().unwrap().clone();
        for (id, thread_arc) in threads_map {
            let mut thread = thread_arc.lock().unwrap();
            thread.shutdown_signal.store(true, Ordering::Relaxed);
            
            if let Some(handle) = thread.handle.take() {
                let _ = handle.join();
            }
            
            info!("Shut down thread {}", id);
        }
        
        self.threads.lock().unwrap().clear();
        
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
    
    let mut manager = THREAD_MANAGER.write().unwrap();
    manager.spawn_managed(config, f)
}

pub fn spawn_with_config<F>(config: ThreadConfig, f: F) -> String
where
    F: FnOnce(Arc<AtomicBool>, Option<Receiver<()>>) + Send + 'static + Clone + UnwindSafe,
{
    let mut manager = THREAD_MANAGER.write().unwrap();
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

pub fn get_thread_info(id: &str) -> Option<ThreadInfo> {
    let manager = THREAD_MANAGER.read().unwrap();
    manager.get_thread_info(id)
}

pub fn list_threads() -> Vec<ThreadInfo> {
    let manager = THREAD_MANAGER.read().unwrap();
    manager.list_threads()
}

pub fn stop_thread(id: &str) -> Result<(), String> {
    let mut manager = THREAD_MANAGER.write().unwrap();
    manager.stop_thread(id)
}

pub fn restart_thread(id: &str) -> Result<(), String> {
    let mut manager = THREAD_MANAGER.write().unwrap();
    manager.restart_thread(id)
}

pub fn shutdown_all() {
    let mut manager = THREAD_MANAGER.write().unwrap();
    manager.shutdown_all();
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
        
        let thread_info = info.unwrap();
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