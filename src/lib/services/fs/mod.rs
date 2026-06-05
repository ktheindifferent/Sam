//! # File Storage Services Module
//!
//! This module provides various file storage service implementations:
//! - `dropbox` - Dropbox cloud storage integration
//! - `dropbox_service` - Enhanced Dropbox service with better error handling
//! - `nextcloud` - Nextcloud/ownCloud self-hosted storage
//! - `seaweedfs` - SeaweedFS distributed file system
//! - `local` - Local file storage abstraction
//!
//! ## Common Features
//!
//! All file storage services implement common operations:
//! - File upload/download
//! - Directory listing
//! - File/folder creation, deletion, and manipulation
//! - Metadata management
//! - Authentication and access control
//!
//! ## Usage
//!
//! ```rust
//! use crate::services::fs::{FileStorageService, StorageConfig};
//!
//! // Create a local file storage service
//! let config = StorageConfig::default();
//! let storage = FileStorageService::new(config, metadata_store);
//!
//! // Upload a file
//! let metadata = storage.store_file(data, "filename.txt", tags, custom_metadata, "user").await?;
//! ```

pub mod dropbox;
pub mod dropbox_service;
pub mod local;
pub mod nextcloud;
pub mod seaweedfs;
pub mod traits;

// Re-export commonly used types
pub use local::{FileMetadata, FileStorageService, SearchQuery, StorageConfig, StorageError};
pub use traits::{FileOperations, FileStorageBackend};

// Re-export service implementations
pub use dropbox_service::DropboxService;
pub use nextcloud::NextCloudService;
pub use seaweedfs::SeaweedFSService;

// Re-export legacy dropbox functions for backwards compatibility
pub use dropbox::{
    create_folder as dropbox_create_folder, create_sam_folder as dropbox_create_sam_folder,
};

/// Initialize the file storage services
pub async fn initialize() -> anyhow::Result<()> {
    log::info!("File storage services initialized");
    local::initialize().await
}
