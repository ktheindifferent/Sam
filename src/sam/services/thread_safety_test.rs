// Standalone test to verify thread manager memory safety after refactoring

use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::HashMap;

// Simplified version to test the pattern we fixed
struct TestManager {
    data: Arc<Mutex<HashMap<String, String>>>,
    monitor_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl TestManager {
    fn new() -> Self {
        let manager = TestManager {
            data: Arc::new(Mutex::new(HashMap::new())),
            monitor_handle: Arc::new(Mutex::new(None)),
        };
        
        manager.start_monitor();
        manager
    }
    
    fn start_monitor(&self) {
        let data = self.data.clone();
        
        let handle = thread::spawn(move || {
            println!("Monitor thread started");
            // Simulate monitoring work
            if let Ok(guard) = data.lock() {
                println!("Monitor accessed data: {} items", guard.len());
            }
            println!("Monitor thread stopped");
        });
        
        // This is the SAFE version - no unsafe block needed!
        if let Ok(mut monitor) = self.monitor_handle.lock() {
            *monitor = Some(handle);
            println!("Monitor handle stored safely");
        }
    }
    
    fn shutdown(&self) {
        if let Ok(mut monitor) = self.monitor_handle.lock() {
            if let Some(handle) = monitor.take() {
                let _ = handle.join();
                println!("Monitor thread joined successfully");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safe_initialization() {
        let manager = TestManager::new();
        thread::sleep(std::time::Duration::from_millis(10));
        manager.shutdown();
        println!("Test passed: No unsafe code needed!");
    }
    
    #[test]
    fn test_concurrent_access() {
        let manager = Arc::new(TestManager::new());
        
        let handles: Vec<_> = (0..5).map(|i| {
            let mgr = manager.clone();
            thread::spawn(move || {
                if let Ok(mut data) = mgr.data.lock() {
                    data.insert(format!("key_{}", i), format!("value_{}", i));
                }
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        manager.shutdown();
        
        // Clone the Arc to avoid borrowing through a temporary deref of Arc<TestManager>
        let data_arc = manager.data.clone();
        if let Ok(data) = data_arc.lock() {
            assert_eq!(data.len(), 5);
            println!("Concurrent access test passed!");
        };
    }
}