# Thread Manager Usage Guide

## Overview

The ThreadManager utility provides comprehensive thread panic recovery and monitoring for all thread::spawn calls throughout the codebase. It includes:

- Automatic panic recovery with configurable retry logic
- Thread health monitoring and metrics
- Graceful shutdown mechanisms
- Integration with Prometheus metrics and tracing
- Thread status tracking and reporting

## Basic Usage

### Simple Thread Spawn

```rust
use crate::services::thread_manager;

// Spawn a simple thread with default configuration
let thread_id = thread_manager::spawn("my_worker", move |shutdown_signal, health_rx| {
    while !shutdown_signal.load(Ordering::Relaxed) {
        // Do work...
        thread::sleep(Duration::from_secs(1));
    }
});
```

### Custom Configuration

```rust
use crate::services::thread_manager::{self, ThreadConfig};

let config = ThreadConfig {
    name: "critical_service".to_string(),
    restart_on_panic: true,
    max_restarts: 5,
    restart_delay_ms: 3000,
    health_check_interval_ms: Some(30000),
    enable_monitoring: true,
};

let thread_id = thread_manager::spawn_with_config(config, move |shutdown_signal, health_rx| {
    log::info!("Thread started");
    
    while !shutdown_signal.load(Ordering::Relaxed) {
        // Do work...
        
        // Check for health monitoring heartbeat
        if let Some(rx) = &health_rx {
            let _ = rx.try_recv();
        }
        
        thread::sleep(Duration::from_secs(1));
    }
    
    log::info!("Thread stopped gracefully");
});
```

### Loop Helper Functions

```rust
// Spawn a thread that runs a loop
thread_manager::spawn_loop("processor", move || {
    // Return true to continue, false to stop
    process_item();
    true
});

// Spawn a thread that runs at intervals
thread_manager::spawn_interval("monitor", Duration::from_secs(60), move || {
    check_system_health();
});
```

## Thread Management

### Stopping Threads

```rust
// Stop a specific thread
if let Err(e) = thread_manager::stop_thread(&thread_id) {
    log::error!("Failed to stop thread: {}", e);
}

// Shutdown all managed threads
thread_manager::shutdown_all();
```

### Monitoring Threads

```rust
// Get information about a specific thread
if let Some(info) = thread_manager::get_thread_info(&thread_id) {
    println!("Thread {} status: {:?}", info.name, info.status);
    println!("Restart count: {}", info.restart_count);
    println!("Panic count: {}", info.panic_count);
}

// List all managed threads
let threads = thread_manager::list_threads();
for thread in threads {
    println!("Thread {}: {:?}", thread.name, thread.status);
}
```

### Restarting Threads

```rust
// Manually restart a thread
if let Err(e) = thread_manager::restart_thread(&thread_id) {
    log::error!("Failed to restart thread: {}", e);
}
```

## Configuration Options

### ThreadConfig Fields

- `name`: Human-readable name for the thread
- `restart_on_panic`: Whether to automatically restart if the thread panics
- `max_restarts`: Maximum number of restart attempts
- `restart_delay_ms`: Delay in milliseconds between restart attempts
- `health_check_interval_ms`: Optional interval for health monitoring
- `enable_monitoring`: Whether to enable detailed monitoring and metrics

## Metrics

The ThreadManager automatically exports Prometheus metrics:

- `thread_manager_active_threads`: Number of currently active managed threads
- `thread_manager_total_threads_created`: Total number of threads created
- `thread_manager_panic_count`: Total number of thread panics
- `thread_manager_restart_count`: Total number of thread restarts

## Migration Guide

### Before (using std::thread::spawn)

```rust
thread::spawn(move || {
    loop {
        // Work that might panic
        process_data();
        thread::sleep(Duration::from_secs(1));
    }
});
```

### After (using ThreadManager)

```rust
use crate::services::thread_manager::{self, ThreadConfig};

let config = ThreadConfig {
    name: "data_processor".to_string(),
    restart_on_panic: true,
    max_restarts: 3,
    restart_delay_ms: 2000,
    health_check_interval_ms: Some(10000),
    enable_monitoring: true,
};

thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
    while !shutdown_signal.load(Ordering::Relaxed) {
        // Work that might panic - will be automatically restarted
        match std::panic::catch_unwind(|| process_data()) {
            Ok(_) => {},
            Err(e) => log::error!("Processing failed: {:?}", e),
        }
        thread::sleep(Duration::from_secs(1));
    }
});
```

## Best Practices

1. **Always use descriptive thread names** - This helps with debugging and monitoring
2. **Set appropriate restart limits** - Prevent infinite restart loops
3. **Implement graceful shutdown** - Always check the shutdown signal
4. **Use health monitoring for critical threads** - Enable health checks for important services
5. **Log thread lifecycle events** - Add logging at thread start and stop
6. **Handle panics gracefully** - Use `catch_unwind` for operations that might panic
7. **Configure monitoring for production** - Enable metrics and monitoring for production deployments

## Thread Status Values

- `Running`: Thread is actively executing
- `Stopped`: Thread has completed normally
- `Panicked`: Thread panicked and may be restarted
- `Restarting`: Thread is in the process of restarting
- `Shutting`: Thread is shutting down

## Testing

The ThreadManager includes comprehensive tests. Run them with:

```bash
cargo test thread_manager
```

## Examples in Codebase

The following services have been updated to use ThreadManager:

- `src/sam/services/rtsp.rs` - RTSP camera management
- `src/sam/services/sound.rs` - Sound processing stages
- `src/sam/services/lifx/*.rs` - LIFX light control
- `src/sam/services/spotify.rs` - Spotify playback
- `src/sam/services/socket.rs` - WebSocket server

Refer to these files for real-world usage examples.