//! Service event system for event-driven status updates
//!
//! Services can emit events via a broadcast channel, and the TUI status updater
//! subscribes to receive real-time status changes instead of polling.

use tokio::sync::broadcast;

/// Events emitted by services
#[derive(Debug, Clone)]
pub enum ServiceEvent {
    /// A service changed status (name, old_status, new_status)
    StatusChanged {
        service: String,
        old_status: String,
        new_status: String,
    },
    /// Health check completed
    HealthCheck {
        service: String,
        healthy: bool,
        message: String,
    },
    /// Metrics update from a service
    MetricsUpdate {
        service: String,
        metric: String,
        value: f64,
    },
    /// Service error
    Error {
        service: String,
        message: String,
    },
}

/// Service group categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ServiceGroup {
    Core,
    AI,
    Home,
    Media,
    Security,
    Network,
    Storage,
}

impl ServiceGroup {
    pub fn label(&self) -> &'static str {
        match self {
            ServiceGroup::Core => "Core Infrastructure",
            ServiceGroup::AI => "AI & Automation",
            ServiceGroup::Home => "Home Automation",
            ServiceGroup::Media => "Media Services",
            ServiceGroup::Security => "Security",
            ServiceGroup::Network => "Network",
            ServiceGroup::Storage => "Storage",
        }
    }
}

/// Mapping of service names to their groups
pub fn service_group(name: &str) -> ServiceGroup {
    match name.to_lowercase().as_str() {
        "redis" | "postgres" | "postgresql" | "docker" | "http_server" | "ssh_server" => ServiceGroup::Core,
        "ollama" | "openai" | "llama" | "rivescript" | "coding_agent" | "copilot" => ServiceGroup::AI,
        "lifx" | "matter" | "mdns" => ServiceGroup::Home,
        "spotify" | "youtube" | "tts" | "stt" | "snapcast" | "rtsp" => ServiceGroup::Media,
        "clamav" | "vulnerability_scanner" => ServiceGroup::Security,
        "crawler" | "sms" | "p2p" | "darknet" => ServiceGroup::Network,
        "backup" | "dropbox" | "nextcloud" | "seaweedfs" => ServiceGroup::Storage,
        _ => ServiceGroup::Core,
    }
}

/// Global event bus for service events
static EVENT_BUS: std::sync::OnceLock<broadcast::Sender<ServiceEvent>> = std::sync::OnceLock::new();

/// Get or initialize the global event bus
pub fn event_bus() -> broadcast::Sender<ServiceEvent> {
    EVENT_BUS.get_or_init(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    }).clone()
}

/// Subscribe to service events
pub fn subscribe() -> broadcast::Receiver<ServiceEvent> {
    event_bus().subscribe()
}

/// Emit a service event
pub fn emit(event: ServiceEvent) {
    let _ = event_bus().send(event);
}

/// Configuration for auto-recovery of a single service
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub max_restarts: u32,
    pub cooldown: std::time::Duration,
    pub backoff_base: std::time::Duration,
    pub backoff_max: std::time::Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            cooldown: std::time::Duration::from_secs(300),
            backoff_base: std::time::Duration::from_secs(2),
            backoff_max: std::time::Duration::from_secs(60),
        }
    }
}

/// Tracks restart attempts for a single service
#[derive(Debug, Clone)]
struct RecoveryState {
    attempt_count: u32,
    last_failure: Option<std::time::Instant>,
}

/// Monitors the event bus for service failures and schedules restarts
/// with exponential backoff.
pub struct ServiceAutoRecovery {
    configs: std::collections::HashMap<String, RecoveryConfig>,
    states: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, RecoveryState>>>,
}

impl ServiceAutoRecovery {
    pub fn new() -> Self {
        Self {
            configs: std::collections::HashMap::new(),
            states: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Register a service for auto-recovery monitoring.
    pub fn register(&mut self, service_name: &str, config: RecoveryConfig) {
        self.configs.insert(service_name.to_string(), config);
    }

    /// Calculate the backoff delay for the current attempt.
    fn backoff_delay(config: &RecoveryConfig, attempt: u32) -> std::time::Duration {
        let base_ms = config.backoff_base.as_millis() as u64;
        let delay_ms = base_ms.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
        let max_ms = config.backoff_max.as_millis() as u64;
        std::time::Duration::from_millis(delay_ms.min(max_ms))
    }

    /// Start the recovery monitor as a background task.
    /// Returns a JoinHandle that can be used to cancel monitoring.
    pub fn spawn_monitor(self) -> tokio::task::JoinHandle<()> {
        let configs = self.configs;
        let states = self.states;

        tokio::spawn(async move {
            let mut rx = subscribe();

            loop {
                match rx.recv().await {
                    Ok(ServiceEvent::Error { service, message }) => {
                        let config = match configs.get(&service) {
                            Some(c) => c.clone(),
                            None => continue,
                        };

                        let should_restart = {
                            let mut guard = states.lock().unwrap();
                            let state = guard.entry(service.clone()).or_insert(RecoveryState {
                                attempt_count: 0,
                                last_failure: None,
                            });

                            // Reset counter if cooldown has elapsed
                            if let Some(last) = state.last_failure {
                                if last.elapsed() > config.cooldown {
                                    state.attempt_count = 0;
                                }
                            }

                            if state.attempt_count < config.max_restarts {
                                state.attempt_count += 1;
                                state.last_failure = Some(std::time::Instant::now());
                                true
                            } else {
                                log::warn!(
                                    "Service '{}' exceeded max restarts ({}), not recovering: {}",
                                    service, config.max_restarts, message
                                );
                                false
                            }
                        };

                        if should_restart {
                            let attempt = states.lock().unwrap()
                                .get(&service)
                                .map(|s| s.attempt_count)
                                .unwrap_or(1);
                            let delay = Self::backoff_delay(&config, attempt);

                            log::info!(
                                "Scheduling restart for '{}' in {:?} (attempt {}/{}): {}",
                                service, delay, attempt, config.max_restarts, message
                            );

                            let svc = service.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(delay).await;
                                log::info!("Auto-recovery: restarting service '{}'", svc);
                                emit(ServiceEvent::StatusChanged {
                                    service: svc.clone(),
                                    old_status: "error".to_string(),
                                    new_status: "restarting".to_string(),
                                });
                            });
                        }
                    }
                    Ok(_) => {} // Ignore non-error events
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("Auto-recovery monitor lagged {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!("Event bus closed, stopping auto-recovery monitor");
                        break;
                    }
                }
            }
        })
    }
}
