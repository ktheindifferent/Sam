use async_trait::async_trait;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use rand::Rng;
use crate::sam::jobs::{JobHandler, JobResult, JobError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailPayload {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
    pub html: Option<String>,
    pub attachments: Option<Vec<String>>,
    pub reply_to: Option<String>,
    pub from: Option<String>,
}

pub struct EmailJobHandler {
    smtp_host: String,
    smtp_port: u16,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
    from_address: String,
}

impl EmailJobHandler {
    pub fn new(
        smtp_host: String,
        smtp_port: u16,
        from_address: String,
    ) -> Self {
        Self {
            smtp_host,
            smtp_port,
            smtp_username: None,
            smtp_password: None,
            from_address,
        }
    }
    
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.smtp_username = Some(username);
        self.smtp_password = Some(password);
        self
    }
    
    async fn send_email(&self, payload: EmailPayload) -> Result<(), String> {
        // In a real implementation, you would use an email library like lettre
        // For now, we'll simulate email sending
        
        info!("Sending email to {:?} with subject: {}", payload.to, payload.subject);
        
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate occasional failures
        if rand::thread_rng().gen::<f32>() < 0.05 {
            return Err("SMTP connection failed".to_string());
        }
        
        info!("Email sent successfully");
        Ok(())
    }
}

#[async_trait]
impl JobHandler for EmailJobHandler {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError> {
        // Parse the payload
        let email_payload: EmailPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::SerializationError(format!("Invalid email payload: {}", e)))?;
        
        // Validate the payload
        self.validate_payload(&serde_json::to_value(&email_payload).unwrap()).await?;
        
        // Send the email
        match self.send_email(email_payload).await {
            Ok(_) => Ok(JobResult::Success(serde_json::json!({
                "status": "sent",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))),
            Err(e) => {
                if e.contains("SMTP connection") {
                    // Transient error, should retry
                    Ok(JobResult::Retry(e))
                } else {
                    // Permanent error
                    Ok(JobResult::Failure(e))
                }
            }
        }
    }
    
    fn max_retries(&self) -> u32 {
        5 // More retries for email since network issues are common
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff with cap
        Duration::from_secs((2_u64.pow(attempt) * 30).min(300))
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(60)) // 1 minute timeout for email
    }
    
    fn name(&self) -> &str {
        "email"
    }
    
    async fn validate_payload(&self, payload: &Value) -> Result<(), JobError> {
        let email_payload: EmailPayload = serde_json::from_value(payload.clone())
            .map_err(|e| JobError::SerializationError(format!("Invalid payload: {}", e)))?;
        
        // Validate email addresses
        if email_payload.to.is_empty() {
            return Err(JobError::ExecutionFailed("No recipients specified".to_string()));
        }
        
        // Basic email validation (in production, use a proper email validation library)
        for email in &email_payload.to {
            if !email.contains('@') {
                return Err(JobError::ExecutionFailed(format!("Invalid email address: {}", email)));
            }
        }
        
        // Validate subject and body
        if email_payload.subject.is_empty() {
            return Err(JobError::ExecutionFailed("Email subject is required".to_string()));
        }
        
        if email_payload.body.is_empty() && email_payload.html.as_ref().map_or(true, |h| h.is_empty()) {
            return Err(JobError::ExecutionFailed("Email body is required".to_string()));
        }
        
        Ok(())
    }
    
    async fn on_success(&self, payload: &Value, _result: &JobResult) -> Result<(), JobError> {
        if let Ok(email_payload) = serde_json::from_value::<EmailPayload>(payload.clone()) {
            info!("Email successfully sent to {:?}", email_payload.to);
        }
        Ok(())
    }
    
    async fn on_failure(&self, payload: &Value, error: &JobError) -> Result<(), JobError> {
        if let Ok(email_payload) = serde_json::from_value::<EmailPayload>(payload.clone()) {
            error!("Failed to send email to {:?}: {}", email_payload.to, error);
        }
        Ok(())
    }
}