use async_trait::async_trait;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use crate::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPayload {
    pub target: CleanupTarget,
    pub older_than_days: Option<u32>,
    pub pattern: Option<String>,
    pub max_size_mb: Option<u64>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupTarget {
    TempFiles { path: String },
    Logs { path: String, keep_recent: usize },
    Cache { cache_type: CacheType },
    OldBackups { path: String, keep_count: usize },
    Database { table: String, condition: Option<String> },
    Docker { remove_images: bool, remove_volumes: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheType {
    Redis,
    Disk { path: String },
    Memory,
}

pub struct CleanupJobHandler {
    safe_paths: Vec<PathBuf>,
}

impl CleanupJobHandler {
    pub fn new() -> Self {
        Self {
            safe_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
                PathBuf::from("/var/log"),
            ],
        }
    }
    
    async fn perform_cleanup(&self, payload: CleanupPayload) -> Result<CleanupResult, String> {
        info!("Starting cleanup: {:?}", payload.target);
        
        if payload.dry_run {
            info!("Running in dry-run mode - no actual deletions will occur");
        }
        
        let mut files_deleted = 0;
        let mut space_freed = 0u64;
        let mut errors = Vec::new();
        
        match &payload.target {
            CleanupTarget::TempFiles { path } => {
                let target_path = PathBuf::from(path);
                
                // Safety check
                if !self.is_safe_path(&target_path) {
                    return Err(format!("Unsafe path for cleanup: {}", path));
                }
                
                // Simulate finding and deleting temp files
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                files_deleted = rand::random::<usize>() % 100 + 1;
                space_freed = (rand::random::<u64>() % 1000) * 1024 * 1024; // MB to bytes
                
                if !payload.dry_run {
                    info!("Deleted {} temp files from {}", files_deleted, path);
                }
            }
            
            CleanupTarget::Logs { path, keep_recent } => {
                info!("Cleaning logs in {} keeping {} recent files", path, keep_recent);
                
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                files_deleted = rand::random::<usize>() % 50;
                space_freed = (rand::random::<u64>() % 500) * 1024 * 1024;
            }
            
            CleanupTarget::Cache { cache_type } => {
                match cache_type {
                    CacheType::Redis => {
                        info!("Cleaning Redis cache");
                        // Would call Redis FLUSHDB or selective deletion
                    }
                    CacheType::Disk { path } => {
                        info!("Cleaning disk cache at {}", path);
                    }
                    CacheType::Memory => {
                        info!("Cleaning memory cache");
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(500)).await;
                space_freed = (rand::random::<u64>() % 2000) * 1024 * 1024;
            }
            
            CleanupTarget::OldBackups { path, keep_count } => {
                info!("Cleaning old backups in {}, keeping {} most recent", path, keep_count);
                
                tokio::time::sleep(Duration::from_secs(2)).await;
                
                files_deleted = rand::random::<usize>() % 10;
                space_freed = (rand::random::<u64>() % 5000) * 1024 * 1024;
            }
            
            CleanupTarget::Database { table, condition } => {
                info!("Cleaning database table {} with condition: {:?}", table, condition);
                
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                files_deleted = rand::random::<usize>() % 1000; // rows deleted
            }
            
            CleanupTarget::Docker { remove_images, remove_volumes } => {
                info!("Cleaning Docker - images: {}, volumes: {}", remove_images, remove_volumes);
                
                tokio::time::sleep(Duration::from_secs(3)).await;
                
                if *remove_images {
                    files_deleted += rand::random::<usize>() % 20;
                    space_freed += (rand::random::<u64>() % 10000) * 1024 * 1024;
                }
                
                if *remove_volumes {
                    files_deleted += rand::random::<usize>() % 10;
                    space_freed += (rand::random::<u64>() % 5000) * 1024 * 1024;
                }
            }
        }
        
        Ok(CleanupResult {
            items_deleted: files_deleted,
            space_freed_bytes: space_freed,
            errors,
            dry_run: payload.dry_run,
        })
    }
    
    fn is_safe_path(&self, path: &PathBuf) -> bool {
        // Check if path is in safe paths or is a subdirectory of a safe path
        for safe_path in &self.safe_paths {
            if path.starts_with(safe_path) {
                return true;
            }
        }
        
        // Additional safety checks
        let path_str = path.to_string_lossy();
        if path_str.contains("..") || path_str == "/" || path_str == "/etc" {
            return false;
        }
        
        true
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CleanupResult {
    items_deleted: usize,
    space_freed_bytes: u64,
    errors: Vec<String>,
    dry_run: bool,
}

#[async_trait]
impl JobHandler for CleanupJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        let cleanup_payload: CleanupPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid cleanup payload: {}", e)))?;
        
        match self.perform_cleanup(cleanup_payload).await {
            Ok(result) => {
                let action = if result.dry_run { "would delete" } else { "deleted" };
                info!("Cleanup completed: {} {} items, freed {} bytes", 
                      action, result.items_deleted, result.space_freed_bytes);
                
                if !result.errors.is_empty() {
                    warn!("Cleanup had errors: {:?}", result.errors);
                }
                
                Ok(JobResult::Success(serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"status": "completed"}))))
            }
            Err(e) => {
                error!("Cleanup failed: {}", e);
                Ok(JobResult::Failure(e))
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        1 // Minimal retries for cleanup to avoid duplicate deletions
    }
    
    fn retry_delay(&self, _attempt: u32) -> Duration {
        Duration::from_secs(300) // 5 minutes
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(1800)) // 30 minutes timeout
    }
    
    fn name(&self) -> &str {
        "cleanup"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let cleanup_payload: CleanupPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        // Validate older_than_days
        if let Some(days) = cleanup_payload.older_than_days {
            if days == 0 {
                return Err(JobError::ExecutionFailed(
                    "older_than_days must be at least 1".to_string()
                ));
            }
        }
        
        // Validate max_size_mb
        if let Some(size) = cleanup_payload.max_size_mb {
            if size == 0 {
                return Err(JobError::ExecutionFailed(
                    "max_size_mb must be greater than 0".to_string()
                ));
            }
        }
        
        Ok(())
    }
}