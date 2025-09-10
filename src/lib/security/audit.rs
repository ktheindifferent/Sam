use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub user_id: Option<String>,
    pub ip_address: String,
    pub user_agent: String,
    pub request_id: String,
    pub resource: String,
    pub action: String,
    pub result: EventResult,
    pub metadata: HashMap<String, Value>,
    pub severity: Severity,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    AuthenticationAttempt,
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationViolation,
    RateLimitExceeded,
    SuspiciousPattern,
    ConfigurationChange,
    DataAccess,
    FileAccessViolation,
    SqlInjectionAttempt,
    XssAttempt,
    PathTraversalAttempt,
    SessionHijackingAttempt,
    BruteForceDetected,
    PrivilegeEscalation,
    ApiAbuseDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventResult {
    Success,
    Failure,
    Blocked,
    Suspicious,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone)]
pub struct AuditLogger {
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    retention_days: u32,
    max_events: usize,
    alert_threshold: HashMap<SecurityEventType, (u32, std::time::Duration)>,
}

impl AuditLogger {
    pub fn new(retention_days: u32, max_events: usize) -> Self {
        let mut alert_threshold = HashMap::new();
        
        alert_threshold.insert(
            SecurityEventType::AuthenticationFailure,
            (5, std::time::Duration::from_secs(300))
        );
        alert_threshold.insert(
            SecurityEventType::RateLimitExceeded,
            (10, std::time::Duration::from_secs(60))
        );
        alert_threshold.insert(
            SecurityEventType::SqlInjectionAttempt,
            (1, std::time::Duration::from_secs(3600))
        );
        alert_threshold.insert(
            SecurityEventType::BruteForceDetected,
            (1, std::time::Duration::from_secs(3600))
        );
        
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            retention_days,
            max_events,
            alert_threshold,
        }
    }

    pub async fn log_event(&self, event: SecurityEvent) {
        let severity = &event.severity;
        let event_type = &event.event_type;
        
        match severity {
            Severity::Critical => {
                error!(
                    "CRITICAL SECURITY EVENT: {:?} - User: {:?}, IP: {}, Resource: {}",
                    event_type, event.user_id, event.ip_address, event.resource
                );
                self.trigger_alert(&event).await;
            }
            Severity::High => {
                warn!(
                    "HIGH SEVERITY EVENT: {:?} - User: {:?}, IP: {}, Resource: {}",
                    event_type, event.user_id, event.ip_address, event.resource
                );
            }
            Severity::Medium => {
                info!(
                    "Security event: {:?} - User: {:?}, IP: {}",
                    event_type, event.user_id, event.ip_address
                );
            }
            Severity::Low => {
                info!("Security audit: {:?}", event_type);
            }
        }

        let mut events = self.events.write().await;
        events.push(event.clone());
        
        if events.len() > self.max_events {
            let excess = events.len() - self.max_events;
            events.drain(0..excess);
        }
        
        self.check_patterns(&event).await;
    }

    pub async fn log_authentication_attempt(
        &self,
        user_id: Option<String>,
        ip_address: String,
        user_agent: String,
        request_id: String,
        success: bool,
    ) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: if success {
                SecurityEventType::AuthenticationSuccess
            } else {
                SecurityEventType::AuthenticationFailure
            },
            user_id,
            ip_address: ip_address.clone(),
            user_agent,
            request_id,
            resource: "/auth/login".to_string(),
            action: "login".to_string(),
            result: if success {
                EventResult::Success
            } else {
                EventResult::Failure
            },
            metadata: HashMap::new(),
            severity: if success {
                Severity::Low
            } else {
                Severity::Medium
            },
            correlation_id: None,
        };
        
        self.log_event(event).await;
        
        if !success {
            self.check_brute_force(&ip_address).await;
        }
    }

    pub async fn log_authorization_violation(
        &self,
        user_id: Option<String>,
        ip_address: String,
        user_agent: String,
        request_id: String,
        resource: String,
        action: String,
        reason: String,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("reason".to_string(), Value::String(reason));
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::AuthorizationViolation,
            user_id,
            ip_address,
            user_agent,
            request_id,
            resource,
            action,
            result: EventResult::Blocked,
            metadata,
            severity: Severity::High,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    pub async fn log_rate_limit_violation(
        &self,
        ip_address: String,
        user_agent: String,
        request_id: String,
        endpoint: String,
        limit: u32,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("limit".to_string(), Value::Number(limit.into()));
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::RateLimitExceeded,
            user_id: None,
            ip_address,
            user_agent,
            request_id,
            resource: endpoint,
            action: "request".to_string(),
            result: EventResult::Blocked,
            metadata,
            severity: Severity::Medium,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    pub async fn log_suspicious_activity(
        &self,
        user_id: Option<String>,
        ip_address: String,
        user_agent: String,
        request_id: String,
        activity_type: String,
        details: HashMap<String, Value>,
    ) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousPattern,
            user_id,
            ip_address,
            user_agent,
            request_id,
            resource: activity_type.clone(),
            action: "detect".to_string(),
            result: EventResult::Suspicious,
            metadata: details,
            severity: Severity::High,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    pub async fn log_injection_attempt(
        &self,
        ip_address: String,
        user_agent: String,
        request_id: String,
        injection_type: &str,
        payload: String,
        target: String,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("payload".to_string(), Value::String(payload));
        metadata.insert("injection_type".to_string(), Value::String(injection_type.to_string()));
        
        let event_type = match injection_type {
            "sql" => SecurityEventType::SqlInjectionAttempt,
            "xss" => SecurityEventType::XssAttempt,
            "path" => SecurityEventType::PathTraversalAttempt,
            _ => SecurityEventType::SuspiciousPattern,
        };
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type,
            user_id: None,
            ip_address,
            user_agent,
            request_id,
            resource: target,
            action: "injection_attempt".to_string(),
            result: EventResult::Blocked,
            metadata,
            severity: Severity::Critical,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    pub async fn log_configuration_change(
        &self,
        user_id: String,
        ip_address: String,
        config_key: String,
        old_value: Option<String>,
        new_value: String,
    ) {
        let mut metadata = HashMap::new();
        if let Some(old) = old_value {
            metadata.insert("old_value".to_string(), Value::String(old));
        }
        metadata.insert("new_value".to_string(), Value::String(new_value));
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::ConfigurationChange,
            user_id: Some(user_id),
            ip_address,
            user_agent: String::new(),
            request_id: uuid::Uuid::new_v4().to_string(),
            resource: config_key,
            action: "update".to_string(),
            result: EventResult::Success,
            metadata,
            severity: Severity::Medium,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    pub async fn log_data_access(
        &self,
        user_id: Option<String>,
        ip_address: String,
        user_agent: String,
        request_id: String,
        data_type: String,
        operation: String,
        sensitivity: String,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("data_type".to_string(), Value::String(data_type.clone()));
        metadata.insert("sensitivity".to_string(), Value::String(sensitivity.clone()));
        
        let severity = match sensitivity.as_str() {
            "high" | "critical" => Severity::High,
            "medium" => Severity::Medium,
            _ => Severity::Low,
        };
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::DataAccess,
            user_id,
            ip_address,
            user_agent,
            request_id,
            resource: data_type,
            action: operation,
            result: EventResult::Success,
            metadata,
            severity,
            correlation_id: None,
        };
        
        self.log_event(event).await;
    }

    async fn check_brute_force(&self, ip_address: &str) {
        let events = self.events.read().await;
        let now = Utc::now();
        let window = chrono::Duration::minutes(5);
        
        let failed_attempts = events
            .iter()
            .filter(|e| {
                e.ip_address == ip_address
                    && e.event_type == SecurityEventType::AuthenticationFailure
                    && e.timestamp > now - window
            })
            .count();
        
        if failed_attempts >= 5 {
            drop(events);
            
            let event = SecurityEvent {
                timestamp: now,
                event_type: SecurityEventType::BruteForceDetected,
                user_id: None,
                ip_address: ip_address.to_string(),
                user_agent: String::new(),
                request_id: uuid::Uuid::new_v4().to_string(),
                resource: "/auth".to_string(),
                action: "brute_force".to_string(),
                result: EventResult::Blocked,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("attempts".to_string(), Value::Number(failed_attempts.into()));
                    m
                },
                severity: Severity::Critical,
                correlation_id: None,
            };
            
            self.log_event(event).await;
        }
    }

    async fn check_patterns(&self, event: &SecurityEvent) {
        if let Some(&(threshold, window)) = self.alert_threshold.get(&event.event_type) {
            let events = self.events.read().await;
            let now = Utc::now();
            let window_start = now - chrono::Duration::from_std(window).unwrap();
            
            let count = events
                .iter()
                .filter(|e| {
                    e.event_type == event.event_type
                        && e.ip_address == event.ip_address
                        && e.timestamp > window_start
                })
                .count();
            
            if count >= threshold as usize {
                warn!(
                    "ALERT: Threshold exceeded for {:?} from IP {} - {} events in {:?}",
                    event.event_type, event.ip_address, count, window
                );
            }
        }
    }

    async fn trigger_alert(&self, event: &SecurityEvent) {
        error!(
            "SECURITY ALERT TRIGGERED: {:?} at {} from IP {}",
            event.event_type, event.timestamp, event.ip_address
        );
        
        // Here you would integrate with external alerting systems:
        // - Send to SIEM
        // - Email notifications
        // - PagerDuty/OpsGenie
        // - Slack/Discord webhooks
    }

    pub async fn get_events_by_type(
        &self,
        event_type: SecurityEventType,
        limit: usize,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.event_type == event_type)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_events_by_ip(
        &self,
        ip_address: &str,
        limit: usize,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.ip_address == ip_address)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_events_by_user(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.user_id.as_ref().map_or(false, |id| id == user_id))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn cleanup_old_events(&self) {
        let mut events = self.events.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(self.retention_days as i64);
        events.retain(|e| e.timestamp > cutoff);
    }

    pub async fn export_events(&self) -> Result<String, serde_json::Error> {
        let events = self.events.read().await;
        serde_json::to_string_pretty(&*events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new(30, 1000);
        
        logger.log_authentication_attempt(
            Some("user123".to_string()),
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            "req-123".to_string(),
            true,
        ).await;
        
        let events = logger.get_events_by_type(SecurityEventType::AuthenticationSuccess, 10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some("user123".to_string()));
    }

    #[tokio::test]
    async fn test_brute_force_detection() {
        let logger = AuditLogger::new(30, 1000);
        let ip = "192.168.1.100".to_string();
        
        for _ in 0..6 {
            logger.log_authentication_attempt(
                None,
                ip.clone(),
                "Mozilla/5.0".to_string(),
                uuid::Uuid::new_v4().to_string(),
                false,
            ).await;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let events = logger.get_events_by_type(SecurityEventType::BruteForceDetected, 10).await;
        assert!(!events.is_empty());
    }
}