# Thread Pool Management System Implementation

## Overview
A comprehensive thread pool management system has been implemented to replace unmanaged `thread::spawn()` calls throughout the codebase. This system prevents thread exhaustion, manages resources efficiently, and provides automatic recovery from panics.

## Key Components

### 1. ThreadPoolManager
The core component that manages a pool of worker threads with the following features:
- **Dynamic scaling**: Automatically scales threads between `core_threads` and `max_threads` based on load
- **Priority queue**: Tasks are executed based on priority (Critical > High > Normal > Low)
- **Backpressure**: Prevents system overload by rejecting tasks when queue is full
- **Panic recovery**: Worker threads automatically recover from panics
- **Resource limits**: Configurable memory and CPU affinity per thread

### 2. ThreadManager
Manages individual long-running threads with:
- **Health monitoring**: Tracks thread health via heartbeats
- **Automatic restart**: Restarts threads on panic (configurable)
- **Lifecycle tracking**: Monitors thread status (Running, Stopped, Panicked, Restarting)
- **Graceful shutdown**: Proper cleanup when threads are stopped

## Configuration

### Thread Pool Configuration
```rust
pub struct ThreadPoolConfig {
    pub max_threads: usize,        // Maximum number of threads (default: CPUs * 4)
    pub core_threads: usize,        // Core threads always running (default: CPUs)
    pub queue_size: usize,          // Task queue size (default: 1000)
    pub keep_alive_ms: u64,         // Thread keep-alive time (default: 60000)
    pub enable_backpressure: bool,  // Enable backpressure (default: true)
    pub backpressure_threshold: f64,// Queue threshold for backpressure (default: 0.8)
    pub enable_auto_scaling: bool,  // Enable auto-scaling (default: true)
    pub scale_up_threshold: f64,    // CPU utilization to scale up (default: 0.75)
    pub scale_down_threshold: f64,  // CPU utilization to scale down (default: 0.25)
    pub monitoring_interval_ms: u64,// Monitoring interval (default: 5000)
}
```

### Thread Configuration
```rust
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
```

## Usage Examples

### Using the Thread Pool for Short Tasks
```rust
use sam::services::thread_manager::{submit_task, submit_task_with_priority, ThreadPriority};

// Submit a normal priority task
let task_id = submit_task("data_processing", || {
    // Process data
    println!("Processing data...");
})?;

// Submit a high priority task
let critical_task_id = submit_task_with_priority(
    "critical_operation",
    ThreadPriority::Critical,
    || {
        // Critical operation
        println!("Executing critical operation...");
    }
)?;
```

### Managing Long-Running Threads
```rust
use sam::services::thread_manager::{spawn, spawn_with_config, ThreadConfig};

// Simple thread spawn with defaults
let thread_id = spawn("background_worker", |shutdown_signal, health_rx| {
    while !shutdown_signal.load(Ordering::Relaxed) {
        // Do work
        thread::sleep(Duration::from_secs(1));
        
        // Check health
        if let Some(rx) = &health_rx {
            let _ = rx.try_recv();
        }
    }
});

// Spawn with custom configuration
let config = ThreadConfig {
    name: "critical_service".to_string(),
    restart_on_panic: true,
    max_restarts: 5,
    priority: ThreadPriority::High,
    ..Default::default()
};

let thread_id = spawn_with_config(config, |shutdown, _| {
    // Thread logic
});
```

## Migrated Services

The following services have been migrated to use the thread pool:

1. **lifx.rs**: LIFX lighting service
   - Uses managed threads with automatic restart
   - Graceful shutdown support

2. **spotify.rs**: Spotify music service
   - Already using thread_manager
   - Health monitoring enabled

3. **rtsp.rs**: RTSP streaming service
   - Deep learning processor threads use high priority
   - Recording threads use normal priority

4. **sprec.rs**: Speech recognition service
   - Builder thread with monitoring

5. **config/mod.rs**: HTTP server
   - Main HTTP server thread

6. **settings.rs**: Settings management
   - Default settings initialization thread

## Monitoring and Metrics

The system provides comprehensive metrics via Prometheus:

- `thread_manager_active_threads`: Number of active threads
- `thread_manager_total_threads_created`: Total threads created
- `thread_manager_panic_count`: Thread panic count
- `thread_manager_restart_count`: Thread restart count
- `thread_manager_queued_tasks`: Tasks waiting in queue
- `thread_manager_rejected_tasks`: Tasks rejected due to backpressure
- `thread_manager_task_latency_seconds`: Task execution latency

## Thread Pool Statistics
```rust
use sam::services::thread_manager::get_pool_stats;

let stats = get_pool_stats();
println!("Active threads: {}", stats.active_threads);
println!("Queued tasks: {}", stats.queued_tasks);
println!("Tasks completed: {}", stats.tasks_completed);
println!("Tasks rejected: {}", stats.tasks_rejected);
```

## Benefits

1. **Resource Management**
   - Prevents thread exhaustion
   - Controlled resource usage
   - Memory and CPU limits

2. **Reliability**
   - Automatic panic recovery
   - Health monitoring
   - Graceful degradation

3. **Performance**
   - Task prioritization
   - Dynamic scaling
   - Efficient queue management

4. **Observability**
   - Comprehensive metrics
   - Thread lifecycle tracking
   - Performance monitoring

## Testing

Comprehensive stress tests have been added to verify:
- Thread pool exhaustion handling
- Panic recovery
- Auto-scaling behavior
- Priority queue ordering
- Concurrent access safety
- Memory pressure handling
- Long-running task management

Run tests with:
```bash
cargo test thread_pool_stress_test
```

## Future Improvements

1. **Work Stealing**: Implement work-stealing between threads for better load balancing
2. **Thread Affinity**: Enhanced CPU affinity support for performance-critical threads
3. **Async Support**: Integration with Tokio for async task execution
4. **Circuit Breaker**: Add circuit breaker pattern for failing services
5. **Thread Pool Profiles**: Pre-configured profiles for different workload types