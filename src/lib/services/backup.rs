use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tar::{Builder, Archive};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use sha2::{Sha256, Digest};
use log::{info, error};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Backup configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupConfig {
    pub base_path: PathBuf,
    pub schedule: BackupSchedule,
    pub retention: RetentionPolicy,
    pub compression: CompressionConfig,
    pub encryption: EncryptionConfig,
    pub targets: Vec<BackupTarget>,
    pub max_parallel_operations: usize,
    pub verify_after_backup: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupSchedule {
    pub enabled: bool,
    pub daily_at: String,  // HH:MM format
    pub weekly_on: Option<String>,  // Day of week
    pub monthly_on: Option<u32>,  // Day of month
    pub incremental_enabled: bool,
    pub full_backup_interval_days: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionPolicy {
    pub daily_backups: u32,
    pub weekly_backups: u32,
    pub monthly_backups: u32,
    pub yearly_backups: u32,
    pub min_free_space_gb: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    Gzip,
    Zstd,
    Lz4,
    Bzip2,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupTarget {
    pub name: String,
    pub target_type: BackupTargetType,
    pub include_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupTargetType {
    Database,
    FileSystem,
    Configuration,
    Redis,
    Custom(String),
}

impl Default for BackupConfig {
    fn default() -> Self {
        BackupConfig {
            base_path: PathBuf::from("/var/sam/backups"),
            schedule: BackupSchedule {
                enabled: true,
                daily_at: "02:00".to_string(),
                weekly_on: Some("Sunday".to_string()),
                monthly_on: Some(1),
                incremental_enabled: true,
                full_backup_interval_days: 7,
            },
            retention: RetentionPolicy {
                daily_backups: 7,
                weekly_backups: 4,
                monthly_backups: 6,
                yearly_backups: 2,
                min_free_space_gb: 10,
            },
            compression: CompressionConfig {
                enabled: true,
                algorithm: CompressionAlgorithm::Gzip,
                level: 6,
            },
            encryption: EncryptionConfig {
                enabled: false,
                algorithm: "AES-256-GCM".to_string(),
                key_path: None,
            },
            targets: vec![
                BackupTarget {
                    name: "database".to_string(),
                    target_type: BackupTargetType::Database,
                    include_paths: vec![],
                    exclude_patterns: vec![],
                },
                BackupTarget {
                    name: "config".to_string(),
                    target_type: BackupTargetType::Configuration,
                    include_paths: vec![PathBuf::from("/etc/sam")],
                    exclude_patterns: vec!["*.tmp".to_string()],
                },
            ],
            max_parallel_operations: 4,
            verify_after_backup: true,
        }
    }
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub name: String,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub size_bytes: u64,
    pub compressed_size_bytes: Option<u64>,
    pub checksum: String,
    pub targets: Vec<String>,
    pub status: BackupStatus,
    pub error: Option<String>,
    pub parent_backup_id: Option<String>,  // For incremental backups
    pub file_count: u64,
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    InProgress,
    Completed,
    Failed,
    Corrupted,
    Verified,
}

/// Backup service
pub struct BackupService {
    config: BackupConfig,
    backups: Arc<RwLock<HashMap<String, BackupMetadata>>>,
    current_operations: Arc<RwLock<Vec<String>>>,
}

impl BackupService {
    /// Create a new backup service
    pub fn new(config: BackupConfig) -> Self {
        BackupService {
            config,
            backups: Arc::new(RwLock::new(HashMap::new())),
            current_operations: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Perform a full backup
    pub async fn perform_full_backup(&self) -> Result<BackupMetadata, BackupError> {
        let backup_id = uuid::Uuid::new_v4().to_string();
        let backup_name = format!("full_backup_{}", Utc::now().format("%Y%m%d_%H%M%S"));
        let start_time = Utc::now();
        
        info!("Starting full backup: {}", backup_name);
        
        // Register backup operation
        {
            let mut ops = self.current_operations.write().await;
            ops.push(backup_id.clone());
        }
        
        // Create backup metadata
        let mut metadata = BackupMetadata {
            id: backup_id.clone(),
            name: backup_name.clone(),
            backup_type: BackupType::Full,
            created_at: start_time,
            completed_at: None,
            size_bytes: 0,
            compressed_size_bytes: None,
            checksum: String::new(),
            targets: Vec::new(),
            status: BackupStatus::InProgress,
            error: None,
            parent_backup_id: None,
            file_count: 0,
            duration_seconds: None,
        };
        
        // Store initial metadata
        {
            let mut backups = self.backups.write().await;
            backups.insert(backup_id.clone(), metadata.clone());
        }
        
        // Create backup directory
        let backup_path = self.config.base_path.join(&backup_name);
        fs::create_dir_all(&backup_path).await?;
        
        // Backup each target
        let mut total_size = 0u64;
        let mut total_files = 0u64;
        
        for target in &self.config.targets {
            match self.backup_target(target, &backup_path).await {
                Ok((size, files)) => {
                    total_size += size;
                    total_files += files;
                    metadata.targets.push(target.name.clone());
                }
                Err(e) => {
                    error!("Failed to backup target {}: {}", target.name, e);
                    metadata.error = Some(format!("Failed to backup {}: {}", target.name, e));
                }
            }
        }
        
        // Create archive if compression is enabled
        let final_path = if self.config.compression.enabled {
            let archive_path = self.config.base_path.join(format!("{}.tar.gz", backup_name));
            let compressed_size = self.create_compressed_archive(&backup_path, &archive_path).await?;
            
            // Remove uncompressed directory
            fs::remove_dir_all(&backup_path).await?;
            
            metadata.compressed_size_bytes = Some(compressed_size);
            archive_path
        } else {
            backup_path
        };
        
        // Calculate checksum
        metadata.checksum = self.calculate_backup_checksum(&final_path).await?;
        
        // Update metadata
        metadata.size_bytes = total_size;
        metadata.file_count = total_files;
        metadata.completed_at = Some(Utc::now());
        metadata.duration_seconds = Some(
            (Utc::now() - start_time).num_seconds() as u64
        );
        metadata.status = BackupStatus::Completed;
        
        // Verify backup if configured
        if self.config.verify_after_backup {
            if self.verify_backup(&metadata).await? {
                metadata.status = BackupStatus::Verified;
            } else {
                metadata.status = BackupStatus::Corrupted;
                metadata.error = Some("Backup verification failed".to_string());
            }
        }
        
        // Update stored metadata
        {
            let mut backups = self.backups.write().await;
            backups.insert(backup_id.clone(), metadata.clone());
        }
        
        // Remove from current operations
        {
            let mut ops = self.current_operations.write().await;
            ops.retain(|id| id != &backup_id);
        }
        
        // Clean up old backups according to retention policy
        self.cleanup_old_backups().await?;
        
        info!("Completed backup: {} ({})", backup_name, metadata.checksum);
        
        Ok(metadata)
    }
    
    /// Perform an incremental backup
    pub async fn perform_incremental_backup(
        &self,
        parent_backup_id: &str,
    ) -> Result<BackupMetadata, BackupError> {
        let parent = self.get_backup_metadata(parent_backup_id).await?;
        
        let backup_id = uuid::Uuid::new_v4().to_string();
        let backup_name = format!("incremental_backup_{}", Utc::now().format("%Y%m%d_%H%M%S"));
        let start_time = Utc::now();
        
        info!("Starting incremental backup: {} (parent: {})", backup_name, parent_backup_id);
        
        // Create backup metadata
        let mut metadata = BackupMetadata {
            id: backup_id.clone(),
            name: backup_name.clone(),
            backup_type: BackupType::Incremental,
            created_at: start_time,
            completed_at: None,
            size_bytes: 0,
            compressed_size_bytes: None,
            checksum: String::new(),
            targets: Vec::new(),
            status: BackupStatus::InProgress,
            error: None,
            parent_backup_id: Some(parent_backup_id.to_string()),
            file_count: 0,
            duration_seconds: None,
        };
        
        // Perform incremental backup logic
        // This would compare with parent backup and only backup changed files
        
        metadata.completed_at = Some(Utc::now());
        metadata.status = BackupStatus::Completed;
        
        Ok(metadata)
    }
    
    /// Restore from backup
    pub async fn restore_backup(
        &self,
        backup_id: &str,
        restore_path: Option<PathBuf>,
    ) -> Result<RestoreResult, BackupError> {
        let metadata = self.get_backup_metadata(backup_id).await?;
        
        info!("Starting restore from backup: {}", metadata.name);
        
        let restore_path = restore_path.unwrap_or_else(|| PathBuf::from("/"));
        let backup_path = self.get_backup_path(&metadata);
        
        // Verify backup before restore
        if !self.verify_backup(&metadata).await? {
            return Err(BackupError::CorruptedBackup);
        }
        
        // Extract backup
        let extracted_path = if metadata.compressed_size_bytes.is_some() {
            let temp_path = self.config.base_path.join("restore_temp");
            self.extract_compressed_archive(&backup_path, &temp_path).await?;
            temp_path
        } else {
            backup_path
        };
        
        // Restore each target
        let mut restored_files = 0u64;
        let mut restored_bytes = 0u64;
        
        for target_name in &metadata.targets {
            let (files, bytes) = self.restore_target(
                target_name,
                &extracted_path,
                &restore_path,
            ).await?;
            
            restored_files += files;
            restored_bytes += bytes;
        }
        
        // Clean up temp directory if used
        if metadata.compressed_size_bytes.is_some() {
            fs::remove_dir_all(&extracted_path).await?;
        }
        
        info!("Restore completed: {} files, {} bytes", restored_files, restored_bytes);
        
        Ok(RestoreResult {
            backup_id: backup_id.to_string(),
            restored_files,
            restored_bytes,
            duration_seconds: 0,
            errors: Vec::new(),
        })
    }
    
    /// List available backups
    pub async fn list_backups(&self) -> Vec<BackupMetadata> {
        let backups = self.backups.read().await;
        let mut list: Vec<BackupMetadata> = backups.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }
    
    /// Get backup metadata
    async fn get_backup_metadata(&self, backup_id: &str) -> Result<BackupMetadata, BackupError> {
        let backups = self.backups.read().await;
        backups
            .get(backup_id)
            .cloned()
            .ok_or(BackupError::BackupNotFound)
    }
    
    /// Backup a single target
    async fn backup_target(
        &self,
        target: &BackupTarget,
        backup_path: &Path,
    ) -> Result<(u64, u64), BackupError> {
        let target_path = backup_path.join(&target.name);
        fs::create_dir_all(&target_path).await?;
        
        match &target.target_type {
            BackupTargetType::Database => {
                self.backup_database(&target_path).await
            }
            BackupTargetType::FileSystem => {
                self.backup_filesystem(target, &target_path).await
            }
            BackupTargetType::Configuration => {
                self.backup_filesystem(target, &target_path).await
            }
            BackupTargetType::Redis => {
                self.backup_redis(&target_path).await
            }
            BackupTargetType::Custom(cmd) => {
                self.backup_custom(cmd, &target_path).await
            }
        }
    }
    
    /// Backup database
    async fn backup_database(&self, target_path: &Path) -> Result<(u64, u64), BackupError> {
        use std::process::Command;
        
        let dump_file = target_path.join("database.sql");
        
        // Execute pg_dump
        let output = Command::new("pg_dump")
            .args([
                "-h", "localhost",
                "-U", "sam",
                "-d", "sam_db",
                "-f", dump_file.to_str().unwrap(),
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(BackupError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        let metadata = fs::metadata(&dump_file).await?;
        Ok((metadata.len(), 1))
    }
    
    /// Backup filesystem
    async fn backup_filesystem(
        &self,
        target: &BackupTarget,
        target_path: &Path,
    ) -> Result<(u64, u64), BackupError> {
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        
        for include_path in &target.include_paths {
            if include_path.exists() {
                let (size, count) = self.copy_directory(
                    include_path,
                    target_path,
                    &target.exclude_patterns,
                ).await?;
                
                total_size += size;
                file_count += count;
            }
        }
        
        Ok((total_size, file_count))
    }
    
    /// Backup Redis
    async fn backup_redis(&self, target_path: &Path) -> Result<(u64, u64), BackupError> {
        use std::process::Command;
        
        let dump_file = target_path.join("redis.rdb");
        
        // Execute redis-cli BGSAVE
        Command::new("redis-cli")
            .args(["BGSAVE"])
            .output()?;
        
        // Wait for save to complete
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Copy dump file
        let redis_dump = PathBuf::from("/var/lib/redis/dump.rdb");
        if redis_dump.exists() {
            fs::copy(&redis_dump, &dump_file).await?;
            let metadata = fs::metadata(&dump_file).await?;
            Ok((metadata.len(), 1))
        } else {
            Ok((0, 0))
        }
    }
    
    /// Execute custom backup command
    async fn backup_custom(&self, command: &str, target_path: &Path) -> Result<(u64, u64), BackupError> {
        use std::process::Command;
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(target_path)
            .output()?;
        
        if !output.status.success() {
            return Err(BackupError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Count files in target directory
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        
        let mut entries = fs::read_dir(target_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    total_size += metadata.len();
                    file_count += 1;
                }
            }
        }
        
        Ok((total_size, file_count))
    }
    
    /// Copy directory recursively
    async fn copy_directory(
        &self,
        source: &Path,
        dest: &Path,
        exclude_patterns: &[String],
    ) -> Result<(u64, u64), BackupError> {
        let total_size = 0u64;
        let file_count = 0u64;
        
        // Recursive copy implementation would go here
        // This is a simplified version
        
        Ok((total_size, file_count))
    }
    
    /// Create compressed archive
    async fn create_compressed_archive(
        &self,
        source_dir: &Path,
        archive_path: &Path,
    ) -> Result<u64, BackupError> {
        use std::fs::File;
        
        let tar_gz = File::create(archive_path)?;
        let encoder = GzEncoder::new(tar_gz, Compression::default());
        let mut archive = Builder::new(encoder);
        
        archive.append_dir_all(".", source_dir)?;
        archive.finish()?;
        
        let metadata = fs::metadata(archive_path).await?;
        Ok(metadata.len())
    }
    
    /// Extract compressed archive
    async fn extract_compressed_archive(
        &self,
        archive_path: &Path,
        dest_dir: &Path,
    ) -> Result<(), BackupError> {
        use std::fs::File;
        
        fs::create_dir_all(dest_dir).await?;
        
        let tar_gz = File::open(archive_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(decoder);
        
        archive.unpack(dest_dir)?;
        
        Ok(())
    }
    
    /// Calculate backup checksum
    async fn calculate_backup_checksum(&self, path: &Path) -> Result<String, BackupError> {
        let mut hasher = Sha256::new();
        
        if path.is_file() {
            let mut file = fs::File::open(path).await?;
            let mut buffer = vec![0; 8192];
            
            loop {
                let n = file.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
        } else {
            // For directories, hash file list and sizes
            let entries = self.list_directory_recursive(path).await?;
            for entry in entries {
                hasher.update(entry.as_bytes());
            }
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// List directory recursively
    fn list_directory_recursive(&self, path: &Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, BackupError>> + Send + '_>> {
        Box::pin(Self::list_directory_recursive_impl(path.to_path_buf()))
    }
    
    /// Implementation for recursive directory listing  
    fn list_directory_recursive_impl(path: PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, BackupError>> + Send>> {
        Box::pin(async move {
        let mut entries = Vec::new();
        
        let mut dir_entries = fs::read_dir(&path).await?;
        while let Some(entry) = dir_entries.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let sub_entries = Self::list_directory_recursive_impl(entry_path).await?;
                entries.extend(sub_entries);
            } else {
                entries.push(entry_path.to_string_lossy().to_string());
            }
        }
        
        entries.sort();
        Ok(entries)
        })
    }
    
    /// Verify backup integrity
    async fn verify_backup(&self, metadata: &BackupMetadata) -> Result<bool, BackupError> {
        let backup_path = self.get_backup_path(metadata);
        let calculated_checksum = self.calculate_backup_checksum(&backup_path).await?;
        Ok(calculated_checksum == metadata.checksum)
    }
    
    /// Get backup path from metadata
    fn get_backup_path(&self, metadata: &BackupMetadata) -> PathBuf {
        if metadata.compressed_size_bytes.is_some() {
            self.config.base_path.join(format!("{}.tar.gz", metadata.name))
        } else {
            self.config.base_path.join(&metadata.name)
        }
    }
    
    /// Restore a target
    async fn restore_target(
        &self,
        target_name: &str,
        source_path: &Path,
        restore_path: &Path,
    ) -> Result<(u64, u64), BackupError> {
        // Restoration logic would go here
        Ok((0, 0))
    }
    
    /// Clean up old backups according to retention policy
    async fn cleanup_old_backups(&self) -> Result<(), BackupError> {
        let backups = self.backups.read().await;
        let mut sorted_backups: Vec<_> = backups.values().cloned().collect();
        sorted_backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Group backups by type
        let daily_backups: Vec<_> = sorted_backups
            .iter()
            .filter(|b| b.created_at > Utc::now() - Duration::days(1))
            .collect();
        
        // Keep only configured number of backups
        if daily_backups.len() > self.config.retention.daily_backups as usize {
            for backup in &daily_backups[self.config.retention.daily_backups as usize..] {
                self.delete_backup(&backup.id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Delete a backup
    async fn delete_backup(&self, backup_id: &str) -> Result<(), BackupError> {
        let metadata = self.get_backup_metadata(backup_id).await?;
        let backup_path = self.get_backup_path(&metadata);
        
        if backup_path.exists() {
            if backup_path.is_dir() {
                fs::remove_dir_all(&backup_path).await?;
            } else {
                fs::remove_file(&backup_path).await?;
            }
        }
        
        let mut backups = self.backups.write().await;
        backups.remove(backup_id);
        
        info!("Deleted backup: {}", metadata.name);
        
        Ok(())
    }
}

/// Restore result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub backup_id: String,
    pub restored_files: u64,
    pub restored_bytes: u64,
    pub duration_seconds: u64,
    pub errors: Vec<String>,
}

/// Backup errors
#[derive(Debug)]
pub enum BackupError {
    IoError(std::io::Error),
    BackupNotFound,
    CorruptedBackup,
    InsufficientSpace,
    CommandFailed(String),
    RestoreFailed(String),
}

impl From<std::io::Error> for BackupError {
    fn from(err: std::io::Error) -> Self {
        BackupError::IoError(err)
    }
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::IoError(e) => write!(f, "IO error: {}", e),
            BackupError::BackupNotFound => write!(f, "Backup not found"),
            BackupError::CorruptedBackup => write!(f, "Backup is corrupted"),
            BackupError::InsufficientSpace => write!(f, "Insufficient disk space"),
            BackupError::CommandFailed(e) => write!(f, "Command failed: {}", e),
            BackupError::RestoreFailed(e) => write!(f, "Restore failed: {}", e),
        }
    }
}

impl std::error::Error for BackupError {}

/// Start the backup scheduler
pub async fn start_scheduler() -> anyhow::Result<()> {
    log::info!("Backup scheduler started");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_config_default() {
        let config = BackupConfig::default();
        assert!(config.schedule.enabled);
        assert_eq!(config.retention.daily_backups, 7);
        assert!(config.compression.enabled);
    }

    #[test]
    fn test_backup_metadata() {
        let metadata = BackupMetadata {
            id: "test-id".to_string(),
            name: "test-backup".to_string(),
            backup_type: BackupType::Full,
            created_at: Utc::now(),
            completed_at: None,
            size_bytes: 1024,
            compressed_size_bytes: Some(512),
            checksum: "abc123".to_string(),
            targets: vec!["database".to_string()],
            status: BackupStatus::Completed,
            error: None,
            parent_backup_id: None,
            file_count: 10,
            duration_seconds: Some(60),
        };
        
        assert_eq!(metadata.id, "test-id");
        assert_eq!(metadata.file_count, 10);
    }
}