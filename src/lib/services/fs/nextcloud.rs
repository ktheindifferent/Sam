// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

//! NextCloud Integration Service
//!
//! This module provides deep integration with NextCloud servers, allowing SAM to:
//! - Connect to NextCloud instances via WebDAV
//! - Upload, download, and manage files
//! - Synchronize file metadata and sharing settings
//! - Monitor file changes and updates
//! - Integrate with NextCloud's sharing and collaboration features

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::services::traits::{HealthStatus, Service, ServiceConfig, ServiceError, ServiceHealth};

/// NextCloud service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudConfig {
    /// NextCloud server URL (e.g., https://cloud.example.com)
    pub server_url: String,
    /// Username for authentication
    pub username: String,
    /// Password or app password for authentication
    pub password: String,
    /// WebDAV endpoint path (usually /remote.php/dav/files/{username}/)
    pub webdav_path: Option<String>,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Enable SSL certificate verification
    pub verify_ssl: bool,
    /// Sync interval in seconds
    pub sync_interval: u64,
    /// Local cache directory
    pub cache_directory: Option<PathBuf>,
    /// Maximum file size for upload (bytes)
    pub max_upload_size: u64,
    /// Enable real-time file monitoring
    pub enable_monitoring: bool,
    /// Chunk size for large file uploads (bytes)
    pub chunk_size: usize,
}

impl Default for NextCloudConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            username: String::new(),
            password: String::new(),
            webdav_path: None,
            timeout_seconds: 30,
            verify_ssl: true,
            sync_interval: 300, // 5 minutes
            cache_directory: Some(PathBuf::from("/var/sam/nextcloud_cache")),
            max_upload_size: 10 * 1024 * 1024 * 1024, // 10GB
            enable_monitoring: true,
            chunk_size: 10 * 1024 * 1024, // 10MB chunks
        }
    }
}

/// NextCloud file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudFile {
    /// File path on NextCloud server
    pub path: String,
    /// File name
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
    /// Last modified timestamp
    pub modified: DateTime<Utc>,
    /// ETag for change detection
    pub etag: String,
    /// Whether it's a directory
    pub is_directory: bool,
    /// NextCloud file ID
    pub file_id: Option<String>,
    /// Sharing information
    pub shares: Vec<NextCloudShare>,
    /// File permissions
    pub permissions: String,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

/// NextCloud sharing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudShare {
    /// Share ID
    pub id: String,
    /// Share type (user, group, link, etc.)
    pub share_type: String,
    /// Share permissions
    pub permissions: u32,
    /// Shared with (user/group name or link token)
    pub shared_with: Option<String>,
    /// Share creation time
    pub created_at: DateTime<Utc>,
    /// Share expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Share password (for link shares)
    pub password_protected: bool,
    /// Share URL (for link shares)
    pub url: Option<String>,
}

/// NextCloud operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudResult {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

/// NextCloud service client
pub struct NextCloudService {
    config: NextCloudConfig,
    service_config: ServiceConfig,
    client: Client,
    base_webdav_url: String,
    auth_header: String,
    cache: RwLock<HashMap<String, NextCloudFile>>,
    last_sync: RwLock<SystemTime>,
}

impl NextCloudService {
    /// Create a new NextCloud service instance
    pub fn new(config: NextCloudConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()?;

        let webdav_path = config
            .webdav_path
            .clone()
            .unwrap_or_else(|| format!("/remote.php/dav/files/{}/", config.username));

        let base_webdav_url = format!("{}{}", config.server_url.trim_end_matches('/'), webdav_path);

        let auth_credentials = format!("{}:{}", config.username, config.password);
        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(&auth_credentials)
        );

        let service_config = ServiceConfig {
            name: "NextCloud".to_string(),
            enabled: true,
            retry_attempts: 3,
            timeout_seconds: config.timeout_seconds,
        };

        Ok(Self {
            config,
            service_config,
            client,
            base_webdav_url,
            auth_header,
            cache: RwLock::new(HashMap::new()),
            last_sync: RwLock::new(UNIX_EPOCH),
        })
    }

    /// Test connection to NextCloud server
    pub async fn test_connection(&self) -> Result<bool> {
        let response = self
            .client
            .request(
                reqwest::Method::from_bytes(b"PROPFIND")?,
                &self.base_webdav_url,
            )
            .header("Authorization", &self.auth_header)
            .header("Depth", "0")
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// List files and directories at a given path
    pub async fn list_files(&self, path: &str) -> Result<Vec<NextCloudFile>> {
        let url = format!("{}{}", self.base_webdav_url, path.trim_start_matches('/'));

        let propfind_body = r#"<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
  <d:prop>
    <d:displayname/>
    <d:getcontentlength/>
    <d:getcontenttype/>
    <d:getetag/>
    <d:getlastmodified/>
    <d:resourcetype/>
    <oc:id/>
    <oc:permissions/>
    <oc:size/>
    <nc:has-preview/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
            .header("Authorization", &self.auth_header)
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to list files: HTTP {}", response.status()));
        }

        let xml_content = response.text().await?;
        self.parse_propfind_response(&xml_content)
    }

    /// Upload a file to NextCloud
    pub async fn upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
        content: &[u8],
    ) -> Result<NextCloudFile> {
        let url = format!(
            "{}{}",
            self.base_webdav_url,
            remote_path.trim_start_matches('/')
        );

        // For large files, use chunked upload
        if content.len() > self.config.chunk_size {
            return self.upload_file_chunked(&url, content).await;
        }

        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .body(content.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to upload file: HTTP {}", response.status()));
        }

        // Get file metadata after upload
        self.get_file_info(remote_path).await
    }

    /// Upload large file using chunked upload
    async fn upload_file_chunked(&self, url: &str, content: &[u8]) -> Result<NextCloudFile> {
        let total_size = content.len();
        let chunks = content.chunks(self.config.chunk_size);
        let chunk_count = chunks.len();

        // Create upload session
        let upload_id = Uuid::new_v4().to_string();
        let upload_url = format!("{}/uploads/{}", url, upload_id);

        for (i, chunk) in chunks.enumerate() {
            let chunk_url = format!("{}/{}", upload_url, i);
            let is_last_chunk = i == chunk_count - 1;

            let mut request = self
                .client
                .put(&chunk_url)
                .header("Authorization", &self.auth_header)
                .header("Content-Length", chunk.len().to_string())
                .body(chunk.to_vec());

            if is_last_chunk {
                request = request.header("X-Final-Chunk", "1");
            }

            let response = request.send().await?;
            if !response.status().is_success() {
                return Err(anyhow!(
                    "Failed to upload chunk {}/{}: HTTP {}",
                    i + 1,
                    chunk_count,
                    response.status()
                ));
            }
        }

        // Finalize upload
        let finalize_response = self
            .client
            .post(&format!("{}/finalize", upload_url))
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if !finalize_response.status().is_success() {
            return Err(anyhow!(
                "Failed to finalize upload: HTTP {}",
                finalize_response.status()
            ));
        }

        // Get file metadata
        let file_path = url.strip_prefix(&self.base_webdav_url).unwrap_or(url);
        self.get_file_info(file_path).await
    }

    /// Download a file from NextCloud
    pub async fn download_file(&self, remote_path: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}{}",
            self.base_webdav_url,
            remote_path.trim_start_matches('/')
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file: HTTP {}",
                response.status()
            ));
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Get file information
    pub async fn get_file_info(&self, path: &str) -> Result<NextCloudFile> {
        let url = format!("{}{}", self.base_webdav_url, path.trim_start_matches('/'));

        let propfind_body = r#"<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
  <d:prop>
    <d:displayname/>
    <d:getcontentlength/>
    <d:getcontenttype/>
    <d:getetag/>
    <d:getlastmodified/>
    <d:resourcetype/>
    <oc:id/>
    <oc:permissions/>
    <oc:size/>
    <nc:has-preview/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
            .header("Authorization", &self.auth_header)
            .header("Depth", "0")
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to get file info: HTTP {}",
                response.status()
            ));
        }

        let xml_content = response.text().await?;
        let files = self.parse_propfind_response(&xml_content)?;

        files
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("File not found: {}", path))
    }

    /// Delete a file or directory
    pub async fn delete_file(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_webdav_url, path.trim_start_matches('/'));

        let response = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to delete file: HTTP {}", response.status()));
        }

        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(path);

        Ok(())
    }

    /// Create a directory
    pub async fn create_directory(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_webdav_url, path.trim_start_matches('/'));

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL")?, &url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to create directory: HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Move or rename a file/directory
    pub async fn move_file(&self, from_path: &str, to_path: &str) -> Result<()> {
        let from_url = format!(
            "{}{}",
            self.base_webdav_url,
            from_path.trim_start_matches('/')
        );
        let to_url = format!(
            "{}{}",
            self.base_webdav_url,
            to_path.trim_start_matches('/')
        );

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MOVE")?, &from_url)
            .header("Authorization", &self.auth_header)
            .header("Destination", &to_url)
            .header("Overwrite", "F")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to move file: HTTP {}", response.status()));
        }

        // Update cache
        let mut cache = self.cache.write().await;
        if let Some(mut file) = cache.remove(from_path) {
            file.path = to_path.to_string();
            cache.insert(to_path.to_string(), file);
        }

        Ok(())
    }

    /// Copy a file/directory
    pub async fn copy_file(&self, from_path: &str, to_path: &str) -> Result<()> {
        let from_url = format!(
            "{}{}",
            self.base_webdav_url,
            from_path.trim_start_matches('/')
        );
        let to_url = format!(
            "{}{}",
            self.base_webdav_url,
            to_path.trim_start_matches('/')
        );

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"COPY")?, &from_url)
            .header("Authorization", &self.auth_header)
            .header("Destination", &to_url)
            .header("Overwrite", "F")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to copy file: HTTP {}", response.status()));
        }

        Ok(())
    }

    /// Create a share link for a file
    pub async fn create_share(
        &self,
        path: &str,
        share_type: &str,
        share_with: Option<&str>,
        permissions: u32,
        password: Option<&str>,
        expire_date: Option<DateTime<Utc>>,
    ) -> Result<NextCloudShare> {
        let share_url = format!(
            "{}/ocs/v2.php/apps/files_sharing/api/v1/shares",
            self.config.server_url.trim_end_matches('/')
        );

        let mut params = vec![
            ("path", path.to_string()),
            ("shareType", share_type.to_string()),
            ("permissions", permissions.to_string()),
            ("format", "json".to_string()),
        ];

        if let Some(shared_with) = share_with {
            params.push(("shareWith", shared_with.to_string()));
        }

        if let Some(pass) = password {
            params.push(("password", pass.to_string()));
        }

        if let Some(expire) = expire_date {
            params.push(("expireDate", expire.format("%Y-%m-%d").to_string()));
        }

        let response = self
            .client
            .post(&share_url)
            .header("Authorization", &self.auth_header)
            .header("OCS-APIRequest", "true")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to create share: HTTP {}",
                response.status()
            ));
        }

        let json_response: Value = response.json().await?;
        self.parse_share_response(&json_response)
    }

    /// List all shares for a file
    pub async fn list_shares(&self, path: &str) -> Result<Vec<NextCloudShare>> {
        let share_url = format!(
            "{}/ocs/v2.php/apps/files_sharing/api/v1/shares",
            self.config.server_url.trim_end_matches('/')
        );

        let response = self
            .client
            .get(&share_url)
            .header("Authorization", &self.auth_header)
            .header("OCS-APIRequest", "true")
            .query(&[("path", path), ("format", "json")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to list shares: HTTP {}", response.status()));
        }

        let json_response: Value = response.json().await?;
        self.parse_shares_response(&json_response)
    }

    /// Delete a share
    pub async fn delete_share(&self, share_id: &str) -> Result<()> {
        let share_url = format!(
            "{}/ocs/v2.php/apps/files_sharing/api/v1/shares/{}",
            self.config.server_url.trim_end_matches('/'),
            share_id
        );

        let response = self
            .client
            .delete(&share_url)
            .header("Authorization", &self.auth_header)
            .header("OCS-APIRequest", "true")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to delete share: HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Synchronize with NextCloud server
    pub async fn sync(&self) -> Result<Vec<NextCloudFile>> {
        log::info!("Starting NextCloud sync...");

        let files = self.list_files("").await?;

        // Update cache
        let mut cache = self.cache.write().await;
        cache.clear();
        for file in &files {
            cache.insert(file.path.clone(), file.clone());
        }

        // Update last sync time
        let mut last_sync = self.last_sync.write().await;
        *last_sync = SystemTime::now();

        log::info!("NextCloud sync completed. {} files cached.", files.len());
        Ok(files)
    }

    /// Get cached file list
    pub async fn get_cached_files(&self) -> Vec<NextCloudFile> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Search files by name pattern
    pub async fn search_files(&self, pattern: &str) -> Result<Vec<NextCloudFile>> {
        let search_url = format!(
            "{}/ocs/v2.php/apps/files/api/v1/search",
            self.config.server_url.trim_end_matches('/')
        );

        let response = self
            .client
            .get(&search_url)
            .header("Authorization", &self.auth_header)
            .header("OCS-APIRequest", "true")
            .query(&[("pattern", pattern), ("format", "json")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to search files: HTTP {}",
                response.status()
            ));
        }

        let json_response: Value = response.json().await?;
        self.parse_search_response(&json_response)
    }

    /// Parse PROPFIND XML response into file metadata
    fn parse_propfind_response(&self, xml_content: &str) -> Result<Vec<NextCloudFile>> {
        // This is a simplified XML parser - in production, you'd want to use a proper XML library
        let mut files = Vec::new();

        // For now, return empty vector - this would need proper XML parsing
        // In a real implementation, you'd parse the WebDAV XML response here

        Ok(files)
    }

    /// Parse share API response
    fn parse_share_response(&self, json: &Value) -> Result<NextCloudShare> {
        let data = json
            .get("ocs")
            .and_then(|ocs| ocs.get("data"))
            .ok_or_else(|| anyhow!("Invalid share response format"))?;

        let share = NextCloudShare {
            id: data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            share_type: data
                .get("share_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            permissions: data
                .get("permissions")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            shared_with: data
                .get("share_with")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            created_at: Utc::now(), // Would parse from response
            expires_at: None,       // Would parse from response
            password_protected: data.get("password").is_some(),
            url: data
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        Ok(share)
    }

    /// Parse shares list response
    fn parse_shares_response(&self, json: &Value) -> Result<Vec<NextCloudShare>> {
        let data = json
            .get("ocs")
            .and_then(|ocs| ocs.get("data"))
            .and_then(|data| data.as_array())
            .ok_or_else(|| anyhow!("Invalid shares response format"))?;

        let mut shares = Vec::new();
        for item in data {
            if let Ok(share) =
                self.parse_share_response(&serde_json::json!({"ocs": {"data": item}}))
            {
                shares.push(share);
            }
        }

        Ok(shares)
    }

    /// Parse search response
    fn parse_search_response(&self, json: &Value) -> Result<Vec<NextCloudFile>> {
        // Implementation would parse the search JSON response
        // For now, return empty vector
        Ok(Vec::new())
    }
}

#[async_trait]
impl Service for NextCloudService {
    async fn start(&mut self) -> Result<(), ServiceError> {
        log::info!("Starting NextCloud service...");

        // Test connection
        if !self
            .test_connection()
            .await
            .map_err(|e| ServiceError::Connection(e.to_string()))?
        {
            return Err(ServiceError::Connection(
                "Failed to connect to NextCloud server".to_string(),
            ));
        }

        // Perform initial sync
        self.sync()
            .await
            .map_err(|e| ServiceError::Initialization(e.to_string()))?;

        log::info!("NextCloud service started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ServiceError> {
        log::info!("Stopping NextCloud service...");

        // Clear cache
        let mut cache = self.cache.write().await;
        cache.clear();

        log::info!("NextCloud service stopped");
        Ok(())
    }

    async fn health_check(&self) -> Result<ServiceHealth, ServiceError> {
        match self.test_connection().await {
            Ok(true) => Ok(ServiceHealth {
                status: HealthStatus::Healthy,
                message: Some("Connected to NextCloud".to_string()),
                last_check: SystemTime::now(),
            }),
            Ok(false) => Ok(ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some("Connection test failed".to_string()),
                last_check: SystemTime::now(),
            }),
            Err(e) => Ok(ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some(format!("Health check error: {}", e)),
                last_check: SystemTime::now(),
            }),
        }
    }

    fn get_config(&self) -> &ServiceConfig {
        &self.service_config
    }

    fn get_name(&self) -> &str {
        &self.service_config.name
    }
}

/// Initialize NextCloud service
pub async fn initialize() -> Result<()> {
    log::info!("NextCloud service initialized");
    Ok(())
}

/// NextCloud service factory
pub struct NextCloudServiceFactory;

impl NextCloudServiceFactory {
    pub fn create_service(config: NextCloudConfig) -> Result<NextCloudService> {
        NextCloudService::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> NextCloudConfig {
        NextCloudConfig {
            server_url: "https://test.nextcloud.com".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            webdav_path: None,
            timeout_seconds: 10,
            verify_ssl: false,
            sync_interval: 60,
            cache_directory: Some(PathBuf::from("/tmp/test_nextcloud")),
            max_upload_size: 1024 * 1024, // 1MB for testing
            enable_monitoring: false,
            chunk_size: 512 * 1024, // 512KB for testing
        }
    }

    #[tokio::test]
    async fn test_nextcloud_service_creation() {
        let config = test_config();
        let service = NextCloudService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_nextcloud_config_default() {
        let config = NextCloudConfig::default();
        assert!(config.verify_ssl);
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.sync_interval, 300);
    }

    #[test]
    fn test_nextcloud_file_serialization() {
        let file = NextCloudFile {
            path: "/test/file.txt".to_string(),
            name: "file.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
            modified: Utc::now(),
            etag: "abc123".to_string(),
            is_directory: false,
            file_id: Some("12345".to_string()),
            shares: vec![],
            permissions: "RGDNVW".to_string(),
            properties: HashMap::new(),
        };

        let json = serde_json::to_string(&file).unwrap();
        let deserialized: NextCloudFile = serde_json::from_str(&json).unwrap();

        assert_eq!(file.path, deserialized.path);
        assert_eq!(file.size, deserialized.size);
    }

    #[test]
    fn test_nextcloud_share_serialization() {
        let share = NextCloudShare {
            id: "123".to_string(),
            share_type: "link".to_string(),
            permissions: 1,
            shared_with: None,
            created_at: Utc::now(),
            expires_at: None,
            password_protected: false,
            url: Some("https://cloud.example.com/s/abc123".to_string()),
        };

        let json = serde_json::to_string(&share).unwrap();
        let deserialized: NextCloudShare = serde_json::from_str(&json).unwrap();

        assert_eq!(share.id, deserialized.id);
        assert_eq!(share.share_type, deserialized.share_type);
    }
}
