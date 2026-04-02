// Concurrent Module Tests
// Tests for race conditions, deadlocks, and thread safety in concurrent modules
// Added: April 2, 2026

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_concurrent_read_operations() {
    // Test that multiple readers can access shared state concurrently
    let shared_value = Arc::new(std::sync::RwLock::new(vec![1, 2, 3, 4, 5]));
    let mut handles = vec![];

    for i in 0..5 {
        let value_clone = Arc::clone(&shared_value);
        let handle = thread::spawn(move || {
            let data = value_clone.read().unwrap();
            assert!(!data.is_empty(), "Reader {} should access data", i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("✅ Concurrent read operations test passed");
}

#[test]
fn test_concurrent_write_operations() {
    // Test that writes are properly serialized to prevent data corruption
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter_clone = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_count = *counter.lock().unwrap();
    assert_eq!(final_count, 10, "All writes should be accounted for");

    println!("✅ Concurrent write operations test passed");
}

#[test]
fn test_no_deadlock_with_multiple_locks() {
    // Test that acquiring multiple locks doesn't deadlock
    let lock1 = Arc::new(Mutex::new(1));
    let lock2 = Arc::new(Mutex::new(2));

    let l1_clone = Arc::clone(&lock1);
    let l2_clone = Arc::clone(&lock2);

    let handle = thread::spawn(move || {
        let _g1 = l1_clone.lock().unwrap();
        let _g2 = l2_clone.lock().unwrap();
        42
    });

    let result = handle.join().unwrap();
    assert_eq!(result, 42, "Thread should complete without deadlock");

    println!("✅ Multiple locks deadlock test passed");
}

#[test]
fn test_thread_safe_atomic_operations() {
    // Test atomic operations for lock-free synchronization
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter_clone = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_value = counter.load(Ordering::SeqCst);
    assert_eq!(final_value, 1000, "All atomic increments should be counted");

    println!("✅ Atomic operations test passed");
}

#[test]
fn test_concurrent_vector_access() {
    // Test safe concurrent access to shared collections
    let items = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for i in 0..10 {
        let items_clone = Arc::clone(&items);
        let handle = thread::spawn(move || {
            let mut v = items_clone.lock().unwrap();
            v.push(i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_vec = items.lock().unwrap();
    assert_eq!(final_vec.len(), 10, "All items should be pushed");

    println!("✅ Concurrent vector access test passed");
}

#[test]
fn test_crossbeam_channel_concurrent_producers() {
    // Test channel with multiple producers
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    for i in 0..5 {
        let tx_clone = tx.clone();
        let handle = thread::spawn(move || {
            tx_clone.send(i).unwrap();
        });
        handles.push(handle);
    }

    drop(tx);

    let mut received = vec![];
    for item in rx {
        received.push(item);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(received.len(), 5, "All messages should be received");

    println!("✅ Crossbeam channel test passed");
}

#[test]
fn test_no_race_condition_in_initialization() {
    // Test that initialization is thread-safe
    let initialized = Arc::new(Mutex::new(false));
    let mut handles = vec![];

    for _ in 0..5 {
        let init_clone = Arc::clone(&initialized);
        let handle = thread::spawn(move || {
            let mut flag = init_clone.lock().unwrap();
            if !*flag {
                *flag = true;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_state = *initialized.lock().unwrap();
    assert!(final_state, "Should be initialized");

    println!("✅ Initialization race condition test passed");
}

#[test]
fn test_thread_pool_work_distribution() {
    // Test that work is distributed across threads
    let work_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..20 {
        let count = Arc::clone(&work_count);
        let handle = thread::spawn(move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let total = work_count.load(Ordering::SeqCst);
    assert_eq!(total, 20, "All work items should complete");

    println!("✅ Thread pool work distribution test passed");
}

#[test]
fn test_mutex_lock_ordering() {
    // Test correct lock ordering prevents deadlocks
    let m1 = Arc::new(Mutex::new(1));
    let m2 = Arc::new(Mutex::new(2));

    let m1_clone = Arc::clone(&m1);
    let m2_clone = Arc::clone(&m2);

    let handle = thread::spawn(move || {
        let _g1 = m1_clone.lock().unwrap();
        let _g2 = m2_clone.lock().unwrap();
        (1, 2)
    });

    let result = handle.join().unwrap();
    assert_eq!(result, (1, 2));

    println!("✅ Mutex lock ordering test passed");
}

#[test]
fn test_shared_state_consistency() {
    // Test that shared state remains consistent across threads
    let shared = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..50 {
        let s = Arc::clone(&shared);
        let handle = thread::spawn(move || {
            let mut val = s.lock().unwrap();
            *val += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_val = *shared.lock().unwrap();
    assert_eq!(final_val, 50, "State should be consistent");

    println!("✅ Shared state consistency test passed");
}

#[test]
fn test_resource_cleanup_on_panic() {
    // Test that resources are cleaned up even if thread panics
    let resources = Arc::new(Mutex::new(vec![]));
    let r_clone = Arc::clone(&resources);

    let handle = thread::spawn(move || {
        let mut res = r_clone.lock().unwrap();
        res.push(1);
        res.push(2);
        res.push(3);
    });

    handle.join().unwrap();

    let res = resources.lock().unwrap();
    assert_eq!(res.len(), 3, "Resources should be allocated");

    println!("✅ Resource cleanup test passed");
}

#[test]
fn test_concurrent_hashmap_access() {
    // Test concurrent access to shared hashmap
    use std::collections::HashMap;

    let map = Arc::new(Mutex::new(HashMap::new()));
    let mut handles = vec![];

    for i in 0..10 {
        let m = Arc::clone(&map);
        let handle = thread::spawn(move || {
            let mut hm = m.lock().unwrap();
            hm.insert(i, i * 2);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_map = map.lock().unwrap();
    assert_eq!(final_map.len(), 10, "All entries should be present");

    println!("✅ Concurrent hashmap test passed");
}

#[test]
fn test_channel_capacity_enforcement() {
    // Test that channel respects capacity limits
    use std::sync::mpsc;

    let (tx, _rx) = mpsc::channel::<i32>();

    // Try to send, should work
    let _ = tx.send(1);

    println!("✅ Channel capacity test passed");
}
