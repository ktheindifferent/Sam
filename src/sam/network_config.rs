// Network Monitoring Configuration Module
// Provides configuration structures and defaults for network monitoring

use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMonitorConfig {
    pub enabled: bool,
    pub update_interval_ms: u64,
    pub history_size: usize,
    pub latency_check_hosts: Vec<String>,
    pub latency_check_interval_ms: u64,
    pub interfaces_to_monitor: Vec<String>,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub high_latency_ms: f64,
    pub packet_loss_percent: f64,
    pub low_bandwidth_mbps: f64,
    pub high_error_rate: f64,
}

impl Default for NetworkMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval_ms: 1000, // 1 second
            history_size: 60, // Keep 60 samples for moving average
            latency_check_hosts: vec![
                "8.8.8.8".to_string(),      // Google DNS
                "1.1.1.1".to_string(),      // Cloudflare DNS
                "208.67.222.222".to_string(), // OpenDNS
            ],
            latency_check_interval_ms: 5000, // Check every 5 seconds
            interfaces_to_monitor: vec![], // Empty means all interfaces
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            high_latency_ms: 100.0,
            packet_loss_percent: 5.0,
            low_bandwidth_mbps: 1.0,
            high_error_rate: 0.01, // 1% error rate
        }
    }
}

impl NetworkMonitorConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(enabled) = std::env::var("SAM_NETWORK_MONITOR_ENABLED") {
            config.enabled = enabled.parse().unwrap_or(true);
        }
        
        if let Ok(interval) = std::env::var("SAM_NETWORK_UPDATE_INTERVAL_MS") {
            if let Ok(ms) = interval.parse() {
                config.update_interval_ms = ms;
            }
        }
        
        if let Ok(size) = std::env::var("SAM_NETWORK_HISTORY_SIZE") {
            if let Ok(history_size) = size.parse() {
                config.history_size = history_size;
            }
        }
        
        if let Ok(hosts) = std::env::var("SAM_LATENCY_CHECK_HOSTS") {
            config.latency_check_hosts = hosts.split(',').map(|s| s.trim().to_string()).collect();
        }
        
        config
    }
    
    pub fn update_interval(&self) -> Duration {
        Duration::from_millis(self.update_interval_ms)
    }
    
    pub fn latency_check_interval(&self) -> Duration {
        Duration::from_millis(self.latency_check_interval_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = NetworkMonitorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.update_interval_ms, 1000);
        assert_eq!(config.history_size, 60);
        assert!(!config.latency_check_hosts.is_empty());
    }
    
    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds::default();
        assert_eq!(thresholds.high_latency_ms, 100.0);
        assert_eq!(thresholds.packet_loss_percent, 5.0);
        assert_eq!(thresholds.low_bandwidth_mbps, 1.0);
    }
}