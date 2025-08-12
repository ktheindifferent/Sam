use async_trait::async_trait;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use crate::sam::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub channel: NotificationChannel,
    pub title: String,
    pub message: String,
    pub priority: NotificationPriority,
    pub metadata: Option<Value>,
    pub retry_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email { to: Vec<String> },
    Sms { to: Vec<String> },
    Slack { channel: String, webhook_url: Option<String> },
    Discord { channel_id: String, webhook_url: Option<String> },
    PushNotification { device_tokens: Vec<String> },
    Webhook { url: String, method: String, headers: Option<Value> },
    InApp { user_ids: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

pub struct NotificationJobHandler {
    webhook_timeout: Duration,
}

impl NotificationJobHandler {
    pub fn new() -> Self {
        Self {
            webhook_timeout: Duration::from_secs(30),
        }
    }
    
    async fn send_notification(&self, payload: NotificationPayload) -> Result<NotificationResult, String> {
        info!("Sending {} notification: {}", 
              format!("{:?}", payload.priority).to_lowercase(), 
              payload.title);
        
        let start_time = std::time::Instant::now();
        let mut delivered_to = Vec::new();
        let mut failed_recipients = Vec::new();
        
        match &payload.channel {
            NotificationChannel::Email { to } => {
                info!("Sending email notification to {} recipients", to.len());
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                for recipient in to {
                    if rand::random::<f32>() > 0.1 {
                        delivered_to.push(recipient.clone());
                    } else {
                        failed_recipients.push(recipient.clone());
                    }
                }
            }
            
            NotificationChannel::Sms { to } => {
                info!("Sending SMS notification to {} recipients", to.len());
                tokio::time::sleep(Duration::from_millis(800)).await;
                
                for recipient in to {
                    if rand::random::<f32>() > 0.05 {
                        delivered_to.push(recipient.clone());
                    } else {
                        failed_recipients.push(recipient.clone());
                    }
                }
            }
            
            NotificationChannel::Slack { channel, webhook_url } => {
                info!("Sending Slack notification to channel: {}", channel);
                tokio::time::sleep(Duration::from_millis(300)).await;
                
                if rand::random::<f32>() > 0.02 {
                    delivered_to.push(channel.clone());
                } else {
                    return Err("Failed to send Slack notification".to_string());
                }
            }
            
            NotificationChannel::Discord { channel_id, webhook_url } => {
                info!("Sending Discord notification to channel: {}", channel_id);
                tokio::time::sleep(Duration::from_millis(300)).await;
                
                if rand::random::<f32>() > 0.02 {
                    delivered_to.push(channel_id.clone());
                } else {
                    return Err("Failed to send Discord notification".to_string());
                }
            }
            
            NotificationChannel::PushNotification { device_tokens } => {
                info!("Sending push notifications to {} devices", device_tokens.len());
                tokio::time::sleep(Duration::from_millis(1000)).await;
                
                for token in device_tokens {
                    if rand::random::<f32>() > 0.15 {
                        delivered_to.push(token.clone());
                    } else {
                        failed_recipients.push(token.clone());
                    }
                }
            }
            
            NotificationChannel::Webhook { url, method, headers } => {
                info!("Sending webhook notification to: {}", url);
                tokio::time::sleep(Duration::from_millis(200)).await;
                
                if rand::random::<f32>() > 0.1 {
                    delivered_to.push(url.clone());
                } else {
                    return Err(format!("Webhook failed: {} {}", method, url));
                }
            }
            
            NotificationChannel::InApp { user_ids } => {
                info!("Sending in-app notification to {} users", user_ids.len());
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                for user_id in user_ids {
                    delivered_to.push(user_id.clone());
                }
            }
        }
        
        Ok(NotificationResult {
            delivered_to,
            failed_recipients,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NotificationResult {
    delivered_to: Vec<String>,
    failed_recipients: Vec<String>,
    duration_ms: u64,
}

#[async_trait]
impl JobHandler for NotificationJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        let notification_payload: NotificationPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid notification payload: {}", e)))?;
        
        match self.send_notification(notification_payload.clone()).await {
            Ok(result) => {
                info!("Notification delivered to {} recipients", result.delivered_to.len());
                
                if !result.failed_recipients.is_empty() {
                    let failure_rate = result.failed_recipients.len() as f32 / 
                                      (result.delivered_to.len() + result.failed_recipients.len()) as f32;
                    
                    if failure_rate > 0.5 && notification_payload.retry_on_failure {
                        // More than 50% failed, retry
                        return Ok(JobResult::Retry(format!(
                            "High failure rate: {} of {} recipients failed",
                            result.failed_recipients.len(),
                            result.delivered_to.len() + result.failed_recipients.len()
                        )));
                    }
                }
                
                Ok(JobResult::Success(serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"status": "completed"}))))
            }
            Err(e) => {
                if notification_payload.retry_on_failure && 
                   (e.contains("timeout") || e.contains("connection") || e.contains("rate")) {
                    Ok(JobResult::Retry(e))
                } else {
                    error!("Notification failed: {}", e);
                    Ok(JobResult::Failure(e))
                }
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        3
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        Duration::from_secs(30 * attempt as u64)
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(120)) // 2 minute timeout
    }
    
    fn name(&self) -> &str {
        "notification"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let notification_payload: NotificationPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        if notification_payload.title.is_empty() {
            return Err(JobError::ExecutionFailed("Notification title is required".to_string()));
        }
        
        if notification_payload.message.is_empty() {
            return Err(JobError::ExecutionFailed("Notification message is required".to_string()));
        }
        
        // Validate channel has recipients
        match &notification_payload.channel {
            NotificationChannel::Email { to } |
            NotificationChannel::Sms { to } => {
                if to.is_empty() {
                    return Err(JobError::ExecutionFailed("No recipients specified".to_string()));
                }
            }
            NotificationChannel::PushNotification { device_tokens } => {
                if device_tokens.is_empty() {
                    return Err(JobError::ExecutionFailed("No device tokens specified".to_string()));
                }
            }
            NotificationChannel::InApp { user_ids } => {
                if user_ids.is_empty() {
                    return Err(JobError::ExecutionFailed("No user IDs specified".to_string()));
                }
            }
            NotificationChannel::Webhook { url, .. } => {
                if url.is_empty() {
                    return Err(JobError::ExecutionFailed("Webhook URL is required".to_string()));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}