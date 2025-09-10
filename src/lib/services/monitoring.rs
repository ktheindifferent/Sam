// Comprehensive Monitoring and Observability Module
// Provides metrics collection, health checks, alerting, and distributed tracing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use log::{info, warn, error};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sentry::{protocol::Event, Level};
use std::collections::BTreeMap;

/// Initialize Sentry with enhanced configuration
pub fn init_sentry() -> sentry::ClientInitGuard {
    sentry::init((
        std::env::var("SENTRY_DSN")
            .unwrap_or_else(|_| "http://2f7ca9e40bcc42589eb9c01e0a8696ea@sentry.alpha.opensam.foundation/5".to_string()),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(
                std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string())
                    .into()
            ),
            attach_stacktrace: true,
            send_default_pii: false,
            sample_rate: 1.0,
            traces_sample_rate: 0.3,
            before_send: Some(std::sync::Arc::new(|mut event: sentry::protocol::Event| {
                // Filter out sensitive data before sending
                // Note: Headers are not directly accessible in the current Sentry API
                // We can still filter other sensitive data from the event
                
                // Clear any sensitive extra data
                event.extra.remove("password");
                event.extra.remove("api_key");
                event.extra.remove("token");
                
                Some(event)
            })),
            ..Default::default()
        }
    ))
}

/// Report a service error to Sentry with context
pub fn report_service_error(service: &str, error: &dyn std::fmt::Display, context: Option<BTreeMap<String, String>>) {
    let mut event = Event {
        message: Some(format!("Service error in {}: {}", service, error)),
        level: Level::Error,
        ..Default::default()
    };
    
    if let Some(ctx) = context {
        for (key, value) in ctx {
            event.extra.insert(key, value.into());
        }
    }
    
    event.tags.insert("service".to_string(), service.to_string());
    sentry::capture_event(event);
}

/// Report a critical system error
pub fn report_critical_error(error: &dyn std::fmt::Display, component: &str) {
    sentry::capture_event(Event {
        message: Some(format!("Critical error in {}: {}", component, error)),
        level: Level::Fatal,
        ..Default::default()
    });
}

/// Create a transaction for performance monitoring
pub fn start_transaction(name: &str, operation: &str) -> sentry::TransactionContext {
    sentry::TransactionContext::new(name, operation)
}

/// Add breadcrumb for debugging
pub fn add_breadcrumb(message: String, category: Option<String>) {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        message: Some(message),
        category,
        level: Level::Info,
        ..Default::default()
    });
}

/// Capture a message with a specific level
pub fn capture_message(message: &str, level: Level) {
    sentry::capture_message(message, level);
}

/// Performance monitoring wrapper
pub struct PerformanceSpan {
    transaction: Option<sentry::TransactionOrSpan>,
}

impl PerformanceSpan {
    /// Start a new performance monitoring span
    pub fn new(name: &str, operation: &str) -> Self {
        let ctx = sentry::TransactionContext::new(name, operation);
        let transaction = sentry::start_transaction(ctx);
        Self {
            transaction: Some(transaction.into()),
        }
    }
    
    /// Complete the span
    pub fn finish(mut self) {
        if let Some(transaction) = self.transaction.take() {
            transaction.finish();
        }
    }
}

impl Drop for PerformanceSpan {
    fn drop(&mut self) {
        if let Some(transaction) = self.transaction.take() {
            transaction.finish();
        }
    }
}

// ==================== Metrics Types ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub unit: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub metrics: Vec<Metric>,
    pub timestamp: DateTime<Utc>,
    pub service_name: String,
    pub instance_id: String,
}

// ==================== Health Check System ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_name: String,
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub uptime: Duration,
    pub last_update: DateTime<Utc>,
    pub version: String,
    pub dependencies: HashMap<String, HealthStatus>,
}

// ==================== Metrics Collector ====================

pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    service_name: String,
    instance_id: String,
    aggregation_window: Duration,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    pub fn new(service_name: String) -> Self {
        use nanoid::nanoid;
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            service_name,
            instance_id: nanoid!(),
            aggregation_window: Duration::from_secs(60),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn increment_counter(&self, name: &str, labels: HashMap<String, String>) {
        let mut metrics = self.metrics.write().await;
        let key = format!("{}:{:?}", name, labels);
        
        metrics.entry(key.clone())
            .and_modify(|m| m.value += 1.0)
            .or_insert(Metric {
                name: name.to_string(),
                metric_type: MetricType::Counter,
                value: 1.0,
                labels,
                timestamp: Utc::now(),
                unit: None,
                description: None,
            });
    }
    
    pub async fn set_gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut metrics = self.metrics.write().await;
        let key = format!("{}:{:?}", name, labels);
        
        metrics.insert(key, Metric {
            name: name.to_string(),
            metric_type: MetricType::Gauge,
            value,
            labels,
            timestamp: Utc::now(),
            unit: None,
            description: None,
        });
    }
    
    pub async fn record_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut histograms = self.histograms.write().await;
        let key = format!("{}:{:?}", name, labels);
        
        histograms.entry(key.clone())
            .or_insert_with(Vec::new)
            .push(value);
        
        // Calculate percentiles periodically
        if let Some(values) = histograms.get(&key) {
            if values.len() >= 100 {
                self.calculate_percentiles(name, values, labels).await;
            }
        }
    }
    
    async fn calculate_percentiles(&self, name: &str, values: &[f64], labels: HashMap<String, String>) {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let p50 = percentile(&sorted, 50.0);
        let p95 = percentile(&sorted, 95.0);
        let p99 = percentile(&sorted, 99.0);
        
        let mut metrics = self.metrics.write().await;
        
        for (percentile_val, percentile_name) in [(p50, "p50"), (p95, "p95"), (p99, "p99")] {
            let mut percentile_labels = labels.clone();
            percentile_labels.insert("percentile".to_string(), percentile_name.to_string());
            
            let key = format!("{}_percentile:{:?}", name, percentile_labels);
            metrics.insert(key, Metric {
                name: format!("{}_percentile", name),
                metric_type: MetricType::Summary,
                value: percentile_val,
                labels: percentile_labels,
                timestamp: Utc::now(),
                unit: None,
                description: None,
            });
        }
    }
    
    pub async fn get_snapshot(&self) -> MetricSnapshot {
        let metrics = self.metrics.read().await;
        
        MetricSnapshot {
            metrics: metrics.values().cloned().collect(),
            timestamp: Utc::now(),
            service_name: self.service_name.clone(),
            instance_id: self.instance_id.clone(),
        }
    }
    
    pub async fn export_prometheus(&self) -> String {
        let snapshot = self.get_snapshot().await;
        let mut output = String::new();
        
        for metric in snapshot.metrics {
            let labels = metric.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(",");
            
            let metric_line = if labels.is_empty() {
                format!("{} {}\n", metric.name, metric.value)
            } else {
                format!("{}{{{}}} {}\n", metric.name, labels, metric.value)
            };
            
            output.push_str(&metric_line);
        }
        
        output
    }
}

// ==================== Health Check Manager ====================

pub struct HealthCheckManager {
    checks: Arc<RwLock<Vec<Box<dyn HealthCheckable + Send + Sync>>>>,
    results: Arc<RwLock<HashMap<String, HealthCheck>>>,
    service_name: String,
    start_time: Instant,
}

#[async_trait::async_trait]
pub trait HealthCheckable {
    async fn check(&self) -> Result<HealthCheck>;
    fn name(&self) -> String;
}

impl HealthCheckManager {
    pub fn new(service_name: String) -> Self {
        Self {
            checks: Arc::new(RwLock::new(Vec::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            service_name,
            start_time: Instant::now(),
        }
    }
    
    pub async fn register_check(&self, check: Box<dyn HealthCheckable + Send + Sync>) {
        self.checks.write().await.push(check);
    }
    
    pub async fn run_checks(&self) -> ServiceHealth {
        let checks = self.checks.read().await;
        let mut check_results = Vec::new();
        let mut overall_status = HealthStatus::Healthy;
        
        for check in checks.iter() {
            let start = Instant::now();
            let result = match check.check().await {
                Ok(mut health_check) => {
                    health_check.response_time_ms = start.elapsed().as_millis() as u64;
                    health_check
                }
                Err(e) => HealthCheck {
                    name: check.name(),
                    status: HealthStatus::Unhealthy(e.to_string()),
                    message: Some(format!("Health check failed: {}", e)),
                    last_check: Utc::now(),
                    response_time_ms: start.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                }
            };
            
            // Update overall status
            match &result.status {
                HealthStatus::Unhealthy(_) => overall_status = result.status.clone(),
                HealthStatus::Degraded(_) if overall_status == HealthStatus::Healthy => {
                    overall_status = result.status.clone();
                }
                _ => {}
            }
            
            check_results.push(result.clone());
            self.results.write().await.insert(result.name.clone(), result);
        }
        
        ServiceHealth {
            service_name: self.service_name.clone(),
            overall_status,
            checks: check_results,
            uptime: self.start_time.elapsed(),
            last_update: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: HashMap::new(),
        }
    }
    
    pub async fn get_health(&self) -> ServiceHealth {
        self.run_checks().await
    }
}

// ==================== Distributed Tracing ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub service_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
    pub tags: HashMap<String, String>,
    pub logs: Vec<SpanLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLog {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub fields: HashMap<String, String>,
}

pub struct Tracer {
    spans: Arc<RwLock<HashMap<String, TraceSpan>>>,
    service_name: String,
}

impl Tracer {
    pub fn new(service_name: String) -> Self {
        Self {
            spans: Arc::new(RwLock::new(HashMap::new())),
            service_name,
        }
    }
    
    pub async fn start_span(&self, operation: String, parent_span_id: Option<String>) -> String {
        use nanoid::nanoid;
        
        let span_id = nanoid!();
        let trace_id = if let Some(pid) = parent_span_id.as_ref() {
            if let Some(span) = self.spans.read().await.get(pid) {
                span.trace_id.clone()
            } else {
                nanoid!()
            }
        } else {
            nanoid!()
        };
        
        let span = TraceSpan {
            trace_id,
            span_id: span_id.clone(),
            parent_span_id,
            operation,
            service_name: self.service_name.clone(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Ok,
            tags: HashMap::new(),
            logs: Vec::new(),
        };
        
        self.spans.write().await.insert(span_id.clone(), span);
        span_id
    }
    
    pub async fn end_span(&self, span_id: &str, status: SpanStatus) {
        if let Some(span) = self.spans.write().await.get_mut(span_id) {
            let end_time = Utc::now();
            let duration = end_time.signed_duration_since(span.start_time);
            
            span.end_time = Some(end_time);
            span.duration_ms = Some(duration.num_milliseconds() as u64);
            span.status = status;
        }
    }
    
    pub async fn add_tag(&self, span_id: &str, key: String, value: String) {
        if let Some(span) = self.spans.write().await.get_mut(span_id) {
            span.tags.insert(key, value);
        }
    }
    
    pub async fn add_log(&self, span_id: &str, level: String, message: String) {
        if let Some(span) = self.spans.write().await.get_mut(span_id) {
            span.logs.push(SpanLog {
                timestamp: Utc::now(),
                level,
                message,
                fields: HashMap::new(),
            });
        }
    }
}

// ==================== Alerting System ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub service: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

pub struct AlertManager {
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    handlers: Arc<RwLock<Vec<Box<dyn AlertHandler + Send + Sync>>>>,
}

#[async_trait::async_trait]
pub trait AlertHandler {
    async fn handle(&self, alert: &Alert) -> Result<()>;
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn register_handler(&self, handler: Box<dyn AlertHandler + Send + Sync>) {
        self.handlers.write().await.push(handler);
    }
    
    pub async fn trigger_alert(&self, name: String, severity: AlertSeverity, message: String, service: String) -> Result<String> {
        use nanoid::nanoid;
        
        let alert = Alert {
            id: nanoid!(),
            name,
            severity: severity.clone(),
            message,
            service,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            resolved: false,
            resolved_at: None,
        };
        
        // Store alert
        self.alerts.write().await.insert(alert.id.clone(), alert.clone());
        
        // Notify handlers
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle(&alert).await {
                error!("Alert handler failed: {}", e);
            }
        }
        
        // Log based on severity
        match severity {
            AlertSeverity::Info => info!("Alert: {}", alert.message),
            AlertSeverity::Warning => warn!("Alert: {}", alert.message),
            AlertSeverity::Error => error!("Alert: {}", alert.message),
            AlertSeverity::Critical => error!("CRITICAL Alert: {}", alert.message),
        }
        
        Ok(alert.id)
    }
    
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<()> {
        if let Some(alert) = self.alerts.write().await.get_mut(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            info!("Alert {} resolved", alert_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Alert not found"))
        }
    }
    
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerts.read().await
            .values()
            .filter(|a| !a.resolved)
            .cloned()
            .collect()
    }
}

// ==================== Helper Functions ====================

fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    let index = (percentile / 100.0 * sorted_data.len() as f64) as usize;
    sorted_data[index.min(sorted_data.len() - 1)]
}

// ==================== Performance Monitor ====================

pub struct PerformanceMonitor {
    metrics: Arc<MetricsCollector>,
}

impl PerformanceMonitor {
    pub fn new(service_name: String) -> Self {
        Self {
            metrics: Arc::new(MetricsCollector::new(service_name)),
        }
    }
    
    pub async fn measure_operation<F, Fut, T>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation_name.to_string());
        
        // Increment operation counter
        self.metrics.increment_counter("operations_total", labels.clone()).await;
        
        match operation().await {
            Ok(result) => {
                let duration = start.elapsed().as_millis() as f64;
                
                // Record success
                labels.insert("status".to_string(), "success".to_string());
                self.metrics.increment_counter("operations_success", labels.clone()).await;
                
                // Record duration
                self.metrics.record_histogram("operation_duration_ms", duration, labels).await;
                
                Ok(result)
            }
            Err(e) => {
                // Record failure
                labels.insert("status".to_string(), "failure".to_string());
                labels.insert("error".to_string(), e.to_string());
                self.metrics.increment_counter("operations_failed", labels).await;
                
                Err(e)
            }
        }
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new("test_service".to_string());
        
        // Test counter
        let mut labels = HashMap::new();
        labels.insert("endpoint".to_string(), "/api/test".to_string());
        collector.increment_counter("requests", labels.clone()).await;
        collector.increment_counter("requests", labels.clone()).await;
        
        // Test gauge
        collector.set_gauge("memory_usage", 1024.0, HashMap::new()).await;
        
        let snapshot = collector.get_snapshot().await;
        assert!(!snapshot.metrics.is_empty());
        assert_eq!(snapshot.service_name, "test_service");
    }
    
    #[tokio::test]
    async fn test_health_check_manager() {
        struct TestCheck;
        
        #[async_trait::async_trait]
        impl HealthCheckable for TestCheck {
            async fn check(&self) -> Result<HealthCheck> {
                Ok(HealthCheck {
                    name: "test_check".to_string(),
                    status: HealthStatus::Healthy,
                    message: None,
                    last_check: Utc::now(),
                    response_time_ms: 0,
                    metadata: HashMap::new(),
                })
            }
            
            fn name(&self) -> String {
                "test_check".to_string()
            }
        }
        
        let manager = HealthCheckManager::new("test_service".to_string());
        manager.register_check(Box::new(TestCheck)).await;
        
        let health = manager.get_health().await;
        assert_eq!(health.overall_status, HealthStatus::Healthy);
        assert_eq!(health.checks.len(), 1);
    }
    
    #[tokio::test]
    async fn test_tracer() {
        let tracer = Tracer::new("test_service".to_string());
        
        let span_id = tracer.start_span("test_operation".to_string(), None).await;
        tracer.add_tag(&span_id, "user_id".to_string(), "123".to_string()).await;
        tracer.add_log(&span_id, "info".to_string(), "Operation started".to_string()).await;
        tracer.end_span(&span_id, SpanStatus::Ok).await;
        
        let spans = tracer.spans.read().await;
        assert!(spans.contains_key(&span_id));
        
        let span = &spans[&span_id];
        assert_eq!(span.operation, "test_operation");
        assert!(span.duration_ms.is_some());
    }
}