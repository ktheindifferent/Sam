//! Error reporting and telemetry

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::{AgentError, ErrorContext, ErrorKind};

/// Error reporter for telemetry and monitoring
pub struct ErrorReporter {
    handlers: Vec<Box<dyn ErrorHandler>>,
    filters: Vec<Box<dyn ErrorFilter>>,
    enrichers: Vec<Box<dyn ErrorEnricher>>,
}

/// Trait for handling errors
pub trait ErrorHandler: Send + Sync {
    /// Handle an error report
    fn handle(&self, report: &ErrorReport);

    /// Handler name
    fn name(&self) -> &str;
}

/// Trait for filtering errors
pub trait ErrorFilter: Send + Sync {
    /// Check if error should be reported
    fn should_report(&self, error: &AgentError) -> bool;

    /// Filter name
    fn name(&self) -> &str;
}

/// Trait for enriching error reports
pub trait ErrorEnricher: Send + Sync {
    /// Enrich error report with additional data
    fn enrich(&self, report: &mut ErrorReport);

    /// Enricher name
    fn name(&self) -> &str;
}

/// Error report with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: ErrorSeverity,
    pub kind: String,
    pub message: String,
    pub context: ErrorContext,
    pub stack_trace: Vec<String>,
    pub tags: HashMap<String, String>,
    pub metrics: ErrorMetrics,
    pub environment: EnvironmentInfo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub occurrence_count: u32,
    pub first_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub affected_users: u32,
    pub impact_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub hostname: String,
    pub process_id: u32,
    pub thread_id: String,
    pub version: String,
    pub environment: String, // dev, staging, prod
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self {
            hostname: "localhost".to_string(), // TODO: Get actual hostname
            process_id: std::process::id(),
            thread_id: format!("{:?}", std::thread::current().id()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENV").unwrap_or_else(|_| "development".to_string()),
        }
    }
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self {
            handlers: vec![Box::new(LogHandler::new()), Box::new(MetricsHandler::new())],
            filters: vec![Box::new(DefaultFilter::new())],
            enrichers: vec![Box::new(DefaultEnricher::new())],
        }
    }

    /// Report an error
    pub fn report(&self, error: AgentError) {
        // Check filters
        for filter in &self.filters {
            if !filter.should_report(&error) {
                debug!("Error filtered by {}: {}", filter.name(), error);
                return;
            }
        }

        // Create report
        let mut report = self.create_report(error);

        // Enrich report
        for enricher in &self.enrichers {
            enricher.enrich(&mut report);
        }

        // Send to handlers
        for handler in &self.handlers {
            handler.handle(&report);
        }
    }

    /// Create error report
    fn create_report(&self, error: AgentError) -> ErrorReport {
        let severity = self.determine_severity(&error);
        let kind = self.determine_kind(&error);
        let context = self.extract_context(&error);

        ErrorReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            severity,
            kind,
            message: error.to_string(),
            context,
            stack_trace: self.capture_stack_trace(),
            tags: HashMap::new(),
            metrics: ErrorMetrics::default(),
            environment: EnvironmentInfo::default(),
        }
    }

    fn determine_severity(&self, error: &AgentError) -> ErrorSeverity {
        match error {
            AgentError::Core { kind, .. } => match kind {
                ErrorKind::Internal | ErrorKind::ServiceUnavailable => ErrorSeverity::Critical,
                ErrorKind::Timeout | ErrorKind::RateLimited => ErrorSeverity::Warning,
                _ => ErrorSeverity::Error,
            },
            AgentError::Configuration(_) => ErrorSeverity::Critical,
            AgentError::Resource(_) => ErrorSeverity::Error,
            _ => ErrorSeverity::Warning,
        }
    }

    fn determine_kind(&self, error: &AgentError) -> String {
        match error {
            AgentError::Provider(_) => "Provider".to_string(),
            AgentError::Execution(_) => "Execution".to_string(),
            AgentError::Analysis(_) => "Analysis".to_string(),
            AgentError::Configuration(_) => "Configuration".to_string(),
            AgentError::Resource(_) => "Resource".to_string(),
            AgentError::Core { kind, .. } => format!("Core::{:?}", kind),
            AgentError::Other(_) => "Other".to_string(),
        }
    }

    fn extract_context(&self, error: &AgentError) -> ErrorContext {
        match error {
            AgentError::Core { context, .. } => context.clone(),
            _ => ErrorContext::default(),
        }
    }

    fn capture_stack_trace(&self) -> Vec<String> {
        // In production, use backtrace crate
        vec!["Stack trace not available".to_string()]
    }

    /// Add custom handler
    pub fn add_handler(&mut self, handler: Box<dyn ErrorHandler>) {
        self.handlers.push(handler);
    }

    /// Add custom filter
    pub fn add_filter(&mut self, filter: Box<dyn ErrorFilter>) {
        self.filters.push(filter);
    }

    /// Add custom enricher
    pub fn add_enricher(&mut self, enricher: Box<dyn ErrorEnricher>) {
        self.enrichers.push(enricher);
    }
}

/// Log handler for error reporting
struct LogHandler {
    min_severity: ErrorSeverity,
}

impl LogHandler {
    fn new() -> Self {
        Self {
            min_severity: ErrorSeverity::Warning,
        }
    }
}

impl ErrorHandler for LogHandler {
    fn handle(&self, report: &ErrorReport) {
        let log_message = format!(
            "[{}] {} - {} ({})",
            report.severity, report.kind, report.message, report.id
        );

        match report.severity {
            ErrorSeverity::Debug => debug!("{}", log_message),
            ErrorSeverity::Info => info!("{}", log_message),
            ErrorSeverity::Warning => warn!("{}", log_message),
            ErrorSeverity::Error => error!("{}", log_message),
            ErrorSeverity::Critical => error!("CRITICAL: {}", log_message),
        }

        // Log detailed context for errors and above
        if matches!(
            report.severity,
            ErrorSeverity::Error | ErrorSeverity::Critical
        ) {
            error!("Error context:\n{}", report.context.format_detailed());
        }
    }

    fn name(&self) -> &str {
        "LogHandler"
    }
}

/// Metrics handler for error reporting
struct MetricsHandler {
    metrics: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ErrorMetrics>>>,
}

impl MetricsHandler {
    fn new() -> Self {
        Self {
            metrics: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl ErrorHandler for MetricsHandler {
    fn handle(&self, report: &ErrorReport) {
        let metrics = self.metrics.clone();
        let error_key = format!("{}:{}", report.kind, report.message);
        let timestamp = report.timestamp; // Copy timestamp before moving into async block

        tokio::spawn(async move {
            let mut metrics_map = metrics.write().await;
            let entry = metrics_map.entry(error_key).or_default();

            entry.occurrence_count += 1;
            entry.last_seen = Some(timestamp);
            if entry.first_seen.is_none() {
                entry.first_seen = Some(timestamp);
            }
        });
    }

    fn name(&self) -> &str {
        "MetricsHandler"
    }
}

/// Default error filter
struct DefaultFilter {
    min_severity: ErrorSeverity,
}

impl DefaultFilter {
    fn new() -> Self {
        Self {
            min_severity: ErrorSeverity::Info,
        }
    }
}

impl ErrorFilter for DefaultFilter {
    fn should_report(&self, _error: &AgentError) -> bool {
        // In production, filter based on severity, frequency, etc.
        true
    }

    fn name(&self) -> &str {
        "DefaultFilter"
    }
}

/// Default error enricher
struct DefaultEnricher;

impl DefaultEnricher {
    fn new() -> Self {
        Self
    }
}

impl ErrorEnricher for DefaultEnricher {
    fn enrich(&self, report: &mut ErrorReport) {
        // Add default tags
        report
            .tags
            .insert("service".to_string(), "coding-agent".to_string());
        report
            .tags
            .insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());

        // Add system info
        // TODO: Add system metrics using sysinfo crate
        // For now, just add placeholder values
        report
            .tags
            .insert("load_avg".to_string(), "0.0".to_string());

        report
            .tags
            .insert("memory_usage".to_string(), "0%".to_string());
    }

    fn name(&self) -> &str {
        "DefaultEnricher"
    }
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRIT"),
        }
    }
}
