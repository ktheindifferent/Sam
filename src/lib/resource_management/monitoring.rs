use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// Resource monitor for tracking system resources
pub struct ResourceMonitor {
    metrics: Arc<RwLock<ResourceMetrics>>,
    history: Arc<RwLock<MetricsHistory>>,
    alerts: Arc<RwLock<Vec<ResourceAlert>>>,
    config: MonitorConfig,
}

/// Monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub enable_monitoring: bool,
    pub collection_interval: Duration,
    pub history_size: usize,
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig {
            enable_monitoring: true,
            collection_interval: Duration::from_secs(60),
            history_size: 1440, // 24 hours at 1 minute intervals
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

/// Alert thresholds
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub memory_warning: f32,
    pub memory_critical: f32,
    pub cpu_warning: f32,
    pub cpu_critical: f32,
    pub disk_warning: f32,
    pub disk_critical: f32,
    pub connections_warning: usize,
    pub connections_critical: usize,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        AlertThresholds {
            memory_warning: 0.8,
            memory_critical: 0.95,
            cpu_warning: 0.8,
            cpu_critical: 0.95,
            disk_warning: 0.8,
            disk_critical: 0.95,
            connections_warning: 80,
            connections_critical: 95,
        }
    }
}

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub memory: MemoryMetrics,
    pub cpu: CpuMetrics,
    pub disk: DiskMetrics,
    pub network: NetworkMetrics,
    pub connections: ConnectionMetrics,
    pub requests: RequestMetrics,
    pub files: FileMetrics,
    pub cleanup: CleanupMetrics,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        ResourceMetrics {
            timestamp: Utc::now(),
            uptime_seconds: 0,
            memory: MemoryMetrics::default(),
            cpu: CpuMetrics::default(),
            disk: DiskMetrics::default(),
            network: NetworkMetrics::default(),
            connections: ConnectionMetrics::default(),
            requests: RequestMetrics::default(),
            files: FileMetrics::default(),
            cleanup: CleanupMetrics::default(),
        }
    }
}

/// Memory metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub buffer_cache: u64,
    pub process_rss: u64,
    pub process_vms: u64,
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub load_average_1m: f32,
    pub load_average_5m: f32,
    pub load_average_15m: f32,
    pub core_count: usize,
    pub process_cpu_percent: f32,
    pub system_cpu_percent: f32,
    pub user_cpu_percent: f32,
}

/// Disk metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub iops_read: u64,
    pub iops_write: u64,
    pub temp_dir_size: u64,
    pub storage_dir_size: u64,
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_in: u64,
    pub errors_out: u64,
    pub active_connections: usize,
}

/// Connection metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionMetrics {
    pub pool_size: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub pending_connections: usize,
    pub failed_connections: u64,
    pub total_connections_created: u64,
    pub average_wait_time_ms: u64,
    pub circuit_breaker_state: String,
}

/// Request metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rate_limited_requests: u64,
    pub average_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub active_requests: usize,
    pub requests_per_second: f32,
}

/// File metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileMetrics {
    pub total_uploads: u64,
    pub successful_uploads: u64,
    pub failed_uploads: u64,
    pub total_bytes_uploaded: u64,
    pub active_uploads: usize,
    pub virus_scans_performed: u64,
    pub virus_threats_detected: u64,
    pub storage_quota_exceeded: u64,
}

/// Cleanup metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanupMetrics {
    pub total_cleanups: u64,
    pub successful_cleanups: u64,
    pub failed_cleanups: u64,
    pub files_cleaned: u64,
    pub bytes_freed: u64,
    pub last_cleanup: Option<DateTime<Utc>>,
    pub next_cleanup: Option<DateTime<Utc>>,
}

/// Metrics history
struct MetricsHistory {
    samples: Vec<ResourceMetrics>,
    max_size: usize,
}

impl MetricsHistory {
    fn new(max_size: usize) -> Self {
        MetricsHistory {
            samples: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn add(&mut self, metrics: ResourceMetrics) {
        if self.samples.len() >= self.max_size {
            self.samples.remove(0);
        }
        self.samples.push(metrics);
    }

    fn get_recent(&self, duration: Duration) -> Vec<ResourceMetrics> {
        let cutoff = Utc::now() - chrono::Duration::from_std(duration).unwrap();

        self.samples
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .cloned()
            .collect()
    }
}

/// Resource alert
#[derive(Debug, Clone, Serialize)]
pub struct ResourceAlert {
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub resource_type: ResourceType,
    pub message: String,
    pub value: f64,
    pub threshold: f64,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Resource type
#[derive(Debug, Clone, Serialize)]
pub enum ResourceType {
    Memory,
    Cpu,
    Disk,
    Network,
    Connections,
    Requests,
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        ResourceMonitor::with_config(MonitorConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        ResourceMonitor {
            metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            history: Arc::new(RwLock::new(MetricsHistory::new(config.history_size))),
            alerts: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Start monitoring
    pub async fn start(&self) {
        if !self.config.enable_monitoring {
            return;
        }

        let metrics = self.metrics.clone();
        let history = self.history.clone();
        let alerts = self.alerts.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.collection_interval);

            loop {
                interval.tick().await;

                // Collect metrics
                let new_metrics = Self::collect_metrics().await;

                // Check for alerts
                let new_alerts = Self::check_alerts(&new_metrics, &config.alert_thresholds);

                // Update current metrics
                *metrics.write().await = new_metrics.clone();

                // Add to history
                history.write().await.add(new_metrics);

                // Add new alerts
                if !new_alerts.is_empty() {
                    let mut alerts = alerts.write().await;
                    alerts.extend(new_alerts);

                    // Keep only recent alerts (last 1000)
                    let len = alerts.len();
                    if len > 1000 {
                        alerts.drain(0..len - 1000);
                    }
                }
            }
        });

        info!("Started resource monitoring");
    }

    /// Collect current metrics
    pub async fn collect_metrics() -> ResourceMetrics {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        let memory = MemoryMetrics {
            total_bytes: sys.total_memory() * 1024,
            used_bytes: sys.used_memory() * 1024,
            free_bytes: sys.free_memory() * 1024,
            usage_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
            swap_total: sys.total_swap() * 1024,
            swap_used: sys.used_swap() * 1024,
            swap_free: sys.free_swap() * 1024,
            buffer_cache: 0, // Platform specific
            process_rss: (std::process::id() as usize)
                .try_into()
                .ok()
                .and_then(|pid| sys.process(pid))
                .map(|p| p.memory())
                .unwrap_or(0)
                * 1024,
            process_vms: (std::process::id() as usize)
                .try_into()
                .ok()
                .and_then(|pid| sys.process(pid))
                .map(|p| p.virtual_memory())
                .unwrap_or(0)
                * 1024,
        };

        let cpu = CpuMetrics {
            usage_percent: sys.global_cpu_usage(),
            load_average_1m: sysinfo::System::load_average().one as f32,
            load_average_5m: sysinfo::System::load_average().five as f32,
            load_average_15m: sysinfo::System::load_average().fifteen as f32,
            core_count: sys.cpus().len(),
            process_cpu_percent: (std::process::id() as usize)
                .try_into()
                .ok()
                .and_then(|pid| sys.process(pid))
                .map(|p| p.cpu_usage())
                .unwrap_or(0.0),
            system_cpu_percent: 0.0, // Would need more complex calculation
            user_cpu_percent: 0.0,   // Would need more complex calculation
        };

        let disk = {
            let mut total = 0u64;
            let mut used = 0u64;

            let disks = sysinfo::Disks::new_with_refreshed_list();
            for disk in disks.list() {
                total += disk.total_space();
                used += disk.total_space() - disk.available_space();
            }

            DiskMetrics {
                total_bytes: total,
                used_bytes: used,
                free_bytes: total - used,
                usage_percent: if total > 0 {
                    (used as f32 / total as f32) * 100.0
                } else {
                    0.0
                },
                read_bytes_per_sec: 0,  // Would need delta calculation
                write_bytes_per_sec: 0, // Would need delta calculation
                iops_read: 0,           // Platform specific
                iops_write: 0,          // Platform specific
                temp_dir_size: 0,       // Would need directory traversal
                storage_dir_size: 0,    // Would need directory traversal
            }
        };

        let network = {
            let networks = sysinfo::Networks::new_with_refreshed_list();
            let mut bytes_sent = 0u64;
            let mut bytes_received = 0u64;
            let mut packets_sent = 0u64;
            let mut packets_received = 0u64;

            for (_interface_name, network_data) in &networks {
                bytes_sent += network_data.total_transmitted();
                bytes_received += network_data.total_received();
                packets_sent += network_data.total_packets_transmitted();
                packets_received += network_data.total_packets_received();
            }

            // for (_name, network) in networks {
            //     bytes_sent += network.total_transmitted();
            //     bytes_received += network.total_received();
            //     packets_sent += network.total_packets_transmitted();
            //     packets_received += network.total_packets_received();
            // }

            NetworkMetrics {
                bytes_sent,
                bytes_received,
                packets_sent,
                packets_received,
                errors_in: 0,          // Platform specific
                errors_out: 0,         // Platform specific
                active_connections: 0, // Would need netstat equivalent
            }
        };

        ResourceMetrics {
            timestamp: Utc::now(),
            uptime_seconds: sysinfo::System::uptime(),
            memory,
            cpu,
            disk,
            network,
            connections: ConnectionMetrics::default(),
            requests: RequestMetrics::default(),
            files: FileMetrics::default(),
            cleanup: CleanupMetrics::default(),
        }
    }

    /// Check for alerts
    fn check_alerts(metrics: &ResourceMetrics, thresholds: &AlertThresholds) -> Vec<ResourceAlert> {
        let mut alerts = Vec::new();

        // Check memory
        if metrics.memory.usage_percent > thresholds.memory_critical * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Critical,
                resource_type: ResourceType::Memory,
                message: format!(
                    "Memory usage critical: {:.1}%",
                    metrics.memory.usage_percent
                ),
                value: metrics.memory.usage_percent as f64,
                threshold: (thresholds.memory_critical * 100.0) as f64,
            });
        } else if metrics.memory.usage_percent > thresholds.memory_warning * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Warning,
                resource_type: ResourceType::Memory,
                message: format!("Memory usage high: {:.1}%", metrics.memory.usage_percent),
                value: metrics.memory.usage_percent as f64,
                threshold: (thresholds.memory_warning * 100.0) as f64,
            });
        }

        // Check CPU
        if metrics.cpu.usage_percent > thresholds.cpu_critical * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Critical,
                resource_type: ResourceType::Cpu,
                message: format!("CPU usage critical: {:.1}%", metrics.cpu.usage_percent),
                value: metrics.cpu.usage_percent as f64,
                threshold: (thresholds.cpu_critical * 100.0) as f64,
            });
        } else if metrics.cpu.usage_percent > thresholds.cpu_warning * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Warning,
                resource_type: ResourceType::Cpu,
                message: format!("CPU usage high: {:.1}%", metrics.cpu.usage_percent),
                value: metrics.cpu.usage_percent as f64,
                threshold: (thresholds.cpu_warning * 100.0) as f64,
            });
        }

        // Check disk
        if metrics.disk.usage_percent > thresholds.disk_critical * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Critical,
                resource_type: ResourceType::Disk,
                message: format!("Disk usage critical: {:.1}%", metrics.disk.usage_percent),
                value: metrics.disk.usage_percent as f64,
                threshold: (thresholds.disk_critical * 100.0) as f64,
            });
        } else if metrics.disk.usage_percent > thresholds.disk_warning * 100.0 {
            alerts.push(ResourceAlert {
                timestamp: Utc::now(),
                severity: AlertSeverity::Warning,
                resource_type: ResourceType::Disk,
                message: format!("Disk usage high: {:.1}%", metrics.disk.usage_percent),
                value: metrics.disk.usage_percent as f64,
                threshold: (thresholds.disk_warning * 100.0) as f64,
            });
        }

        alerts
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get metrics history
    pub async fn get_history(&self, duration: Duration) -> Vec<ResourceMetrics> {
        self.history.read().await.get_recent(duration)
    }

    /// Get recent alerts
    pub async fn get_alerts(&self) -> Vec<ResourceAlert> {
        self.alerts.read().await.clone()
    }

    /// Record cleanup success
    pub fn record_cleanup_success(&self) {
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            let mut m = metrics.write().await;
            m.cleanup.successful_cleanups += 1;
            m.cleanup.total_cleanups += 1;
            m.cleanup.last_cleanup = Some(Utc::now());
        });
    }

    /// Record cleanup failure
    pub fn record_cleanup_failure(&self) {
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            let mut m = metrics.write().await;
            m.cleanup.failed_cleanups += 1;
            m.cleanup.total_cleanups += 1;
        });
    }

    /// Update connection metrics
    pub async fn update_connection_metrics(&self, metrics: ConnectionMetrics) {
        let mut m = self.metrics.write().await;
        m.connections = metrics;
    }

    /// Update request metrics
    pub async fn update_request_metrics(&self, metrics: RequestMetrics) {
        let mut m = self.metrics.write().await;
        m.requests = metrics;
    }

    /// Update file metrics
    pub async fn update_file_metrics(&self, metrics: FileMetrics) {
        let mut m = self.metrics.write().await;
        m.files = metrics;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_monitor_creation() {
        let monitor = ResourceMonitor::new();
        let metrics = monitor.get_metrics().await;

        assert!(metrics.timestamp <= Utc::now());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let metrics = ResourceMonitor::collect_metrics().await;

        // Basic sanity checks
        assert!(metrics.memory.total_bytes > 0);
        assert!(metrics.cpu.core_count > 0);
        assert!(metrics.disk.total_bytes > 0);
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds::default();

        assert!(thresholds.memory_warning < thresholds.memory_critical);
        assert!(thresholds.cpu_warning < thresholds.cpu_critical);
        assert!(thresholds.disk_warning < thresholds.disk_critical);
    }
}
