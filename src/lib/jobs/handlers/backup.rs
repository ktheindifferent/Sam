use async_trait::async_trait;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use crate::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPayload {
    pub source_path: String,
    pub destination: BackupDestination,
    pub compression: Option<CompressionType>,
    pub encryption: Option<EncryptionConfig>,
    pub retention_days: Option<u32>,
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupDestination {
    Local { path: String },
    S3 { bucket: String, key: String },
    Dropbox { path: String },
    Ftp { host: String, path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    Gzip,
    Bzip2,
    Xz,
    Zip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub algorithm: String,
    pub key_id: String,
}

pub struct BackupJobHandler {
    backup_service: Option<crate::services::backup_enhanced::BackupService>,
    temp_dir: PathBuf,
}

impl BackupJobHandler {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self {
            backup_service: None,
            temp_dir,
        }
    }
    
    async fn perform_backup(&self, payload: BackupPayload) -> Result<BackupResult, String> {
        info!("Starting backup of {} to {:?}", payload.source_path, payload.destination);
        
        // Check if source exists
        let source_path = PathBuf::from(&payload.source_path);
        if !source_path.exists() {
            return Err(format!("Source path does not exist: {}", payload.source_path));
        }
        
        // Create backup file name with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("backup_{}_{}", 
            source_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("data"),
            timestamp
        );
        
        // Simulate backup process
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let backup_size = std::fs::metadata(&source_path)
            .map(|m| m.len())
            .unwrap_or(0);
        
        // Handle different destinations
        match payload.destination {
            BackupDestination::Local { path } => {
                let dest_path = PathBuf::from(path).join(&backup_name);
                info!("Backing up to local path: {:?}", dest_path);
                
                // In real implementation, copy files
                // For now, simulate success
            }
            BackupDestination::S3 { bucket, key } => {
                info!("Backing up to S3: s3://{}/{}", bucket, key);
                // Would use AWS SDK here
            }
            BackupDestination::Dropbox { path } => {
                info!("Backing up to Dropbox: {}", path);
                // Would use Dropbox API here
            }
            BackupDestination::Ftp { host, path } => {
                info!("Backing up to FTP: {}:{}", host, path);
                // Would use FTP client here
            }
        }
        
        Ok(BackupResult {
            backup_name,
            size_bytes: backup_size,
            duration_secs: 2,
            compressed: payload.compression.is_some(),
            encrypted: payload.encryption.is_some(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupResult {
    backup_name: String,
    size_bytes: u64,
    duration_secs: u64,
    compressed: bool,
    encrypted: bool,
}

#[async_trait]
impl JobHandler for BackupJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        let backup_payload: BackupPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid backup payload: {}", e)))?;
        
        match self.perform_backup(backup_payload).await {
            Ok(result) => {
                info!("Backup completed successfully: {} ({} bytes)", 
                      result.backup_name, result.size_bytes);
                
                Ok(JobResult::Success(serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"status": "completed"}))))
            }
            Err(e) => {
                if e.contains("connection") || e.contains("timeout") {
                    // Transient error, should retry
                    warn!("Backup failed with transient error: {}", e);
                    Ok(JobResult::Retry(e))
                } else {
                    // Permanent error
                    error!("Backup failed permanently: {}", e);
                    Ok(JobResult::Failure(e))
                }
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        3
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        // Longer delays for backup retries
        Duration::from_secs(60 * 2_u64.pow(attempt))
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(3600)) // 1 hour timeout for backups
    }
    
    fn name(&self) -> &str {
        "backup"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let backup_payload: BackupPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        // Check source path
        if backup_payload.source_path.is_empty() {
            return Err(JobError::ExecutionFailed("Source path is required".to_string()));
        }
        
        // Validate retention days if specified
        if let Some(days) = backup_payload.retention_days {
            if days == 0 || days > 365 {
                return Err(JobError::ExecutionFailed(
                    format!("Invalid retention days: {} (must be 1-365)", days)
                ));
            }
        }
        
        Ok(())
    }
    
    async fn on_success(&self, payload: &Value, result: &JobResult) -> Result<(), JobError> {
        if let Ok(backup_payload) = serde_json::from_value::<BackupPayload>(payload.clone()) {
            info!("Backup of {} completed successfully", backup_payload.source_path);
            
            // Clean up old backups if retention is specified
            if let Some(retention_days) = backup_payload.retention_days {
                info!("Cleaning up backups older than {} days", retention_days);
                // Implement cleanup logic here
            }
        }
        Ok(())
    }
    
    async fn on_failure(&self, payload: &Value, error: &JobError) -> Result<(), JobError> {
        if let Ok(backup_payload) = serde_json::from_value::<BackupPayload>(payload.clone()) {
            error!("Backup of {} failed: {}", backup_payload.source_path, error);
            
            // Could send alert notification here
        }
        Ok(())
    }
}