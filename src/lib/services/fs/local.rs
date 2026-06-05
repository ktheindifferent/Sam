use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

/// File metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub filename: String,
    pub original_name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: String,
    pub checksum: String,
    pub tags: Vec<String>,
    pub custom_metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub version: u32,
    pub is_compressed: bool,
    pub compression_ratio: Option<f32>,
    pub encryption_status: EncryptionStatus,
    pub permissions: FilePermissions,
    pub thumbnail_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub owner: String,
    pub group: Option<String>,
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub delete: Vec<String>,
    pub share: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionStatus {
    None,
    Encrypted { algorithm: String },
    PartiallyEncrypted,
}

/// File storage configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub max_file_size: u64,
    pub allowed_extensions: Vec<String>,
    pub enable_compression: bool,
    pub compression_threshold: u64,
    pub enable_encryption: bool,
    pub enable_versioning: bool,
    pub max_versions: u32,
    pub enable_thumbnails: bool,
    pub thumbnail_sizes: Vec<(u32, u32)>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            base_path: PathBuf::from("/var/sam/storage"),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            allowed_extensions: vec![],
            enable_compression: true,
            compression_threshold: 1024 * 1024, // 1MB
            enable_encryption: false,
            enable_versioning: true,
            max_versions: 10,
            enable_thumbnails: true,
            thumbnail_sizes: vec![(128, 128), (256, 256), (512, 512)],
        }
    }
}

/// File storage service
pub struct FileStorageService {
    config: StorageConfig,
    metadata_store: Box<dyn MetadataStore>,
}

impl FileStorageService {
    /// Create a new file storage service
    pub fn new(config: StorageConfig, metadata_store: Box<dyn MetadataStore>) -> Self {
        FileStorageService {
            config,
            metadata_store,
        }
    }

    /// Store a file with metadata
    pub async fn store_file(
        &self,
        data: Vec<u8>,
        filename: &str,
        tags: Vec<String>,
        custom_metadata: HashMap<String, serde_json::Value>,
        user: &str,
    ) -> Result<FileMetadata, StorageError> {
        // Validate file
        self.validate_file(&data, filename)?;

        // Generate file ID and path
        let file_id = Uuid::new_v4().to_string();
        let storage_path = self.generate_storage_path(&file_id, filename);

        // Create directories if needed
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&data);

        // Compress if needed
        let (final_data, is_compressed, compression_ratio) =
            if self.should_compress(&data, filename) {
                let compressed = self.compress_data(&data).await?;
                let ratio = compressed.len() as f32 / data.len() as f32;
                (compressed, true, Some(ratio))
            } else {
                (data.clone(), false, None)
            };

        // Encrypt if needed
        let (final_data, encryption_status) = if self.config.enable_encryption {
            let encrypted = self.encrypt_data(&final_data).await?;
            (
                encrypted,
                EncryptionStatus::Encrypted {
                    algorithm: "AES-256-GCM".to_string(),
                },
            )
        } else {
            (final_data, EncryptionStatus::None)
        };

        // Write file to disk
        let mut file = fs::File::create(&storage_path).await?;
        file.write_all(&final_data).await?;
        file.sync_all().await?;

        // Generate thumbnails if applicable
        let thumbnail_path = if self.is_image(filename) && self.config.enable_thumbnails {
            Some(self.generate_thumbnails(&data, &file_id).await?)
        } else {
            None
        };

        // Detect MIME type
        let mime_type = self.detect_mime_type(filename, &data);

        // Create metadata
        let metadata = FileMetadata {
            id: file_id.clone(),
            filename: self.sanitize_filename(filename),
            original_name: filename.to_string(),
            path: storage_path,
            size: data.len() as u64,
            mime_type,
            checksum,
            tags,
            custom_metadata,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Utc::now(),
            version: 1,
            is_compressed,
            compression_ratio,
            encryption_status,
            permissions: FilePermissions {
                owner: user.to_string(),
                group: None,
                read: vec![user.to_string()],
                write: vec![user.to_string()],
                delete: vec![user.to_string()],
                share: vec![],
            },
            thumbnail_path,
        };

        // Store metadata
        self.metadata_store.store(&metadata).await?;

        info!("Stored file {} with ID {}", filename, file_id);

        Ok(metadata)
    }

    /// Retrieve a file by ID
    pub async fn get_file(
        &self,
        file_id: &str,
        user: &str,
    ) -> Result<(Vec<u8>, FileMetadata), StorageError> {
        // Get metadata
        let mut metadata = self.metadata_store.get(file_id).await?;

        // Check permissions
        if !self.has_read_permission(&metadata, user) {
            return Err(StorageError::PermissionDenied);
        }

        // Read file from disk
        let mut data = fs::read(&metadata.path).await?;

        // Decrypt if needed
        if matches!(
            metadata.encryption_status,
            EncryptionStatus::Encrypted { .. }
        ) {
            data = self.decrypt_data(&data).await?;
        }

        // Decompress if needed
        if metadata.is_compressed {
            data = self.decompress_data(&data).await?;
        }

        // Update access time
        metadata.accessed_at = Utc::now();
        self.metadata_store.update(&metadata).await?;

        Ok((data, metadata))
    }

    /// Update file metadata
    pub async fn update_metadata(
        &self,
        file_id: &str,
        tags: Option<Vec<String>>,
        custom_metadata: Option<HashMap<String, serde_json::Value>>,
        user: &str,
    ) -> Result<FileMetadata, StorageError> {
        let mut metadata = self.metadata_store.get(file_id).await?;

        // Check permissions
        if !self.has_write_permission(&metadata, user) {
            return Err(StorageError::PermissionDenied);
        }

        // Update fields
        if let Some(tags) = tags {
            metadata.tags = tags;
        }

        if let Some(custom) = custom_metadata {
            metadata.custom_metadata.extend(custom);
        }

        metadata.updated_at = Utc::now();

        // Store updated metadata
        self.metadata_store.update(&metadata).await?;

        Ok(metadata)
    }

    /// Search files by tags and metadata
    pub async fn search_files(
        &self,
        query: SearchQuery,
        user: &str,
    ) -> Result<Vec<FileMetadata>, StorageError> {
        let results = self.metadata_store.search(&query).await?;

        // Filter by permissions
        let filtered: Vec<FileMetadata> = results
            .into_iter()
            .filter(|m| self.has_read_permission(m, user))
            .collect();

        Ok(filtered)
    }

    /// Create a new version of a file
    pub async fn create_version(
        &self,
        file_id: &str,
        data: Vec<u8>,
        user: &str,
    ) -> Result<FileMetadata, StorageError> {
        if !self.config.enable_versioning {
            return Err(StorageError::VersioningDisabled);
        }

        let mut metadata = self.metadata_store.get(file_id).await?;

        // Check permissions
        if !self.has_write_permission(&metadata, user) {
            return Err(StorageError::PermissionDenied);
        }

        // Archive current version
        let version_path = self.generate_version_path(&metadata.id, metadata.version);
        fs::copy(&metadata.path, &version_path).await?;

        // Store new version
        let checksum = self.calculate_checksum(&data);

        // Process data (compress/encrypt as needed)
        let (final_data, is_compressed, compression_ratio) =
            if self.should_compress(&data, &metadata.filename) {
                let compressed = self.compress_data(&data).await?;
                let ratio = compressed.len() as f32 / data.len() as f32;
                (compressed, true, Some(ratio))
            } else {
                (data.clone(), false, None)
            };

        // Write new version
        let mut file = fs::File::create(&metadata.path).await?;
        file.write_all(&final_data).await?;
        file.sync_all().await?;

        // Update metadata
        metadata.version += 1;
        metadata.size = data.len() as u64;
        metadata.checksum = checksum;
        metadata.updated_at = Utc::now();
        metadata.is_compressed = is_compressed;
        metadata.compression_ratio = compression_ratio;

        // Clean up old versions if needed
        if metadata.version > self.config.max_versions {
            self.cleanup_old_versions(&metadata.id, metadata.version - self.config.max_versions)
                .await?;
        }

        self.metadata_store.update(&metadata).await?;

        info!("Created version {} of file {}", metadata.version, file_id);

        Ok(metadata)
    }

    /// Delete a file
    pub async fn delete_file(&self, file_id: &str, user: &str) -> Result<(), StorageError> {
        let metadata = self.metadata_store.get(file_id).await?;

        // Check permissions
        if !self.has_delete_permission(&metadata, user) {
            return Err(StorageError::PermissionDenied);
        }

        // Delete file
        fs::remove_file(&metadata.path).await?;

        // Delete thumbnails if they exist
        if let Some(thumb_path) = metadata.thumbnail_path {
            let _ = fs::remove_dir_all(thumb_path).await;
        }

        // Delete all versions
        if self.config.enable_versioning {
            for version in 1..metadata.version {
                let version_path = self.generate_version_path(&metadata.id, version);
                let _ = fs::remove_file(version_path).await;
            }
        }

        // Delete metadata
        self.metadata_store.delete(file_id).await?;

        info!("Deleted file {}", file_id);

        Ok(())
    }

    /// Share a file with other users
    pub async fn share_file(
        &self,
        file_id: &str,
        share_with: Vec<String>,
        permissions: Vec<String>,
        user: &str,
    ) -> Result<FileMetadata, StorageError> {
        let mut metadata = self.metadata_store.get(file_id).await?;

        // Check if user can share
        if !metadata.permissions.share.contains(&user.to_string())
            && metadata.permissions.owner != user
        {
            return Err(StorageError::PermissionDenied);
        }

        // Add permissions
        for user in share_with {
            if permissions.contains(&"read".to_string())
                && !metadata.permissions.read.contains(&user)
            {
                metadata.permissions.read.push(user.clone());
            }
            if permissions.contains(&"write".to_string())
                && !metadata.permissions.write.contains(&user)
            {
                metadata.permissions.write.push(user.clone());
            }
            if permissions.contains(&"delete".to_string())
                && !metadata.permissions.delete.contains(&user)
            {
                metadata.permissions.delete.push(user.clone());
            }
        }

        metadata.updated_at = Utc::now();
        self.metadata_store.update(&metadata).await?;

        Ok(metadata)
    }

    // Helper methods

    fn validate_file(&self, data: &[u8], filename: &str) -> Result<(), StorageError> {
        // Check file size
        if data.len() as u64 > self.config.max_file_size {
            return Err(StorageError::FileTooLarge);
        }

        // Check extension if restrictions are configured
        if !self.config.allowed_extensions.is_empty() {
            let extension = Path::new(filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            if !self
                .config
                .allowed_extensions
                .contains(&extension.to_string())
            {
                return Err(StorageError::InvalidFileType);
            }
        }

        Ok(())
    }

    fn generate_storage_path(&self, file_id: &str, filename: &str) -> PathBuf {
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Organize by year/month/day
        let now = Utc::now();
        self.config
            .base_path
            .join(now.format("%Y").to_string())
            .join(now.format("%m").to_string())
            .join(now.format("%d").to_string())
            .join(format!("{}.{}", file_id, extension))
    }

    fn generate_version_path(&self, file_id: &str, version: u32) -> PathBuf {
        self.config
            .base_path
            .join("versions")
            .join(format!("{}_v{}", file_id, version))
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn should_compress(&self, data: &[u8], filename: &str) -> bool {
        if !self.config.enable_compression {
            return false;
        }

        if data.len() < self.config.compression_threshold as usize {
            return false;
        }

        // Don't compress already compressed formats
        let compressed_extensions = [
            "zip", "gz", "bz2", "xz", "7z", "rar", "jpg", "jpeg", "png", "mp4", "mp3",
        ];
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        !compressed_extensions.contains(&extension.as_str())
    }

    async fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    async fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        // Placeholder for encryption implementation
        // Would use AES-256-GCM or similar
        Ok(data.to_vec())
    }

    async fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        // Placeholder for decryption implementation
        Ok(data.to_vec())
    }

    fn detect_mime_type(&self, filename: &str, data: &[u8]) -> String {
        // Use file extension and magic bytes to detect MIME type
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "pdf" => "application/pdf",
            "doc" | "docx" => "application/msword",
            "xls" | "xlsx" => "application/vnd.ms-excel",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "mp4" => "video/mp4",
            "mp3" => "audio/mpeg",
            "txt" => "text/plain",
            "html" => "text/html",
            "json" => "application/json",
            "xml" => "application/xml",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    fn is_image(&self, filename: &str) -> bool {
        let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "svg"];
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        image_extensions.contains(&extension.as_str())
    }

    async fn generate_thumbnails(
        &self,
        data: &[u8],
        file_id: &str,
    ) -> Result<PathBuf, StorageError> {
        // Placeholder for thumbnail generation
        // Would use image processing library
        let thumb_dir = self.config.base_path.join("thumbnails").join(file_id);
        fs::create_dir_all(&thumb_dir).await?;
        Ok(thumb_dir)
    }

    fn sanitize_filename(&self, filename: &str) -> String {
        // Remove potentially dangerous characters from filename
        filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
            .collect()
    }

    fn has_read_permission(&self, metadata: &FileMetadata, user: &str) -> bool {
        metadata.permissions.owner == user || metadata.permissions.read.contains(&user.to_string())
    }

    fn has_write_permission(&self, metadata: &FileMetadata, user: &str) -> bool {
        metadata.permissions.owner == user || metadata.permissions.write.contains(&user.to_string())
    }

    fn has_delete_permission(&self, metadata: &FileMetadata, user: &str) -> bool {
        metadata.permissions.owner == user
            || metadata.permissions.delete.contains(&user.to_string())
    }

    async fn cleanup_old_versions(
        &self,
        file_id: &str,
        versions_to_delete: u32,
    ) -> Result<(), StorageError> {
        for version in 1..=versions_to_delete {
            let version_path = self.generate_version_path(file_id, version);
            let _ = fs::remove_file(version_path).await;
        }
        Ok(())
    }
}

/// Search query for files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub tags: Option<Vec<String>>,
    pub filename_pattern: Option<String>,
    pub mime_type: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub custom_metadata: Option<HashMap<String, serde_json::Value>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Storage errors
#[derive(Debug)]
pub enum StorageError {
    IoError(std::io::Error),
    MetadataError(String),
    FileTooLarge,
    InvalidFileType,
    FileNotFound,
    PermissionDenied,
    VersioningDisabled,
    EncryptionError,
    CompressionError,
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(e) => write!(f, "IO error: {}", e),
            StorageError::MetadataError(e) => write!(f, "Metadata error: {}", e),
            StorageError::FileTooLarge => write!(f, "File exceeds maximum size"),
            StorageError::InvalidFileType => write!(f, "File type not allowed"),
            StorageError::FileNotFound => write!(f, "File not found"),
            StorageError::PermissionDenied => write!(f, "Permission denied"),
            StorageError::VersioningDisabled => write!(f, "Versioning is disabled"),
            StorageError::EncryptionError => write!(f, "Encryption error"),
            StorageError::CompressionError => write!(f, "Compression error"),
        }
    }
}

impl std::error::Error for StorageError {}

/// Metadata storage trait
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn store(&self, metadata: &FileMetadata) -> Result<(), StorageError>;
    async fn get(&self, file_id: &str) -> Result<FileMetadata, StorageError>;
    async fn update(&self, metadata: &FileMetadata) -> Result<(), StorageError>;
    async fn delete(&self, file_id: &str) -> Result<(), StorageError>;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<FileMetadata>, StorageError>;
}

/// Initialize the file storage service
pub async fn initialize() -> anyhow::Result<()> {
    info!("File storage service initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.max_file_size, 1024 * 1024 * 1024);
        assert!(config.enable_compression);
        assert!(config.enable_versioning);
    }

    #[test]
    fn test_file_metadata() {
        let metadata = FileMetadata {
            id: "test-id".to_string(),
            filename: "test.txt".to_string(),
            original_name: "test.txt".to_string(),
            path: PathBuf::from("/storage/test.txt"),
            size: 1024,
            mime_type: "text/plain".to_string(),
            checksum: "abc123".to_string(),
            tags: vec!["test".to_string()],
            custom_metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Utc::now(),
            version: 1,
            is_compressed: false,
            compression_ratio: None,
            encryption_status: EncryptionStatus::None,
            permissions: FilePermissions {
                owner: "user1".to_string(),
                group: None,
                read: vec!["user1".to_string()],
                write: vec!["user1".to_string()],
                delete: vec!["user1".to_string()],
                share: vec![],
            },
            thumbnail_path: None,
        };

        assert_eq!(metadata.id, "test-id");
        assert_eq!(metadata.size, 1024);
    }

    #[test]
    fn test_search_query() {
        let query = SearchQuery {
            tags: Some(vec!["important".to_string()]),
            filename_pattern: Some("*.pdf".to_string()),
            mime_type: Some("application/pdf".to_string()),
            min_size: Some(1024),
            max_size: Some(1024 * 1024),
            created_after: None,
            created_before: None,
            custom_metadata: None,
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(query.limit, Some(10));
        assert!(query.tags.is_some());
    }
}
