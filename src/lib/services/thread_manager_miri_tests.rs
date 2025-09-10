#![cfg(test)]
#![cfg(miri)]

use super::thread_manager::*;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;
use std::time::Duration;

#[test]
fn miri_test_thread_manager_memory_safety() {
    let manager = ThreadManager::new();
    
    // Test concurrent access to thread manager
    let manager_arc = Arc::new(manager);
    let handles: Vec<_> = (0..3).map(|i| {
        let manager_clone = manager_arc.clone();
        thread::spawn(move || {
            // This should be safe with our refactored Arc<Mutex<>> approach
            manager_clone.get_thread_info(&format!("test_{}", i));
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn miri_test_spawn_and_shutdown_safety() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    let thread_id = spawn("miri_test", move |shutdown, _| {
        while !shutdown.load(Ordering::Relaxed) {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    thread::sleep(Duration::from_millis(10));
    
    // This should be memory-safe with proper locking
    let _ = stop_thread(&thread_id);
    
    assert!(counter.load(Ordering::Relaxed) > 0);
}

#[test]
fn miri_test_concurrent_thread_operations() {
    // Test concurrent spawn operations
    let handles: Vec<_> = (0..5).map(|i| {
        thread::spawn(move || {
            spawn(&format!("concurrent_{}", i), |shutdown, _| {
                while !shutdown.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(1));
                }
            })
        })
    }).collect();
    
    let thread_ids: Vec<String> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    
    // Concurrent info retrieval
    let info_handles: Vec<_> = thread_ids.iter().map(|id| {
        let id_clone = id.clone();
        thread::spawn(move || {
            get_thread_info(&id_clone)
        })
    }).collect();
    
    for handle in info_handles {
        handle.join().unwrap();
    }
    
    // Cleanup
    for id in thread_ids {
        let _ = stop_thread(&id);
    }
}

#[test]
fn miri_test_monitor_handle_safety() {
    // This test specifically checks the safety of our monitor_handle refactoring
    let manager1 = ThreadManager::new();
    let manager2 = ThreadManager::new();
    
    // Both managers should have their own monitor handles without interference
    drop(manager1);
    drop(manager2);
    
    // No undefined behavior should occur
}

#[test]
fn miri_test_thread_restart_memory_safety() {
    let shared_data = Arc::new(AtomicBool::new(false));
    let data_clone = shared_data.clone();
    
    let thread_id = spawn("restart_test", move |shutdown, _| {
        data_clone.store(true, Ordering::Relaxed);
        while !shutdown.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    thread::sleep(Duration::from_millis(10));
    
    // Test restart operation for memory safety
    let _ = restart_thread(&thread_id);
    
    thread::sleep(Duration::from_millis(10));
    
    // Cleanup
    let _ = stop_thread(&thread_id);
    
    assert!(shared_data.load(Ordering::Relaxed));
}

#[test]
fn miri_test_shutdown_all_safety() {
    // Spawn multiple threads
    for i in 0..3 {
        spawn(&format!("shutdown_test_{}", i), |shutdown, _| {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(1));
            }
        });
    }
    
    thread::sleep(Duration::from_millis(10));
    
    // This should safely shutdown all threads without memory issues
    shutdown_all();
    
    // Verify no threads remain
    let threads = list_threads();
    assert_eq!(threads.len(), 0);
}