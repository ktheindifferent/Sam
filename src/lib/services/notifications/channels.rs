//! Notification delivery channels.
//!
//! Each channel implements `NotificationChannel` and delivers alerts through a
//! specific medium (WebSocket push, SMS, webhook POST, email stub).

use super::Notification;
use async_trait::async_trait;

/// Trait for notification delivery channels.
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, notification: &Notification) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// WebSocket channel – emits a WsMessage::Alert on the broadcast bus
// ---------------------------------------------------------------------------
pub struct WebSocketChannel;

#[async_trait]
impl NotificationChannel for WebSocketChannel {
    fn name(&self) -> &str {
        "websocket"
    }

    async fn send(&self, notification: &Notification) -> Result<(), String> {
        // Emit as a service event so any connected WebSocket clients pick it up
        // through the existing alert pipeline in websocket/mod.rs.
        crate::services::events::emit(crate::services::events::ServiceEvent::Error {
            service: notification
                .service
                .clone()
                .unwrap_or_else(|| "notification".to_string()),
            message: format!(
                "[{}] {}: {}",
                notification.severity, notification.rule_name, notification.message
            ),
        });
        log::debug!(
            "WebSocket notification sent for rule '{}'",
            notification.rule_id
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SMS channel – wraps the existing sms::send_sms() helper
// ---------------------------------------------------------------------------
pub struct SmsChannel {
    pub recipients: Vec<String>,
}

#[async_trait]
impl NotificationChannel for SmsChannel {
    fn name(&self) -> &str {
        "sms"
    }

    async fn send(&self, notification: &Notification) -> Result<(), String> {
        let body = format!(
            "SAM Alert [{}]: {}",
            notification.severity, notification.message
        );
        for recipient in &self.recipients {
            crate::services::sms::send_sms(recipient, &body).await?;
        }
        log::info!(
            "SMS notification sent to {} recipients for rule '{}'",
            self.recipients.len(),
            notification.rule_id
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Webhook channel – HTTP POST with JSON body
// ---------------------------------------------------------------------------
pub struct WebhookChannel {
    pub url: String,
}

#[async_trait]
impl NotificationChannel for WebhookChannel {
    fn name(&self) -> &str {
        "webhook"
    }

    async fn send(&self, notification: &Notification) -> Result<(), String> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "rule_id": notification.rule_id,
            "rule_name": notification.rule_name,
            "severity": notification.severity,
            "message": notification.message,
            "service": notification.service,
            "timestamp": notification.timestamp.to_rfc3339(),
        });

        client
            .post(&self.url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Webhook POST failed: {}", e))?;

        log::info!(
            "Webhook notification sent to {} for rule '{}'",
            self.url,
            notification.rule_id
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Email channel – stub that logs the notification
// ---------------------------------------------------------------------------
pub struct EmailChannel {
    pub recipient: String,
}

#[async_trait]
impl NotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, notification: &Notification) -> Result<(), String> {
        // Stub: real implementation would use lettre or similar
        log::info!(
            "Email notification (stub) to {}: [{}] {} - {}",
            self.recipient,
            notification.severity,
            notification.rule_name,
            notification.message
        );
        Ok(())
    }
}
