// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::{Request, Response};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use anyhow::{Result, Context};
use log::{debug, warn, error};
use std::sync::Arc;
use lazy_static::lazy_static;

// Import our resource management modules
use crate::sam::resource_management::{TempFile, CleanupGuard, ResourceCleanup};

lazy_static! {
    // Global runtime for async operations
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create runtime");
}

/// Handle observations API endpoints with proper resource management
pub fn handle(
    _current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/observations" {
        return handle_list_observations(request);
    }

    if request.url().contains("/api/observations/file/") {
        return handle_get_observation_file(request);
    }

    if request.url().contains("/api/observations/vwav/") {
        return handle_visual_wav(request);
    }

    Ok(Response::empty_404())
}

/// Handle listing observations
fn handle_list_observations(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let skip = request.get_param("skip");
    let skip_number: usize = skip
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let objects = crate::sam::memory::Observation::select_lite(
        Some(1),
        Some(skip_number),
        Some("timestamp DESC".to_string()),
        None,
    )?;
    
    Ok(Response::json(&objects))
}

/// Handle getting observation file
fn handle_get_observation_file(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let oid = extract_oid_from_url(request.url(), 4)?;
    
    // Get observation from database
    let observation = get_observation_by_oid(&oid)?;
    
    let wav_data = observation.observation_file
        .ok_or_else(|| crate::sam::http::Error::from(
            anyhow::anyhow!("Observation file not found")
        ))?;
    
    Ok(Response::from_data("audio/wav", wav_data))
}

/// Handle visual WAV generation with proper resource cleanup
fn handle_visual_wav(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let oid = extract_oid_from_url(request.url(), 4)?;
    
    // Get observation from database
    let observation = get_observation_by_oid(&oid)?;
    
    let wav_data = observation.observation_file
        .ok_or_else(|| crate::sam::http::Error::from(
            anyhow::anyhow!("Observation file not found")
        ))?;
    
    // Check for cached result first
    let cache_path = get_cache_path(&observation.oid);
    if cache_path.exists() {
        debug!("Using cached visual WAV for observation {}", observation.oid);
        let data = std::fs::read(&cache_path)
            .map_err(|e| crate::sam::http::Error::from(e))?;
        return Ok(Response::from_data("video/mp4", data));
    }
    
    // Process with proper resource cleanup
    let result = RUNTIME.block_on(async {
        process_visual_wav_async(observation.oid, wav_data).await
    });
    
    match result {
        Ok(data) => Ok(Response::from_data("video/mp4", data)),
        Err(e) => {
            error!("Failed to process visual WAV: {}", e);
            Err(crate::sam::http::Error::from(e))
        }
    }
}

/// Process visual WAV with automatic cleanup
async fn process_visual_wav_async(oid: String, wav_data: Vec<u8>) -> Result<Vec<u8>> {
    // Create temp directory if it doesn't exist
    let temp_dir = PathBuf::from("/opt/sam/tmp/observations/vwav");
    tokio::fs::create_dir_all(&temp_dir).await
        .context("Failed to create temp directory")?;
    
    // Create resource cleanup manager
    let mut cleanup = ResourceCleanup::new();
    
    // Create temporary files with automatic cleanup
    let base_path = temp_dir.join(&oid);
    let wav_path = format!("{}.wav", base_path.display());
    let wav_16_path = format!("{}.16.wav", base_path.display());
    let wts_path = format!("{}.16.wav.wts", base_path.display());
    let mp4_path = format!("{}.16.wav.mp4", base_path.display());
    
    // Register all files for cleanup
    cleanup.add_file(PathBuf::from(&wav_path));
    cleanup.add_file(PathBuf::from(&wav_16_path));
    cleanup.add_file(PathBuf::from(&wts_path));
    cleanup.add_file(PathBuf::from(&mp4_path));
    
    // Create cleanup guard to ensure cleanup happens even on error
    let _cleanup_guard = CleanupGuard::new(cleanup, |c| {
        RUNTIME.spawn(async move {
            if let Err(e) = c.cleanup().await {
                warn!("Cleanup failed: {}", e);
            }
        });
    });
    
    // Write WAV data to temp file
    tokio::fs::write(&wav_path, &wav_data).await
        .context("Failed to write WAV file")?;
    
    // Process with ffmpeg
    let ffmpeg_cmd = format!(
        "ffmpeg -y -i {} -ar 16000 -ac 1 -c:a pcm_s16le {}",
        wav_path, wav_16_path
    );
    
    run_command_with_timeout(&ffmpeg_cmd, 30)
        .context("FFmpeg processing failed")?;
    
    // Process with whisper
    let whisper_cmd = format!(
        "/opt/sam/bin/whisper -m /opt/sam/models/ggml-large.bin -f {} -owts",
        wav_16_path
    );
    
    run_command_with_timeout(&whisper_cmd, 120)
        .context("Whisper processing failed")?;
    
    // Patch whisper output
    crate::sam::services::stt::patch_whisper_wts(wts_path.clone())
        .context("Failed to patch whisper WTS")?;
    
    // Make WTS executable and run it
    let chmod_cmd = format!("chmod +x {}", wts_path);
    run_command_with_timeout(&chmod_cmd, 5)
        .context("Failed to make WTS executable")?;
    
    run_command_with_timeout(&wts_path, 60)
        .context("Failed to run WTS")?;
    
    // Read the generated MP4 file
    let mp4_data = tokio::fs::read(&mp4_path).await
        .context("Failed to read generated MP4")?;
    
    // Cache the result for future use
    if let Err(e) = cache_result(&oid, &mp4_data).await {
        warn!("Failed to cache result for {}: {}", oid, e);
    }
    
    Ok(mp4_data)
}

/// Run a command with timeout
fn run_command_with_timeout(cmd: &str, timeout_secs: u64) -> Result<()> {
    use std::process::{Command, Stdio};
    use std::time::Duration;
    
    debug!("Running command: {}", cmd);
    
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;
    
    // Wait with timeout
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                } else {
                    return Err(anyhow::anyhow!("Command failed with status: {}", status));
                }
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    // Kill the process
                    let _ = child.kill();
                    return Err(anyhow::anyhow!("Command timeout after {} seconds", timeout_secs));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to wait for command: {}", e));
            }
        }
    }
}

/// Extract OID from URL
fn extract_oid_from_url(url: &str, index: usize) -> Result<String, crate::sam::http::Error> {
    let parts: Vec<&str> = url.split('/').collect();
    
    parts.get(index)
        .map(|s| s.to_string())
        .ok_or_else(|| crate::sam::http::Error::from(
            anyhow::anyhow!("Invalid URL format")
        ))
}

/// Get observation by OID
fn get_observation_by_oid(oid: &str) -> Result<crate::sam::memory::Observation, crate::sam::http::Error> {
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String(oid.to_string()));
    pg_query.query_columns.push("oid =".to_string());
    
    let observations = crate::sam::memory::Observation::select(None, None, None, Some(pg_query))
        .map_err(|e| crate::sam::http::Error::from(e))?;
    
    observations.into_iter()
        .next()
        .ok_or_else(|| crate::sam::http::Error::from(
            anyhow::anyhow!("Observation not found")
        ))
}

/// Get cache path for observation
fn get_cache_path(oid: &str) -> PathBuf {
    PathBuf::from(format!("/opt/sam/tmp/observations/vwav/{}.wav.16.wav.mp4", oid))
}

/// Cache the result for future use
async fn cache_result(oid: &str, data: &[u8]) -> Result<()> {
    let cache_path = get_cache_path(oid);
    
    // Ensure cache directory exists
    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    // Write cache file
    tokio::fs::write(&cache_path, data).await?;
    
    debug!("Cached visual WAV result for observation {}", oid);
    Ok(())
}

/// Clean up old cache files (can be called periodically)
pub async fn cleanup_old_cache(max_age_hours: u64) -> Result<()> {
    let cache_dir = PathBuf::from("/opt/sam/tmp/observations/vwav");
    
    if !cache_dir.exists() {
        return Ok(());
    }
    
    let max_age = std::time::Duration::from_secs(max_age_hours * 3600);
    let now = std::time::SystemTime::now();
    
    let mut entries = tokio::fs::read_dir(&cache_dir).await?;
    let mut deleted_count = 0;
    let mut freed_bytes = 0u64;
    
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        
        if metadata.is_file() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age > max_age {
                        freed_bytes += metadata.len();
                        if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                            warn!("Failed to delete cache file {:?}: {}", entry.path(), e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    if deleted_count > 0 {
        info!("Cleaned up {} cache files, freed {} bytes", deleted_count, freed_bytes);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_oid_from_url() {
        let url = "/api/observations/vwav/test123/extra";
        let oid = extract_oid_from_url(url, 4).unwrap();
        assert_eq!(oid, "test123");
    }

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path("test123");
        assert!(path.to_string_lossy().contains("test123"));
        assert!(path.to_string_lossy().ends_with(".mp4"));
    }
}