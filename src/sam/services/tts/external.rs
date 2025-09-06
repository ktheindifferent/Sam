use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use log::{info, warn};
use reqwest;
use std::time::Duration;
use super::{TtsRequest, TtsResult, AudioFormat};
use crate::sam::services::environment::get_env_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTTSConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub default_voice: String,
    pub default_language: String,
}

impl Default for ExternalTTSConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("TTS_URL")
                .unwrap_or_else(|_| "http://localhost:8002/tts".to_string()),
            api_key: std::env::var("TTS_API_KEY").ok(),
            timeout: Duration::from_secs(30),
            default_voice: std::env::var("TTS_DEFAULT_VOICE")
                .unwrap_or_else(|_| "default".to_string()),
            default_language: std::env::var("TTS_DEFAULT_LANGUAGE")
                .unwrap_or_else(|_| "en-US".to_string()),
        }
    }
}

pub struct ExternalTTSService {
    config: ExternalTTSConfig,
    client: reqwest::Client,
}

impl ExternalTTSService {
    pub fn new() -> Result<Self> {
        let config = ExternalTTSConfig::default();
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .context("Failed to build HTTP client for TTS")?;
        
        Ok(Self { config, client })
    }
    
    pub fn with_config(config: ExternalTTSConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .context("Failed to build HTTP client for TTS")?;
        
        Ok(Self { config, client })
    }
    
    pub async fn synthesize(&self, request: TtsRequest) -> Result<TtsResult> {
        let env_config = get_env_config();
        
        if env_config.is_caprover && env_config.tts_url.is_none() {
            warn!("TTS_URL not configured in CapRover mode - TTS service unavailable");
            return Ok(TtsResult {
                audio_data: Vec::new(),
                format: request.format,
                duration_ms: 0,
                cached: false,
            });
        }
        
        let endpoint = env_config.tts_url
            .as_ref()
            .unwrap_or(&self.config.endpoint);
        
        info!("Sending text to external TTS service: {}", endpoint);
        
        let tts_request = ExternalTTSRequest {
            text: request.text,
            voice: request.voice.unwrap_or(self.config.default_voice.clone()),
            language: request.language.unwrap_or(self.config.default_language.clone()),
            speed: request.speed.unwrap_or(1.0),
            pitch: request.pitch.unwrap_or(1.0),
            volume: request.volume.unwrap_or(1.0),
            format: format_to_string(&request.format),
        };
        
        let mut http_request = self.client
            .post(endpoint)
            .json(&tts_request);
        
        if let Some(api_key) = &self.config.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let start_time = std::time::Instant::now();
        let response = http_request
            .send()
            .await
            .context("Failed to send request to TTS service")?;
        
        let duration_ms = start_time.elapsed().as_millis();
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "TTS service returned error {}: {}", 
                status, error_text
            ));
        }
        
        let audio_data = response.bytes().await
            .context("Failed to read TTS response")?
            .to_vec();
        
        Ok(TtsResult {
            audio_data,
            format: request.format,
            duration_ms,
            cached: false,
        })
    }
    
    pub async fn health_check(&self) -> Result<()> {
        let env_config = get_env_config();
        let endpoint = env_config.tts_url
            .as_ref()
            .unwrap_or(&self.config.endpoint);
        
        let health_url = format!("{}/health", endpoint.trim_end_matches("/tts"));
        
        let response = self.client
            .get(&health_url)
            .send()
            .await
            .context("Failed to check TTS service health")?;
        
        if response.status().is_success() {
            info!("External TTS service is healthy");
            Ok(())
        } else {
            Err(anyhow::anyhow!("TTS service health check failed: {}", response.status()))
        }
    }
    
    pub async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        let env_config = get_env_config();
        let endpoint = env_config.tts_url
            .as_ref()
            .unwrap_or(&self.config.endpoint);
        
        let voices_url = format!("{}/voices", endpoint.trim_end_matches("/tts"));
        
        let response = self.client
            .get(&voices_url)
            .send()
            .await
            .context("Failed to fetch voice list")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch voices: {}", response.status()));
        }
        
        let voices: Vec<VoiceInfo> = response.json().await
            .context("Failed to parse voices response")?;
        
        Ok(voices)
    }
}

#[derive(Debug, Serialize)]
struct ExternalTTSRequest {
    text: String,
    voice: String,
    language: String,
    speed: f32,
    pitch: f32,
    volume: f32,
    format: String,
}

#[derive(Debug, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: Option<String>,
}

fn format_to_string(format: &AudioFormat) -> String {
    match format {
        AudioFormat::Wav => "wav",
        AudioFormat::Mp3 => "mp3",
        AudioFormat::Ogg => "ogg",
        AudioFormat::Flac => "flac",
    }.to_string()
}

/// Get the appropriate TTS service based on environment configuration
pub async fn get_tts_service() -> Result<Box<dyn TTSServiceTrait>> {
    let env_config = get_env_config();
    
    if env_config.is_caprover || env_config.tts_url.is_some() {
        info!("Using external TTS service");
        Ok(Box::new(ExternalTTSService::new()?))
    } else {
        // Fall back to local TTS service
        info!("Using local TTS service");
        Ok(Box::new(LocalTTSWrapper::new()?))
    }
}

/// Trait for TTS services
pub trait TTSServiceTrait: Send + Sync {
    fn synthesize_sync(&self, request: TtsRequest) -> Result<TtsResult>;
}

impl TTSServiceTrait for ExternalTTSService {
    fn synthesize_sync(&self, request: TtsRequest) -> Result<TtsResult> {
        tokio::runtime::Handle::current()
            .block_on(self.synthesize(request))
    }
}

/// Wrapper for local TTS service to implement the trait
struct LocalTTSWrapper;

impl LocalTTSWrapper {
    fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl TTSServiceTrait for LocalTTSWrapper {
    fn synthesize_sync(&self, request: TtsRequest) -> Result<TtsResult> {
        // This would call the actual local TTS implementation
        // For now, return a placeholder
        warn!("Local TTS service not fully implemented");
        Ok(TtsResult {
            audio_data: Vec::new(),
            format: request.format,
            duration_ms: 0,
            cached: false,
        })
    }
}