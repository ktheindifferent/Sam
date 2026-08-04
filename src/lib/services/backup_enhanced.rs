// Enhanced Backup Service Implementation
// Adds missing backup execution, restore functionality, and verification

use super::backup::*;
use super::error_handling::ServiceError;
use super::monitoring::{HealthCheck, HealthCheckable, HealthStatus, MetricsCollector};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use hex;
use log::{error, info, warn};
use nanoid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::{Archive, Builder};
use tokio::fs;
use tokio::sync::{RwLock, Semaphore};

// ==================== Backup Metadata ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub backup_type: BackupType,
    pub size_bytes: u64,
    pub checksum: String,
    pub targets: Vec<BackupTargetInfo>,
    pub compression: Option<CompressionInfo>,
    pub encryption: Option<EncryptionInfo>,
    pub parent_backup_id: Option<String>, // For incremental backups
    pub retention_policy: RetentionPolicy,
    pub verified: bool,
    pub restore_points: Vec<RestorePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupTargetInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub level: u32,
    pub original_size: u64,
    pub compressed_size: u64,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    pub algorithm: String,
    pub key_id: String,
    pub iv: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePoint {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ==================== Backup Service Implementation ====================

pub struct BackupService {
    config: Arc<BackupConfig>,
    metadata_store: Arc<RwLock<HashMap<String, BackupMetadata>>>,
    metrics: Arc<MetricsCollector>,
    semaphore: Arc<Semaphore>,
    is_running: Arc<RwLock<bool>>,
}

impl BackupService {
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config: Arc::new(config.clone()),
            metadata_store: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(MetricsCollector::new("backup_service".to_string())),
            semaphore: Arc::new(Semaphore::new(config.max_parallel_operations)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Execute a full backup
    pub async fn execute_full_backup(&self) -> Result<BackupMetadata> {
        info!("Starting full backup");

        // Check if backup is already running
        if *self.is_running.read().await {
            return Err(
                ServiceError::ServiceUnavailable("Backup already in progress".to_string()).into(),
            );
        }
        *self.is_running.write().await = true;

        let backup_id = nanoid::nanoid!();
        let timestamp = Utc::now();
        let backup_dir = self.get_backup_path(&backup_id, &timestamp);

        // Create backup directory
        fs::create_dir_all(&backup_dir)
            .await
            .context("Failed to create backup directory")?;

        let mut backup_targets = Vec::new();
        let mut total_size = 0u64;

        // Process each target
        for target in &self.config.targets {
            let _permit = self.semaphore.acquire().await?;

            let target_info = self.backup_target(target, &backup_dir).await?;
            total_size += target_info.size_bytes;
            backup_targets.push(target_info);

            // Update metrics
            self.metrics
                .increment_counter("backup_targets_processed", HashMap::new())
                .await;
        }

        // Create archive if compression is enabled
        let (final_path, compression_info) = if self.config.compression.enabled {
            self.compress_backup(&backup_dir, &backup_id).await?
        } else {
            (backup_dir.clone(), None)
        };

        // Calculate checksum
        let checksum = self.calculate_checksum(&final_path).await?;

        // Create metadata
        let metadata = BackupMetadata {
            id: backup_id.clone(),
            timestamp,
            backup_type: BackupType::Full,
            size_bytes: total_size,
            checksum,
            targets: backup_targets,
            compression: compression_info,
            encryption: None, // TODO: Implement encryption
            parent_backup_id: None,
            retention_policy: self.config.retention.clone(),
            verified: false,
            restore_points: vec![],
        };

        // Verify if configured
        if self.config.verify_after_backup {
            self.verify_backup(&metadata).await?;
        }

        // Store metadata
        self.store_metadata(&metadata).await?;

        // Clean up old backups according to retention policy
        self.apply_retention_policy().await?;

        *self.is_running.write().await = false;

        info!("Full backup completed: {}", backup_id);
        self.metrics
            .increment_counter("backups_completed", HashMap::new())
            .await;

        Ok(metadata)
    }

    /// Execute an incremental backup
    pub async fn execute_incremental_backup(&self, parent_id: &str) -> Result<BackupMetadata> {
        info!("Starting incremental backup based on parent: {}", parent_id);

        // Get parent metadata
        let parent_metadata = self
            .get_metadata(parent_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Parent backup not found"))?;

        let backup_id = nanoid::nanoid!();
        let timestamp = Utc::now();
        let backup_dir = self.get_backup_path(&backup_id, &timestamp);

        fs::create_dir_all(&backup_dir).await?;

        let mut backup_targets = Vec::new();
        let mut total_size = 0u64;

        // Process only changed files
        for target in &self.config.targets {
            let target_info = self
                .backup_incremental_target(target, &backup_dir, &parent_metadata.timestamp)
                .await?;

            total_size += target_info.size_bytes;
            backup_targets.push(target_info);
        }

        let checksum = self.calculate_checksum(&backup_dir).await?;

        let metadata = BackupMetadata {
            id: backup_id.clone(),
            timestamp,
            backup_type: BackupType::Incremental,
            size_bytes: total_size,
            checksum,
            targets: backup_targets,
            compression: None,
            encryption: None,
            parent_backup_id: Some(parent_id.to_string()),
            retention_policy: self.config.retention.clone(),
            verified: false,
            restore_points: vec![],
        };

        self.store_metadata(&metadata).await?;

        info!("Incremental backup completed: {}", backup_id);
        Ok(metadata)
    }

    /// Restore from backup
    pub async fn restore_backup(&self, backup_id: &str, restore_path: &Path) -> Result<()> {
        info!("Starting restore from backup: {}", backup_id);

        let metadata = self
            .get_metadata(backup_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Backup not found"))?;

        // Verify backup before restore
        self.verify_backup(&metadata).await?;

        let backup_path = self.get_backup_path(&metadata.id, &metadata.timestamp);

        // Decompress if needed
        let source_path = if metadata.compression.is_some() {
            self.decompress_backup(&backup_path.with_extension("tar.gz"), backup_id)
                .await?
        } else {
            backup_path
        };

        // Restore each target
        for target_info in &metadata.targets {
            let target_backup_path = source_path.join(&target_info.name);
            let target_restore_path = restore_path.join(&target_info.name);

            self.restore_target(&target_backup_path, &target_restore_path)
                .await?;

            info!(
                "Restored target: {} to {:?}",
                target_info.name, target_restore_path
            );
        }

        self.metrics
            .increment_counter("restores_completed", HashMap::new())
            .await;
        info!("Restore completed successfully");

        Ok(())
    }

    /// Verify backup integrity
    pub async fn verify_backup(&self, metadata: &BackupMetadata) -> Result<()> {
        info!("Verifying backup: {}", metadata.id);

        let backup_path = self.get_backup_path(&metadata.id, &metadata.timestamp);
        let backup_artifact_path = if metadata.compression.is_some() {
            backup_path.with_extension("tar.gz")
        } else {
            backup_path.clone()
        };

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&backup_artifact_path).await?;
        if calculated_checksum != metadata.checksum {
            return Err(anyhow::anyhow!(
                "Backup verification failed: checksum mismatch"
            ));
        }

        if metadata.compression.is_some() {
            info!("Compressed backup archive verification successful");
            return Ok(());
        }

        // Verify each target
        for target in &metadata.targets {
            let target_path = backup_path.join(&target.name);
            if !target_path.exists() {
                return Err(anyhow::anyhow!("Target missing: {}", target.name));
            }

            // Verify target checksum
            let target_checksum = self.calculate_checksum(&target_path).await?;
            if target_checksum != target.checksum {
                return Err(anyhow::anyhow!(
                    "Target verification failed: {}",
                    target.name
                ));
            }
        }

        info!("Backup verification successful");
        Ok(())
    }

    /// Apply retention policy to remove old backups
    pub async fn apply_retention_policy(&self) -> Result<()> {
        let mut metadata_store = self.metadata_store.write().await;
        let mut backups: Vec<BackupMetadata> = metadata_store.values().cloned().collect();

        // Sort by timestamp (newest first)
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let now = Utc::now();
        let mut daily_count = 0;
        let mut weekly_count = 0;
        let mut monthly_count = 0;
        let mut yearly_count = 0;

        let mut to_delete = Vec::new();

        for backup in backups {
            let age = now.signed_duration_since(backup.timestamp);

            // Determine retention category
            if age < ChronoDuration::days(1) || daily_count < self.config.retention.daily_backups {
                daily_count += 1;
            } else if age < ChronoDuration::weeks(1)
                || weekly_count < self.config.retention.weekly_backups
            {
                weekly_count += 1;
            } else if age < ChronoDuration::days(30)
                || monthly_count < self.config.retention.monthly_backups
            {
                monthly_count += 1;
            } else if yearly_count < self.config.retention.yearly_backups {
                yearly_count += 1;
            } else {
                to_delete.push(backup.id.clone());
            }
        }

        // Delete old backups
        for backup_id in to_delete {
            if let Some(metadata) = metadata_store.remove(&backup_id) {
                let backup_path = self.get_backup_path(&metadata.id, &metadata.timestamp);
                if let Err(e) = fs::remove_dir_all(&backup_path).await {
                    error!("Failed to delete old backup {}: {}", backup_id, e);
                } else {
                    info!("Deleted old backup: {}", backup_id);
                    self.metrics
                        .increment_counter("backups_deleted", HashMap::new())
                        .await;
                }
            }
        }

        Ok(())
    }

    // ==================== Helper Methods ====================

    async fn backup_target(
        &self,
        target: &BackupTarget,
        backup_dir: &Path,
    ) -> Result<BackupTargetInfo> {
        let target_backup_dir = backup_dir.join(&target.name);
        fs::create_dir_all(&target_backup_dir).await?;

        let mut total_size = 0u64;
        let mut file_count = 0usize;
        let mut hasher = Sha256::default();

        for include_path in &target.include_paths {
            if include_path.is_file() {
                let file_name = include_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid file path: no file name"))?;
                let dest = target_backup_dir.join(file_name);
                fs::copy(include_path, &dest).await?;

                let metadata = fs::metadata(&dest).await?;
                total_size += metadata.len();
                file_count += 1;

                let content = fs::read(&dest).await?;
                hasher.update(&content);
            } else if include_path.is_dir() {
                self.backup_directory(
                    include_path,
                    &target_backup_dir,
                    &mut total_size,
                    &mut file_count,
                    &mut hasher,
                )
                .await?;
            }
        }

        let checksum = format!("{:x}", hasher.finalize());

        Ok(BackupTargetInfo {
            name: target.name.clone(),
            path: target_backup_dir,
            size_bytes: total_size,
            file_count,
            checksum,
        })
    }

    async fn backup_directory(
        &self,
        source: &Path,
        dest: &Path,
        total_size: &mut u64,
        file_count: &mut usize,
        hasher: &mut Sha256,
    ) -> Result<()> {
        let mut entries = fs::read_dir(source).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let relative = path.strip_prefix(source)?;
            let dest_path = dest.join(relative);

            if path.is_file() {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).await?;
                }

                fs::copy(&path, &dest_path).await?;
                let metadata = fs::metadata(&dest_path).await?;
                *total_size += metadata.len();
                *file_count += 1;

                let content = fs::read(&dest_path).await?;
                hasher.update(&content);
            } else if path.is_dir() {
                fs::create_dir_all(&dest_path).await?;
                Box::pin(self.backup_directory(&path, &dest_path, total_size, file_count, hasher))
                    .await?;
            }
        }

        Ok(())
    }

    async fn backup_incremental_target(
        &self,
        target: &BackupTarget,
        backup_dir: &Path,
        since: &DateTime<Utc>,
    ) -> Result<BackupTargetInfo> {
        let target_backup_dir = backup_dir.join(&target.name);
        fs::create_dir_all(&target_backup_dir).await?;

        let mut total_size = 0u64;
        let mut file_count = 0usize;
        let mut hasher = Sha256::default();

        for include_path in &target.include_paths {
            self.backup_changed_files(
                include_path,
                &target_backup_dir,
                since,
                &mut total_size,
                &mut file_count,
                &mut hasher,
            )
            .await?;
        }

        let checksum = format!("{:x}", hasher.finalize());

        Ok(BackupTargetInfo {
            name: target.name.clone(),
            path: target_backup_dir,
            size_bytes: total_size,
            file_count,
            checksum,
        })
    }

    async fn backup_changed_files(
        &self,
        source: &Path,
        dest: &Path,
        since: &DateTime<Utc>,
        total_size: &mut u64,
        file_count: &mut usize,
        hasher: &mut Sha256,
    ) -> Result<()> {
        if source.is_file() {
            let metadata = fs::metadata(source).await?;
            let modified = metadata.modified()?;
            let modified_datetime = DateTime::<Utc>::from(modified);

            if modified_datetime > *since {
                let file_name = source
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid source path: no file name"))?;
                let dest_file = dest.join(file_name);
                fs::copy(source, &dest_file).await?;
                *total_size += metadata.len();
                *file_count += 1;

                let content = fs::read(&dest_file).await?;
                hasher.update(&content);
            }
        } else if source.is_dir() {
            let mut entries = fs::read_dir(source).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let relative = path.strip_prefix(source)?;
                let dest_path = dest.join(relative);

                Box::pin(self.backup_changed_files(
                    &path, &dest_path, since, total_size, file_count, hasher,
                ))
                .await?;
            }
        }

        Ok(())
    }

    async fn restore_target(&self, source: &Path, dest: &Path) -> Result<()> {
        if source.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::copy(source, dest).await?;
        } else if source.is_dir() {
            fs::create_dir_all(dest).await?;

            let mut entries = fs::read_dir(source).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let relative = path.strip_prefix(source)?;
                let dest_path = dest.join(relative);

                Box::pin(self.restore_target(&path, &dest_path)).await?;
            }
        }

        Ok(())
    }

    async fn compress_backup(
        &self,
        backup_dir: &Path,
        _backup_id: &str,
    ) -> Result<(PathBuf, Option<CompressionInfo>)> {
        let archive_path = backup_dir.with_extension("tar.gz");
        let tar_gz = std::fs::File::create(&archive_path)?;
        let encoder = GzEncoder::new(tar_gz, Compression::new(self.config.compression.level));
        let mut tar = Builder::new(encoder);

        tar.append_dir_all(".", backup_dir)?;
        tar.finish()?;

        let original_size = self.calculate_dir_size(backup_dir).await?;
        let compressed_size = fs::metadata(&archive_path).await?.len();

        // Remove uncompressed directory
        fs::remove_dir_all(backup_dir).await?;

        let compression_info = CompressionInfo {
            algorithm: format!("{:?}", self.config.compression.algorithm),
            level: self.config.compression.level,
            original_size,
            compressed_size,
            ratio: compressed_size as f64 / original_size as f64,
        };

        Ok((archive_path, Some(compression_info)))
    }

    async fn decompress_backup(&self, archive_path: &Path, _backup_id: &str) -> Result<PathBuf> {
        let extract_dir = archive_path.with_extension("");
        fs::create_dir_all(&extract_dir).await?;

        let tar_gz = std::fs::File::open(archive_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(decoder);

        archive.unpack(&extract_dir)?;

        Ok(extract_dir)
    }

    async fn calculate_checksum(&self, path: &Path) -> Result<String> {
        let mut hasher = Sha256::default();

        if path.is_file() {
            let content = fs::read(path).await?;
            hasher.update(&content);
        } else if path.is_dir() {
            self.hash_directory(path, &mut hasher).await?;
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn hash_directory(&self, dir: &Path, hasher: &mut Sha256) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;
        let mut paths = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            paths.push(entry.path());
        }

        // Sort for consistent hashing
        paths.sort();

        for path in paths {
            if path.is_file() {
                let content = fs::read(&path).await?;
                hasher.update(&content);
            } else if path.is_dir() {
                Box::pin(self.hash_directory(&path, hasher)).await?;
            }
        }

        Ok(())
    }

    async fn calculate_dir_size(&self, dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = fs::metadata(&path).await?;

            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += Box::pin(self.calculate_dir_size(&path)).await?;
            }
        }

        Ok(total_size)
    }

    fn get_backup_path(&self, backup_id: &str, timestamp: &DateTime<Utc>) -> PathBuf {
        self.config
            .base_path
            .join(timestamp.format("%Y-%m-%d").to_string())
            .join(backup_id)
    }

    async fn store_metadata(&self, metadata: &BackupMetadata) -> Result<()> {
        self.metadata_store
            .write()
            .await
            .insert(metadata.id.clone(), metadata.clone());

        // Also persist to disk
        let metadata_file = self.config.base_path.join("metadata.json");
        let all_metadata: Vec<BackupMetadata> =
            self.metadata_store.read().await.values().cloned().collect();
        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(metadata_file, json).await?;

        Ok(())
    }

    async fn get_metadata(&self, backup_id: &str) -> Option<BackupMetadata> {
        self.metadata_store.read().await.get(backup_id).cloned()
    }

    pub async fn list_backups(&self) -> Vec<BackupMetadata> {
        self.metadata_store.read().await.values().cloned().collect()
    }
}

// ==================== Health Check Implementation ====================

#[async_trait::async_trait]
impl HealthCheckable for BackupService {
    async fn check(&self) -> Result<HealthCheck> {
        let mut health = HealthCheck {
            name: "backup_service".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            last_check: Utc::now(),
            response_time_ms: 0,
            metadata: HashMap::new(),
        };

        // Check if backup directory is accessible
        if let Err(e) = fs::metadata(&self.config.base_path).await {
            health.status =
                HealthStatus::Unhealthy(format!("Backup directory inaccessible: {}", e));
            return Ok(health);
        }

        // Check available disk space
        let available_space = self.get_available_space().await?;
        let min_space_bytes = self.config.retention.min_free_space_gb * 1024 * 1024 * 1024;

        if available_space < min_space_bytes {
            health.status = HealthStatus::Degraded(format!(
                "Low disk space: {} GB available, {} GB required",
                available_space / (1024 * 1024 * 1024),
                self.config.retention.min_free_space_gb
            ));
        }

        // Add metadata
        health.metadata.insert(
            "backup_count".to_string(),
            self.metadata_store.read().await.len().to_string(),
        );
        health.metadata.insert(
            "is_running".to_string(),
            self.is_running.read().await.to_string(),
        );
        health.metadata.insert(
            "available_space_gb".to_string(),
            (available_space / (1024 * 1024 * 1024)).to_string(),
        );

        Ok(health)
    }

    fn name(&self) -> String {
        "backup_service".to_string()
    }
}

impl BackupService {
    async fn get_available_space(&self) -> Result<u64> {
        // Platform-specific implementation would go here
        // For now, return a dummy value
        Ok(100 * 1024 * 1024 * 1024) // 100 GB
    }
}

// Add required imports for encryption
use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::engine::general_purpose;
use base64::Engine;

impl BackupService {
    // ==================== Encryption Methods ====================

    /// Encrypt a file using AES-256-GCM
    #[allow(dead_code)]
    async fn encrypt_file(&self, file_path: &Path, _backup_id: &str) -> Result<PathBuf> {
        let key = self.get_or_create_encryption_key().await?;
        let cipher = Aes256Gcm::new(&key);

        // Read the file
        let plaintext = fs::read(file_path).await?;

        // Generate a random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Create encrypted file path
        let encrypted_path = file_path.with_extension("enc");

        // Write encrypted data with nonce prepended
        let mut encrypted_data = Vec::new();
        encrypted_data.extend_from_slice(nonce);
        encrypted_data.extend_from_slice(&ciphertext);

        fs::write(&encrypted_path, encrypted_data).await?;

        // Remove original file
        fs::remove_file(file_path).await?;

        info!("Encrypted backup file: {:?}", encrypted_path);
        Ok(encrypted_path)
    }

    /// Decrypt a file using AES-256-GCM
    #[allow(dead_code)]
    async fn decrypt_file(&self, file_path: &Path, _backup_id: &str) -> Result<PathBuf> {
        let key = self.get_or_create_encryption_key().await?;
        let cipher = Aes256Gcm::new(&key);

        // Read the encrypted file
        let encrypted_data = fs::read(file_path).await?;

        if encrypted_data.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted file: too short"));
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the data
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        // Create decrypted file path
        let decrypted_path = file_path.with_extension("tar.gz");

        // Write decrypted data
        fs::write(&decrypted_path, plaintext).await?;

        info!("Decrypted backup file: {:?}", decrypted_path);
        Ok(decrypted_path)
    }

    /// Get or create the encryption key
    #[allow(dead_code)]
    async fn get_or_create_encryption_key(&self) -> Result<Key<Aes256Gcm>> {
        // Try to get key from environment variable
        if let Ok(key_str) = std::env::var("BACKUP_ENCRYPTION_KEY") {
            let key_bytes = general_purpose::STANDARD
                .decode(&key_str)
                .context("Failed to decode encryption key from base64")?;

            if key_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid key length: expected 32 bytes"));
            }

            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            return Ok(Key::<Aes256Gcm>::from(key_array));
        }

        // Try to read key from secure file
        let key_file = Path::new("/etc/sam/backup_key");
        if key_file.exists() {
            let key_data = fs::read_to_string(key_file).await?;
            let key_bytes = general_purpose::STANDARD
                .decode(key_data.trim())
                .context("Failed to decode key from file")?;

            if key_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid key in file: expected 32 bytes"));
            }

            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            return Ok(Key::<Aes256Gcm>::from(key_array));
        }

        // Generate a new key
        let key = Aes256Gcm::generate_key(&mut OsRng);

        // Save the key securely
        let key_str = general_purpose::STANDARD.encode(key.as_slice());

        // Create directory if it doesn't exist
        fs::create_dir_all("/etc/sam").await?;

        // Write key with secure permissions
        fs::write(key_file, &key_str).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(key_file, permissions)?;
        }

        warn!("Generated new encryption key and saved to /etc/sam/backup_key");
        Ok(key)
    }

    /// Get a key identifier for metadata
    #[allow(dead_code)]
    fn get_key_id(&self) -> String {
        // Generate a key ID based on the key's hash
        // This allows tracking which key was used without exposing the key
        if let Ok(key) = std::env::var("BACKUP_ENCRYPTION_KEY") {
            let mut hasher = Sha256::default();
            hasher.update(key.as_bytes());
            let hash = hasher.finalize();
            hex::encode(&hash[..8])
        } else {
            "default".to_string()
        }
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_backup_and_restore() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for tests");
        let backup_dir = TempDir::new().expect("Failed to create backup directory for tests");
        let restore_dir = TempDir::new().expect("Failed to create restore directory for tests");

        // Create test files
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")
            .await
            .expect("Failed to write test file");

        // Configure backup
        let mut config = BackupConfig::default();
        config.base_path = backup_dir.path().to_path_buf();
        config.targets = vec![BackupTarget {
            name: "test_target".to_string(),
            target_type: BackupTargetType::FileSystem,
            include_paths: vec![test_file.clone()],
            exclude_patterns: vec![],
        }];

        let service = BackupService::new(config);

        // Execute backup
        let metadata = service
            .execute_full_backup()
            .await
            .expect("Failed to execute backup");
        assert_eq!(metadata.backup_type, BackupType::Full);

        // Verify backup
        service
            .verify_backup(&metadata)
            .await
            .expect("Failed to verify backup");

        // Restore backup
        service
            .restore_backup(&metadata.id, restore_dir.path())
            .await
            .expect("Failed to restore backup");

        // Verify restored file
        let restored_file = restore_dir.path().join("test_target").join("test.txt");
        assert!(restored_file.exists());

        let content = fs::read_to_string(restored_file)
            .await
            .expect("Failed to read restored file");
        assert_eq!(content, "test content");
    }
}
