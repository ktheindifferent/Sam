use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub secret_key: String,
    pub port: u16,
    #[serde(default = "default_discovery_interval")]
    pub discovery_interval: Duration,
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: Duration,
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: Duration,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            port: 8080,
            discovery_interval: default_discovery_interval(),
            refresh_interval: default_refresh_interval(),
            socket_timeout: default_socket_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

fn default_discovery_interval() -> Duration {
    Duration::from_secs(60)
}

fn default_refresh_interval() -> Duration {
    Duration::from_secs(1)
}

fn default_socket_timeout() -> Duration {
    Duration::from_secs(5)
}

fn default_max_retries() -> u32 {
    3
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.secret_key.is_empty() {
            return Err("Secret key cannot be empty".to_string());
        }
        if self.port == 0 {
            return Err("Port must be greater than 0".to_string());
        }
        if self.discovery_interval.as_secs() == 0 {
            return Err("Discovery interval must be greater than 0".to_string());
        }
        if self.refresh_interval.as_secs() == 0 {
            return Err("Refresh interval must be greater than 0".to_string());
        }
        Ok(())
    }
}