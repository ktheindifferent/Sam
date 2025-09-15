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
use std::io::Read;
use chrono::{DateTime, Utc};
use dropbox_sdk::{
    files, default_client::UserAuthDefaultClient,
    oauth2::Authorization
};
use thiserror::Error;
use crate::services::traits::{Service, ServiceConfig, ServiceHealth, HealthStatus, ServiceError};

#[derive(Error, Debug)]
pub enum DropboxError {
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, DropboxError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropboxConfig {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub app_key: String,
}

impl Default for DropboxConfig {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            refresh_token: None,
            client_id: "ogyeqdms81svfke".to_string(),
            client_secret: String::new(),
            app_key: "ogyeqdms81svfke".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropboxFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub is_folder: bool,
    pub mime_type: String,
    pub content_hash: Option<String>,
    pub rev: Option<String>,
}

pub struct DropboxService {
    config: DropboxConfig,
    service_config: ServiceConfig,
    client: Option<UserAuthDefaultClient>,
    cache: RwLock<HashMap<String, DropboxFile>>,
    last_sync: RwLock<SystemTime>,
}

impl DropboxService {
    pub fn new(config: DropboxConfig) -> Self {
        let service_config = ServiceConfig {
            name: "Dropbox".to_string(),
            enabled: true,
            retry_attempts: 3,
            timeout_seconds: 30,
        };

        Self {
            config,
            service_config,
            client: None,
            cache: RwLock::new(HashMap::new()),
            last_sync: RwLock::new(SystemTime::UNIX_EPOCH),
        }
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        if self.config.access_token.is_empty() {
            return Err(DropboxError::Authentication("No access token provided".to_string()));
        }

        // Create authorization using the load method with access token
        let saved_token = format!("1&{}", self.config.access_token);
        let auth = Authorization::load(self.config.client_id.clone(), &saved_token)
            .ok_or_else(|| DropboxError::Authentication("Failed to create authorization".to_string()))?;

        let client = UserAuthDefaultClient::new(auth);
        self.client = Some(client);
        Ok(())
    }

    pub async fn test_connection(&self) -> Result<bool> {
        match &self.client {
            Some(client) => {
                match dropbox_sdk::users::get_current_account(client) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(DropboxError::Api(format!("Connection test failed: {:?}", e))),
                }
            },
            None => Err(DropboxError::Authentication("Client not authenticated".to_string())),
        }
    }

    pub async fn list_files(&self, path: &str, limit: Option<u32>) -> Result<Vec<DropboxFile>> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let path = if path.is_empty() || path == "/" { "" } else { path };

        let list_arg = files::ListFolderArg::new(path.to_string())
            .with_limit(limit.unwrap_or(100));

        match files::list_folder(client, &list_arg) {
            Ok(Ok(result)) => {
                let mut files = Vec::new();

                for entry in result.entries {
                    match entry {
                        files::Metadata::File(file_metadata) => {
                            let file_name = file_metadata.name.clone();
                            files.push(DropboxFile {
                                id: file_metadata.id,
                                name: file_metadata.name,
                                path: file_metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                                size: file_metadata.size,
                                modified: file_metadata.server_modified.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now()),
                                is_folder: false,
                                mime_type: self.get_mime_type(&file_name),
                                content_hash: file_metadata.content_hash,
                                rev: Some(file_metadata.rev),
                            });
                        },
                        files::Metadata::Folder(folder_metadata) => {
                            files.push(DropboxFile {
                                id: folder_metadata.id,
                                name: folder_metadata.name,
                                path: folder_metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                                size: 0,
                                modified: Utc::now(),
                                is_folder: true,
                                mime_type: "application/x-directory".to_string(),
                                content_hash: None,
                                rev: None,
                            });
                        },
                        _ => {} // Ignore deleted entries
                    }
                }

                Ok(files)
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("List folder error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to list files: {:?}", e))),
        }
    }

    pub async fn upload_file(&self, _local_path: &Path, remote_path: &str, content: &[u8]) -> Result<DropboxFile> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let remote_path = if !remote_path.starts_with('/') {
            format!("/{}", remote_path)
        } else {
            remote_path.to_string()
        };

        let upload_arg = files::UploadArg::new(remote_path.clone())
            .with_mode(files::WriteMode::Overwrite)
            .with_autorename(true);

        match files::upload(client, &upload_arg, content) {
            Ok(Ok(metadata)) => {
                let file_name = metadata.name.clone();
                Ok(DropboxFile {
                    id: metadata.id,
                    name: metadata.name,
                    path: metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                    size: metadata.size,
                    modified: metadata.server_modified.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now()),
                    is_folder: false,
                    mime_type: self.get_mime_type(&file_name),
                    content_hash: metadata.content_hash,
                    rev: Some(metadata.rev),
                })
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("Upload error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to upload file: {:?}", e))),
        }
    }

    pub async fn download_file(&self, remote_path: &str) -> Result<Vec<u8>> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let download_arg = files::DownloadArg::new(remote_path.to_string());

        match files::download(client, &download_arg, None, None) {
            Ok(Ok(http_result)) => {
                match http_result.body {
                    Some(mut reader) => {
                        let mut content = Vec::new();
                        reader.read_to_end(&mut content)
                            .map_err(|e| DropboxError::Network(format!("Failed to read response body: {}", e)))?;
                        Ok(content)
                    },
                    None => Err(DropboxError::Api("No content in download response".to_string())),
                }
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("Download error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to download file: {:?}", e))),
        }
    }

    pub async fn delete_file(&self, remote_path: &str) -> Result<()> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let delete_arg = files::DeleteArg::new(remote_path.to_string());

        match files::delete_v2(client, &delete_arg) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(DropboxError::Api(format!("Delete error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to delete file: {:?}", e))),
        }
    }

    pub async fn create_folder(&self, path: &str) -> Result<DropboxFile> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let folder_arg = files::CreateFolderArg::new(path.to_string())
            .with_autorename(false);

        match files::create_folder_v2(client, &folder_arg) {
            Ok(Ok(result)) => {
                Ok(DropboxFile {
                    id: result.metadata.id,
                    name: result.metadata.name,
                    path: result.metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                    size: 0,
                    modified: Utc::now(),
                    is_folder: true,
                    mime_type: "application/x-directory".to_string(),
                    content_hash: None,
                    rev: None,
                })
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("Create folder error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to create folder: {:?}", e))),
        }
    }

    pub async fn move_file(&self, from_path: &str, to_path: &str) -> Result<DropboxFile> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let move_arg = files::RelocationArg::new(
            from_path.to_string(),
            to_path.to_string()
        )
        .with_allow_shared_folder(false)
        .with_autorename(false)
        .with_allow_ownership_transfer(false);

        match files::move_v2(client, &move_arg) {
            Ok(Ok(result)) => {
                match result.metadata {
                    files::Metadata::File(metadata) => {
                        let file_name = metadata.name.clone();
                        Ok(DropboxFile {
                            id: metadata.id,
                            name: metadata.name,
                            path: metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                            size: metadata.size,
                            modified: metadata.server_modified.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now()),
                            is_folder: false,
                            mime_type: self.get_mime_type(&file_name),
                            content_hash: metadata.content_hash,
                            rev: Some(metadata.rev),
                        })
                    },
                    files::Metadata::Folder(metadata) => {
                        Ok(DropboxFile {
                            id: metadata.id,
                            name: metadata.name,
                            path: metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                            size: 0,
                            modified: Utc::now(),
                            is_folder: true,
                            mime_type: "application/x-directory".to_string(),
                            content_hash: None,
                            rev: None,
                        })
                    },
                    _ => Err(DropboxError::Api("Unexpected metadata type".to_string())),
                }
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("Move error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to move file: {:?}", e))),
        }
    }

    pub async fn copy_file(&self, from_path: &str, to_path: &str) -> Result<DropboxFile> {
        let client = self.client.as_ref().ok_or_else(||
            DropboxError::Authentication("Client not authenticated".to_string())
        )?;

        let copy_arg = files::RelocationArg::new(
            from_path.to_string(),
            to_path.to_string()
        )
        .with_allow_shared_folder(false)
        .with_autorename(false)
        .with_allow_ownership_transfer(false);

        match files::copy_v2(client, &copy_arg) {
            Ok(Ok(result)) => {
                match result.metadata {
                    files::Metadata::File(metadata) => {
                        let file_name = metadata.name.clone();
                        Ok(DropboxFile {
                            id: metadata.id,
                            name: metadata.name,
                            path: metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                            size: metadata.size,
                            modified: metadata.server_modified.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now()),
                            is_folder: false,
                            mime_type: self.get_mime_type(&file_name),
                            content_hash: metadata.content_hash,
                            rev: Some(metadata.rev),
                        })
                    },
                    files::Metadata::Folder(metadata) => {
                        Ok(DropboxFile {
                            id: metadata.id,
                            name: metadata.name,
                            path: metadata.path_lower.unwrap_or_else(|| "unknown".to_string()),
                            size: 0,
                            modified: Utc::now(),
                            is_folder: true,
                            mime_type: "application/x-directory".to_string(),
                            content_hash: None,
                            rev: None,
                        })
                    },
                    _ => Err(DropboxError::Api("Unexpected metadata type".to_string())),
                }
            },
            Ok(Err(e)) => Err(DropboxError::Api(format!("Copy error: {:?}", e))),
            Err(e) => Err(DropboxError::Api(format!("Failed to copy file: {:?}", e))),
        }
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
impl Service for DropboxService {
    async fn start(&mut self) -> std::result::Result<(), ServiceError> {
        match self.authenticate().await {
            Ok(_) => {
                log::info!("Dropbox service started successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to start Dropbox service: {}", e);
                Err(ServiceError::Initialization(e.to_string()))
            }
        }
    }

    async fn stop(&mut self) -> std::result::Result<(), ServiceError> {
        self.client = None;
        log::info!("Dropbox service stopped");
        Ok(())
    }

    async fn health_check(&self) -> std::result::Result<ServiceHealth, ServiceError> {
        match self.test_connection().await {
            Ok(true) => Ok(ServiceHealth {
                status: HealthStatus::Healthy,
                message: Some("Dropbox connection successful".to_string()),
                last_check: SystemTime::now(),
            }),
            Ok(false) => Ok(ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some("Dropbox connection failed".to_string()),
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
        "dropbox"
    }
}