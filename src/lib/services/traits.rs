use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub enabled: bool,
    pub retry_attempts: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug)]
pub enum ServiceError {
    Initialization(String),
    Connection(String),
    Timeout(String),
    InvalidConfiguration(String),
    Runtime(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceError::Initialization(msg) => write!(f, "Initialization error: {}", msg),
            ServiceError::Connection(msg) => write!(f, "Connection error: {}", msg),
            ServiceError::Timeout(msg) => write!(f, "Timeout error: {}", msg),
            ServiceError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            ServiceError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl Error for ServiceError {}

#[async_trait]
pub trait Service: Send + Sync {
    async fn start(&mut self) -> Result<(), ServiceError>;
    async fn stop(&mut self) -> Result<(), ServiceError>;
    async fn health_check(&self) -> Result<ServiceHealth, ServiceError>;
    fn get_config(&self) -> &ServiceConfig;
    fn get_name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[async_trait]
pub trait ServiceFactory: Send + Sync {
    type Service: Service;
    type Config;

    async fn create(config: Self::Config) -> Result<Self::Service, ServiceError>;
    fn validate_config(config: &Self::Config) -> Result<(), ServiceError>;
}

pub trait RetryPolicy: Send + Sync {
    fn should_retry(&self, attempt: u32, error: &dyn Error) -> bool;
    fn get_delay(&self, attempt: u32) -> std::time::Duration;
}

#[derive(Clone)]
pub struct ExponentialBackoff {
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl ExponentialBackoff {
    pub fn new(max_attempts: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
        }
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32, _error: &dyn Error) -> bool {
        attempt < self.max_attempts
    }

    fn get_delay(&self, attempt: u32) -> std::time::Duration {
        let delay = self.base_delay_ms * 2_u64.pow(attempt);
        let delay = delay.min(self.max_delay_ms);
        std::time::Duration::from_millis(delay)
    }
}

pub struct ServiceRegistry {
    services: Vec<Arc<dyn Service>>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    pub fn register(&mut self, service: Arc<dyn Service>) {
        self.services.push(service);
    }

    pub async fn start_all(&mut self) -> Result<(), ServiceError> {
        for service in &self.services {
            log::info!("Starting service: {}", service.get_name());
            if let Err(e) = Arc::get_mut(&mut service.clone())
                .ok_or_else(|| ServiceError::Runtime("Cannot get mutable reference".to_string()))?
                .start()
                .await
            {
                log::error!("Failed to start service {}: {}", service.get_name(), e);
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn stop_all(&mut self) -> Result<(), ServiceError> {
        for service in &self.services {
            log::info!("Stopping service: {}", service.get_name());
            if let Err(e) = Arc::get_mut(&mut service.clone())
                .ok_or_else(|| ServiceError::Runtime("Cannot get mutable reference".to_string()))?
                .stop()
                .await
            {
                log::error!("Failed to stop service {}: {}", service.get_name(), e);
            }
        }
        Ok(())
    }

    pub async fn health_check_all(&self) -> Vec<(String, ServiceHealth)> {
        let mut results = Vec::new();
        for service in &self.services {
            match service.health_check().await {
                Ok(health) => results.push((service.get_name().to_string(), health)),
                Err(e) => {
                    log::error!("Health check failed for {}: {}", service.get_name(), e);
                    results.push((
                        service.get_name().to_string(),
                        ServiceHealth {
                            status: HealthStatus::Unhealthy,
                            message: Some(e.to_string()),
                            last_check: std::time::SystemTime::now(),
                        },
                    ));
                }
            }
        }
        results
    }
}
