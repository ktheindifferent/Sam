//! Telemetry service for sharing crawl data with OpenSAM Foundation
//! 
//! This service handles the secure transmission of crawled data to the OSF
//! telemetry server to contribute to the community knowledge base.

use anyhow::Result;
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use reqwest::Client;
use crate::services::crawler::CrawledContent;

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub osf_endpoint: String,
    pub batch_size: usize,
    pub timeout_seconds: u64,
    pub is_osf_server: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: env::var("TELEMETRY_ENABLED").unwrap_or_else(|_| "true".to_string()).to_lowercase() == "true",
            osf_endpoint: env::var("OSF_TELEMETRY_ENDPOINT").unwrap_or_else(|_| "https://sam.alpha.opensam.foundation/api/telemetry".to_string()),
            batch_size: env::var("TELEMETRY_BATCH_SIZE").unwrap_or_else(|_| "100".to_string()).parse().unwrap_or(100),
            timeout_seconds: env::var("TELEMETRY_TIMEOUT").unwrap_or_else(|_| "30".to_string()).parse().unwrap_or(30),
            is_osf_server: env::var("IS_OSF").unwrap_or_else(|_| "false".to_string()).to_lowercase() == "true",
        }
    }
}

/// Telemetry payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub version: String,
    pub instance_id: String,
    pub timestamp: i64,
    pub content: Vec<TelemetryContent>,
}

/// Simplified content structure for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryContent {
    pub url: String,
    pub content_hash: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_text: String, // Note: May be truncated for privacy/bandwidth
    pub status_code: i16,
    pub content_type: Option<String>,
    pub content_length: i64,
    pub language: Option<String>,
    pub crawled_at: i64,
}

impl From<&CrawledContent> for TelemetryContent {
    fn from(content: &CrawledContent) -> Self {
        // Truncate content text for bandwidth/privacy (keep first 2000 chars)
        // Use char_indices to find safe truncation point that respects UTF-8 boundaries
        let truncated_text = if content.content_text.chars().count() > 2000 {
            let truncate_at = content.content_text
                .char_indices()
                .nth(2000)
                .map(|(i, _)| i)
                .unwrap_or(content.content_text.len());
            format!("{}...[truncated]", &content.content_text[..truncate_at])
        } else {
            content.content_text.clone()
        };

        Self {
            url: content.url.clone(),
            content_hash: content.content_hash.clone(),
            title: content.title.clone(),
            description: content.description.clone(),
            content_text: truncated_text,
            status_code: content.status_code,
            content_type: content.content_type.clone(),
            content_length: content.content_length,
            language: content.language.clone(),
            crawled_at: content.crawled_at,
        }
    }
}

/// Telemetry service
pub struct TelemetryService {
    config: TelemetryConfig,
    client: Client,
    instance_id: String,
}

impl TelemetryService {
    /// Create a new telemetry service
    pub fn new() -> Self {
        let config = TelemetryConfig::default();
        
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("SAM-Telemetry/0.0.2")
            .build()
            .unwrap_or_else(|_| Client::new());
        
        // Generate a unique instance ID based on hostname and random component
        let instance_id = format!(
            "{}-{}", 
            gethostname::gethostname().to_string_lossy(),
            uuid::Uuid::new_v4().to_string()[..8].to_string()
        );
        
        Self {
            config,
            client,
            instance_id,
        }
    }

    /// Check if telemetry should be sent
    pub fn should_send_telemetry(&self) -> bool {
        self.config.enabled && !self.config.is_osf_server
    }
    
    /// Send crawled content to OSF telemetry endpoint
    pub async fn send_content_batch(&self, content: Vec<CrawledContent>) -> Result<Vec<i64>> {
        if !self.should_send_telemetry() {
            debug!("Telemetry disabled or running on OSF server, skipping send");
            return Ok(Vec::new());
        }
        
        if content.is_empty() {
            return Ok(Vec::new());
        }
        
        let telemetry_content: Vec<TelemetryContent> = content.iter()
            .map(TelemetryContent::from)
            .collect();
        
        let payload = TelemetryPayload {
            version: "0.0.2".to_string(),
            instance_id: self.instance_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            content: telemetry_content,
        };
        
        info!("Sending {} content items to OSF telemetry endpoint", content.len());
        
        match self.client
            .post(&format!("{}/submit", self.config.osf_endpoint))
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let content_ids: Vec<i64> = content.iter().map(|c| c.id).collect();
                    info!("Successfully sent {} items to OSF telemetry", content.len());
                    Ok(content_ids)
                } else {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    error!("OSF telemetry endpoint returned error {}: {}", status, error_text);
                    Err(anyhow::anyhow!("Telemetry send failed: {} - {}", status, error_text))
                }
            }
            Err(e) => {
                error!("Failed to send telemetry data: {}", e);
                Err(anyhow::anyhow!("Failed to send telemetry: {}", e))
            }
        }
    }
    
    /// Process and send unshared content
    pub async fn process_unshared_content(&self) -> Result<usize> {
        if !self.should_send_telemetry() {
            debug!("Telemetry disabled or running on OSF server, skipping processing");
            return Ok(0);
        }
        
        info!("Processing unshared content for telemetry");
        
        let unshared_content = CrawledContent::get_unshared_content(self.config.batch_size).await?;
        
        if unshared_content.is_empty() {
            debug!("No unshared content found");
            return Ok(0);
        }
        
        info!("Found {} unshared content items", unshared_content.len());
        
        match self.send_content_batch(unshared_content).await {
            Ok(successful_ids) => {
                if !successful_ids.is_empty() {
                    CrawledContent::mark_batch_telemetry_shared(successful_ids.clone()).await?;
                    info!("Marked {} items as telemetry shared", successful_ids.len());
                    Box::pin(self.process_unshared_content()).await.map(|next_count| successful_ids.len() + next_count)
                } else {
                    Ok(0)
                }
            }
            Err(e) => {
                warn!("Failed to send telemetry batch: {}", e);
                Err(e)
            }
        }
    }
}

/// Global telemetry service instance
static TELEMETRY_SERVICE: once_cell::sync::Lazy<TelemetryService> = 
    once_cell::sync::Lazy::new(|| TelemetryService::new());

/// Get the global telemetry service
pub fn get_telemetry_service() -> &'static TelemetryService {
    &TELEMETRY_SERVICE
}

/// Service management functions
pub async fn start_service() -> Result<()> {
    let service = get_telemetry_service();
    if service.should_send_telemetry() {
        info!("Telemetry service started - will send data to OSF");
    } else {
        info!("Telemetry service started - sending disabled (IS_OSF={} or TELEMETRY_ENABLED=false)", 
              service.config.is_osf_server);
    }
    Ok(())
}

pub async fn stop_service() -> Result<()> {
    info!("Telemetry service stopped");
    Ok(())
}

pub async fn is_service_running() -> bool {
    true // Telemetry service is always "running" but may not send data
}

pub async fn service_status() -> crate::websocket::ServiceStatus {
    use chrono::Utc;
    let service = get_telemetry_service();
    
    if service.should_send_telemetry() {
        crate::websocket::ServiceStatus {
            state: "healthy".to_string(),
            message: Some(format!("Active - sending to {}", service.config.osf_endpoint)),
            progress: None,
            last_check: Utc::now(),
        }
    } else if service.config.is_osf_server {
        crate::websocket::ServiceStatus {
            state: "healthy".to_string(),
            message: Some("Disabled - running on OSF server".to_string()),
            progress: None,
            last_check: Utc::now(),
        }
    } else {
        crate::websocket::ServiceStatus {
            state: "healthy".to_string(),
            message: Some("Disabled - telemetry turned off".to_string()),
            progress: None,
            last_check: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_telemetry_config_defaults() {
        // Temporarily set environment for test
        env::set_var("TELEMETRY_ENABLED", "true");
        env::set_var("IS_OSF", "false");
        
        let config = TelemetryConfig::default();
        assert!(config.enabled);
        assert!(!config.is_osf_server);
        assert_eq!(config.batch_size, 100);
        
        // Clean up
        env::remove_var("TELEMETRY_ENABLED");
        env::remove_var("IS_OSF");
    }
    
    #[test]
    fn test_should_send_telemetry() {
        // Should send when enabled and not OSF server
        let mut config = TelemetryConfig::default();
        config.enabled = true;
        config.is_osf_server = false;
        let service = TelemetryService {
            config,
            client: Client::new(),
            instance_id: "test-instance".to_string(),
        };
        assert!(service.should_send_telemetry());
        
        // Should not send when disabled
        let mut config = TelemetryConfig::default();
        config.enabled = false;
        config.is_osf_server = false;
        let service = TelemetryService {
            config,
            client: Client::new(),
            instance_id: "test-instance".to_string(),
        };
        assert!(!service.should_send_telemetry());
        
        // Should not send when running on OSF server
        let mut config = TelemetryConfig::default();
        config.enabled = true;
        config.is_osf_server = true;
        let service = TelemetryService {
            config,
            client: Client::new(),
            instance_id: "test-instance".to_string(),
        };
        assert!(!service.should_send_telemetry());
    }
    
    #[test]
    fn test_telemetry_content_conversion() {
        let mut content = CrawledContent::new(
            "https://example.com".to_string(),
            "This is test content",
            None,
            200
        );
        content.id = 1;
        content.title = Some("Test Title".to_string());
        content.language = Some("en".to_string());
        
        let telemetry_content = TelemetryContent::from(&content);
        
        assert_eq!(telemetry_content.url, "https://example.com");
        assert_eq!(telemetry_content.title, Some("Test Title".to_string()));
        assert_eq!(telemetry_content.language, Some("en".to_string()));
        assert_eq!(telemetry_content.status_code, 200);
    }
    
    #[test]
    fn test_content_truncation() {
        let long_content = "A".repeat(3000);
        let mut content = CrawledContent::new(
            "https://example.com".to_string(),
            &long_content,
            None,
            200
        );
        content.id = 1;
        
        let telemetry_content = TelemetryContent::from(&content);
        
        assert!(telemetry_content.content_text.len() < long_content.len());
        assert!(telemetry_content.content_text.contains("[truncated]"));
        assert!(telemetry_content.content_text.starts_with("A"));
        // Verify it's truncated at character boundary (should be exactly 2000 chars + truncation suffix)
        let content_without_suffix = telemetry_content.content_text.replace("...[truncated]", "");
        assert_eq!(content_without_suffix.chars().count(), 2000);
    }

    #[test]
    fn test_utf8_content_truncation() {
        // Create content with multi-byte UTF-8 characters like the one causing the panic
        let utf8_content = "Créer un cluster Kubernetes local avec Kind ê".repeat(100); // Contains multi-byte chars
        let mut content = CrawledContent::new(
            "https://example.com".to_string(),
            &utf8_content,
            None,
            200
        );
        content.id = 1;
        
        let telemetry_content = TelemetryContent::from(&content);
        
        // Should not panic and should handle UTF-8 correctly
        if utf8_content.chars().count() > 2000 {
            assert!(telemetry_content.content_text.contains("[truncated]"));
            let content_without_suffix = telemetry_content.content_text.replace("...[truncated]", "");
            assert_eq!(content_without_suffix.chars().count(), 2000);
        }
        
        // Verify the string is still valid UTF-8
        assert!(telemetry_content.content_text.is_ascii() || !telemetry_content.content_text.is_empty());
    }
}