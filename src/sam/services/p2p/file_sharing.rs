// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use sha2::{Sha256, Digest};
use log::{info, warn, error, debug};

const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64KB
const MAX_CONCURRENT_TRANSFERS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub mime_type: Option<String>,
    pub chunks: Vec<ChunkInfo>,
    pub created_at: u64,
    pub modified_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub index: u32,
    pub offset: u64,
    pub size: usize,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferRequest {
    List {
        path: Option<String>,
        recursive: bool,
    },
    Get {
        file_id: String,
        chunk_index: Option<u32>,
    },
    Put {
        metadata: FileMetadata,
        chunk_index: u32,
        data: Vec<u8>,
    },
    Delete {
        file_id: String,
    },
    Resume {
        file_id: String,
        received_chunks: Vec<u32>,
    },
    Search {
        query: String,
        file_type: Option<String>,
        max_results: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferResponse {
    List {
        files: Vec<FileMetadata>,
    },
    Chunk {
        file_id: String,
        chunk_index: u32,
        data: Vec<u8>,
    },
    Metadata {
        metadata: FileMetadata,
    },
    Progress {
        file_id: String,
        chunks_received: u32,
        total_chunks: u32,
        bytes_transferred: u64,
        total_bytes: u64,
    },
    Success {
        file_id: String,
        message: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct FileTransferStatus {
    pub file_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub chunks_transferred: u32,
    pub total_chunks: u32,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bps: f64,
    pub eta_seconds: Option<u64>,
    pub peers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferState {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

pub struct FileTransferManager {
    shared_dir: PathBuf,
    temp_dir: PathBuf,
    chunk_size: usize,
    max_concurrent: usize,
    transfers: Arc<RwLock<HashMap<String, TransferInfo>>>,
    file_index: Arc<RwLock<HashMap<String, FileMetadata>>>,
}

struct TransferInfo {
    metadata: FileMetadata,
    status: FileTransferStatus,
    received_chunks: Vec<bool>,
    file_handle: Option<File>,
    start_time: std::time::Instant,
}

impl FileTransferManager {
    pub fn new(shared_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = shared_dir.join(".transfers");
        fs::create_dir_all(&shared_dir)?;
        fs::create_dir_all(&temp_dir)?;
        
        let mut manager = Self {
            shared_dir: shared_dir.clone(),
            temp_dir,
            chunk_size: DEFAULT_CHUNK_SIZE,
            max_concurrent: MAX_CONCURRENT_TRANSFERS,
            transfers: Arc::new(RwLock::new(HashMap::new())),
            file_index: Arc::new(RwLock::new(HashMap::new())),
        };
        
        manager.index_shared_files()?;
        
        Ok(manager)
    }

    pub fn index_shared_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut index = self.file_index.blocking_write();
        index.clear();
        
        self.index_directory(&self.shared_dir.clone(), &mut index)?;
        
        info!("Indexed {} files in shared directory", index.len());
        Ok(())
    }

    fn index_directory(&self, dir: &Path, index: &mut HashMap<String, FileMetadata>) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(metadata) = self.create_file_metadata(&path) {
                    index.insert(metadata.id.clone(), metadata);
                }
            } else if path.is_dir() && !path.starts_with(&self.temp_dir) {
                self.index_directory(&path, index)?;
            }
        }
        
        Ok(())
    }

    pub fn create_file_metadata(&self, path: &Path) -> Result<FileMetadata, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let file_meta = file.metadata()?;
        let size = file_meta.len();
        
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; self.chunk_size];
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut chunk_index = 0u32;
        
        let mut file = File::open(path)?;
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            let chunk_data = &buffer[..bytes_read];
            hasher.update(chunk_data);
            
            let mut chunk_hasher = Sha256::new();
            chunk_hasher.update(chunk_data);
            let chunk_hash = hex::encode(chunk_hasher.finalize());
            
            chunks.push(ChunkInfo {
                index: chunk_index,
                offset,
                size: bytes_read,
                hash: chunk_hash,
            });
            
            offset += bytes_read as u64;
            chunk_index += 1;
        }
        
        let file_hash = hex::encode(hasher.finalize());
        let file_id = format!("{}_{}", file_hash.chars().take(16).collect::<String>(), size);
        
        Ok(FileMetadata {
            id: file_id,
            name: path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size,
            hash: file_hash,
            mime_type: mime_guess::from_path(path)
                .first()
                .map(|m| m.to_string()),
            chunks,
            created_at: file_meta.created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            modified_at: file_meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        })
    }

    pub async fn handle_request(&self, request: FileTransferRequest, peer_id: String) -> FileTransferResponse {
        match request {
            FileTransferRequest::List { path, recursive } => {
                self.handle_list(path, recursive).await
            }
            FileTransferRequest::Get { file_id, chunk_index } => {
                self.handle_get(file_id, chunk_index).await
            }
            FileTransferRequest::Put { metadata, chunk_index, data } => {
                self.handle_put(metadata, chunk_index, data, peer_id).await
            }
            FileTransferRequest::Delete { file_id } => {
                self.handle_delete(file_id).await
            }
            FileTransferRequest::Resume { file_id, received_chunks } => {
                self.handle_resume(file_id, received_chunks).await
            }
            FileTransferRequest::Search { query, file_type, max_results } => {
                self.handle_search(query, file_type, max_results).await
            }
        }
    }

    async fn handle_list(&self, path: Option<String>, recursive: bool) -> FileTransferResponse {
        let index = self.file_index.read().await;
        
        let files: Vec<FileMetadata> = if let Some(path) = path {
            index.values()
                .filter(|f| f.name.starts_with(&path))
                .cloned()
                .collect()
        } else {
            index.values().cloned().collect()
        };
        
        FileTransferResponse::List { files }
    }

    async fn handle_get(&self, file_id: String, chunk_index: Option<u32>) -> FileTransferResponse {
        let index = self.file_index.read().await;
        
        if let Some(metadata) = index.get(&file_id) {
            if let Some(chunk_idx) = chunk_index {
                // Send specific chunk
                if let Some(chunk_info) = metadata.chunks.get(chunk_idx as usize) {
                    if let Ok(data) = self.read_chunk(&metadata.name, chunk_info).await {
                        return FileTransferResponse::Chunk {
                            file_id,
                            chunk_index: chunk_idx,
                            data,
                        };
                    }
                }
            } else {
                // Send metadata
                return FileTransferResponse::Metadata {
                    metadata: metadata.clone(),
                };
            }
        }
        
        FileTransferResponse::Error {
            message: format!("File not found: {}", file_id),
        }
    }

    async fn read_chunk(&self, filename: &str, chunk_info: &ChunkInfo) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let path = self.shared_dir.join(filename);
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(chunk_info.offset))?;
        
        let mut buffer = vec![0; chunk_info.size];
        file.read_exact(&mut buffer)?;
        
        Ok(buffer)
    }

    async fn handle_put(&self, metadata: FileMetadata, chunk_index: u32, data: Vec<u8>, peer_id: String) -> FileTransferResponse {
        let file_id = metadata.id.clone();
        let mut transfers = self.transfers.write().await;
        
        // Initialize transfer if new
        if !transfers.contains_key(&file_id) {
            let temp_path = self.temp_dir.join(&file_id);
            let file_handle = match File::create(&temp_path) {
                Ok(f) => f,
                Err(e) => {
                    return FileTransferResponse::Error {
                        message: format!("Failed to create temp file: {}", e),
                    };
                }
            };
            
            let total_chunks = metadata.chunks.len() as u32;
            
            let transfer_info = TransferInfo {
                metadata: metadata.clone(),
                status: FileTransferStatus {
                    file_id: file_id.clone(),
                    direction: TransferDirection::Download,
                    state: TransferState::InProgress,
                    chunks_transferred: 0,
                    total_chunks,
                    bytes_transferred: 0,
                    total_bytes: metadata.size,
                    speed_bps: 0.0,
                    eta_seconds: None,
                    peers: vec![peer_id.clone()],
                },
                received_chunks: vec![false; total_chunks as usize],
                file_handle: Some(file_handle),
                start_time: std::time::Instant::now(),
            };
            
            transfers.insert(file_id.clone(), transfer_info);
        }
        
        // Write chunk
        if let Some(transfer) = transfers.get_mut(&file_id) {
            if chunk_index < transfer.received_chunks.len() as u32 {
                if let Some(chunk_info) = metadata.chunks.get(chunk_index as usize) {
                    if let Some(file) = &mut transfer.file_handle {
                        if let Err(e) = file.seek(SeekFrom::Start(chunk_info.offset)) {
                            return FileTransferResponse::Error {
                                message: format!("Seek error: {}", e),
                            };
                        }
                        
                        if let Err(e) = file.write_all(&data) {
                            return FileTransferResponse::Error {
                                message: format!("Write error: {}", e),
                            };
                        }
                        
                        transfer.received_chunks[chunk_index as usize] = true;
                        transfer.status.chunks_transferred += 1;
                        transfer.status.bytes_transferred += data.len() as u64;
                        
                        // Update speed and ETA
                        let elapsed = transfer.start_time.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            transfer.status.speed_bps = transfer.status.bytes_transferred as f64 / elapsed;
                            let remaining_bytes = transfer.status.total_bytes - transfer.status.bytes_transferred;
                            if transfer.status.speed_bps > 0.0 {
                                transfer.status.eta_seconds = Some((remaining_bytes as f64 / transfer.status.speed_bps) as u64);
                            }
                        }
                        
                        // Check if transfer is complete
                        if transfer.received_chunks.iter().all(|&received| received) {
                            transfer.status.state = TransferState::Completed;
                            
                            // Move file to shared directory
                            let temp_path = self.temp_dir.join(&file_id);
                            let final_path = self.shared_dir.join(&metadata.name);
                            
                            if let Err(e) = fs::rename(&temp_path, &final_path) {
                                return FileTransferResponse::Error {
                                    message: format!("Failed to move file: {}", e),
                                };
                            }
                            
                            // Add to index
                            let mut index = self.file_index.write().await;
                            index.insert(file_id.clone(), metadata.clone());
                            
                            return FileTransferResponse::Success {
                                file_id,
                                message: format!("File {} received successfully", metadata.name),
                            };
                        }
                        
                        return FileTransferResponse::Progress {
                            file_id,
                            chunks_received: transfer.status.chunks_transferred,
                            total_chunks: transfer.status.total_chunks,
                            bytes_transferred: transfer.status.bytes_transferred,
                            total_bytes: transfer.status.total_bytes,
                        };
                    }
                }
            }
        }
        
        FileTransferResponse::Error {
            message: "Transfer error".to_string(),
        }
    }

    async fn handle_delete(&self, file_id: String) -> FileTransferResponse {
        let mut index = self.file_index.write().await;
        
        if let Some(metadata) = index.remove(&file_id) {
            let path = self.shared_dir.join(&metadata.name);
            if let Err(e) = fs::remove_file(&path) {
                return FileTransferResponse::Error {
                    message: format!("Failed to delete file: {}", e),
                };
            }
            
            FileTransferResponse::Success {
                file_id,
                message: format!("File {} deleted", metadata.name),
            }
        } else {
            FileTransferResponse::Error {
                message: format!("File not found: {}", file_id),
            }
        }
    }

    async fn handle_resume(&self, file_id: String, received_chunks: Vec<u32>) -> FileTransferResponse {
        let transfers = self.transfers.read().await;
        
        if let Some(transfer) = transfers.get(&file_id) {
            let missing_chunks: Vec<u32> = (0..transfer.status.total_chunks)
                .filter(|i| !received_chunks.contains(i))
                .collect();
            
            FileTransferResponse::Metadata {
                metadata: transfer.metadata.clone(),
            }
        } else {
            FileTransferResponse::Error {
                message: format!("Transfer not found: {}", file_id),
            }
        }
    }

    async fn handle_search(&self, query: String, file_type: Option<String>, max_results: usize) -> FileTransferResponse {
        let index = self.file_index.read().await;
        let query_lower = query.to_lowercase();
        
        let mut files: Vec<FileMetadata> = index.values()
            .filter(|f| {
                let name_matches = f.name.to_lowercase().contains(&query_lower);
                let type_matches = file_type.as_ref()
                    .map(|t| f.mime_type.as_ref().map(|m| m.contains(t)).unwrap_or(false))
                    .unwrap_or(true);
                name_matches && type_matches
            })
            .take(max_results)
            .cloned()
            .collect();
        
        FileTransferResponse::List { files }
    }

    pub async fn get_transfer_status(&self, file_id: &str) -> Option<FileTransferStatus> {
        let transfers = self.transfers.read().await;
        transfers.get(file_id).map(|t| t.status.clone())
    }

    pub async fn pause_transfer(&self, file_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transfers = self.transfers.write().await;
        
        if let Some(transfer) = transfers.get_mut(file_id) {
            transfer.status.state = TransferState::Paused;
            Ok(())
        } else {
            Err(format!("Transfer not found: {}", file_id).into())
        }
    }

    pub async fn resume_transfer(&self, file_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transfers = self.transfers.write().await;
        
        if let Some(transfer) = transfers.get_mut(file_id) {
            transfer.status.state = TransferState::InProgress;
            Ok(())
        } else {
            Err(format!("Transfer not found: {}", file_id).into())
        }
    }

    pub async fn cancel_transfer(&self, file_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transfers = self.transfers.write().await;
        
        if let Some(mut transfer) = transfers.remove(file_id) {
            transfer.status.state = TransferState::Cancelled;
            
            // Clean up temp file
            let temp_path = self.temp_dir.join(file_id);
            let _ = fs::remove_file(&temp_path);
            
            Ok(())
        } else {
            Err(format!("Transfer not found: {}", file_id).into())
        }
    }
}