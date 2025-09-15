// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::SystemTime;
use chrono::{DateTime, Utc};
use reqwest::Client;
use thiserror::Error;
use crate::services::traits::{Service, ServiceConfig, ServiceHealth, HealthStatus, ServiceError};

#[derive(Error, Debug)]
pub enum SeaweedError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

type Result<T> = std::result::Result<T, SeaweedError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeaweedFSConfig {
    pub master_url: String,
    pub filer_url: String,
    pub volume_server_url: String,
    pub collection: String,
    pub replication: String,
    pub ttl: Option<String>,
    pub data_center: Option<String>,
    pub rack: Option<String>,
}

impl Default for SeaweedFSConfig {
    fn default() -> Self {
        Self {
            master_url: "http://localhost:9333".to_string(),
            filer_url: "http://localhost:8888".to_string(),
            volume_server_url: "http://localhost:8080".to_string(),
            collection: "sam".to_string(),
            replication: "000".to_string(), // No replication by default
            ttl: None,
            data_center: None,
            rack: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeaweedFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub is_folder: bool,
    pub mime_type: String,
    pub fid: Option<String>, // SeaweedFS file ID
    pub url: Option<String>, // Direct access URL
}

#[derive(Debug, Deserialize)]
pub struct AssignResponse {
    pub fid: String,
    pub url: String,
    pub public_url: String,
    pub count: u64,
}

#[derive(Debug, Deserialize)]
pub struct UploadResult {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct DirListEntry {
    pub name: String,
    #[serde(rename = "Mtime")]
    pub mtime: String,
    #[serde(rename = "Mode")]
    pub mode: u32,
    #[serde(rename = "Size")]
    pub size: Option<u64>,
    #[serde(rename = "IsDir")]
    pub is_dir: bool,
}

#[derive(Debug, Deserialize)]
pub struct DirListResponse {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Entries")]
    pub entries: Option<Vec<DirListEntry>>,
}

pub struct SeaweedFSService {
    config: SeaweedFSConfig,
    service_config: ServiceConfig,
    client: Client,
    cache: RwLock<HashMap<String, SeaweedFile>>,
    last_sync: RwLock<SystemTime>,
}

impl SeaweedFSService {
    pub fn new(config: SeaweedFSConfig) -> Self {
        let service_config = ServiceConfig {
            name: "SeaweedFS".to_string(),
            enabled: true,
            retry_attempts: 3,
            timeout_seconds: 30,
        };

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            service_config,
            client,
            cache: RwLock::new(HashMap::new()),
            last_sync: RwLock::new(SystemTime::UNIX_EPOCH),
        }
    }

    pub async fn test_connection(&self) -> Result<bool> {
        // Test master server
        let master_status_url = format!("{}/dir/status", self.config.master_url);
        let master_response = self.client.get(&master_status_url).send().await?;

        if !master_response.status().is_success() {
            return Ok(false);
        }

        // Test filer server
        let filer_status_url = format!("{}/", self.config.filer_url);
        let filer_response = self.client.get(&filer_status_url).send().await?;

        Ok(filer_response.status().is_success())
    }

    pub async fn assign_file_key(&self) -> Result<AssignResponse> {
        let mut assign_url = format!("{}/dir/assign", self.config.master_url);

        let mut params = vec![("collection", self.config.collection.as_str())];

        if !self.config.replication.is_empty() {
            params.push(("replication", &self.config.replication));
        }

        if let Some(ref ttl) = self.config.ttl {
            params.push(("ttl", ttl));
        }

        if let Some(ref dc) = self.config.data_center {
            params.push(("dataCenter", dc));
        }

        if let Some(ref rack) = self.config.rack {
            params.push(("rack", rack));
        }

        let response = self.client.get(&assign_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to assign file key: {}", response.status())
            ));
        }

        let assign_response: AssignResponse = response.json().await?;
        Ok(assign_response)
    }

    pub async fn list_files(&self, path: &str, limit: Option<u32>) -> Result<Vec<SeaweedFile>> {
        let normalized_path = if path.is_empty() || path == "/" { "/" } else { path };

        let list_url = format!("{}{}", self.config.filer_url, normalized_path);
        let mut query_params = vec![("pretty", "y")];

        let limit_string;
        if let Some(limit) = limit {
            limit_string = limit.to_string();
            query_params.push(("limit", &limit_string));
        }

        let response = self.client.get(&list_url)
            .query(&query_params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to list files: {}", response.status())
            ));
        }

        let dir_response: DirListResponse = response.json().await?;
        let mut files = Vec::new();

        if let Some(entries) = dir_response.entries {
            for entry in entries {
                let full_path = if normalized_path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", normalized_path, entry.name)
                };

                let modified = DateTime::parse_from_rfc3339(&entry.mtime)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let entry_name = entry.name.clone();
                files.push(SeaweedFile {
                    id: entry_name.clone(),
                    name: entry_name.clone(),
                    path: full_path,
                    size: entry.size.unwrap_or(0),
                    modified,
                    is_folder: entry.is_dir,
                    mime_type: if entry.is_dir {
                        "application/x-directory".to_string()
                    } else {
                        self.get_mime_type(&entry_name)
                    },
                    fid: None,
                    url: None,
                });
            }
        }

        Ok(files)
    }

    pub async fn upload_file(&self, local_path: &Path, remote_path: &str, content: &[u8]) -> Result<SeaweedFile> {
        // First, assign a file key
        let assign_response = self.assign_file_key().await?;

        // Upload to volume server
        let upload_url = format!("http://{}/{}", assign_response.url, assign_response.fid);

        let filename = local_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed");

        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(content.to_vec())
                .file_name(filename.to_string())
                .mime_str(&self.get_mime_type(filename)).unwrap_or(
                    reqwest::multipart::Part::bytes(content.to_vec())
                        .file_name(filename.to_string())
                )
            );

        let response = self.client.post(&upload_url)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to upload file: {}", response.status())
            ));
        }

        let _upload_result: UploadResult = response.json().await?;

        // Link file in filer
        let filer_path = if remote_path.starts_with('/') {
            remote_path.to_string()
        } else {
            format!("/{}", remote_path)
        };

        let filer_url = format!("{}{}", self.config.filer_url, filer_path);

        let link_response = self.client.post(&filer_url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "fid": assign_response.fid,
                "size": content.len()
            }))
            .send()
            .await?;

        if !link_response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to link file in filer: {}", link_response.status())
            ));
        }

        Ok(SeaweedFile {
            id: assign_response.fid.clone(),
            name: filename.to_string(),
            path: filer_path.clone(),
            size: content.len() as u64,
            modified: Utc::now(),
            is_folder: false,
            mime_type: self.get_mime_type(filename),
            fid: Some(assign_response.fid),
            url: Some(format!("{}{}", self.config.filer_url, filer_path)),
        })
    }

    pub async fn download_file(&self, remote_path: &str) -> Result<Vec<u8>> {
        let download_url = format!("{}{}", self.config.filer_url, remote_path);

        let response = self.client.get(&download_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to download file: {}", response.status())
            ));
        }

        let content = response.bytes().await?;
        Ok(content.to_vec())
    }

    pub async fn delete_file(&self, remote_path: &str) -> Result<()> {
        let delete_url = format!("{}{}", self.config.filer_url, remote_path);

        let response = self.client.delete(&delete_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to delete file: {}", response.status())
            ));
        }

        Ok(())
    }

    pub async fn create_folder(&self, path: &str) -> Result<SeaweedFile> {
        let folder_path = if path.starts_with('/') {
            format!("{}/", path)
        } else {
            format!("/{}/", path)
        };

        let create_url = format!("{}{}", self.config.filer_url, folder_path);

        let response = self.client.post(&create_url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SeaweedError::Api(
                format!("Failed to create folder: {}", response.status())
            ));
        }

        let folder_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("folder")
            .to_string();

        Ok(SeaweedFile {
            id: folder_name.clone(),
            name: folder_name,
            path: folder_path.clone(),
            size: 0,
            modified: Utc::now(),
            is_folder: true,
            mime_type: "application/x-directory".to_string(),
            fid: None,
            url: Some(format!("{}{}", self.config.filer_url, folder_path)),
        })
    }

    pub async fn move_file(&self, from_path: &str, to_path: &str) -> Result<SeaweedFile> {
        // SeaweedFS doesn't have native move, so we copy then delete
        let content = self.download_file(from_path).await?;
        let moved_file = self.upload_file(Path::new(to_path), to_path, &content).await?;
        self.delete_file(from_path).await?;
        Ok(moved_file)
    }

    pub async fn copy_file(&self, from_path: &str, to_path: &str) -> Result<SeaweedFile> {
        let content = self.download_file(from_path).await?;
        self.upload_file(Path::new(to_path), to_path, &content).await
    }

    pub fn create_streaming_url(&self, file_path: &str) -> String {
        format!("{}{}", self.config.filer_url, file_path)
    }

    fn get_mime_type(&self, filename: &str) -> String {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("webp") => "image/webp".to_string(),
            Some("mp4") => "video/mp4".to_string(),
            Some("avi") => "video/x-msvideo".to_string(),
            Some("mov") => "video/quicktime".to_string(),
            Some("mkv") => "video/x-matroska".to_string(),
            Some("mp3") => "audio/mpeg".to_string(),
            Some("wav") => "audio/wav".to_string(),
            Some("flac") => "audio/flac".to_string(),
            Some("ogg") => "audio/ogg".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("doc") => "application/msword".to_string(),
            Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            Some("txt") => "text/plain".to_string(),
            Some("json") => "application/json".to_string(),
            Some("xml") => "application/xml".to_string(),
            Some("html") => "text/html".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

#[async_trait]
impl Service for SeaweedFSService {
    async fn start(&mut self) -> std::result::Result<(), ServiceError> {
        match self.test_connection().await {
            Ok(true) => {
                log::info!("SeaweedFS service started successfully");
                Ok(())
            }
            Ok(false) => {
                let error_msg = "SeaweedFS servers are not accessible".to_string();
                log::error!("Failed to start SeaweedFS service: {}", error_msg);
                Err(ServiceError::Connection(error_msg))
            }
            Err(e) => {
                log::error!("Failed to start SeaweedFS service: {}", e);
                Err(ServiceError::Initialization(e.to_string()))
            }
        }
    }

    async fn stop(&mut self) -> std::result::Result<(), ServiceError> {
        log::info!("SeaweedFS service stopped");
        Ok(())
    }

    async fn health_check(&self) -> std::result::Result<ServiceHealth, ServiceError> {
        match self.test_connection().await {
            Ok(true) => Ok(ServiceHealth {
                status: HealthStatus::Healthy,
                message: Some("SeaweedFS connection successful".to_string()),
                last_check: SystemTime::now(),
            }),
            Ok(false) => Ok(ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some("SeaweedFS servers not accessible".to_string()),
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
        "seaweedfs"
    }
}