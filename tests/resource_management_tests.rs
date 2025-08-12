use sam::resource_management::*;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;

#[tokio::test]
async fn test_temp_file_automatic_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file_path;
    
    {
        let temp_file = TempFile::new(temp_dir.path()).unwrap();
        temp_file_path = temp_file.path().to_path_buf();
        
        // Write some data
        temp_file.write(b"test data").await.unwrap();
        
        // Verify file exists
        assert!(temp_file_path.exists());
    } // temp_file dropped here
    
    // Give async cleanup time to run
    sleep(Duration::from_millis(200)).await;
    
    // File should be cleaned up
    assert!(!temp_file_path.exists());
}

#[tokio::test]
async fn test_temp_file_persist() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = TempFile::new(temp_dir.path()).unwrap();
    
    temp_file.write(b"persistent data").await.unwrap();
    let path = temp_file.persist();
    
    // File should still exist after persist
    assert!(path.exists());
    
    // Clean up manually
    tokio::fs::remove_file(path).await.unwrap();
}

#[tokio::test]
async fn test_temp_file_move() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = TempFile::new(temp_dir.path()).unwrap();
    let dest_path = temp_dir.path().join("moved_file.txt");
    
    temp_file.write(b"data to move").await.unwrap();
    temp_file.move_to(&dest_path).await.unwrap();
    
    // Original file should not exist
    // Destination should exist
    assert!(dest_path.exists());
    
    // Read and verify content
    let content = tokio::fs::read(&dest_path).await.unwrap();
    assert_eq!(content, b"data to move");
}

#[tokio::test]
async fn test_file_upload_limits() {
    let limits = FileLimits {
        max_file_size: 1024 * 1024, // 1MB
        max_concurrent_uploads: 2,
        max_user_storage: 10 * 1024 * 1024, // 10MB
        allowed_extensions: vec!["jpg".to_string(), "png".to_string()],
        blocked_extensions: vec!["exe".to_string()],
    };
    
    let limiter = FileLimiter::new(limits);
    
    // Test allowed upload
    let check = limiter.check_upload("user1", 512 * 1024, "jpg").await.unwrap();
    assert!(matches!(check, FileUploadCheck::Allowed { .. }));
    
    // Test blocked extension
    let check = limiter.check_upload("user1", 512 * 1024, "exe").await.unwrap();
    assert!(matches!(check, FileUploadCheck::Rejected { .. }));
    
    // Test file too large
    let check = limiter.check_upload("user1", 2 * 1024 * 1024, "jpg").await.unwrap();
    match check {
        FileUploadCheck::Rejected { reason } => {
            assert!(matches!(reason, FileRejectionReason::FileTooLarge { .. }));
        }
        _ => panic!("Expected rejection for large file"),
    }
    
    // Test disallowed extension
    let check = limiter.check_upload("user1", 512 * 1024, "pdf").await.unwrap();
    assert!(matches!(check, FileUploadCheck::Rejected { .. }));
}

#[tokio::test]
async fn test_concurrent_upload_limits() {
    let limits = FileLimits {
        max_file_size: 1024 * 1024,
        max_concurrent_uploads: 2,
        max_user_storage: 10 * 1024 * 1024,
        allowed_extensions: vec![],
        blocked_extensions: vec![],
    };
    
    let limiter = FileLimiter::new(limits);
    
    // First upload should succeed
    let check1 = limiter.check_upload("user1", 100, "txt").await.unwrap();
    assert!(matches!(check1, FileUploadCheck::Allowed { .. }));
    
    // Second upload should succeed
    let check2 = limiter.check_upload("user1", 100, "txt").await.unwrap();
    assert!(matches!(check2, FileUploadCheck::Allowed { .. }));
    
    // Third upload should be rejected (concurrent limit reached)
    let check3 = limiter.check_upload("user1", 100, "txt").await.unwrap();
    match check3 {
        FileUploadCheck::Rejected { reason } => {
            assert!(matches!(reason, FileRejectionReason::TooManyConcurrentUploads { .. }));
        }
        _ => panic!("Expected rejection for too many concurrent uploads"),
    }
}

#[tokio::test]
async fn test_request_limits() {
    let limits = RequestLimits {
        max_body_size: 1024 * 1024, // 1MB
        max_header_size: 8192,
        max_processing_time: Duration::from_secs(5),
        max_concurrent_per_ip: 3,
    };
    
    let limiter = RequestLimiter::new(limits);
    
    // Test allowed request
    let check = limiter.check_request("192.168.1.1", 512 * 1024, 4096).await.unwrap();
    assert!(matches!(check, RequestCheck::Allowed { .. }));
    
    // Test body too large
    let check = limiter.check_request("192.168.1.1", 2 * 1024 * 1024, 4096).await.unwrap();
    match check {
        RequestCheck::Rejected { reason } => {
            assert!(matches!(reason, RequestRejectionReason::BodyTooLarge { .. }));
        }
        _ => panic!("Expected rejection for large body"),
    }
    
    // Test headers too large
    let check = limiter.check_request("192.168.1.1", 512 * 1024, 10000).await.unwrap();
    match check {
        RequestCheck::Rejected { reason } => {
            assert!(matches!(reason, RequestRejectionReason::HeadersTooLarge { .. }));
        }
        _ => panic!("Expected rejection for large headers"),
    }
}

#[tokio::test]
async fn test_memory_limits() {
    let limits = MemoryLimits {
        max_allocation: 1024 * 1024, // 1MB
        max_buffer_size: 64 * 1024,
        warning_threshold: 0.8,
        critical_threshold: 0.95,
    };
    
    let limiter = MemoryLimiter::new(limits);
    
    // Test allowed allocation
    let check = limiter.check_allocation(512 * 1024).await.unwrap();
    assert!(matches!(check, MemoryCheck::Allowed { .. }));
    
    // Test allocation too large
    let check = limiter.check_allocation(2 * 1024 * 1024).await.unwrap();
    match check {
        MemoryCheck::Rejected { reason } => {
            assert!(matches!(reason, MemoryRejectionReason::AllocationTooLarge { .. }));
        }
        _ => panic!("Expected rejection for large allocation"),
    }
    
    // Test release
    limiter.release(256 * 1024).await;
    let usage = limiter.get_usage().await;
    assert!(usage < 512 * 1024);
}

#[tokio::test]
async fn test_limited_buffer() {
    let mut buffer = LimitedBuffer::new(10);
    
    // Write within limit
    assert_eq!(buffer.write(b"hello").unwrap(), 5);
    assert_eq!(buffer.size(), 5);
    assert!(!buffer.is_full());
    
    // Write up to limit
    assert_eq!(buffer.write(b"world").unwrap(), 5);
    assert_eq!(buffer.size(), 10);
    assert!(buffer.is_full());
    
    // Try to exceed limit
    assert!(buffer.write(b"!").is_err());
    
    // Read and clear
    let data = buffer.read();
    assert_eq!(&data, b"helloworld");
    assert_eq!(buffer.size(), 0);
    assert!(!buffer.is_full());
}

#[tokio::test]
async fn test_cleanup_guard() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    let cleaned = Arc::new(AtomicBool::new(false));
    let cleaned_clone = cleaned.clone();
    
    {
        let _guard = CleanupGuard::new(
            42,
            move |_value| {
                cleaned_clone.store(true, Ordering::SeqCst);
            },
        );
    } // guard dropped here
    
    assert!(cleaned.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_resource_cleanup_batch() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");
    let dir1 = temp_dir.path().join("dir1");
    
    // Create test files and directory
    tokio::fs::write(&file1, b"test1").await.unwrap();
    tokio::fs::write(&file2, b"test2").await.unwrap();
    tokio::fs::create_dir(&dir1).await.unwrap();
    
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(dir1.exists());
    
    // Create cleanup manager
    let mut cleanup = ResourceCleanup::new();
    cleanup.add_file(file1.clone());
    cleanup.add_file(file2.clone());
    cleanup.add_directory(dir1.clone());
    
    // Execute cleanup
    cleanup.cleanup().await.unwrap();
    
    // All should be cleaned up
    assert!(!file1.exists());
    assert!(!file2.exists());
    assert!(!dir1.exists());
}

#[tokio::test]
async fn test_resource_manager_integration() {
    let config = ResourceConfig {
        file_limits: FileLimits {
            max_file_size: 1024 * 1024,
            max_concurrent_uploads: 5,
            max_user_storage: 10 * 1024 * 1024,
            allowed_extensions: vec!["txt".to_string()],
            blocked_extensions: vec![],
            enable_virus_scan: false,
            temp_cleanup_interval: 3600,
            temp_max_age: 86400,
        },
        request_limits: RequestLimits::default(),
        pool_config: PoolConfig::default(),
        cleanup_config: CleanupConfig {
            enable_auto_cleanup: false, // Disable for testing
            cleanup_interval: 3600,
            temp_dir: TempDir::new().unwrap().path().to_path_buf(),
            max_temp_size: 1024 * 1024,
            orphan_age_threshold: 3600,
        },
        memory_limits: MemoryLimits::default(),
    };
    
    let manager = ResourceManager::new(config);
    
    // Test upload permission check
    let permission = manager.check_upload_allowed("user1", 512 * 1024, "txt").await.unwrap();
    assert!(matches!(permission, UploadPermission::Allowed { .. }));
    
    // Test blocked upload
    let permission = manager.check_upload_allowed("user1", 2 * 1024 * 1024, "txt").await.unwrap();
    assert!(matches!(permission, UploadPermission::Denied { .. }));
    
    // Test file processing
    let file_data = b"test file content".to_vec();
    let result = manager.process_upload(file_data, "test.txt", "user1").await;
    assert!(result.is_ok());
    
    if let Ok(processed) = result {
        assert_eq!(processed.size, 17);
        assert_eq!(processed.mime_type, "text/plain");
        assert!(processed.path.exists());
        
        // Clean up
        tokio::fs::remove_file(processed.path).await.unwrap();
    }
}

#[tokio::test]
async fn test_connection_pool_basic() {
    use sam::resource_management::pool::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    // Mock connection
    struct MockConnection {
        id: u64,
    }
    
    #[async_trait]
    impl PooledConnection for MockConnection {
        async fn is_valid(&self) -> bool {
            true
        }
        
        async fn close(self) {
            // No-op
        }
    }
    
    // Mock factory
    struct MockFactory {
        counter: Arc<AtomicU64>,
    }
    
    #[async_trait]
    impl ConnectionFactory<MockConnection> for MockFactory {
        async fn create(&self) -> anyhow::Result<MockConnection> {
            let id = self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(MockConnection { id })
        }
        
        async fn validate(&self, _conn: &MockConnection) -> bool {
            true
        }
    }
    
    let config = PoolConfig {
        max_connections: 3,
        min_connections: 1,
        ..Default::default()
    };
    
    let factory = Arc::new(MockFactory {
        counter: Arc::new(AtomicU64::new(1)),
    });
    
    let pool = ConnectionPool::new(config, factory).await.unwrap();
    
    // Get connections
    let conn1 = pool.get().await.unwrap();
    assert_eq!(conn1.id, 1);
    
    let conn2 = pool.get().await.unwrap();
    assert_eq!(conn2.id, 2);
    
    // Return first connection
    drop(conn1);
    
    // Wait for async return
    sleep(Duration::from_millis(100)).await;
    
    // Get connection again - should reuse
    let conn3 = pool.get().await.unwrap();
    assert_eq!(conn3.id, 1); // Reused connection
    
    // Get metrics
    let metrics = pool.get_metrics().await;
    assert!(metrics.total_created >= 2);
    assert!(metrics.total_checkouts >= 3);
}

#[test]
fn test_resource_config_from_env() {
    use sam::http::resource_middleware::ResourceConfig;
    
    // Set some env vars
    std::env::set_var("MAX_FILE_SIZE", "52428800"); // 50MB
    std::env::set_var("MAX_CONCURRENT_UPLOADS", "20");
    std::env::set_var("ENABLE_VIRUS_SCAN", "false");
    
    let config = ResourceConfig::from_env();
    
    assert_eq!(config.file_limits.max_file_size, 52428800);
    assert_eq!(config.file_limits.max_concurrent_uploads, 20);
    assert!(!config.file_limits.enable_virus_scan);
    
    // Clean up env vars
    std::env::remove_var("MAX_FILE_SIZE");
    std::env::remove_var("MAX_CONCURRENT_UPLOADS");
    std::env::remove_var("ENABLE_VIRUS_SCAN");
}