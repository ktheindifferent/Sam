//! # File Storage Traits
//!
//! Common traits and interfaces for file storage services

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Common file metadata structure across all storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub is_folder: bool,
    pub mime_type: String,
    pub checksum: Option<String>,
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

/// File storage operation result
pub type FileResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Core file operations trait that all storage backends must implement
#[async_trait]
pub trait FileOperations: Send + Sync {
    /// List files and folders at the given path
    async fn list_files(&self, path: &str, limit: Option<u32>) -> FileResult<Vec<FileInfo>>;

    /// Upload a file to the storage backend
    async fn upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
        content: &[u8],
    ) -> FileResult<FileInfo>;

    /// Download a file from the storage backend
    async fn download_file(&self, remote_path: &str) -> FileResult<Vec<u8>>;

    /// Delete a file or folder
    async fn delete_file(&self, remote_path: &str) -> FileResult<()>;

    /// Create a new folder
    async fn create_folder(&self, path: &str) -> FileResult<FileInfo>;

    /// Move a file or folder from one location to another
    async fn move_file(&self, from_path: &str, to_path: &str) -> FileResult<FileInfo>;

    /// Copy a file or folder
    async fn copy_file(&self, from_path: &str, to_path: &str) -> FileResult<FileInfo>;

    /// Check if a path exists
    async fn exists(&self, path: &str) -> FileResult<bool>;

    /// Get metadata for a specific file or folder
    async fn get_metadata(&self, path: &str) -> FileResult<FileInfo>;
}

/// Storage backend configuration and management
#[async_trait]
pub trait FileStorageBackend: FileOperations {
    /// Authentication and connection setup
    async fn authenticate(&mut self) -> FileResult<()>;

    /// Test the connection to the storage backend
    async fn test_connection(&self) -> FileResult<bool>;

    /// Get the backend name/identifier
    fn get_backend_name(&self) -> &str;

    /// Get backend-specific configuration
    fn get_config(&self) -> serde_json::Value;

    /// Health check for the storage backend
    async fn health_check(&self) -> FileResult<bool>;
}

/// Storage quota and usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub file_count: u64,
    pub folder_count: u64,
}

/// Extended operations for storage backends that support them
#[async_trait]
pub trait ExtendedFileOperations: FileOperations {
    /// Get storage usage statistics
    async fn get_usage(&self) -> FileResult<StorageUsage>;

    /// Search files by name pattern or metadata
    async fn search_files(
        &self,
        query: &str,
        filters: Option<HashMap<String, String>>,
    ) -> FileResult<Vec<FileInfo>>;

    /// Generate a shareable link for a file
    async fn create_share_link(
        &self,
        path: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> FileResult<String>;

    /// Get file versions (if versioning is supported)
    async fn get_file_versions(&self, path: &str) -> FileResult<Vec<FileInfo>>;

    /// Restore a specific version of a file
    async fn restore_version(&self, path: &str, version_id: &str) -> FileResult<FileInfo>;
}

/// Batch operations for efficient bulk operations
#[async_trait]
pub trait BatchFileOperations: FileOperations {
    /// Upload multiple files in a batch
    async fn batch_upload(&self, files: Vec<(String, Vec<u8>)>) -> FileResult<Vec<FileInfo>>;

    /// Download multiple files in a batch
    async fn batch_download(&self, paths: Vec<String>) -> FileResult<Vec<(String, Vec<u8>)>>;

    /// Delete multiple files in a batch
    async fn batch_delete(&self, paths: Vec<String>) -> FileResult<Vec<String>>;
}

/// Stream operations for large files
#[async_trait]
pub trait StreamingFileOperations: FileOperations {
    /// Stream upload for large files
    async fn stream_upload(
        &self,
        remote_path: &str,
        content: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
    ) -> FileResult<FileInfo>;

    /// Stream download for large files
    async fn stream_download(
        &self,
        remote_path: &str,
    ) -> FileResult<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;
}

/// Synchronization operations for keeping local and remote storage in sync
#[async_trait]
pub trait SyncOperations: FileOperations {
    /// Sync a local directory with remote storage
    async fn sync_directory(
        &self,
        local_path: &Path,
        remote_path: &str,
        bidirectional: bool,
    ) -> FileResult<SyncResult>;

    /// Get changes since last sync
    async fn get_changes_since(&self, timestamp: DateTime<Utc>) -> FileResult<Vec<FileChange>>;
}

/// Sync operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded_files: Vec<String>,
    pub downloaded_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub conflicts: Vec<String>,
    pub errors: Vec<String>,
}

/// File change information for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub timestamp: DateTime<Utc>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Moved { from: String, to: String },
}
