//! Alert rule engine for the notification system.
//!
//! Evaluates incoming `ServiceEvent`s against user-defined rules and produces
//! `Notification` values when a rule fires.

use super::{Notification, Severity};
use crate::services::events::ServiceEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// A single user-defined alert rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub channels: Option<Vec<String>>,
    pub condition: AlertCondition,
}

fn default_enabled() -> bool {
    true
}
fn default_severity() -> Severity {
    Severity::Warning
}

/// Conditions that can trigger an alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    /// CPU usage exceeded for N seconds.
    CpuThreshold {
        percent: f64,
        duration_seconds: Option<u64>,
    },
    /// Memory usage exceeded for N seconds.
    MemoryThreshold {
        percent: f64,
        duration_seconds: Option<u64>,
    },
    /// Disk usage exceeded.
    DiskThreshold { percent: f64 },
    /// A specific service has been down for N seconds.
    ServiceDown {
        service: String,
        duration_seconds: u64,
    },
    /// A specific service changed to a given state string.
    ServiceStatus { service: String, state: String },
    /// Error rate exceeded for a service within a window.
    ErrorRate {
        service: String,
        count: u32,
        window_seconds: u64,
    },
}

/// Internal state for cooldown tracking.
struct CooldownState {
    last_fired: HashMap<String, Instant>,
}

/// The rule engine evaluates events against rules.
pub struct RuleEngine {
    rules: Vec<AlertRule>,
    cooldown_secs: u64,
    cooldown: Mutex<CooldownState>,
    /// Tracks error counts per service for ErrorRate rules.
    error_counts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RuleEngine {
    pub fn new(rules: Vec<AlertRule>, cooldown_secs: u64) -> Self {
        Self {
            rules,
            cooldown_secs,
            cooldown: Mutex::new(CooldownState {
                last_fired: HashMap::new(),
            }),
            error_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Evaluate a service event against all enabled rules.
    /// Returns the first notification that fires (respecting cooldown).
    pub fn evaluate_event(&self, event: &ServiceEvent) -> Option<Notification> {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if let Some(notification) = self.check_rule(rule, event) {
                // Cooldown check
                let mut state = self.cooldown.lock().unwrap();
                if let Some(last) = state.last_fired.get(&rule.id) {
                    if last.elapsed().as_secs() < self.cooldown_secs {
                        continue;
                    }
                }
                state.last_fired.insert(rule.id.clone(), Instant::now());
                return Some(notification);
            }
        }
        None
    }

    fn check_rule(&self, rule: &AlertRule, event: &ServiceEvent) -> Option<Notification> {
        match (&rule.condition, event) {
            // CPU threshold from metrics update
            (
                AlertCondition::CpuThreshold { percent, .. },
                ServiceEvent::MetricsUpdate {
                    metric, value, ..
                },
            ) if metric == "cpu_usage" && *value > *percent => Some(self.build_notification(
                rule,
                format!("CPU usage at {:.1}% (threshold: {:.1}%)", value, percent),
                None,
            )),

            // Memory threshold from metrics update
            (
                AlertCondition::MemoryThreshold { percent, .. },
                ServiceEvent::MetricsUpdate {
                    metric, value, ..
                },
            ) if metric == "memory_usage" && *value > *percent => Some(self.build_notification(
                rule,
                format!(
                    "Memory usage at {:.1}% (threshold: {:.1}%)",
                    value, percent
                ),
                None,
            )),

            // Disk threshold
            (
                AlertCondition::DiskThreshold { percent },
                ServiceEvent::MetricsUpdate {
                    metric, value, ..
                },
            ) if metric == "disk_usage" && *value > *percent => Some(self.build_notification(
                rule,
                format!("Disk usage at {:.1}% (threshold: {:.1}%)", value, percent),
                None,
            )),

            // Service status change to a specific state
            (
                AlertCondition::ServiceStatus {
                    service: target_svc,
                    state: target_state,
                },
                ServiceEvent::StatusChanged {
                    service,
                    new_status,
                    ..
                },
            ) if service == target_svc && new_status == target_state => {
                Some(self.build_notification(
                    rule,
                    format!("Service '{}' changed to '{}'", service, new_status),
                    Some(service.clone()),
                ))
            }

            // Service down (triggered by error event)
            (
                AlertCondition::ServiceDown {
                    service: target_svc,
                    ..
                },
                ServiceEvent::Error {
                    service, message, ..
                },
            ) if service == target_svc => Some(self.build_notification(
                rule,
                format!("Service '{}' error: {}", service, message),
                Some(service.clone()),
            )),

            // Error rate tracking
            (
                AlertCondition::ErrorRate {
                    service: target_svc,
                    count: threshold,
                    window_seconds,
                },
                ServiceEvent::Error { service, .. },
            ) if service == target_svc => {
                let mut counts = self.error_counts.lock().unwrap();
                let entries = counts.entry(service.clone()).or_default();
                entries.push(Instant::now());

                // Prune old entries outside the window
                let cutoff = Instant::now()
                    - std::time::Duration::from_secs(*window_seconds);
                entries.retain(|t| *t > cutoff);

                if entries.len() >= *threshold as usize {
                    entries.clear(); // Reset after firing
                    Some(self.build_notification(
                        rule,
                        format!(
                            "Service '{}' hit {} errors in {}s",
                            service, threshold, window_seconds
                        ),
                        Some(service.clone()),
                    ))
                } else {
                    None
                }
            }

            // Health check failure
            (
                AlertCondition::ServiceDown {
                    service: target_svc,
                    ..
                },
                ServiceEvent::HealthCheck {
                    service,
                    healthy,
                    message,
                },
            ) if service == target_svc && !healthy => Some(self.build_notification(
                rule,
                format!("Service '{}' health check failed: {}", service, message),
                Some(service.clone()),
            )),

            _ => None,
        }
    }

    fn build_notification(
        &self,
        rule: &AlertRule,
        message: String,
        service: Option<String>,
    ) -> Notification {
        Notification {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity.clone(),
            message,
            service,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine(rules: Vec<AlertRule>) -> RuleEngine {
        RuleEngine::new(rules, 0) // no cooldown for tests
    }

    #[test]
    fn test_cpu_threshold_fires() {
        let engine = make_engine(vec![AlertRule {
            id: "cpu_high".into(),
            name: "CPU High".into(),
            enabled: true,
            severity: Severity::Warning,
            channels: None,
            condition: AlertCondition::CpuThreshold {
                percent: 80.0,
                duration_seconds: None,
            },
        }]);

        let event = ServiceEvent::MetricsUpdate {
            service: "system".into(),
            metric: "cpu_usage".into(),
            value: 95.0,
        };

        let result = engine.evaluate_event(&event);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("95.0%"));
    }

    #[test]
    fn test_cpu_threshold_below_no_fire() {
        let engine = make_engine(vec![AlertRule {
            id: "cpu_high".into(),
            name: "CPU High".into(),
            enabled: true,
            severity: Severity::Warning,
            channels: None,
            condition: AlertCondition::CpuThreshold {
                percent: 80.0,
                duration_seconds: None,
            },
        }]);

        let event = ServiceEvent::MetricsUpdate {
            service: "system".into(),
            metric: "cpu_usage".into(),
            value: 50.0,
        };

        assert!(engine.evaluate_event(&event).is_none());
    }

    #[test]
    fn test_service_down_fires_on_error() {
        let engine = make_engine(vec![AlertRule {
            id: "redis_down".into(),
            name: "Redis Down".into(),
            enabled: true,
            severity: Severity::Error,
            channels: None,
            condition: AlertCondition::ServiceDown {
                service: "redis".into(),
                duration_seconds: 0,
            },
        }]);

        let event = ServiceEvent::Error {
            service: "redis".into(),
            message: "Connection refused".into(),
        };

        let result = engine.evaluate_event(&event);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("redis"));
    }

    #[test]
    fn test_disabled_rule_does_not_fire() {
        let engine = make_engine(vec![AlertRule {
            id: "disabled".into(),
            name: "Disabled".into(),
            enabled: false,
            severity: Severity::Warning,
            channels: None,
            condition: AlertCondition::CpuThreshold {
                percent: 1.0,
                duration_seconds: None,
            },
        }]);

        let event = ServiceEvent::MetricsUpdate {
            service: "system".into(),
            metric: "cpu_usage".into(),
            value: 99.0,
        };

        assert!(engine.evaluate_event(&event).is_none());
    }

    #[test]
    fn test_cooldown_prevents_rapid_fire() {
        let engine = RuleEngine::new(
            vec![AlertRule {
                id: "cpu".into(),
                name: "CPU".into(),
                enabled: true,
                severity: Severity::Warning,
                channels: None,
                condition: AlertCondition::CpuThreshold {
                    percent: 80.0,
                    duration_seconds: None,
                },
            }],
            300, // 5 minute cooldown
        );

        let event = ServiceEvent::MetricsUpdate {
            service: "system".into(),
            metric: "cpu_usage".into(),
            value: 95.0,
        };

        // First should fire
        assert!(engine.evaluate_event(&event).is_some());
        // Second should be suppressed by cooldown
        assert!(engine.evaluate_event(&event).is_none());
    }

    #[test]
    fn test_error_rate_accumulation() {
        let engine = make_engine(vec![AlertRule {
            id: "redis_errors".into(),
            name: "Redis Errors".into(),
            enabled: true,
            severity: Severity::Error,
            channels: None,
            condition: AlertCondition::ErrorRate {
                service: "redis".into(),
                count: 3,
                window_seconds: 60,
            },
        }]);

        let event = ServiceEvent::Error {
            service: "redis".into(),
            message: "timeout".into(),
        };

        // First two shouldn't fire
        assert!(engine.evaluate_event(&event).is_none());
        assert!(engine.evaluate_event(&event).is_none());
        // Third should fire
        assert!(engine.evaluate_event(&event).is_some());
    }
}
