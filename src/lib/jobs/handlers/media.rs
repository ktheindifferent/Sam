use async_trait::async_trait;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use crate::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPayload {
    pub input_path: String,
    pub output_path: String,
    pub operation: MediaOperation,
    pub format: Option<String>,
    pub quality: Option<u8>,
    pub metadata: Option<MediaMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaOperation {
    Transcode { codec: String, bitrate: Option<u32> },
    Resize { width: u32, height: u32, maintain_aspect: bool },
    Thumbnail { width: u32, height: u32, timestamp_ms: Option<u64> },
    Extract { start_ms: u64, duration_ms: u64 },
    Compress { target_size_mb: Option<u32> },
    Watermark { image_path: String, position: WatermarkPosition },
    Convert { format: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
}

pub struct MediaProcessingJobHandler {
    temp_dir: PathBuf,
    ffmpeg_path: Option<PathBuf>,
}

impl MediaProcessingJobHandler {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            ffmpeg_path: None,
        }
    }
    
    async fn process_media(&self, payload: MediaPayload) -> Result<MediaResult, String> {
        info!("Processing media: {} -> {}", payload.input_path, payload.output_path);
        
        // Check if input file exists
        let input_path = PathBuf::from(&payload.input_path);
        if !input_path.exists() {
            return Err(format!("Input file does not exist: {}", payload.input_path));
        }
        
        let start_time = std::time::Instant::now();
        
        // Simulate media processing based on operation
        match &payload.operation {
            MediaOperation::Transcode { codec, bitrate } => {
                info!("Transcoding to codec: {}, bitrate: {:?}", codec, bitrate);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            MediaOperation::Resize { width, height, maintain_aspect } => {
                info!("Resizing to {}x{}, maintain aspect: {}", width, height, maintain_aspect);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            MediaOperation::Thumbnail { width, height, timestamp_ms } => {
                info!("Generating thumbnail {}x{} at {:?}ms", width, height, timestamp_ms);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            MediaOperation::Extract { start_ms, duration_ms } => {
                info!("Extracting from {}ms for {}ms", start_ms, duration_ms);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            MediaOperation::Compress { target_size_mb } => {
                info!("Compressing to target size: {:?}MB", target_size_mb);
                tokio::time::sleep(Duration::from_secs(4)).await;
            }
            MediaOperation::Watermark { image_path, position } => {
                info!("Adding watermark from {} at {:?}", image_path, position);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            MediaOperation::Convert { format } => {
                info!("Converting to format: {}", format);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
        
        // Simulate file size
        let output_size = rand::random::<u64>() % 100_000_000;
        
        Ok(MediaResult {
            output_path: payload.output_path.clone(),
            size_bytes: output_size,
            duration_secs: start_time.elapsed().as_secs(),
            format: payload.format.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MediaResult {
    output_path: String,
    size_bytes: u64,
    duration_secs: u64,
    format: Option<String>,
}

#[async_trait]
impl JobHandler for MediaProcessingJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        let media_payload: MediaPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid media payload: {}", e)))?;
        
        match self.process_media(media_payload).await {
            Ok(result) => {
                info!("Media processing completed: {} ({} bytes)", 
                      result.output_path, result.size_bytes);
                
                Ok(JobResult::Success(serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"status": "completed"}))))
            }
            Err(e) => {
                if e.contains("codec") || e.contains("format not supported") {
                    // Permanent error
                    error!("Media processing failed permanently: {}", e);
                    Ok(JobResult::Failure(e))
                } else {
                    // Transient error, should retry
                    Ok(JobResult::Retry(e))
                }
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        2 // Fewer retries for media processing as it's resource intensive
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        Duration::from_secs(60 * attempt as u64)
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(7200)) // 2 hour timeout for media processing
    }
    
    fn name(&self) -> &str {
        "media_processing"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let media_payload: MediaPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        if media_payload.input_path.is_empty() {
            return Err(JobError::ExecutionFailed("Input path is required".to_string()));
        }
        
        if media_payload.output_path.is_empty() {
            return Err(JobError::ExecutionFailed("Output path is required".to_string()));
        }
        
        // Validate quality if specified
        if let Some(quality) = media_payload.quality {
            if quality > 100 {
                return Err(JobError::ExecutionFailed(
                    format!("Quality {} is invalid (must be 0-100)", quality)
                ));
            }
        }
        
        Ok(())
    }
}