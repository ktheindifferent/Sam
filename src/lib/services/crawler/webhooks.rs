//! Webhook notifications for crawler events
//!
//! This module provides webhook functionality to notify external systems
//! about crawler events like job completion, failures, and milestones.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Webhook event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    JobStarted,
    JobCompleted,
    JobFailed,
    JobCancelled,
    JobPaused,
    JobResumed,
    MilestoneReached,
    RateLimitHit,
    ErrorThresholdExceeded,
}

/// Webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: WebhookEvent,
    pub timestamp: DateTime<Utc>,
    pub job_id: String,
    pub job_url: String,
    pub data: serde_json::Value,
    pub metadata: WebhookMetadata,
}

/// Metadata about the crawl job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMetadata {
    pub pages_crawled: usize,
    pub pages_failed: usize,
    pub bytes_downloaded: u64,
    pub duration_seconds: Option<u64>,
    pub success_rate: f64,
    pub avg_response_time_ms: Option<f64>,
    pub depth_reached: usize,
    pub domains_crawled: Vec<String>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL to send notifications to
    pub url: String,

    /// Events to send notifications for
    pub events: Vec<WebhookEvent>,

    /// Secret for webhook signature (optional)
    pub secret: Option<String>,

    /// Custom headers to include
    pub headers: Vec<(String, String)>,

    /// Retry configuration
    pub retry_count: usize,

    /// Timeout for webhook requests (seconds)
    pub timeout_seconds: u64,

    /// Include full crawl data in payload
    pub include_full_data: bool,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            events: vec![WebhookEvent::JobCompleted, WebhookEvent::JobFailed],
            secret: None,
            headers: Vec::new(),
            retry_count: 3,
            timeout_seconds: 30,
            include_full_data: false,
        }
    }
}

/// Webhook sender
pub struct WebhookSender {
    client: Client,
    config: WebhookConfig,
}

impl WebhookSender {
    /// Create a new webhook sender
    pub fn new(config: WebhookConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    /// Send a webhook notification
    pub async fn send(
        &self,
        event: WebhookEvent,
        job_id: String,
        job_url: String,
        data: serde_json::Value,
        metadata: WebhookMetadata,
    ) -> Result<()> {
        // Check if this event should be sent
        if !self.config.events.contains(&event) {
            debug!(
                "Skipping webhook for event {:?} (not in configured events)",
                event
            );
            return Ok(());
        }

        let payload = WebhookPayload {
            event: event.clone(),
            timestamp: Utc::now(),
            job_id: job_id.clone(),
            job_url,
            data: if self.config.include_full_data {
                data
            } else {
                json!({})
            },
            metadata,
        };

        // Retry logic
        let mut last_error = None;
        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                info!(
                    "Retrying webhook (attempt {}/{})",
                    attempt, self.config.retry_count
                );
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt as u32))).await;
            }

            match self.send_webhook(&payload).await {
                Ok(_) => {
                    info!("Webhook sent successfully for event {:?}", event);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Webhook attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Webhook failed after retries")))
    }

    /// Send the actual webhook request
    async fn send_webhook(&self, payload: &WebhookPayload) -> Result<()> {
        let mut request = self.client.post(&self.config.url).json(payload);

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add signature if secret is configured
        if let Some(secret) = &self.config.secret {
            let signature = self.generate_signature(payload, secret)?;
            request = request.header("X-Webhook-Signature", signature);
        }

        // Add standard headers
        request = request
            .header("X-Webhook-Event", format!("{:?}", payload.event))
            .header("X-Webhook-Job-Id", &payload.job_id)
            .header("User-Agent", "SAM-Crawler-Webhook/1.0");

        // Send with timeout
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            request.send(),
        )
        .await
        .context("Webhook request timed out")?
        .context("Failed to send webhook request")?;

        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "No response body".to_string());
            return Err(anyhow::anyhow!(
                "Webhook failed with status {}: {}",
                status,
                body
            ));
        }

        Ok(())
    }

    /// Generate HMAC signature for webhook payload
    fn generate_signature(&self, payload: &WebhookPayload, secret: &str) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let json = serde_json::to_string(payload)?;

        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).context("Invalid secret key")?;
        mac.update(json.as_bytes());

        let result = mac.finalize();
        Ok(format!("sha256={}", hex::encode(result.into_bytes())))
    }
}

/// Webhook notification service
pub struct WebhookNotificationService {
    senders: Vec<WebhookSender>,
}

impl WebhookNotificationService {
    /// Create a new notification service
    pub fn new(configs: Vec<WebhookConfig>) -> Self {
        let senders = configs.into_iter().map(WebhookSender::new).collect();

        Self { senders }
    }

    /// Send job started notification
    pub async fn notify_job_started(&self, job_id: String, job_url: String) {
        let metadata = WebhookMetadata {
            pages_crawled: 0,
            pages_failed: 0,
            bytes_downloaded: 0,
            duration_seconds: None,
            success_rate: 0.0,
            avg_response_time_ms: None,
            depth_reached: 0,
            domains_crawled: Vec::new(),
        };

        for sender in &self.senders {
            if let Err(e) = sender
                .send(
                    WebhookEvent::JobStarted,
                    job_id.clone(),
                    job_url.clone(),
                    json!({}),
                    metadata.clone(),
                )
                .await
            {
                error!("Failed to send job started webhook: {}", e);
            }
        }
    }

    /// Send job completed notification
    pub async fn notify_job_completed(
        &self,
        job_id: String,
        job_url: String,
        metadata: WebhookMetadata,
    ) {
        let data = json!({
            "status": "completed",
            "message": format!("Crawl job completed successfully. Crawled {} pages.", metadata.pages_crawled)
        });

        for sender in &self.senders {
            if let Err(e) = sender
                .send(
                    WebhookEvent::JobCompleted,
                    job_id.clone(),
                    job_url.clone(),
                    data.clone(),
                    metadata.clone(),
                )
                .await
            {
                error!("Failed to send job completed webhook: {}", e);
            }
        }
    }

    /// Send job failed notification
    pub async fn notify_job_failed(
        &self,
        job_id: String,
        job_url: String,
        error: String,
        metadata: WebhookMetadata,
    ) {
        let data = json!({
            "status": "failed",
            "error": error,
            "message": format!("Crawl job failed after crawling {} pages.", metadata.pages_crawled)
        });

        for sender in &self.senders {
            if let Err(e) = sender
                .send(
                    WebhookEvent::JobFailed,
                    job_id.clone(),
                    job_url.clone(),
                    data.clone(),
                    metadata.clone(),
                )
                .await
            {
                error!("Failed to send job failed webhook: {}", e);
            }
        }
    }

    /// Send milestone reached notification
    pub async fn notify_milestone(
        &self,
        job_id: String,
        job_url: String,
        milestone: String,
        metadata: WebhookMetadata,
    ) {
        let data = json!({
            "milestone": milestone,
            "message": format!("Milestone reached: {}", milestone)
        });

        for sender in &self.senders {
            if let Err(e) = sender
                .send(
                    WebhookEvent::MilestoneReached,
                    job_id.clone(),
                    job_url.clone(),
                    data.clone(),
                    metadata.clone(),
                )
                .await
            {
                error!("Failed to send milestone webhook: {}", e);
            }
        }
    }
}

/// Create webhook service from job configuration
pub fn create_webhook_service(webhook_url: Option<String>) -> Option<WebhookNotificationService> {
    webhook_url.map(|url| {
        let config = WebhookConfig {
            url,
            ..Default::default()
        };
        WebhookNotificationService::new(vec![config])
    })
}

/// Validate webhook URL
pub async fn validate_webhook_url(url: &str) -> Result<()> {
    // Parse URL
    let _ = url::Url::parse(url).context("Invalid webhook URL")?;

    // Send test request
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let test_payload = json!({
        "test": true,
        "message": "Webhook validation test from SAM Crawler"
    });

    let response = client
        .post(url)
        .json(&test_payload)
        .header("User-Agent", "SAM-Crawler-Webhook/1.0")
        .send()
        .await
        .context("Failed to send test webhook")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Webhook returned status: {}",
            response.status()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event: WebhookEvent::JobCompleted,
            timestamp: Utc::now(),
            job_id: "test-job".to_string(),
            job_url: "https://example.com".to_string(),
            data: json!({"test": "data"}),
            metadata: WebhookMetadata {
                pages_crawled: 100,
                pages_failed: 5,
                bytes_downloaded: 1024000,
                duration_seconds: Some(60),
                success_rate: 0.95,
                avg_response_time_ms: Some(250.0),
                depth_reached: 3,
                domains_crawled: vec!["example.com".to_string()],
            },
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("job_completed"));
        assert!(json.contains("test-job"));
    }

    #[test]
    fn test_signature_generation() {
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: Some("test-secret".to_string()),
            ..Default::default()
        };

        let sender = WebhookSender::new(config);
        let payload = WebhookPayload {
            event: WebhookEvent::JobCompleted,
            timestamp: Utc::now(),
            job_id: "test".to_string(),
            job_url: "https://example.com".to_string(),
            data: json!({}),
            metadata: WebhookMetadata {
                pages_crawled: 0,
                pages_failed: 0,
                bytes_downloaded: 0,
                duration_seconds: None,
                success_rate: 0.0,
                avg_response_time_ms: None,
                depth_reached: 0,
                domains_crawled: Vec::new(),
            },
        };

        let signature = sender.generate_signature(&payload, "test-secret").unwrap();
        assert!(signature.starts_with("sha256="));
    }
}
