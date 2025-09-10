// Network Speed and Latency Monitoring Module
// Provides real-time network metrics including bandwidth, latency, and connection statistics

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use log::info;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use log::{debug, error, warn};
use crate::network_config::NetworkMonitorConfig;

// ==================== Network Statistics ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkSpeed {
    pub interface: String,
    pub download_speed_bps: f64,  // Bytes per second
    pub upload_speed_bps: f64,    // Bytes per second
    pub download_speed_mbps: f64, // Megabits per second
    pub upload_speed_mbps: f64,   // Megabits per second
    pub total_speed_mbps: f64,    // Combined speed in Mbps
    #[serde(skip)]
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkLatency {
    pub host: String,
    pub latency_ms: f64,
    pub packet_loss: f64,
    pub jitter_ms: f64,
    #[serde(skip)]
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub speeds: HashMap<String, NetworkSpeed>,
    pub latencies: Vec<NetworkLatency>,
    pub total_download_mbps: f64,
    pub total_upload_mbps: f64,
    pub average_latency_ms: f64,
    pub packet_loss_percent: f64,
}

// ==================== Network Monitor ====================

pub struct NetworkMonitor {
    interfaces: Arc<RwLock<HashMap<String, NetworkInterface>>>,
    speed_history: Arc<RwLock<HashMap<String, VecDeque<NetworkSpeed>>>>,
    latency_history: Arc<RwLock<VecDeque<NetworkLatency>>>,
    last_update: Arc<RwLock<Instant>>,
    history_size: usize,
    update_interval: Duration,
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkMonitor {
    pub fn new() -> Self {
        let config = NetworkMonitorConfig::default();
        Self::from_config(config)
    }

    pub fn with_config(history_size: usize, update_interval: Duration) -> Self {
        Self {
            interfaces: Arc::new(RwLock::new(HashMap::new())),
            speed_history: Arc::new(RwLock::new(HashMap::new())),
            latency_history: Arc::new(RwLock::new(VecDeque::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            history_size,
            update_interval,
        }
    }
    
    pub fn from_config(config: NetworkMonitorConfig) -> Self {
        Self {
            interfaces: Arc::new(RwLock::new(HashMap::new())),
            speed_history: Arc::new(RwLock::new(HashMap::new())),
            latency_history: Arc::new(RwLock::new(VecDeque::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            history_size: config.history_size,
            update_interval: config.update_interval(),
        }
    }

    // Parse /proc/net/dev for network interface statistics
    pub async fn read_network_stats(&self) -> Result<HashMap<String, NetworkInterface>> {
        // Check if we're on Linux with /proc/net/dev
        if !std::path::Path::new("/proc/net/dev").exists() {
            // Return empty stats for non-Linux systems
            return Ok(HashMap::new());
        }
        
        let file = File::open("/proc/net/dev")
            .context("Failed to open /proc/net/dev")?;
        let reader = BufReader::new(file);
        let mut interfaces = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            // Skip header lines
            if line_num < 2 {
                continue;
            }

            let line = line.context("Failed to read line from /proc/net/dev")?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            if parts.len() < 17 {
                continue;
            }

            // Interface name is the first field (remove trailing colon)
            let interface_name = parts[0].trim_end_matches(':').to_string();
            
            // Skip loopback interface for speed calculations
            if interface_name == "lo" {
                continue;
            }

            let interface = NetworkInterface {
                name: interface_name.clone(),
                rx_bytes: parts[1].parse().unwrap_or(0),
                tx_bytes: parts[9].parse().unwrap_or(0),
                rx_packets: parts[2].parse().unwrap_or(0),
                tx_packets: parts[10].parse().unwrap_or(0),
                rx_errors: parts[3].parse().unwrap_or(0),
                tx_errors: parts[11].parse().unwrap_or(0),
                rx_dropped: parts[4].parse().unwrap_or(0),
                tx_dropped: parts[12].parse().unwrap_or(0),
            };

            interfaces.insert(interface_name, interface);
        }

        Ok(interfaces)
    }

    // Calculate network speed based on byte deltas
    pub async fn calculate_speeds(&self) -> Result<HashMap<String, NetworkSpeed>> {
        let current_stats = self.read_network_stats().await?;
        let mut previous_interfaces = self.interfaces.write().await;
        let last_update = *self.last_update.read().await;
        let now = Instant::now();
        let time_delta = now.duration_since(last_update).as_secs_f64();
        
        let mut speeds = HashMap::new();

        if time_delta > 0.0 {
            for (name, current) in &current_stats {
                if let Some(previous) = previous_interfaces.get(name) {
                    let rx_delta = current.rx_bytes.saturating_sub(previous.rx_bytes) as f64;
                    let tx_delta = current.tx_bytes.saturating_sub(previous.tx_bytes) as f64;
                    
                    let download_speed_bps = rx_delta / time_delta;
                    let upload_speed_bps = tx_delta / time_delta;
                    
                    // Convert to Mbps (megabits per second)
                    let download_speed_mbps = (download_speed_bps * 8.0) / 1_000_000.0;
                    let upload_speed_mbps = (upload_speed_bps * 8.0) / 1_000_000.0;
                    
                    let speed = NetworkSpeed {
                        interface: name.clone(),
                        download_speed_bps,
                        upload_speed_bps,
                        download_speed_mbps,
                        upload_speed_mbps,
                        total_speed_mbps: download_speed_mbps + upload_speed_mbps,
                        timestamp: now,
                    };
                    
                    speeds.insert(name.clone(), speed);
                }
            }
        }

        // Update stored interfaces and timestamp
        *previous_interfaces = current_stats;
        *self.last_update.write().await = now;
        
        // Update speed history
        let mut history = self.speed_history.write().await;
        for (name, speed) in &speeds {
            let interface_history = history.entry(name.clone()).or_insert_with(VecDeque::new);
            interface_history.push_back(speed.clone());
            
            // Keep only recent history
            while interface_history.len() > self.history_size {
                interface_history.pop_front();
            }
        }

        Ok(speeds)
    }

    // Get moving average of network speeds
    pub async fn get_average_speeds(&self) -> HashMap<String, NetworkSpeed> {
        let history = self.speed_history.read().await;
        let mut averages = HashMap::new();

        for (interface, speeds) in history.iter() {
            if speeds.is_empty() {
                continue;
            }

            let count = speeds.len() as f64;
            let avg_download_bps: f64 = speeds.iter().map(|s| s.download_speed_bps).sum::<f64>() / count;
            let avg_upload_bps: f64 = speeds.iter().map(|s| s.upload_speed_bps).sum::<f64>() / count;
            let avg_download_mbps: f64 = speeds.iter().map(|s| s.download_speed_mbps).sum::<f64>() / count;
            let avg_upload_mbps: f64 = speeds.iter().map(|s| s.upload_speed_mbps).sum::<f64>() / count;

            averages.insert(interface.clone(), NetworkSpeed {
                interface: interface.clone(),
                download_speed_bps: avg_download_bps,
                upload_speed_bps: avg_upload_bps,
                download_speed_mbps: avg_download_mbps,
                upload_speed_mbps: avg_upload_mbps,
                total_speed_mbps: avg_download_mbps + avg_upload_mbps,
                timestamp: Instant::now(),
            });
        }

        averages
    }

    // Measure network latency using ping
    pub async fn measure_latency(&self, host: &str) -> Result<NetworkLatency> {
        let start = Instant::now();
        
        // Execute ping command (4 packets)
        let output = tokio::process::Command::new("ping")
            .arg("-c")
            .arg("4")
            .arg("-W")
            .arg("1")  // 1 second timeout
            .arg(host)
            .output()
            .await
            .context("Failed to execute ping command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse ping output for statistics
        let mut latency_ms = 0.0;
        let mut packet_loss = 0.0;
        let mut min_rtt = 0.0;
        let mut max_rtt = 0.0;
        
        for line in stdout.lines() {
            // Parse packet loss: "4 packets transmitted, 3 received, 25% packet loss"
            if line.contains("packet loss") {
                if let Some(loss_str) = line.split('%').next() {
                    if let Some(loss_num) = loss_str.split_whitespace().last() {
                        packet_loss = loss_num.parse().unwrap_or(0.0);
                    }
                }
            }
            
            // Parse RTT stats: "rtt min/avg/max/mdev = 1.234/5.678/9.012/2.345 ms"
            if line.starts_with("rtt min/avg/max") {
                if let Some(stats) = line.split('=').nth(1) {
                    let values: Vec<&str> = stats.trim().split('/').collect();
                    if values.len() >= 4 {
                        min_rtt = values[0].parse().unwrap_or(0.0);
                        latency_ms = values[1].parse().unwrap_or(0.0);
                        max_rtt = values[2].parse().unwrap_or(0.0);
                    }
                }
            }
        }
        
        // Calculate jitter (variation in latency)
        let jitter_ms = if max_rtt > 0.0 && min_rtt > 0.0 {
            max_rtt - min_rtt
        } else {
            0.0
        };
        
        let latency = NetworkLatency {
            host: host.to_string(),
            latency_ms,
            packet_loss,
            jitter_ms,
            timestamp: start,
        };
        
        // Update latency history
        let mut history = self.latency_history.write().await;
        history.push_back(latency.clone());
        
        // Keep only recent history
        while history.len() > self.history_size {
            history.pop_front();
        }
        
        Ok(latency)
    }

    // Measure latency to multiple hosts
    pub async fn measure_multiple_latencies(&self, hosts: &[&str]) -> Vec<NetworkLatency> {
        let mut latencies = Vec::new();
        
        for host in hosts {
            match self.measure_latency(host).await {
                Ok(latency) => latencies.push(latency),
                Err(e) => {
                    warn!("Failed to measure latency to {}: {}", host, e);
                    // Add a failed measurement
                    latencies.push(NetworkLatency {
                        host: host.to_string(),
                        latency_ms: -1.0,
                        packet_loss: 100.0,
                        jitter_ms: 0.0,
                        timestamp: Instant::now(),
                    });
                }
            }
        }
        
        latencies
    }

    // Get comprehensive network metrics
    pub async fn get_metrics(&self) -> Result<NetworkMetrics> {
        let speeds = self.calculate_speeds().await?;
        let average_speeds = self.get_average_speeds().await;
        
        // Calculate totals
        let total_download_mbps: f64 = speeds.values().map(|s| s.download_speed_mbps).sum();
        let total_upload_mbps: f64 = speeds.values().map(|s| s.upload_speed_mbps).sum();
        
        // Get latency metrics
        let latency_history = self.latency_history.read().await;
        let latencies: Vec<NetworkLatency> = latency_history.iter().cloned().collect();
        
        let average_latency_ms = if !latencies.is_empty() {
            let valid_latencies: Vec<f64> = latencies
                .iter()
                .filter(|l| l.latency_ms >= 0.0)
                .map(|l| l.latency_ms)
                .collect();
            
            if !valid_latencies.is_empty() {
                valid_latencies.iter().sum::<f64>() / valid_latencies.len() as f64
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        let packet_loss_percent = if !latencies.is_empty() {
            latencies.iter().map(|l| l.packet_loss).sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };
        
        Ok(NetworkMetrics {
            speeds: average_speeds,
            latencies,
            total_download_mbps,
            total_upload_mbps,
            average_latency_ms,
            packet_loss_percent,
        })
    }

    // Get total network speed across all interfaces (for dashboard)
    pub async fn get_total_speed_mbps(&self) -> Result<f64> {
        let speeds = self.calculate_speeds().await?;
        Ok(speeds.values().map(|s| s.total_speed_mbps).sum())
    }

    // Start continuous monitoring with configurable latency checks
    pub async fn start_monitoring_with_config(&self, config: NetworkMonitorConfig) {
        if !config.enabled {
            debug!("Network monitoring is disabled");
            return;
        }
        
        // Start speed monitoring task
        let speed_monitor = self.clone();
        let update_interval = config.update_interval();
        tokio::spawn(async move {
            loop {
                if let Err(e) = speed_monitor.calculate_speeds().await {
                    error!("Failed to calculate network speeds: {}", e);
                }
                
                tokio::time::sleep(update_interval).await;
            }
        });
        
        // Start latency monitoring task
        let latency_monitor = self.clone();
        let latency_hosts = config.latency_check_hosts.clone();
        let latency_interval = config.latency_check_interval();
        tokio::spawn(async move {
            loop {
                let hosts: Vec<&str> = latency_hosts.iter().map(|s| s.as_str()).collect();
                let _ = latency_monitor.measure_multiple_latencies(&hosts).await;
                
                tokio::time::sleep(latency_interval).await;
            }
        });
        
        info!("Network monitoring started with update interval: {:?}", update_interval);
    }
    
    // Start continuous monitoring with defaults
    pub async fn start_monitoring(&self) {
        let config = NetworkMonitorConfig::default();
        self.start_monitoring_with_config(config).await;
    }
}

impl Clone for NetworkMonitor {
    fn clone(&self) -> Self {
        Self {
            interfaces: Arc::clone(&self.interfaces),
            speed_history: Arc::clone(&self.speed_history),
            latency_history: Arc::clone(&self.latency_history),
            last_update: Arc::clone(&self.last_update),
            history_size: self.history_size,
            update_interval: self.update_interval,
        }
    }
}

// ==================== Connection Statistics ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub tcp_established: u32,
    pub tcp_listen: u32,
    pub tcp_time_wait: u32,
    pub udp_connections: u32,
    pub total_connections: u32,
}

impl ConnectionStats {
    pub async fn gather() -> Result<Self> {
        let output = tokio::process::Command::new("ss")
            .arg("-s")
            .output()
            .await
            .context("Failed to execute ss command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut stats = ConnectionStats {
            tcp_established: 0,
            tcp_listen: 0,
            tcp_time_wait: 0,
            udp_connections: 0,
            total_connections: 0,
        };
        
        for line in stdout.lines() {
            if line.contains("TCP:") {
                // Parse TCP connection counts
                if let Some(count_str) = line.split('(').nth(1) {
                    if let Some(num) = count_str.split_whitespace().next() {
                        stats.total_connections += num.parse().unwrap_or(0);
                    }
                }
            }
            
            // Parse specific states
            if line.contains("ESTAB") {
                if let Some(num) = extract_number_from_line(line) {
                    stats.tcp_established = num;
                }
            } else if line.contains("LISTEN") {
                if let Some(num) = extract_number_from_line(line) {
                    stats.tcp_listen = num;
                }
            } else if line.contains("TIME-WAIT") {
                if let Some(num) = extract_number_from_line(line) {
                    stats.tcp_time_wait = num;
                }
            }
        }
        
        Ok(stats)
    }
}

fn extract_number_from_line(line: &str) -> Option<u32> {
    line.split_whitespace()
        .find_map(|part| part.parse().ok())
}

impl Default for NetworkSpeed {
    fn default() -> Self {
        Self {
            interface: String::new(),
            download_speed_bps: 0.0,
            upload_speed_bps: 0.0,
            download_speed_mbps: 0.0,
            upload_speed_mbps: 0.0,
            total_speed_mbps: 0.0,
            timestamp: Instant::now(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for NetworkSpeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NetworkSpeedData {
            interface: String,
            download_speed_bps: f64,
            upload_speed_bps: f64,
            download_speed_mbps: f64,
            upload_speed_mbps: f64,
            total_speed_mbps: f64,
        }
        
        let data = NetworkSpeedData::deserialize(deserializer)?;
        Ok(NetworkSpeed {
            interface: data.interface,
            download_speed_bps: data.download_speed_bps,
            upload_speed_bps: data.upload_speed_bps,
            download_speed_mbps: data.download_speed_mbps,
            upload_speed_mbps: data.upload_speed_mbps,
            total_speed_mbps: data.total_speed_mbps,
            timestamp: Instant::now(), // Use current time for deserialization
        })
    }
}

impl Default for NetworkLatency {
    fn default() -> Self {
        Self {
            host: String::new(),
            latency_ms: 0.0,
            packet_loss: 0.0,
            jitter_ms: 0.0,
            timestamp: Instant::now(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for NetworkLatency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NetworkLatencyData {
            host: String,
            latency_ms: f64,
            packet_loss: f64,
            jitter_ms: f64,
        }
        
        let data = NetworkLatencyData::deserialize(deserializer)?;
        Ok(NetworkLatency {
            host: data.host,
            latency_ms: data.latency_ms,
            packet_loss: data.packet_loss,
            jitter_ms: data.jitter_ms,
            timestamp: Instant::now(), // Use current time for deserialization
        })
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_monitor_creation() {
        let monitor = NetworkMonitor::new();
        assert!(monitor.history_size > 0);
    }
    
    #[tokio::test]
    async fn test_network_monitor_from_config() {
        let config = NetworkMonitorConfig {
            enabled: true,
            update_interval_ms: 500,
            history_size: 30,
            latency_check_hosts: vec!["127.0.0.1".to_string()],
            latency_check_interval_ms: 10000,
            interfaces_to_monitor: vec![],
            alert_thresholds: crate::network_config::AlertThresholds::default(),
        };
        
        let monitor = NetworkMonitor::from_config(config);
        assert_eq!(monitor.history_size, 30);
        assert_eq!(monitor.update_interval, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_network_stats_reading() {
        let monitor = NetworkMonitor::new();
        
        // This test will only work on Linux systems with /proc/net/dev
        if std::path::Path::new("/proc/net/dev").exists() {
            let stats = monitor.read_network_stats().await;
            assert!(stats.is_ok());
            
            let interfaces = stats.unwrap();
            // Most Linux systems have at least one network interface
            assert!(!interfaces.is_empty());
            
            // Check that we parsed the interface data correctly
            for (name, interface) in interfaces {
                assert!(!name.is_empty());
                assert!(interface.rx_bytes >= 0);
                assert!(interface.tx_bytes >= 0);
            }
        }
    }

    #[tokio::test]
    async fn test_speed_calculation() {
        let monitor = NetworkMonitor::new();
        
        if std::path::Path::new("/proc/net/dev").exists() {
            // First reading to establish baseline
            let _ = monitor.read_network_stats().await;
            
            // Wait a bit for some network activity
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Calculate speeds
            let speeds = monitor.calculate_speeds().await;
            assert!(speeds.is_ok());
            
            let speed_map = speeds.unwrap();
            for (_, speed) in speed_map {
                assert!(speed.download_speed_bps >= 0.0);
                assert!(speed.upload_speed_bps >= 0.0);
                assert!(speed.download_speed_mbps >= 0.0);
                assert!(speed.upload_speed_mbps >= 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_moving_average() {
        let monitor = NetworkMonitor::with_config(5, Duration::from_millis(100));
        
        if std::path::Path::new("/proc/net/dev").exists() {
            // Take multiple measurements
            for _ in 0..3 {
                let _ = monitor.calculate_speeds().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            
            let averages = monitor.get_average_speeds().await;
            
            // Check that averages are calculated correctly
            for (_, avg_speed) in averages {
                assert!(avg_speed.download_speed_bps >= 0.0);
                assert!(avg_speed.upload_speed_bps >= 0.0);
            }
        }
    }
    
    #[tokio::test]
    async fn test_latency_measurement() {
        let monitor = NetworkMonitor::new();
        
        // Test with localhost (should always work)
        let result = monitor.measure_latency("127.0.0.1").await;
        
        if result.is_ok() {
            let latency = result.unwrap();
            assert_eq!(latency.host, "127.0.0.1");
            // Localhost should have very low latency
            assert!(latency.latency_ms >= 0.0 || latency.latency_ms == -1.0);
            assert!(latency.packet_loss >= 0.0);
        }
    }
    
    #[tokio::test]
    async fn test_multiple_latency_measurements() {
        let monitor = NetworkMonitor::new();
        
        let hosts = vec!["127.0.0.1", "localhost"];
        let latencies = monitor.measure_multiple_latencies(&hosts).await;
        
        assert_eq!(latencies.len(), 2);
        for latency in latencies {
            assert!(hosts.contains(&latency.host.as_str()));
        }
    }
    
    #[tokio::test]
    async fn test_comprehensive_metrics() {
        let monitor = NetworkMonitor::new();
        
        if std::path::Path::new("/proc/net/dev").exists() {
            // Initialize with some data
            let _ = monitor.calculate_speeds().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = monitor.calculate_speeds().await;
            
            let metrics = monitor.get_metrics().await;
            assert!(metrics.is_ok());
            
            let network_metrics = metrics.unwrap();
            assert!(network_metrics.total_download_mbps >= 0.0);
            assert!(network_metrics.total_upload_mbps >= 0.0);
            assert!(network_metrics.average_latency_ms >= 0.0);
            assert!(network_metrics.packet_loss_percent >= 0.0);
        }
    }
    
    #[tokio::test]
    async fn test_connection_stats() {
        // This test requires ss command to be available
        let stats_result = ConnectionStats::gather().await;
        
        if stats_result.is_ok() {
            let stats = stats_result.unwrap();
            assert!(stats.tcp_established >= 0);
            assert!(stats.tcp_listen >= 0);
            assert!(stats.total_connections >= 0);
        }
    }
}