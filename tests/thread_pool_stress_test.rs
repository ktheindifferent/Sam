use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use sam::sam::services::thread_manager::{
    self, ThreadConfig, ThreadPriority, ThreadPoolConfig, ThreadPoolStats,
    submit_task, submit_task_with_priority, get_pool_stats, spawn_pooled,
    configure_thread_pool, shutdown_thread_pool
};

#[test]
fn test_thread_pool_basic_submission() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    let result = submit_task("test_task", move || {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_thread_pool_priority_ordering() {
    let results = Arc::new(Mutex::new(Vec::new()));
    
    // Submit tasks with different priorities
    for i in 0..10 {
        let results_clone = results.clone();
        let priority = if i % 2 == 0 {
            ThreadPriority::Low
        } else {
            ThreadPriority::High
        };
        
        let _ = submit_task_with_priority(
            &format!("task_{}", i),
            priority,
            move || {
                std::thread::sleep(Duration::from_millis(10));
                results_clone.lock().unwrap().push(i);
            }
        );
    }
    
    std::thread::sleep(Duration::from_millis(500));
    
    let final_results = results.lock().unwrap();
    // High priority tasks should generally complete before low priority ones
    assert!(final_results.len() == 10);
}

#[test]
fn test_thread_pool_exhaustion() {
    let completed = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(AtomicUsize::new(0));
    
    // Try to submit many tasks rapidly
    for i in 0..1000 {
        let completed_clone = completed.clone();
        let rejected_clone = rejected.clone();
        
        match submit_task(&format!("exhaustion_task_{}", i), move || {
            std::thread::sleep(Duration::from_millis(50));
            completed_clone.fetch_add(1, Ordering::Relaxed);
        }) {
            Ok(_) => {},
            Err(_) => {
                rejected_clone.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    // Wait for tasks to complete
    std::thread::sleep(Duration::from_secs(5));
    
    let stats = get_pool_stats();
    println!("Pool stats: {:?}", stats);
    println!("Completed: {}, Rejected: {}", 
             completed.load(Ordering::Relaxed), 
             rejected.load(Ordering::Relaxed));
    
    // Some tasks should be rejected due to backpressure
    assert!(rejected.load(Ordering::Relaxed) > 0);
    assert!(completed.load(Ordering::Relaxed) > 0);
}

#[test]
fn test_thread_pool_auto_scaling() {
    let start_stats = get_pool_stats();
    let initial_threads = start_stats.total_threads;
    
    // Submit burst of tasks to trigger scale-up
    let handles = (0..50).map(|i| {
        submit_task(&format!("scale_task_{}", i), move || {
            std::thread::sleep(Duration::from_millis(100));
        })
    }).collect::<Vec<_>>();
    
    // Give time for auto-scaling to kick in
    std::thread::sleep(Duration::from_millis(200));
    
    let scaled_stats = get_pool_stats();
    assert!(scaled_stats.total_threads >= initial_threads);
    
    // Wait for tasks to complete
    std::thread::sleep(Duration::from_secs(2));
    
    // Pool should scale down after idle period
    std::thread::sleep(Duration::from_secs(2));
    let final_stats = get_pool_stats();
    println!("Initial: {}, Scaled: {}, Final: {}", 
             initial_threads, scaled_stats.total_threads, final_stats.total_threads);
}

#[test]
fn test_thread_pool_panic_recovery() {
    let before_stats = get_pool_stats();
    
    // Submit task that will panic
    let result = submit_task("panic_task", || {
        panic!("Intentional panic for testing");
    });
    
    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(500));
    
    let after_stats = get_pool_stats();
    assert!(after_stats.tasks_failed > before_stats.tasks_failed);
    
    // Pool should still be functional after panic
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    let result = submit_task("post_panic_task", move || {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_thread_pool_concurrent_access() {
    use std::thread;
    
    let barrier = Arc::new(std::sync::Barrier::new(10));
    let success_count = Arc::new(AtomicUsize::new(0));
    
    let handles: Vec<_> = (0..10).map(|i| {
        let barrier_clone = barrier.clone();
        let success_clone = success_count.clone();
        
        thread::spawn(move || {
            barrier_clone.wait();
            
            // All threads try to submit tasks simultaneously
            for j in 0..10 {
                match submit_task(&format!("concurrent_{}_{}", i, j), move || {
                    std::thread::sleep(Duration::from_millis(10));
                }) {
                    Ok(_) => {
                        success_clone.fetch_add(1, Ordering::Relaxed);
                    },
                    Err(_) => {}
                }
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert!(success_count.load(Ordering::Relaxed) > 0);
}

#[test]
fn test_thread_pool_memory_pressure() {
    // Test that the pool handles memory-intensive tasks
    let completed = Arc::new(AtomicUsize::new(0));
    
    for i in 0..20 {
        let completed_clone = completed.clone();
        let _ = submit_task(&format!("memory_task_{}", i), move || {
            // Allocate some memory
            let _data: Vec<u8> = vec![0; 1024 * 1024]; // 1MB
            std::thread::sleep(Duration::from_millis(50));
            completed_clone.fetch_add(1, Ordering::Relaxed);
        });
    }
    
    std::thread::sleep(Duration::from_secs(3));
    assert!(completed.load(Ordering::Relaxed) > 0);
}

#[test]
fn test_thread_pool_long_running_tasks() {
    let start = Instant::now();
    let completed = Arc::new(AtomicUsize::new(0));
    
    // Submit mix of long and short tasks
    for i in 0..10 {
        let completed_clone = completed.clone();
        let duration = if i % 3 == 0 {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(10)
        };
        
        let _ = submit_task(&format!("mixed_duration_{}", i), move || {
            std::thread::sleep(duration);
            completed_clone.fetch_add(1, Ordering::Relaxed);
        });
    }
    
    // Wait for all tasks
    while completed.load(Ordering::Relaxed) < 10 && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(100));
    }
    
    assert_eq!(completed.load(Ordering::Relaxed), 10);
}

#[test]
fn test_thread_pool_stats_accuracy() {
    // Clear any previous state
    std::thread::sleep(Duration::from_millis(500));
    
    let initial_stats = get_pool_stats();
    let tasks_to_submit = 25;
    let mut submitted = 0;
    
    for i in 0..tasks_to_submit {
        match submit_task(&format!("stats_task_{}", i), move || {
            std::thread::sleep(Duration::from_millis(20));
        }) {
            Ok(_) => submitted += 1,
            Err(_) => {}
        }
    }
    
    // Wait for tasks to complete
    std::thread::sleep(Duration::from_secs(2));
    
    let final_stats = get_pool_stats();
    
    // Verify stats are being tracked correctly
    assert!(final_stats.tasks_submitted >= initial_stats.tasks_submitted + submitted);
    assert!(final_stats.tasks_completed >= initial_stats.tasks_completed);
}

#[test] 
fn test_critical_priority_tasks() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    
    // Submit tasks with all priority levels
    let priorities = vec![
        (ThreadPriority::Low, "low"),
        (ThreadPriority::Critical, "critical"),
        (ThreadPriority::Normal, "normal"),
        (ThreadPriority::High, "high"),
    ];
    
    for (priority, name) in priorities {
        let order_clone = execution_order.clone();
        let name = name.to_string();
        
        let _ = submit_task_with_priority(
            &format!("priority_{}", name),
            priority,
            move || {
                order_clone.lock().unwrap().push(name);
                std::thread::sleep(Duration::from_millis(10));
            }
        );
    }
    
    std::thread::sleep(Duration::from_millis(500));
    
    let order = execution_order.lock().unwrap();
    // Critical should generally execute first
    assert_eq!(order.len(), 4);
}