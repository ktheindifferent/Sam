//! Notification and alerting system for SAM.
//!
//! Subscribes to the `ServiceEvent` broadcast channel, evaluates configurable
//! alert rules, and delivers notifications via pluggable channels (SMS,
//! WebSocket, webhook, email stub).

pub mod channels;
pub mod http_handler;
pub mod rules;

// Re-export the HTTP handler so existing `notifications::handle` paths still work.
pub use http_handler::handle;

use crate::services::events::{self, ServiceEvent};
use channels::{NotificationChannel, SmsChannel, WebSocketChannel, WebhookChannel};
use rules::{AlertRule, RuleEngine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Severity levels for notifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// A notification ready for delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: Severity,
    pub message: String,
    pub service: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// User-facing notification configuration (loaded from `~/.sam/config.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    pub enabled: Option<bool>,
    pub default_channels: Option<Vec<String>>,
    pub sms_recipients: Option<Vec<String>>,
    pub webhook_urls: Option<Vec<String>>,
    pub cooldown_seconds: Option<u64>,
    pub rules: Option<Vec<AlertRule>>,
}

/// Shared state for the notification service.
pub struct NotificationService {
    engine: RuleEngine,
    channels: Vec<Arc<dyn NotificationChannel>>,
}

impl NotificationService {
    /// Build from user config.
    pub fn from_config(config: &NotificationConfig) -> Self {
        let rules = config.rules.clone().unwrap_or_default();
        let cooldown = config.cooldown_seconds.unwrap_or(300);
        let engine = RuleEngine::new(rules, cooldown);

        let default_channels = config
            .default_channels
            .clone()
            .unwrap_or_else(|| vec!["websocket".to_string()]);

        let mut channels: Vec<Arc<dyn NotificationChannel>> = Vec::new();

        if default_channels.contains(&"websocket".to_string()) {
            channels.push(Arc::new(WebSocketChannel));
        }
        if default_channels.contains(&"sms".to_string()) {
            if let Some(recipients) = &config.sms_recipients {
                channels.push(Arc::new(SmsChannel {
                    recipients: recipients.clone(),
                }));
            }
        }
        if default_channels.contains(&"webhook".to_string()) {
            if let Some(urls) = &config.webhook_urls {
                for url in urls {
                    channels.push(Arc::new(WebhookChannel {
                        url: url.clone(),
                    }));
                }
            }
        }

        Self { engine, channels }
    }

    /// Spawn the background notification monitoring task.
    pub fn spawn(config: NotificationConfig) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let service = Arc::new(RwLock::new(Self::from_config(&config)));
            let mut rx = events::subscribe();

            log::info!("NotificationService started with {} rules",
                config.rules.as_ref().map(|r| r.len()).unwrap_or(0));

            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let svc = service.read().await;
                        if let Some(notification) = svc.engine.evaluate_event(&event) {
                            for channel in &svc.channels {
                                if let Err(e) = channel.send(&notification).await {
                                    log::warn!(
                                        "Failed to send notification via {}: {}",
                                        channel.name(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("NotificationService lagged {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        log::info!("Event bus closed, stopping NotificationService");
                        break;
                    }
                }
            }
        })
    }
}
