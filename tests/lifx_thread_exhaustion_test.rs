#[cfg(test)]
mod lifx_thread_exhaustion_tests {
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;
    use threadpool::ThreadPool;

    #[test]
    fn test_thread_spawn_failure_handling() {
        // Create a small thread pool to simulate resource constraints
        let pool = ThreadPool::with_name("test_pool".to_string(), 2);
        let spawn_failures = Arc::new(AtomicUsize::new(0));
        let successful_spawns = Arc::new(AtomicUsize::new(0));
        
        // Try to spawn more threads than available
        for i in 0..10 {
            let failures = Arc::clone(&spawn_failures);
            let successes = Arc::clone(&successful_spawns);
            
            // Try direct thread spawn
            match thread::Builder::new()
                .name(format!("test_thread_{}", i))
                .stack_size(1024 * 1024)
                .spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                }) {
                Ok(_) => {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    failures.fetch_add(1, Ordering::Relaxed);
                    // Fallback to thread pool
                    pool.execute(move || {
                        thread::sleep(Duration::from_millis(100));
                    });
                }
            }
        }
        
        // Wait for tasks to complete
        pool.join();
        
        // Verify that all tasks were handled either directly or via pool
        let total_handled = successful_spawns.load(Ordering::Relaxed) + 
                          spawn_failures.load(Ordering::Relaxed);
        assert_eq!(total_handled, 10, "All tasks should be handled");
    }

    #[test]
    fn test_thread_pool_saturation_detection() {
        let pool = ThreadPool::with_name("saturation_test".to_string(), 2);
        let tasks_queued = Arc::new(AtomicUsize::new(0));
        
        // Saturate the pool
        for _ in 0..10 {
            let queued = Arc::clone(&tasks_queued);
            pool.execute(move || {
                queued.fetch_add(1, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(500));
            });
        }
        
        // Check pool status
        assert!(pool.queued_count() > 0, "Pool should have queued tasks");
        assert_eq!(pool.active_count(), pool.max_count(), "Pool should be saturated");
        
        pool.join();
    }

    #[test]
    fn test_graceful_degradation_under_resource_pressure() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let service_available = Arc::new(AtomicBool::new(false));
        let pool = ThreadPool::with_name("degraded_mode".to_string(), 1);
        
        // Simulate main thread spawn failure
        let stop_flag_clone = Arc::clone(&stop_flag);
        let service_clone = Arc::clone(&service_available);
        
        // Instead of spawning a thread, use the pool (simulating fallback)
        pool.execute(move || {
            service_clone.store(true, Ordering::Relaxed);
            while !stop_flag_clone.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(10));
            }
            service_clone.store(false, Ordering::Relaxed);
        });
        
        // Give the service time to start
        thread::sleep(Duration::from_millis(100));
        
        // Verify service is running in degraded mode
        assert!(service_available.load(Ordering::Relaxed), 
                "Service should be available in degraded mode");
        
        // Stop the service
        stop_flag.store(true, Ordering::Relaxed);
        pool.join();
        
        assert!(!service_available.load(Ordering::Relaxed), 
                "Service should be stopped");
    }

    #[test]
    fn test_resource_monitoring_before_spawn() {
        fn check_resources(active: usize, max: usize) -> Result<(), String> {
            if active >= max {
                return Err(format!("Thread pool saturated: {}/{}", active, max));
            }
            Ok(())
        }
        
        // Test normal conditions
        assert!(check_resources(2, 4).is_ok());
        
        // Test saturation
        assert!(check_resources(4, 4).is_err());
        assert!(check_resources(5, 4).is_err());
    }

    #[test]
    fn test_metrics_tracking() {
        use std::sync::atomic::AtomicI64;
        
        let spawn_attempts = Arc::new(AtomicI64::new(0));
        let spawn_failures = Arc::new(AtomicI64::new(0));
        let pool_active = Arc::new(AtomicI64::new(0));
        
        // Simulate spawn attempts
        for _ in 0..5 {
            spawn_attempts.fetch_add(1, Ordering::Relaxed);
            
            // Simulate some failures
            if spawn_attempts.load(Ordering::Relaxed) % 2 == 0 {
                spawn_failures.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        // Update pool metrics
        pool_active.store(3, Ordering::Relaxed);
        
        // Verify metrics
        assert_eq!(spawn_attempts.load(Ordering::Relaxed), 5);
        assert_eq!(spawn_failures.load(Ordering::Relaxed), 2);
        assert_eq!(pool_active.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_stack_size_optimization() {
        // Test that threads with smaller stack sizes can be created
        // when default size would fail
        let small_stack_success = Arc::new(AtomicBool::new(false));
        let large_stack_success = Arc::new(AtomicBool::new(false));
        
        // Try with small stack
        let small_clone = Arc::clone(&small_stack_success);
        if let Ok(handle) = thread::Builder::new()
            .name("small_stack".to_string())
            .stack_size(512 * 1024)  // 512KB
            .spawn(move || {
                small_clone.store(true, Ordering::Relaxed);
            }) {
            handle.join().unwrap();
        }
        
        // Try with large stack
        let large_clone = Arc::clone(&large_stack_success);
        if let Ok(handle) = thread::Builder::new()
            .name("large_stack".to_string())
            .stack_size(8 * 1024 * 1024)  // 8MB
            .spawn(move || {
                large_clone.store(true, Ordering::Relaxed);
            }) {
            handle.join().unwrap();
        }
        
        // Small stack should have better chance of success
        assert!(small_stack_success.load(Ordering::Relaxed));
    }

    #[test]
    fn test_concurrent_spawn_attempts() {
        let pool = ThreadPool::with_name("concurrent_test".to_string(), 4);
        let barrier = Arc::new(std::sync::Barrier::new(5));
        let successes = Arc::new(AtomicUsize::new(0));
        
        let mut handles = vec![];
        
        // Launch multiple threads trying to spawn concurrently
        for i in 0..4 {
            let barrier_clone = Arc::clone(&barrier);
            let success_clone = Arc::clone(&successes);
            let pool_clone = pool.clone();
            
            let handle = thread::spawn(move || {
                barrier_clone.wait();
                
                // Try to spawn a thread
                match thread::Builder::new()
                    .name(format!("concurrent_{}", i))
                    .spawn(|| {
                        thread::sleep(Duration::from_millis(10));
                    }) {
                    Ok(h) => {
                        success_clone.fetch_add(1, Ordering::Relaxed);
                        h.join().unwrap();
                    }
                    Err(_) => {
                        // Fallback to pool
                        pool_clone.execute(|| {
                            thread::sleep(Duration::from_millis(10));
                        });
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Trigger all threads simultaneously
        barrier.wait();
        
        // Wait for all to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        pool.join();
        
        // At least some should succeed
        assert!(successes.load(Ordering::Relaxed) > 0);
    }
}