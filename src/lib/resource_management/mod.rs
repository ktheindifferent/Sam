use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::{RwLock, Semaphore};
// use tokio::io::{AsyncWriteExt};
use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::time::interval;
pub mod cleanup;
pub mod limits;
pub mod monitoring;
pub mod pool;

pub use cleanup::{CleanupGuard, ResourceCleanup, TempFile};
pub use limits::ResourceLimits;
pub use monitoring::{ResourceMetrics, ResourceMonitor};
pub use pool::{ConnectionPool, PooledConnection};

/// Resource management configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ResourceConfig {
    /// File upload limits
    pub file_limits: FileLimits,
    /// Request processing limits
    pub request_limits: RequestLimits,
    /// Connection pool configuration
    pub pool_config: PoolConfig,
    /// Cleanup configuration
    pub cleanup_config: CleanupConfig,
    /// Memory limits
    pub memory_limits: MemoryLimits,
}

/// File upload limits configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileLimits {
    /// Maximum file size in bytes (default: 100MB)
    pub max_file_size: usize,
    /// Maximum concurrent uploads per user
    pub max_concurrent_uploads: usize,
    /// Maximum total storage per user in bytes
    pub max_user_storage: usize,
    /// Allowed file extensions (empty = all allowed)
    pub allowed_extensions: Vec<String>,
    /// Blocked file extensions
    pub blocked_extensions: Vec<String>,
    /// Enable virus scanning
    pub enable_virus_scan: bool,
    /// Temporary file cleanup interval in seconds
    pub temp_cleanup_interval: u64,
    /// Temporary file max age in seconds
    pub temp_max_age: u64,
}

impl Default for FileLimits {
    fn default() -> Self {
        FileLimits {
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_concurrent_uploads: 10,
            max_user_storage: 10 * 1024 * 1024 * 1024, // 10GB
            allowed_extensions: vec![],
            blocked_extensions: vec![
                ".exe".to_string(),
                ".dll".to_string(),
                ".bat".to_string(),
                ".cmd".to_string(),
                ".com".to_string(),
                ".scr".to_string(),
                ".vbs".to_string(),
                ".js".to_string(),
            ],
            enable_virus_scan: true,
            temp_cleanup_interval: 3600, // 1 hour
            temp_max_age: 86400,         // 24 hours
        }
    }
}

/// Request processing limits
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestLimits {
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Maximum request processing time in seconds
    pub max_processing_time: u64,
    /// Maximum concurrent requests per IP
    pub max_concurrent_per_ip: usize,
    /// Maximum header size in bytes
    pub max_header_size: usize,
    /// Enable request cancellation on client disconnect
    pub enable_cancellation: bool,
}

impl Default for RequestLimits {
    fn default() -> Self {
        RequestLimits {
            max_body_size: 10 * 1024 * 1024, // 10MB
            max_processing_time: 300,        // 5 minutes
            max_concurrent_per_ip: 100,
            max_header_size: 8192, // 8KB
            enable_cancellation: true,
        }
    }
}

/// Connection pool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolConfig {
    /// Maximum connections in pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Idle timeout in seconds
    pub idle_timeout: u64,
    /// Maximum lifetime in seconds
    pub max_lifetime: u64,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Enable circuit breaker
    pub enable_circuit_breaker: bool,
    /// Circuit breaker threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker reset timeout in seconds
    pub circuit_breaker_reset_timeout: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            max_connections: 100,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
            health_check_interval: 60,
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_timeout: 60,
        }
    }
}

/// Cleanup configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CleanupConfig {
    /// Enable automatic cleanup
    pub enable_auto_cleanup: bool,
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
    /// Temporary directory path
    pub temp_dir: PathBuf,
    /// Maximum temp directory size in bytes
    pub max_temp_size: usize,
    /// Orphaned file age threshold in seconds
    pub orphan_age_threshold: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        CleanupConfig {
            enable_auto_cleanup: true,
            cleanup_interval: 3600, // 1 hour
            temp_dir: PathBuf::from("/opt/sam/tmp"),
            max_temp_size: 10 * 1024 * 1024 * 1024, // 10GB
            orphan_age_threshold: 86400,            // 24 hours
        }
    }
}

/// Memory limits configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryLimits {
    /// Maximum memory per request in bytes
    pub max_memory_per_request: usize,
    /// Maximum buffer size for streaming
    pub max_buffer_size: usize,
    /// Enable memory monitoring
    pub enable_monitoring: bool,
    /// Memory warning threshold (percentage)
    pub warning_threshold: f32,
    /// Memory critical threshold (percentage)
    pub critical_threshold: f32,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        MemoryLimits {
            max_memory_per_request: 512 * 1024 * 1024, // 512MB
            max_buffer_size: 64 * 1024,                // 64KB
            enable_monitoring: true,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
        }
    }
}

/// Resource manager
pub struct ResourceManager {
    config: Arc<ResourceConfig>,
    upload_semaphores: Arc<RwLock<HashMap<String, Arc<Semaphore>>>>,
    request_semaphores: Arc<RwLock<HashMap<String, Arc<Semaphore>>>>,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
    monitor: Arc<ResourceMonitor>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(config: ResourceConfig) -> Self {
        let config = Arc::new(config);
        let monitor = Arc::new(ResourceMonitor::new());

        ResourceManager {
            config,
            upload_semaphores: Arc::new(RwLock::new(HashMap::new())),
            request_semaphores: Arc::new(RwLock::new(HashMap::new())),
            cleanup_handle: None,
            monitor,
        }
    }

    /// Start background cleanup tasks
    pub async fn start_cleanup(&mut self) {
        if !self.config.cleanup_config.enable_auto_cleanup {
            return;
        }

        let config = self.config.clone();
        let monitor = self.monitor.clone();

        let handle = tokio::spawn(async move {
            let mut interval =
                interval(Duration::from_secs(config.cleanup_config.cleanup_interval));

            loop {
                interval.tick().await;

                if let Err(e) = cleanup_temp_files(&config.cleanup_config).await {
                    error!("Cleanup task failed: {}", e);
                    monitor.record_cleanup_failure();
                } else {
                    monitor.record_cleanup_success();
                }
            }
        });

        self.cleanup_handle = Some(handle);
        info!("Started resource cleanup background task");
    }

    /// Check if file upload is allowed
    pub async fn check_upload_allowed(
        &self,
        user_id: &str,
        file_size: usize,
        file_extension: &str,
    ) -> Result<UploadPermission> {
        // Check file size
        if file_size > self.config.file_limits.max_file_size {
            return Ok(UploadPermission::Denied {
                reason: format!(
                    "File size {} exceeds maximum allowed size {}",
                    file_size, self.config.file_limits.max_file_size
                ),
            });
        }

        // Check file extension
        if !self.config.file_limits.allowed_extensions.is_empty()
            && !self
                .config
                .file_limits
                .allowed_extensions
                .contains(&file_extension.to_string())
        {
            return Ok(UploadPermission::Denied {
                reason: format!("File extension {} is not allowed", file_extension),
            });
        }

        if self
            .config
            .file_limits
            .blocked_extensions
            .contains(&file_extension.to_string())
        {
            return Ok(UploadPermission::Denied {
                reason: format!("File extension {} is blocked", file_extension),
            });
        }

        // Get or create user semaphore
        let semaphore = self.get_or_create_upload_semaphore(user_id).await;

        // Try to acquire permit
        match semaphore.try_acquire_owned() {
            Ok(permit) => Ok(UploadPermission::Allowed {
                permit: Some(permit),
            }),
            Err(_) => Ok(UploadPermission::Denied {
                reason: format!(
                    "Maximum concurrent uploads ({}) reached for user",
                    self.config.file_limits.max_concurrent_uploads
                ),
            }),
        }
    }

    /// Get or create upload semaphore for user
    async fn get_or_create_upload_semaphore(&self, user_id: &str) -> Arc<Semaphore> {
        let mut semaphores = self.upload_semaphores.write().await;

        semaphores
            .entry(user_id.to_string())
            .or_insert_with(|| {
                Arc::new(Semaphore::new(
                    self.config.file_limits.max_concurrent_uploads,
                ))
            })
            .clone()
    }

    /// Process file upload with virus scanning
    pub async fn process_upload(
        &self,
        file_data: Vec<u8>,
        file_name: &str,
        user_id: &str,
    ) -> Result<ProcessedFile> {
        // Create temp file with automatic cleanup
        let temp_file = TempFile::new(&self.config.cleanup_config.temp_dir)?;

        // Write data to temp file
        temp_file.write(&file_data).await?;

        // Virus scan if enabled
        if self.config.file_limits.enable_virus_scan {
            if let Err(e) = scan_file(temp_file.path()).await {
                warn!("Virus scan failed for file {}: {}", file_name, e);
                return Err(anyhow::anyhow!("File failed virus scan"));
            }
        }

        // Calculate checksum
        let checksum = calculate_checksum(&file_data);

        // Move to permanent storage
        let permanent_path = self.get_permanent_path(user_id, file_name, &checksum)?;
        temp_file.move_to(&permanent_path).await?;

        Ok(ProcessedFile {
            path: permanent_path,
            checksum,
            size: file_data.len(),
            mime_type: detect_mime_type(file_name),
        })
    }

    /// Get permanent storage path for file
    fn get_permanent_path(
        &self,
        user_id: &str,
        file_name: &str,
        checksum: &str,
    ) -> Result<PathBuf> {
        let base_path = PathBuf::from("/opt/sam/storage");
        let user_path = base_path.join(user_id);

        // Create user directory if it doesn't exist
        std::fs::create_dir_all(&user_path)?;

        // Generate unique filename with checksum
        let extension = Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let unique_name = format!(
            "{}_{}.{}",
            file_name.trim_end_matches(&format!(".{}", extension)),
            &checksum[..8],
            extension
        );

        Ok(user_path.join(unique_name))
    }

    /// Get resource metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        self.monitor.get_metrics().await
    }
}

/// Upload permission result
#[derive(Debug)]
pub enum UploadPermission {
    Allowed {
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
    },
    Denied {
        reason: String,
    },
}

/// Processed file information
#[derive(Debug, Clone)]
pub struct ProcessedFile {
    pub path: PathBuf,
    pub checksum: String,
    pub size: usize,
    pub mime_type: String,
}

/// Clean up temporary files
async fn cleanup_temp_files(config: &CleanupConfig) -> Result<()> {
    let mut total_size = 0usize;
    let mut deleted_count = 0u32;
    let now = SystemTime::now();

    // Read temp directory
    let mut entries = fs::read_dir(&config.temp_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;

        if metadata.is_file() {
            let age = now.duration_since(metadata.modified()?)?;

            // Delete if older than threshold
            if age.as_secs() > config.orphan_age_threshold {
                let size = metadata.len() as usize;
                if let Err(e) = fs::remove_file(entry.path()).await {
                    warn!("Failed to delete temp file {:?}: {}", entry.path(), e);
                } else {
                    total_size += size;
                    deleted_count += 1;
                }
            }
        }
    }

    if deleted_count > 0 {
        info!(
            "Cleaned up {} temp files, freed {} bytes",
            deleted_count, total_size
        );
    }

    Ok(())
}

/// Scan file for viruses (placeholder - integrate with ClamAV)
async fn scan_file(path: &Path) -> Result<()> {
    // TODO: Integrate with ClamAV
    // For now, just check for suspicious patterns

    let content = fs::read(path).await?;

    // Check for common malware signatures (very basic)
    let suspicious_patterns: &[&[u8]] = &[
        b"EICAR",     // EICAR test virus
        b"X5O!P%@AP", // Another EICAR pattern
    ];

    for pattern in suspicious_patterns {
        if content.windows(pattern.len()).any(|w| w == *pattern) {
            return Err(anyhow::anyhow!("Suspicious pattern detected"));
        }
    }

    Ok(())
}

/// Calculate SHA256 checksum
fn calculate_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Detect MIME type from filename
fn detect_mime_type(filename: &str) -> String {
    let extension = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "html" => "text/html",
        "json" => "application/json",
        "xml" => "application/xml",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_config_defaults() {
        let config = ResourceConfig::default();
        assert_eq!(config.file_limits.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.request_limits.max_body_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_checksum_calculation() {
        let data = b"test data";
        let checksum = calculate_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(detect_mime_type("test.jpg"), "image/jpeg");
        assert_eq!(detect_mime_type("document.pdf"), "application/pdf");
        assert_eq!(detect_mime_type("unknown.xyz"), "application/octet-stream");
    }
}
