use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::{Level, Metadata, Record};
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramVec, TextEncoder, Encoder
};
use opentelemetry::{
    global,
    sdk::{trace, Resource},
    trace::{Tracer, TracerProvider},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    pub level: String,
    pub output: LogOutput,
    pub format: LogFormat,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub otlp_endpoint: Option<String>,
    pub log_file_path: Option<String>,
    pub max_log_size_mb: u64,
    pub max_log_files: u32,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogOutput {
    Stdout,
    File,
    Both,
    Syslog,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".to_string(),
            output: LogOutput::Both,
            format: LogFormat::Json,
            enable_metrics: true,
            enable_tracing: true,
            otlp_endpoint: None,
            log_file_path: Some("/var/log/sam/sam.log".to_string()),
            max_log_size_mb: 100,
            max_log_files: 10,
            buffer_size: 8192,
        }
    }
}

/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub module: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: HashMap<String, serde_json::Value>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

/// Metrics collector
pub struct MetricsCollector {
    request_counter: CounterVec,
    response_time_histogram: HistogramVec,
    active_connections_gauge: GaugeVec,
    error_counter: CounterVec,
    cpu_usage_gauge: GaugeVec,
    memory_usage_gauge: GaugeVec,
    disk_usage_gauge: GaugeVec,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let request_counter = register_counter_vec!(
            "sam_http_requests_total",
            "Total number of HTTP requests",
            &["method", "endpoint", "status"]
        )?;
        
        let response_time_histogram = register_histogram_vec!(
            "sam_http_response_time_seconds",
            "HTTP response time in seconds",
            &["method", "endpoint"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;
        
        let active_connections_gauge = register_gauge_vec!(
            "sam_active_connections",
            "Number of active connections",
            &["type"]
        )?;
        
        let error_counter = register_counter_vec!(
            "sam_errors_total",
            "Total number of errors",
            &["service", "error_type"]
        )?;
        
        let cpu_usage_gauge = register_gauge_vec!(
            "sam_cpu_usage_percent",
            "CPU usage percentage",
            &["core"]
        )?;
        
        let memory_usage_gauge = register_gauge_vec!(
            "sam_memory_usage_bytes",
            "Memory usage in bytes",
            &["type"]
        )?;
        
        let disk_usage_gauge = register_gauge_vec!(
            "sam_disk_usage_bytes",
            "Disk usage in bytes",
            &["mount"]
        )?;
        
        Ok(MetricsCollector {
            request_counter,
            response_time_histogram,
            active_connections_gauge,
            error_counter,
            cpu_usage_gauge,
            memory_usage_gauge,
            disk_usage_gauge,
        })
    }
    
    /// Record an HTTP request
    pub fn record_request(&self, method: &str, endpoint: &str, status: u16, duration: f64) {
        self.request_counter
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
        
        self.response_time_histogram
            .with_label_values(&[method, endpoint])
            .observe(duration);
    }
    
    /// Record an error
    pub fn record_error(&self, service: &str, error_type: &str) {
        self.error_counter
            .with_label_values(&[service, error_type])
            .inc();
    }
    
    /// Update active connections
    pub fn set_active_connections(&self, conn_type: &str, count: f64) {
        self.active_connections_gauge
            .with_label_values(&[conn_type])
            .set(count);
    }
    
    /// Update system metrics
    pub fn update_system_metrics(&self) {
        use sysinfo::{System, SystemExt, CpuExt};
        
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // CPU usage
        for (i, cpu) in sys.cpus().iter().enumerate() {
            self.cpu_usage_gauge
                .with_label_values(&[&i.to_string()])
                .set(cpu.cpu_usage() as f64);
        }
        
        // Memory usage
        self.memory_usage_gauge
            .with_label_values(&["used"])
            .set(sys.used_memory() as f64);
        
        self.memory_usage_gauge
            .with_label_values(&["total"])
            .set(sys.total_memory() as f64);
        
        // Disk usage
        for disk in sys.disks() {
            let mount_point = disk.mount_point().to_string_lossy();
            self.disk_usage_gauge
                .with_label_values(&[&mount_point])
                .set((disk.total_space() - disk.available_space()) as f64);
        }
    }
    
    /// Export metrics in Prometheus format
    pub fn export_metrics(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

/// Logging and monitoring manager
pub struct LoggingManager {
    config: LogConfig,
    metrics: Arc<MetricsCollector>,
    log_buffer: Arc<RwLock<Vec<LogEntry>>>,
    tracer: Option<Box<dyn Tracer>>,
}

impl LoggingManager {
    /// Initialize the logging system
    pub async fn init(config: LogConfig) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        // Initialize logger
        Self::init_logger(&config)?;
        
        // Initialize metrics
        let metrics = Arc::new(MetricsCollector::new()?);
        
        // Initialize tracing if enabled
        let tracer = if config.enable_tracing {
            Some(Self::init_tracing(&config)?)
        } else {
            None
        };
        
        let manager = Arc::new(LoggingManager {
            config,
            metrics,
            log_buffer: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            tracer,
        });
        
        // Start background tasks
        manager.start_background_tasks();
        
        Ok(manager)
    }
    
    /// Initialize the logger
    fn init_logger(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
        let log_level = match config.level.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        };
        
        match config.output {
            LogOutput::Stdout => {
                env_logger::Builder::new()
                    .filter_level(log_level)
                    .init();
            }
            LogOutput::File => {
                if let Some(path) = &config.log_file_path {
                    // Setup file logging with rotation
                    Self::setup_file_logging(path, config.max_log_size_mb, config.max_log_files)?;
                }
            }
            LogOutput::Both => {
                // Setup both stdout and file logging
                env_logger::Builder::new()
                    .filter_level(log_level)
                    .init();
                
                if let Some(path) = &config.log_file_path {
                    Self::setup_file_logging(path, config.max_log_size_mb, config.max_log_files)?;
                }
            }
            LogOutput::Syslog => {
                // Setup syslog logging
                #[cfg(unix)]
                {
                    use syslog::{Facility, Formatter3164};
                    
                    let formatter = Formatter3164 {
                        facility: Facility::LOG_USER,
                        hostname: None,
                        process: "sam".into(),
                        pid: std::process::id(),
                    };
                    
                    let logger = syslog::unix(formatter)?;
                    log::set_boxed_logger(Box::new(logger))?;
                    log::set_max_level(log_level);
                }
            }
        }
        
        Ok(())
    }
    
    /// Setup file logging with rotation
    fn setup_file_logging(
        path: &str,
        max_size_mb: u64,
        max_files: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use std::path::Path;
        
        // Create log directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // TODO: Implement proper log rotation
        // This would use a library like `tracing-appender` or `log4rs`
        
        Ok(())
    }
    
    /// Initialize OpenTelemetry tracing
    fn init_tracing(config: &LogConfig) -> Result<Box<dyn Tracer>, Box<dyn std::error::Error>> {
        let otlp_endpoint = config.otlp_endpoint.clone()
            .unwrap_or_else(|| "http://localhost:4317".to_string());
        
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(otlp_endpoint)
            )
            .with_trace_config(
                trace::config()
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", "sam"),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
            )
            .install_batch(opentelemetry::runtime::Tokio)?;
        
        Ok(Box::new(tracer))
    }
    
    /// Start background tasks for metrics collection
    fn start_background_tasks(&self) {
        let metrics = self.metrics.clone();
        
        // System metrics collector (every 10 seconds)
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                metrics.update_system_metrics();
            }
        });
        
        // Log buffer flusher (every 5 seconds)
        let buffer = self.log_buffer.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                let mut buf = buffer.write().await;
                if !buf.is_empty() {
                    // Flush logs to persistent storage
                    // TODO: Implement actual persistence
                    buf.clear();
                }
            }
        });
    }
    
    /// Log a structured message
    pub async fn log(&self, entry: LogEntry) {
        // Add to buffer
        let mut buffer = self.log_buffer.write().await;
        buffer.push(entry.clone());
        
        // Format and output based on configuration
        match self.config.format {
            LogFormat::Json => {
                println!("{}", serde_json::to_string(&entry).unwrap());
            }
            LogFormat::Text => {
                println!(
                    "[{}] {} {} - {}",
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    entry.level,
                    entry.module,
                    entry.message
                );
            }
            LogFormat::Compact => {
                println!("{} {} {}", entry.level, entry.module, entry.message);
            }
        }
    }
    
    /// Create a new span for tracing
    pub fn create_span(&self, name: &str) -> Option<opentelemetry::trace::Span> {
        self.tracer.as_ref().map(|tracer| {
            use opentelemetry::trace::Tracer;
            tracer.start(name)
        })
    }
    
    /// Get metrics collector
    pub fn metrics(&self) -> Arc<MetricsCollector> {
        self.metrics.clone()
    }
    
    /// Get recent logs
    pub async fn get_recent_logs(&self, limit: usize) -> Vec<LogEntry> {
        let buffer = self.log_buffer.read().await;
        let start = if buffer.len() > limit {
            buffer.len() - limit
        } else {
            0
        };
        buffer[start..].to_vec()
    }
    
    /// Search logs
    pub async fn search_logs(&self, query: LogSearchQuery) -> Vec<LogEntry> {
        let buffer = self.log_buffer.read().await;
        
        buffer
            .iter()
            .filter(|entry| {
                // Filter by level
                if let Some(ref level) = query.level {
                    if entry.level != *level {
                        return false;
                    }
                }
                
                // Filter by module
                if let Some(ref module) = query.module {
                    if !entry.module.contains(module) {
                        return false;
                    }
                }
                
                // Filter by message
                if let Some(ref message) = query.message_contains {
                    if !entry.message.contains(message) {
                        return false;
                    }
                }
                
                // Filter by time range
                if let Some(ref after) = query.after {
                    if entry.timestamp < *after {
                        return false;
                    }
                }
                
                if let Some(ref before) = query.before {
                    if entry.timestamp > *before {
                        return false;
                    }
                }
                
                true
            })
            .take(query.limit.unwrap_or(100))
            .cloned()
            .collect()
    }
    
    /// Export metrics endpoint handler
    pub fn export_metrics_handler(&self) -> String {
        self.metrics.export_metrics()
    }
}

/// Log search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSearchQuery {
    pub level: Option<String>,
    pub module: Option<String>,
    pub message_contains: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Custom logger implementation
pub struct SamLogger {
    manager: Arc<LoggingManager>,
}

impl SamLogger {
    pub fn new(manager: Arc<LoggingManager>) -> Self {
        SamLogger { manager }
    }
}

impl log::Log for SamLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let level = match self.manager.config.level.to_lowercase().as_str() {
            "trace" => Level::Trace,
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            _ => Level::Info,
        };
        metadata.level() <= level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: record.level().to_string(),
                message: record.args().to_string(),
                module: record.module_path().unwrap_or("unknown").to_string(),
                file: record.file().map(|s| s.to_string()),
                line: record.line(),
                fields: HashMap::new(),
                trace_id: None,
                span_id: None,
            };
            
            let manager = self.manager.clone();
            tokio::spawn(async move {
                manager.log(entry).await;
            });
        }
    }

    fn flush(&self) {}
}

/// Helper macros for structured logging
#[macro_export]
macro_rules! log_with_fields {
    ($level:expr, $msg:expr, $($key:expr => $value:expr),*) => {
        {
            let mut fields = std::collections::HashMap::new();
            $(
                fields.insert($key.to_string(), serde_json::json!($value));
            )*
            
            let entry = LogEntry {
                timestamp: chrono::Utc::now(),
                level: $level.to_string(),
                message: $msg.to_string(),
                module: module_path!().to_string(),
                file: Some(file!().to_string()),
                line: Some(line!()),
                fields,
                trace_id: None,
                span_id: None,
            };
            
            // Log the entry
            log::log!($level, "{}", serde_json::to_string(&entry).unwrap());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.enable_metrics);
        assert!(config.enable_tracing);
    }

    #[test]
    fn test_log_entry() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            module: "test".to_string(),
            file: Some("test.rs".to_string()),
            line: Some(42),
            fields: HashMap::new(),
            trace_id: None,
            span_id: None,
        };
        
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new().unwrap();
        
        // Record some metrics
        collector.record_request("GET", "/api/test", 200, 0.1);
        collector.record_error("test_service", "connection_error");
        collector.set_active_connections("websocket", 5.0);
        
        // Export metrics
        let metrics_text = collector.export_metrics();
        assert!(metrics_text.contains("sam_http_requests_total"));
        assert!(metrics_text.contains("sam_errors_total"));
    }

    #[test]
    fn test_log_search_query() {
        let query = LogSearchQuery {
            level: Some("ERROR".to_string()),
            module: Some("sam::services".to_string()),
            message_contains: Some("failed".to_string()),
            after: None,
            before: None,
            limit: Some(50),
        };
        
        assert_eq!(query.level, Some("ERROR".to_string()));
        assert_eq!(query.limit, Some(50));
    }
}