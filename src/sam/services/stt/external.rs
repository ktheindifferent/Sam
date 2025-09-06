use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use log::{info, warn, error};
use reqwest;
use std::time::Duration;
use super::STTPrediction;
use crate::sam::services::environment::get_env_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSTTConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub model: Option<String>,
}

impl Default for ExternalSTTConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("STT_URL")
                .unwrap_or_else(|_| "http://localhost:8001/stt".to_string()),
            api_key: std::env::var("STT_API_KEY").ok(),
            timeout: Duration::from_secs(30),
            model: std::env::var("STT_MODEL").ok(),
        }
    }
}

pub struct ExternalSTTService {
    config: ExternalSTTConfig,
    client: reqwest::Client,
}

impl ExternalSTTService {
    pub fn new() -> Result<Self> {
        let config = ExternalSTTConfig::default();
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .context("Failed to build HTTP client for STT")?;
        
        Ok(Self { config, client })
    }
    
    pub fn with_config(config: ExternalSTTConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .context("Failed to build HTTP client for STT")?;
        
        Ok(Self { config, client })
    }
    
    pub async fn transcribe(&self, audio_data: Vec<u8>, format: &str) -> Result<STTPrediction> {
        let env_config = get_env_config();
        
        if env_config.is_caprover && env_config.stt_url.is_none() {
            warn!("STT_URL not configured in CapRover mode - STT service unavailable");
            return Ok(STTPrediction {
                text: String::new(),
                confidence: 0.0,
                language: None,
                duration_ms: 0,
            });
        }
        
        let endpoint = env_config.stt_url
            .as_ref()
            .unwrap_or(&self.config.endpoint);
        
        info!("Sending audio to external STT service: {}", endpoint);
        
        let mut request = self.client
            .post(endpoint)
            .header("Content-Type", format!("audio/{}", format));
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        if let Some(model) = &self.config.model {
            request = request.header("X-STT-Model", model);
        }
        
        let response = request
            .body(audio_data)
            .send()
            .await
            .context("Failed to send request to STT service")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "STT service returned error {}: {}", 
                status, error_text
            ));
        }
        
        let result: STTResponse = response.json().await
            .context("Failed to parse STT response")?;
        
        Ok(STTPrediction {
            text: result.text,
            confidence: result.confidence.unwrap_or(1.0),
            language: result.language,
            duration_ms: result.processing_time_ms.unwrap_or(0),
        })
    }
    
    pub async fn health_check(&self) -> Result<()> {
        let env_config = get_env_config();
        let endpoint = env_config.stt_url
            .as_ref()
            .unwrap_or(&self.config.endpoint);
        
        let health_url = format!("{}/health", endpoint.trim_end_matches("/stt"));
        
        let response = self.client
            .get(&health_url)
            .send()
            .await
            .context("Failed to check STT service health")?;
        
        if response.status().is_success() {
            info!("External STT service is healthy");
            Ok(())
        } else {
            Err(anyhow::anyhow!("STT service health check failed: {}", response.status()))
        }
    }
}

#[derive(Debug, Deserialize)]
struct STTResponse {
    text: String,
    confidence: Option<f32>,
    language: Option<String>,
    processing_time_ms: Option<u64>,
}

/// Get the appropriate STT service based on environment configuration
pub async fn get_stt_service() -> Result<Box<dyn STTServiceTrait>> {
    let env_config = get_env_config();
    
    if env_config.is_caprover || env_config.stt_url.is_some() {
        info!("Using external STT service");
        Ok(Box::new(ExternalSTTService::new()?))
    } else {
        // Fall back to local Whisper service
        info!("Using local Whisper STT service");
        Ok(Box::new(LocalWhisperWrapper::new()?))
    }
}

/// Trait for STT services
pub trait STTServiceTrait: Send + Sync {
    fn transcribe_sync(&self, audio_data: Vec<u8>, format: &str) -> Result<STTPrediction>;
}

impl STTServiceTrait for ExternalSTTService {
    fn transcribe_sync(&self, audio_data: Vec<u8>, format: &str) -> Result<STTPrediction> {
        tokio::runtime::Handle::current()
            .block_on(self.transcribe(audio_data, format))
    }
}

/// Wrapper for local Whisper service to implement the trait
struct LocalWhisperWrapper;

impl LocalWhisperWrapper {
    fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl STTServiceTrait for LocalWhisperWrapper {
    fn transcribe_sync(&self, _audio_data: Vec<u8>, _format: &str) -> Result<STTPrediction> {
        // This would call the actual Whisper implementation
        // For now, return a placeholder
        warn!("Local Whisper service not fully implemented");
        Ok(STTPrediction {
            text: String::new(),
            confidence: 0.0,
            language: None,
            duration_ms: 0,
        })
    }
}