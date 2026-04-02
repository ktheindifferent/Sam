use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::{Level, Metadata, Record};
use prometheus::{
    register_gauge_vec, register_histogram_vec, GaugeVec, HistogramVec, TextEncoder, Encoder,
    register_int_counter_vec, IntCounterVec,
};
use tracing::{error, warn, info, debug, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

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

/// Structured log entry with correlation IDs
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
    pub correlation_id: Option<String>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub service: String,
    pub environment: String,
    pub version: String,
}

/// Enhanced Metrics collector with business metrics
pub struct MetricsCollector {
    // HTTP Metrics
    request_counter: IntCounterVec,
    response_time_histogram: HistogramVec,
    request_size_histogram: HistogramVec,
    response_size_histogram: HistogramVec,
    
    // Connection Metrics
    active_connections_gauge: GaugeVec,
    connection_errors_counter: IntCounterVec,
    
    // Error Metrics
    error_counter: IntCounterVec,
    error_rate_gauge: GaugeVec,
    
    // System Metrics
    cpu_usage_gauge: GaugeVec,
    memory_usage_gauge: GaugeVec,
    disk_usage_gauge: GaugeVec,
    thread_count_gauge: GaugeVec,
    
    // Business Metrics
    active_users_gauge: GaugeVec,
    operations_per_second_gauge: GaugeVec,
    
    // Service-specific Metrics
    lifx_operations_counter: IntCounterVec,
    spotify_operations_counter: IntCounterVec,
    media_operations_counter: IntCounterVec,
    p2p_messages_counter: IntCounterVec,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let request_counter = register_int_counter_vec!(
            "sam_http_requests_total",
            "Total number of HTTP requests",
            &["method", "endpoint", "status", "service"]
        )?;
        
        let response_time_histogram = register_histogram_vec!(
            "sam_http_response_time_seconds",
            "HTTP response time in seconds",
            &["method", "endpoint", "service"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;
        
        let request_size_histogram = register_histogram_vec!(
            "sam_http_request_size_bytes",
            "HTTP request size in bytes",
            &["method", "endpoint"],
            vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
        )?;
        
        let response_size_histogram = register_histogram_vec!(
            "sam_http_response_size_bytes",
            "HTTP response size in bytes",
            &["method", "endpoint"],
            vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
        )?;
        
        let active_connections_gauge = register_gauge_vec!(
            "sam_active_connections",
            "Number of active connections",
            &["type", "service"]
        )?;
        
        let connection_errors_counter = register_int_counter_vec!(
            "sam_connection_errors_total",
            "Total number of connection errors",
            &["type", "reason"]
        )?;
        
        let error_counter = register_int_counter_vec!(
            "sam_errors_total",
            "Total number of errors",
            &["service", "error_type", "severity"]
        )?;
        
        let error_rate_gauge = register_gauge_vec!(
            "sam_error_rate_per_minute",
            "Error rate per minute",
            &["service"]
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
        
        let thread_count_gauge = register_gauge_vec!(
            "sam_thread_count",
            "Number of threads",
            &["state"]
        )?;
        
        // Business metrics
        let active_users_gauge = register_gauge_vec!(
            "sam_active_users",
            "Number of active users",
            &["type"]
        )?;
        
        let operations_per_second_gauge = register_gauge_vec!(
            "sam_operations_per_second",
            "Operations per second",
            &["operation_type"]
        )?;
        
        // Service-specific metrics
        let lifx_operations_counter = register_int_counter_vec!(
            "sam_lifx_operations_total",
            "Total LIFX operations",
            &["operation", "status"]
        )?;
        
        let spotify_operations_counter = register_int_counter_vec!(
            "sam_spotify_operations_total",
            "Total Spotify operations",
            &["operation", "status"]
        )?;
        
        let media_operations_counter = register_int_counter_vec!(
            "sam_media_operations_total",
            "Total media operations",
            &["operation", "media_type", "status"]
        )?;
        
        let p2p_messages_counter = register_int_counter_vec!(
            "sam_p2p_messages_total",
            "Total P2P messages",
            &["direction", "message_type"]
        )?;
        
        Ok(MetricsCollector {
            request_counter,
            response_time_histogram,
            request_size_histogram,
            response_size_histogram,
            active_connections_gauge,
            connection_errors_counter,
            error_counter,
            error_rate_gauge,
            cpu_usage_gauge,
            memory_usage_gauge,
            disk_usage_gauge,
            thread_count_gauge,
            active_users_gauge,
            operations_per_second_gauge,
            lifx_operations_counter,
            spotify_operations_counter,
            media_operations_counter,
            p2p_messages_counter,
        })
    }
    
    /// Record an HTTP request with enhanced metrics
    pub fn record_request(&self, method: &str, endpoint: &str, status: u16, duration: f64, service: &str, request_size: f64, response_size: f64) {
        self.request_counter
            .with_label_values(&[method, endpoint, &status.to_string(), service])
            .inc();
        
        self.response_time_histogram
            .with_label_values(&[method, endpoint, service])
            .observe(duration);
        
        self.request_size_histogram
            .with_label_values(&[method, endpoint])
            .observe(request_size);
        
        self.response_size_histogram
            .with_label_values(&[method, endpoint])
            .observe(response_size);
    }
    
    /// Record an error with severity
    pub fn record_error(&self, service: &str, error_type: &str, severity: &str) {
        self.error_counter
            .with_label_values(&[service, error_type, severity])
            .inc();
    }
    
    /// Record LIFX operation
    pub fn record_lifx_operation(&self, operation: &str, status: &str) {
        self.lifx_operations_counter
            .with_label_values(&[operation, status])
            .inc();
    }
    
    /// Record Spotify operation
    pub fn record_spotify_operation(&self, operation: &str, status: &str) {
        self.spotify_operations_counter
            .with_label_values(&[operation, status])
            .inc();
    }
    
    /// Record media operation
    pub fn record_media_operation(&self, operation: &str, media_type: &str, status: &str) {
        self.media_operations_counter
            .with_label_values(&[operation, media_type, status])
            .inc();
    }
    
    /// Record P2P message
    pub fn record_p2p_message(&self, direction: &str, message_type: &str) {
        self.p2p_messages_counter
            .with_label_values(&[direction, message_type])
            .inc();
    }
    
    /// Update business metrics
    pub fn update_business_metrics(&self, active_users: f64, ops_per_second: f64) {
        self.active_users_gauge
            .with_label_values(&["concurrent"])
            .set(active_users);
        
        self.operations_per_second_gauge
            .with_label_values(&["total"])
            .set(ops_per_second);
    }
    
    /// Update active connections with service context
    pub fn set_active_connections(&self, conn_type: &str, service: &str, count: f64) {
        self.active_connections_gauge
            .with_label_values(&[conn_type, service])
            .set(count);
    }
    
    /// Record connection error
    pub fn record_connection_error(&self, conn_type: &str, reason: &str) {
        self.connection_errors_counter
            .with_label_values(&[conn_type, reason])
            .inc();
    }
    
    /// Update system metrics
    pub fn update_system_metrics(&self) {
        use sysinfo::System;
        
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
        
        // Thread count
        self.thread_count_gauge
            .with_label_values(&["active"])
            .set(sys.processes().len() as f64);
        
        // Disk usage
        let disks = sysinfo::Disks::new_with_refreshed_list();
        for disk in disks.list() {
            let mount_point = disk.mount_point().to_string_lossy();
            self.disk_usage_gauge
                .with_label_values(&[&mount_point])
                .set((disk.total_space() - disk.available_space()) as f64);
        }
    }
    
    /// Update error rates
    pub fn update_error_rate(&self, service: &str, rate: f64) {
        self.error_rate_gauge
            .with_label_values(&[service])
            .set(rate);
    }
    
    /// Export metrics in Prometheus format
    pub fn export_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)
            .map_err(|e| format!("Failed to encode metrics: {}", e).into())?;
        String::from_utf8(buffer)
            .map_err(|e| format!("Invalid UTF-8 in metrics buffer: {}", e).into())
    }
}

/// Enhanced Logging and monitoring manager with correlation tracking
pub struct LoggingManager {
    config: LogConfig,
    metrics: Arc<MetricsCollector>,
    log_buffer: Arc<RwLock<Vec<LogEntry>>>,
    // tracer: Option<Box<dyn OtelTracer<Span = opentelemetry_sdk::trace::Span>>>, // Disabled for now due to dyn compatibility issues
    error_tracker: Arc<ErrorTracker>,
    correlation_store: Arc<RwLock<HashMap<String, CorrelationContext>>>,
}

/// Correlation context for request tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationContext {
    pub correlation_id: String,
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Error tracking and categorization
pub struct ErrorTracker {
    errors: Arc<RwLock<Vec<TrackedError>>>,
    error_patterns: Arc<RwLock<HashMap<String, ErrorPattern>>>,
    alerts_triggered: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedError {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub error_type: String,
    pub category: ErrorCategory,
    pub message: String,
    pub stack_trace: Option<String>,
    pub service: String,
    pub correlation_id: Option<String>,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    Network,
    Database,
    Authentication,
    Authorization,
    Validation,
    BusinessLogic,
    System,
    External,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl LoggingManager {
    /// Initialize the enhanced logging system with tracing
    pub async fn init(config: LogConfig) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        // Initialize structured logging with tracing
        Self::init_structured_logging(&config)?;
        
        // Initialize metrics
        let metrics = Arc::new(MetricsCollector::new()?);
        
        // Initialize OpenTelemetry tracing (disabled for now)
        // let tracer = if config.enable_tracing {
        //     Some(Self::init_otel_tracing(&config)?)
        // } else {
        //     None
        // };
        
        // Initialize error tracker
        let error_tracker = Arc::new(ErrorTracker::new());
        
        let manager = Arc::new(LoggingManager {
            config,
            metrics,
            log_buffer: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            // tracer, // Disabled for now
            error_tracker,
            correlation_store: Arc::new(RwLock::new(HashMap::new())),
        });
        
        // Start background tasks
        manager.start_background_tasks();
        
        Ok(manager)
    }
    
    /// Initialize structured logging with tracing
    fn init_structured_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&config.level));
        
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);
        
        match config.format {
            LogFormat::Json => {
                let json_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true);
                Registry::default().with(env_filter).with(json_layer).init();
            },
            LogFormat::Text | LogFormat::Compact => {
                Registry::default().with(env_filter).with(fmt_layer).init();
            }
        }
        
        // TODO: Setup file output if configured (requires restructuring subscriber initialization)
        
        Ok(())
    }
    
    /// Setup file logging with rotation
    #[allow(dead_code)]
    fn setup_file_logging(
        path: &str,
        _max_size_mb: u64,
        _max_files: u32,
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
    
    /// Initialize OpenTelemetry distributed tracing (disabled for now)
    /*
    fn init_otel_tracing(config: &LogConfig) -> Result<Box<dyn OtelTracer<Span = opentelemetry_sdk::trace::Span>>, Box<dyn std::error::Error>> {
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
                        KeyValue::new("service.environment", "production"),
                        KeyValue::new("telemetry.sdk.name", "opentelemetry"),
                        KeyValue::new("telemetry.sdk.language", "rust"),
                    ]))
                    .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;
        
        // Set global tracer provider
        global::set_tracer_provider(tracer.provider().unwrap().clone());
        
        Ok(Box::new(tracer))
    }
    */
    
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
    
    /// Log a structured message with correlation ID
    pub async fn log_with_context(&self, mut entry: LogEntry, correlation_id: Option<String>) {
        // Add correlation context
        if let Some(corr_id) = correlation_id {
            entry.correlation_id = Some(corr_id.clone());
            
            if let Some(context) = self.get_correlation_context(&corr_id).await {
                entry.request_id = Some(context.request_id);
                entry.user_id = context.user_id;
            }
        }
        
        // Add service metadata
        entry.service = "sam".to_string();
        entry.environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());
        entry.version = env!("CARGO_PKG_VERSION").to_string();
        
        // Track errors
        if entry.level == "ERROR" || entry.level == "CRITICAL" {
            self.error_tracker.track_error(&entry).await;
        }
        
        // Add to buffer
        let mut buffer = self.log_buffer.write().await;
        if buffer.len() >= 10000 {
            buffer.drain(0..1000); // Remove oldest entries if buffer is full
        }
        buffer.push(entry.clone());
        
        // Use tracing for output (without dynamic target)
        match entry.level.as_str() {
            "TRACE" => trace!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
            "DEBUG" => debug!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
            "INFO" => info!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
            "WARN" => warn!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
            "ERROR" => error!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
            _ => info!(module = %entry.module, correlation_id = ?entry.correlation_id, "{}", entry.message),
        }
    }
    
    /// Create a new correlation context for request tracking
    pub async fn create_correlation_context(&self, user_id: Option<String>) -> String {
        use nanoid::nanoid;
        let correlation_id = nanoid!();
        let request_id = nanoid!();
        
        let context = CorrelationContext {
            correlation_id: correlation_id.clone(),
            request_id,
            user_id,
            session_id: None,
            parent_span_id: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };
        
        self.correlation_store.write().await.insert(correlation_id.clone(), context);
        correlation_id
    }
    
    /// Get correlation context
    pub async fn get_correlation_context(&self, correlation_id: &str) -> Option<CorrelationContext> {
        self.correlation_store.read().await.get(correlation_id).cloned()
    }
    
    /// Create a new span for distributed tracing (disabled for now)
    pub fn create_span(&self, _name: &str, _correlation_id: Option<String>) -> Option<()> {
        // Disabled for now due to dyn compatibility issues
        None
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
                correlation_id: None,
                request_id: None,
                user_id: None,
                service: "sam".to_string(),
                environment: "production".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            
            let manager = self.manager.clone();
            tokio::spawn(async move {
                manager.log_with_context(entry, None).await;
            });
        }
    }

    fn flush(&self) {}
}

impl Default for ErrorTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorTracker {
    pub fn new() -> Self {
        ErrorTracker {
            errors: Arc::new(RwLock::new(Vec::new())),
            error_patterns: Arc::new(RwLock::new(HashMap::new())),
            alerts_triggered: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn track_error(&self, entry: &LogEntry) {
        use nanoid::nanoid;
        
        let category = Self::categorize_error(&entry.message);
        let error = TrackedError {
            id: nanoid!(),
            timestamp: entry.timestamp,
            error_type: entry.fields.get("error_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            category,
            message: entry.message.clone(),
            stack_trace: entry.fields.get("stack_trace")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            service: entry.service.clone(),
            correlation_id: entry.correlation_id.clone(),
            user_id: entry.user_id.clone(),
            metadata: HashMap::new(),
        };
        
        // Store error
        let mut errors = self.errors.write().await;
        errors.push(error.clone());
        
        // Keep only last 10000 errors
        if errors.len() > 10000 {
            errors.drain(0..1000);
        }
        
        // Update error patterns
        self.update_error_patterns(&error).await;
        
        // Check if alert needs to be triggered
        self.check_alert_conditions(&error).await;
    }
    
    fn categorize_error(message: &str) -> ErrorCategory {
        let message_lower = message.to_lowercase();
        
        if message_lower.contains("connection") || message_lower.contains("timeout") || message_lower.contains("network") {
            ErrorCategory::Network
        } else if message_lower.contains("database") || message_lower.contains("sql") || message_lower.contains("postgres") {
            ErrorCategory::Database
        } else if message_lower.contains("auth") || message_lower.contains("token") || message_lower.contains("credential") {
            ErrorCategory::Authentication
        } else if message_lower.contains("permission") || message_lower.contains("forbidden") || message_lower.contains("unauthorized") {
            ErrorCategory::Authorization
        } else if message_lower.contains("validation") || message_lower.contains("invalid") || message_lower.contains("required") {
            ErrorCategory::Validation
        } else if message_lower.contains("business") || message_lower.contains("workflow") || message_lower.contains("process") {
            ErrorCategory::BusinessLogic
        } else if message_lower.contains("system") || message_lower.contains("memory") || message_lower.contains("disk") {
            ErrorCategory::System
        } else if message_lower.contains("external") || message_lower.contains("api") || message_lower.contains("third-party") {
            ErrorCategory::External
        } else {
            ErrorCategory::Unknown
        }
    }
    
    async fn update_error_patterns(&self, error: &TrackedError) {
        let mut patterns = self.error_patterns.write().await;
        let pattern_key = format!("{}:{:?}", error.error_type, error.category);
        
        patterns.entry(pattern_key.clone())
            .and_modify(|p| {
                p.count += 1;
                p.last_seen = error.timestamp;
                
                // Escalate severity based on frequency
                if p.count > 100 {
                    p.severity = ErrorSeverity::Critical;
                } else if p.count > 50 {
                    p.severity = ErrorSeverity::High;
                } else if p.count > 10 {
                    p.severity = ErrorSeverity::Medium;
                }
            })
            .or_insert(ErrorPattern {
                pattern: pattern_key,
                count: 1,
                first_seen: error.timestamp,
                last_seen: error.timestamp,
                severity: ErrorSeverity::Low,
            });
    }
    
    async fn check_alert_conditions(&self, error: &TrackedError) {
        let patterns = self.error_patterns.read().await;
        let pattern_key = format!("{}:{:?}", error.error_type, error.category);
        
        if let Some(pattern) = patterns.get(&pattern_key) {
            // Alert on critical errors or high frequency
            if matches!(pattern.severity, ErrorSeverity::Critical) || pattern.count > 50 {
                let mut alerts = self.alerts_triggered.write().await;
                let last_alert = alerts.get(&pattern_key);
                
                // Only alert once per hour for the same pattern
                let should_alert = last_alert.is_none_or(|last| {
                    error.timestamp.signed_duration_since(*last).num_seconds() > 3600
                });
                
                if should_alert {
                    alerts.insert(pattern_key.clone(), error.timestamp);
                    error!("ALERT: Error pattern {} has occurred {} times. Severity: {:?}", 
                           pattern_key, pattern.count, pattern.severity);
                }
            }
        }
    }
    
    pub async fn get_error_summary(&self) -> HashMap<String, u64> {
        let patterns = self.error_patterns.read().await;
        patterns.iter()
            .map(|(k, v)| (k.clone(), v.count))
            .collect()
    }
    
    pub async fn get_recent_errors(&self, limit: usize) -> Vec<TrackedError> {
        let errors = self.errors.read().await;
        let start = if errors.len() > limit {
            errors.len() - limit
        } else {
            0
        };
        errors[start..].to_vec()
    }
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
                correlation_id: None,
                request_id: None,
                user_id: None,
                service: "sam".to_string(),
                environment: "production".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            
            // Log the entry with proper error handling
            match serde_json::to_string(&entry) {
                Ok(json_str) => log::log!($level, "{}", json_str),
                Err(e) => log::log!($level, "Failed to serialize log entry: {} (original message: {})", e, $msg),
            }
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
            correlation_id: None,
            request_id: None,
            user_id: None,
            service: "sam".to_string(),
            environment: "test".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new().unwrap();
        
        // Record some metrics
        collector.record_request("GET", "/api/test", 200, 0.1, "sam", 1024.0, 2048.0);
        collector.record_error("test_service", "connection_error", "high");
        collector.set_active_connections("websocket", "sam", 5.0);
        
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