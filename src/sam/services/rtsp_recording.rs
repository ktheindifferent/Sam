// RTSP Recording Module
// Provides recording capabilities for RTSP streams with triggers, storage management, and playback

use crate::sam::memory::{Thing, PostgresQueries, PGCol};
use crate::sam::services::errors::ServiceError;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::sync::mpsc;
use tokio::task;
use tokio::time;

// Recording Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub thing_oid: String,
    pub rtsp_url: String,
    pub storage_path: PathBuf,
    pub network_storage: Option<NetworkStorage>,
    pub encoding: VideoEncoding,
    pub segment_duration: Duration,
    pub retention_days: u32,
    pub triggers: Vec<RecordingTrigger>,
    pub max_storage_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStorage {
    pub storage_type: StorageType,
    pub host: String,
    pub path: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    NAS,
    S3,
    FTP,
    SMB,
    WebDAV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoding {
    pub codec: VideoCodec,
    pub bitrate: u32,
    pub fps: u32,
    pub resolution: Resolution,
    pub audio_codec: AudioCodec,
    pub audio_bitrate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    VP9,
    AV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCodec {
    AAC,
    MP3,
    Opus,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingTrigger {
    Continuous,
    Motion,
    Schedule(ScheduleTrigger),
    Manual,
    Event(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTrigger {
    pub days_of_week: Vec<u8>, // 0 = Sunday, 6 = Saturday
    pub start_time: String,    // HH:MM format
    pub end_time: String,       // HH:MM format
}

// Recording Session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub session_id: String,
    pub thing_oid: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub trigger: RecordingTrigger,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub duration_seconds: Option<u64>,
    pub metadata: RecordingMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub codec: String,
    pub resolution: String,
    pub fps: f32,
    pub bitrate: u32,
    pub events: Vec<RecordingEvent>,
    pub thumbnails: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub description: String,
    pub data: Option<serde_json::Value>,
}

// Recording Manager
pub struct RecordingManager {
    configs: Arc<Mutex<HashMap<String, RecordingConfig>>>,
    active_recordings: Arc<Mutex<HashMap<String, RecordingHandle>>>,
    storage_manager: StorageManager,
    metadata_store: MetadataStore,
}

struct RecordingHandle {
    process: std::process::Child,
    session: RecordingSession,
    start_time: SystemTime,
}

impl RecordingManager {
    pub fn new(base_storage_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_storage_path)?;
        
        Ok(Self {
            configs: Arc::new(Mutex::new(HashMap::new())),
            active_recordings: Arc::new(Mutex::new(HashMap::new())),
            storage_manager: StorageManager::new(base_storage_path.clone())?,
            metadata_store: MetadataStore::new()?,
        })
    }

    pub fn add_camera(&mut self, config: RecordingConfig) -> Result<()> {
        let mut configs = self.configs.lock().unwrap();
        configs.insert(config.thing_oid.clone(), config);
        Ok(())
    }

    pub async fn start_recording(
        &self,
        thing_oid: String,
        trigger: RecordingTrigger,
    ) -> Result<String> {
        let configs = self.configs.lock().unwrap();
        let config = configs.get(&thing_oid)
            .ok_or_else(|| anyhow::anyhow!("Camera config not found"))?
            .clone();
        drop(configs);

        // Generate session ID and file path
        let session_id = format!("{}_{}", thing_oid, Utc::now().timestamp());
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("{}_{}.mp4", thing_oid, timestamp);
        let file_path = config.storage_path.join(&filename);

        // Prepare FFmpeg command
        let ffmpeg_cmd = self.build_ffmpeg_command(&config, &file_path)?;
        
        // Start recording process
        let process = Command::new("ffmpeg")
            .args(&ffmpeg_cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let session = RecordingSession {
            session_id: session_id.clone(),
            thing_oid: thing_oid.clone(),
            start_time: Utc::now(),
            end_time: None,
            trigger: trigger.clone(),
            file_path: file_path.clone(),
            file_size: 0,
            duration_seconds: None,
            metadata: RecordingMetadata {
                codec: format!("{:?}", config.encoding.codec),
                resolution: format!("{}x{}", config.encoding.resolution.width, config.encoding.resolution.height),
                fps: config.encoding.fps as f32,
                bitrate: config.encoding.bitrate,
                events: Vec::new(),
                thumbnails: Vec::new(),
            },
        };

        let handle = RecordingHandle {
            process,
            session: session.clone(),
            start_time: SystemTime::now(),
        };

        let mut recordings = self.active_recordings.lock().unwrap();
        recordings.insert(session_id.clone(), handle);

        // Store metadata in database
        self.metadata_store.save_session(&session).await?;

        // Schedule automatic stop based on segment duration
        let recordings_clone = self.active_recordings.clone();
        let session_id_clone = session_id.clone();
        let segment_duration = config.segment_duration;
        tokio::spawn(async move {
            time::sleep(segment_duration).await;
            if let Ok(mut recordings) = recordings_clone.lock() {
                if let Some(mut handle) = recordings.remove(&session_id_clone) {
                    let _ = handle.process.kill();
                }
            }
        });

        Ok(session_id)
    }

    pub async fn stop_recording(&self, session_id: &str) -> Result<RecordingSession> {
        let mut recordings = self.active_recordings.lock().unwrap();
        let mut handle = recordings.remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Recording not found"))?;

        // Stop the FFmpeg process gracefully
        handle.process.kill()?;
        handle.process.wait()?;

        // Update session metadata
        let duration = handle.start_time.elapsed()?.as_secs();
        handle.session.end_time = Some(Utc::now());
        handle.session.duration_seconds = Some(duration);
        
        // Get file size
        if let Ok(metadata) = fs::metadata(&handle.session.file_path) {
            handle.session.file_size = metadata.len();
        }

        // Generate thumbnail
        self.generate_thumbnail(&handle.session).await?;

        // Update metadata in database
        self.metadata_store.update_session(&handle.session).await?;

        // Upload to network storage if configured
        if let Some(configs) = self.configs.lock().unwrap().get(&handle.session.thing_oid) {
            if let Some(network_storage) = &configs.network_storage {
                self.upload_to_network_storage(&handle.session, network_storage).await?;
            }
        }

        Ok(handle.session)
    }

    pub async fn check_triggers(&self) -> Result<()> {
        let configs = self.configs.lock().unwrap().clone();
        
        for (thing_oid, config) in configs {
            for trigger in &config.triggers {
                if self.should_trigger(trigger).await? {
                    if !self.is_recording(&thing_oid) {
                        self.start_recording(thing_oid.clone(), trigger.clone()).await?;
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn should_trigger(&self, trigger: &RecordingTrigger) -> Result<bool> {
        match trigger {
            RecordingTrigger::Continuous => Ok(true),
            RecordingTrigger::Motion => {
                // Check if motion was detected recently
                // This would integrate with the DL module
                Ok(false) // Placeholder
            }
            RecordingTrigger::Schedule(schedule) => {
                let now = Utc::now();
                let weekday = now.weekday().num_days_from_sunday() as u8;
                
                if !schedule.days_of_week.contains(&weekday) {
                    return Ok(false);
                }
                
                let current_time = now.format("%H:%M").to_string();
                Ok(current_time >= schedule.start_time && current_time <= schedule.end_time)
            }
            RecordingTrigger::Manual => Ok(false),
            RecordingTrigger::Event(_) => Ok(false), // Placeholder for event-based triggers
        }
    }

    fn is_recording(&self, thing_oid: &str) -> bool {
        let recordings = self.active_recordings.lock().unwrap();
        recordings.values().any(|h| h.session.thing_oid == thing_oid)
    }

    fn build_ffmpeg_command(&self, config: &RecordingConfig, output_path: &Path) -> Result<Vec<String>> {
        let mut args = vec![
            "-i".to_string(),
            config.rtsp_url.clone(),
            "-rtsp_transport".to_string(),
            "tcp".to_string(),
        ];

        // Video encoding settings
        match config.encoding.codec {
            VideoCodec::H264 => {
                args.extend_from_slice(&[
                    "-c:v".to_string(), "libx264".to_string(),
                    "-preset".to_string(), "medium".to_string(),
                    "-crf".to_string(), "23".to_string(),
                ]);
            }
            VideoCodec::H265 => {
                args.extend_from_slice(&[
                    "-c:v".to_string(), "libx265".to_string(),
                    "-preset".to_string(), "medium".to_string(),
                    "-crf".to_string(), "28".to_string(),
                ]);
            }
            VideoCodec::VP9 => {
                args.extend_from_slice(&[
                    "-c:v".to_string(), "libvpx-vp9".to_string(),
                    "-crf".to_string(), "30".to_string(),
                    "-b:v".to_string(), "0".to_string(),
                ]);
            }
            VideoCodec::AV1 => {
                args.extend_from_slice(&[
                    "-c:v".to_string(), "libaom-av1".to_string(),
                    "-crf".to_string(), "30".to_string(),
                ]);
            }
        }

        // Bitrate and FPS
        args.extend_from_slice(&[
            "-b:v".to_string(), format!("{}k", config.encoding.bitrate),
            "-r".to_string(), config.encoding.fps.to_string(),
        ]);

        // Resolution
        args.extend_from_slice(&[
            "-s".to_string(),
            format!("{}x{}", config.encoding.resolution.width, config.encoding.resolution.height),
        ]);

        // Audio encoding
        match config.encoding.audio_codec {
            AudioCodec::AAC => {
                args.extend_from_slice(&[
                    "-c:a".to_string(), "aac".to_string(),
                    "-b:a".to_string(), format!("{}k", config.encoding.audio_bitrate),
                ]);
            }
            AudioCodec::MP3 => {
                args.extend_from_slice(&[
                    "-c:a".to_string(), "libmp3lame".to_string(),
                    "-b:a".to_string(), format!("{}k", config.encoding.audio_bitrate),
                ]);
            }
            AudioCodec::Opus => {
                args.extend_from_slice(&[
                    "-c:a".to_string(), "libopus".to_string(),
                    "-b:a".to_string(), format!("{}k", config.encoding.audio_bitrate),
                ]);
            }
            AudioCodec::None => {
                args.extend_from_slice(&["-an".to_string()]);
            }
        }

        // Output settings
        args.extend_from_slice(&[
            "-movflags".to_string(), "+faststart".to_string(),
            "-y".to_string(), // Overwrite output file
            output_path.to_string_lossy().to_string(),
        ]);

        Ok(args)
    }

    async fn generate_thumbnail(&self, session: &RecordingSession) -> Result<()> {
        let thumbnail_path = session.file_path.with_extension("jpg");
        
        let output = Command::new("ffmpeg")
            .args(&[
                "-i", session.file_path.to_str().unwrap(),
                "-ss", "00:00:01",
                "-vframes", "1",
                "-vf", "scale=320:240",
                "-y",
                thumbnail_path.to_str().unwrap(),
            ])
            .output()?;

        if output.status.success() {
            log::info!("Generated thumbnail for session {}", session.session_id);
        } else {
            log::error!("Failed to generate thumbnail: {:?}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    async fn upload_to_network_storage(
        &self,
        session: &RecordingSession,
        storage: &NetworkStorage,
    ) -> Result<()> {
        match storage.storage_type {
            StorageType::NAS | StorageType::SMB => {
                // Mount network drive and copy file
                let mount_point = format!("/mnt/{}", session.thing_oid);
                fs::create_dir_all(&mount_point)?;
                
                let mount_cmd = format!(
                    "mount -t cifs //{}/{} {} -o username={},password={}",
                    storage.host,
                    storage.path,
                    mount_point,
                    storage.username.as_deref().unwrap_or("guest"),
                    storage.password.as_deref().unwrap_or(""),
                );
                
                Command::new("sh").arg("-c").arg(&mount_cmd).output()?;
                
                let dest_path = format!("{}/{}", mount_point, session.file_path.file_name().unwrap().to_str().unwrap());
                fs::copy(&session.file_path, &dest_path)?;
                
                Command::new("umount").arg(&mount_point).output()?;
            }
            StorageType::S3 => {
                // Use AWS CLI or rusoto for S3 upload
                let s3_path = format!("s3://{}/{}/{}", 
                    storage.host,
                    storage.path,
                    session.file_path.file_name().unwrap().to_str().unwrap()
                );
                
                Command::new("aws")
                    .args(&["s3", "cp", session.file_path.to_str().unwrap(), &s3_path])
                    .output()?;
            }
            StorageType::FTP => {
                // Use FTP client for upload
                let ftp_cmd = format!(
                    "curl -T {} ftp://{}:{}/{}/ --user {}:{}",
                    session.file_path.to_str().unwrap(),
                    storage.host,
                    21,
                    storage.path,
                    storage.username.as_deref().unwrap_or("anonymous"),
                    storage.password.as_deref().unwrap_or(""),
                );
                
                Command::new("sh").arg("-c").arg(&ftp_cmd).output()?;
            }
            StorageType::WebDAV => {
                // Use WebDAV client for upload
                let webdav_url = format!("https://{}/{}/{}",
                    storage.host,
                    storage.path,
                    session.file_path.file_name().unwrap().to_str().unwrap()
                );
                
                Command::new("curl")
                    .args(&[
                        "-T", session.file_path.to_str().unwrap(),
                        "-u", &format!("{}:{}", 
                            storage.username.as_deref().unwrap_or(""),
                            storage.password.as_deref().unwrap_or("")),
                        &webdav_url,
                    ])
                    .output()?;
            }
        }
        
        log::info!("Uploaded recording {} to network storage", session.session_id);
        Ok(())
    }
}

// Storage Manager
pub struct StorageManager {
    base_path: PathBuf,
    retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub max_days: u32,
    pub max_size_gb: f64,
}

impl StorageManager {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        Ok(Self {
            base_path,
            retention_policy: RetentionPolicy {
                max_days: 30,
                max_size_gb: 100.0,
            },
        })
    }

    pub async fn cleanup_old_recordings(&self) -> Result<()> {
        let cutoff_date = Utc::now() - ChronoDuration::days(self.retention_policy.max_days as i64);
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension() == Some(std::ffi::OsStr::new("mp4")) {
                let metadata = fs::metadata(&path)?;
                let modified = metadata.modified()?;
                let modified_dt = DateTime::<Utc>::from(modified);
                
                if modified_dt < cutoff_date {
                    fs::remove_file(&path)?;
                    log::info!("Deleted old recording: {:?}", path);
                }
            }
        }
        
        Ok(())
    }

    pub async fn check_storage_usage(&self) -> Result<f64> {
        let mut total_size = 0u64;
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let metadata = fs::metadata(entry.path())?;
            total_size += metadata.len();
        }
        
        let size_gb = total_size as f64 / (1024.0 * 1024.0 * 1024.0);
        
        if size_gb > self.retention_policy.max_size_gb {
            log::warn!("Storage usage ({:.2} GB) exceeds limit ({:.2} GB)", size_gb, self.retention_policy.max_size_gb);
            self.cleanup_oldest_recordings(size_gb - self.retention_policy.max_size_gb).await?;
        }
        
        Ok(size_gb)
    }

    async fn cleanup_oldest_recordings(&self, size_to_free_gb: f64) -> Result<()> {
        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension() == Some(std::ffi::OsStr::new("mp4")) {
                let metadata = fs::metadata(&path)?;
                files.push((path, metadata.modified()?));
            }
        }
        
        // Sort by modification time (oldest first)
        files.sort_by_key(|f| f.1);
        
        let mut freed_size = 0f64;
        let size_to_free = size_to_free_gb * 1024.0 * 1024.0 * 1024.0;
        
        for (path, _) in files {
            if freed_size >= size_to_free {
                break;
            }
            
            let metadata = fs::metadata(&path)?;
            let file_size = metadata.len() as f64;
            
            fs::remove_file(&path)?;
            freed_size += file_size;
            log::info!("Deleted recording to free space: {:?}", path);
        }
        
        Ok(())
    }
}

// Metadata Store
pub struct MetadataStore {
    // This would typically use PostgreSQL
}

impl MetadataStore {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn save_session(&self, session: &RecordingSession) -> Result<()> {
        // Save to PostgreSQL
        task::spawn_blocking({
            let session = session.clone();
            move || -> Result<()> {
                let mut client = crate::sam::memory::Config::client()?;
                
                client.execute(
                    "INSERT INTO recording_sessions (session_id, thing_oid, start_time, trigger, file_path, metadata) 
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (session_id) DO UPDATE SET
                     end_time = EXCLUDED.end_time,
                     file_size = EXCLUDED.file_size,
                     duration_seconds = EXCLUDED.duration_seconds,
                     metadata = EXCLUDED.metadata",
                    &[
                        &session.session_id,
                        &session.thing_oid,
                        &session.start_time,
                        &serde_json::to_string(&session.trigger)?,
                        &session.file_path.to_string_lossy().to_string(),
                        &serde_json::to_value(&session.metadata)?,
                    ],
                )?;
                
                Ok(())
            }
        }).await??;
        
        Ok(())
    }

    pub async fn update_session(&self, session: &RecordingSession) -> Result<()> {
        self.save_session(session).await
    }

    pub async fn get_sessions(
        &self,
        thing_oid: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<RecordingSession>> {
        task::spawn_blocking(move || -> Result<Vec<RecordingSession>> {
            let mut client = crate::sam::memory::Config::client()?;
            
            let mut query = "SELECT * FROM recording_sessions WHERE 1=1".to_string();
            let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = Vec::new();
            
            if let Some(oid) = thing_oid {
                query.push_str(&format!(" AND thing_oid = ${}", params.len() + 1));
                params.push(Box::new(oid));
            }
            
            if let Some(start) = start_time {
                query.push_str(&format!(" AND start_time >= ${}", params.len() + 1));
                params.push(Box::new(start));
            }
            
            if let Some(end) = end_time {
                query.push_str(&format!(" AND start_time <= ${}", params.len() + 1));
                params.push(Box::new(end));
            }
            
            query.push_str(" ORDER BY start_time DESC");
            
            // Note: This is a simplified query execution
            // In practice, you'd need to properly handle the dynamic parameters
            let rows = client.query(&query, &[])?;
            
            let mut sessions = Vec::new();
            for row in rows {
                // Parse row into RecordingSession
                // This is simplified - you'd need proper parsing
            }
            
            Ok(sessions)
        }).await?
    }

    pub async fn add_event(&self, session_id: &str, event: RecordingEvent) -> Result<()> {
        task::spawn_blocking({
            let session_id = session_id.to_string();
            let event = event.clone();
            move || -> Result<()> {
                let mut client = crate::sam::memory::Config::client()?;
                
                client.execute(
                    "UPDATE recording_sessions 
                     SET metadata = jsonb_set(metadata, '{events}', 
                         coalesce(metadata->'events', '[]'::jsonb) || $1::jsonb)
                     WHERE session_id = $2",
                    &[&serde_json::to_value(&event)?, &session_id],
                )?;
                
                Ok(())
            }
        }).await??;
        
        Ok(())
    }
}

// Playback API
pub struct PlaybackService {
    metadata_store: MetadataStore,
    base_storage_path: PathBuf,
}

impl PlaybackService {
    pub fn new(base_storage_path: PathBuf) -> Result<Self> {
        Ok(Self {
            metadata_store: MetadataStore::new()?,
            base_storage_path,
        })
    }

    pub async fn get_recording(&self, session_id: &str) -> Result<PathBuf> {
        let sessions = self.metadata_store.get_sessions(None, None, None).await?;
        
        let session = sessions.iter()
            .find(|s| s.session_id == session_id)
            .ok_or_else(|| anyhow::anyhow!("Recording not found"))?;
        
        Ok(session.file_path.clone())
    }

    pub async fn get_stream_url(&self, session_id: &str) -> Result<String> {
        let file_path = self.get_recording(session_id).await?;
        
        // Generate HLS stream for web playback
        let hls_path = file_path.with_extension("m3u8");
        
        if !hls_path.exists() {
            self.generate_hls_stream(&file_path, &hls_path).await?;
        }
        
        Ok(format!("/playback/{}", session_id))
    }

    async fn generate_hls_stream(&self, input: &Path, output: &Path) -> Result<()> {
        let output = Command::new("ffmpeg")
            .args(&[
                "-i", input.to_str().unwrap(),
                "-c:v", "copy",
                "-c:a", "copy",
                "-hls_time", "10",
                "-hls_list_size", "0",
                "-hls_segment_filename", &format!("{}.%03d.ts", output.with_extension("").to_str().unwrap()),
                "-y",
                output.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to generate HLS stream: {:?}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    pub async fn search_recordings(
        &self,
        thing_oid: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        event_type: Option<String>,
    ) -> Result<Vec<RecordingSession>> {
        let mut sessions = self.metadata_store.get_sessions(thing_oid, start_time, end_time).await?;
        
        if let Some(event) = event_type {
            sessions.retain(|s| {
                s.metadata.events.iter().any(|e| e.event_type == event)
            });
        }
        
        Ok(sessions)
    }

    pub async fn export_recording(
        &self,
        session_id: &str,
        format: ExportFormat,
    ) -> Result<PathBuf> {
        let input_path = self.get_recording(session_id).await?;
        let export_path = input_path.with_extension(format.extension());
        
        let ffmpeg_args = match format {
            ExportFormat::MP4 => vec![
                "-i", input_path.to_str().unwrap(),
                "-c", "copy",
                "-y",
                export_path.to_str().unwrap(),
            ],
            ExportFormat::AVI => vec![
                "-i", input_path.to_str().unwrap(),
                "-c:v", "mpeg4",
                "-c:a", "mp3",
                "-y",
                export_path.to_str().unwrap(),
            ],
            ExportFormat::WebM => vec![
                "-i", input_path.to_str().unwrap(),
                "-c:v", "libvpx-vp9",
                "-c:a", "libopus",
                "-y",
                export_path.to_str().unwrap(),
            ],
            ExportFormat::GIF => vec![
                "-i", input_path.to_str().unwrap(),
                "-vf", "fps=10,scale=320:-1:flags=lanczos",
                "-y",
                export_path.to_str().unwrap(),
            ],
        };
        
        let output = Command::new("ffmpeg").args(&ffmpeg_args).output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Export failed: {:?}", String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(export_path)
    }
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    MP4,
    AVI,
    WebM,
    GIF,
}

impl ExportFormat {
    fn extension(&self) -> &str {
        match self {
            ExportFormat::MP4 => "mp4",
            ExportFormat::AVI => "avi",
            ExportFormat::WebM => "webm",
            ExportFormat::GIF => "gif",
        }
    }
}

// SQL table creation
pub fn create_recording_tables() -> Result<()> {
    let mut client = crate::sam::memory::Config::client()?;
    
    client.execute(
        "CREATE TABLE IF NOT EXISTS recording_sessions (
            session_id VARCHAR PRIMARY KEY,
            thing_oid VARCHAR NOT NULL,
            start_time TIMESTAMPTZ NOT NULL,
            end_time TIMESTAMPTZ,
            trigger JSONB NOT NULL,
            file_path TEXT NOT NULL,
            file_size BIGINT DEFAULT 0,
            duration_seconds BIGINT,
            metadata JSONB NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )",
        &[],
    )?;
    
    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_recording_sessions_thing_oid ON recording_sessions(thing_oid)",
        &[],
    )?;
    
    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_recording_sessions_start_time ON recording_sessions(start_time)",
        &[],
    )?;
    
    Ok(())
}