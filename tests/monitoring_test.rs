// Integration tests for monitoring and observability

use sam::sam::logging::{LoggingManager, LogConfig, LogEntry, LogFormat, LogOutput};
use sam::sam::services::monitoring::{
    MetricsCollector, HealthCheckManager, Tracer, AlertManager,
    HealthCheckable, AlertHandler, AlertSeverity, HealthCheck, HealthStatus,
    SpanStatus, PerformanceMonitor,
};
use std::collections::HashMap;
use chrono::Utc;
use async_trait::async_trait;

#[tokio::test]
async fn test_structured_logging() {
    let config = LogConfig::default();
    let manager = LoggingManager::init(config).await.unwrap();
    
    // Create correlation context
    let correlation_id = manager.create_correlation_context(Some("user123".to_string())).await;
    
    // Log with context
    let entry = LogEntry {
        timestamp: Utc::now(),
        level: "INFO".to_string(),
        message: "Test log message".to_string(),
        module: "test".to_string(),
        file: Some("test.rs".to_string()),
        line: Some(42),
        fields: HashMap::new(),
        trace_id: None,
        span_id: None,
        correlation_id: Some(correlation_id.clone()),
        request_id: None,
        user_id: Some("user123".to_string()),
        service: "test_service".to_string(),
        environment: "test".to_string(),
        version: "1.0.0".to_string(),
    };
    
    manager.log_with_context(entry, Some(correlation_id.clone())).await;
    
    // Verify correlation context
    let context = manager.get_correlation_context(&correlation_id).await;
    assert!(context.is_some());
    assert_eq!(context.unwrap().user_id, Some("user123".to_string()));
}

#[tokio::test]
async fn test_metrics_collection() {
    let collector = MetricsCollector::new("test_service".to_string());
    
    // Record various metrics
    let mut labels = HashMap::new();
    labels.insert("endpoint".to_string(), "/api/test".to_string());
    
    collector.increment_counter("test_requests", labels.clone()).await;
    collector.set_gauge("test_gauge", 42.0, labels.clone()).await;
    collector.record_histogram("test_latency", 100.0, labels.clone()).await;
    
    // Record service-specific metrics
    collector.record_lifx_operation("set_color", "success").await;
    collector.record_spotify_operation("play_track", "success").await;
    collector.record_media_operation("transcode", "video", "success").await;
    collector.record_p2p_message("inbound", "data").await;
    
    // Get snapshot
    let snapshot = collector.get_snapshot().await;
    assert!(!snapshot.metrics.is_empty());
    assert_eq!(snapshot.service_name, "test_service");
    
    // Export Prometheus format
    let prometheus_output = collector.export_prometheus().await;
    assert!(prometheus_output.contains("test_requests"));
}

#[tokio::test]
async fn test_health_checks() {
    struct TestHealthCheck {
        name: String,
        healthy: bool,
    }
    
    #[async_trait]
    impl HealthCheckable for TestHealthCheck {
        async fn check(&self) -> anyhow::Result<HealthCheck> {
            Ok(HealthCheck {
                name: self.name.clone(),
                status: if self.healthy { 
                    HealthStatus::Healthy 
                } else { 
                    HealthStatus::Unhealthy("Test failure".to_string())
                },
                message: None,
                last_check: Utc::now(),
                response_time_ms: 10,
                metadata: HashMap::new(),
            })
        }
        
        fn name(&self) -> String {
            self.name.clone()
        }
    }
    
    let manager = HealthCheckManager::new("test_service".to_string());
    
    // Register checks
    manager.register_check(Box::new(TestHealthCheck {
        name: "database".to_string(),
        healthy: true,
    })).await;
    
    manager.register_check(Box::new(TestHealthCheck {
        name: "redis".to_string(),
        healthy: false,
    })).await;
    
    // Run checks
    let health = manager.get_health().await;
    
    assert_eq!(health.service_name, "test_service");
    assert_eq!(health.checks.len(), 2);
    assert!(matches!(health.overall_status, HealthStatus::Unhealthy(_)));
}

#[tokio::test]
async fn test_distributed_tracing() {
    let tracer = Tracer::new("test_service".to_string());
    
    // Start root span
    let root_span = tracer.start_span("test_operation".to_string(), None).await;
    
    // Add tags and logs
    tracer.add_tag(&root_span, "user_id".to_string(), "123".to_string()).await;
    tracer.add_log(&root_span, "INFO".to_string(), "Starting operation".to_string()).await;
    
    // Start child span
    let child_span = tracer.start_span("child_operation".to_string(), Some(root_span.clone())).await;
    
    // End spans
    tracer.end_span(&child_span, SpanStatus::Ok).await;
    tracer.end_span(&root_span, SpanStatus::Ok).await;
    
    // Verify spans have proper relationships
    let spans = tracer.spans.read().await;
    assert_eq!(spans.len(), 2);
    
    let child = spans.get(&child_span).unwrap();
    assert_eq!(child.parent_span_id, Some(root_span.clone()));
}

#[tokio::test]
async fn test_error_tracking() {
    let config = LogConfig::default();
    let manager = LoggingManager::init(config).await.unwrap();
    
    // Log various error types
    let error_messages = vec![
        ("Connection timeout", "ERROR"),
        ("Database query failed", "ERROR"),
        ("Authentication failed", "ERROR"),
        ("Invalid input validation", "ERROR"),
        ("System memory low", "CRITICAL"),
    ];
    
    for (message, level) in error_messages {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
            module: "test".to_string(),
            file: None,
            line: None,
            fields: HashMap::new(),
            trace_id: None,
            span_id: None,
            correlation_id: None,
            request_id: None,
            user_id: None,
            service: "test_service".to_string(),
            environment: "test".to_string(),
            version: "1.0.0".to_string(),
        };
        
        manager.log_with_context(entry, None).await;
    }
    
    // Get error summary
    let summary = manager.error_tracker.get_error_summary().await;
    assert!(!summary.is_empty());
    
    // Get recent errors
    let recent = manager.error_tracker.get_recent_errors(10).await;
    assert_eq!(recent.len(), 5);
}

#[tokio::test]
async fn test_alerting() {
    struct TestAlertHandler {
        alerts_received: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }
    
    #[async_trait]
    impl AlertHandler for TestAlertHandler {
        async fn handle(&self, alert: &sam::sam::services::monitoring::Alert) -> anyhow::Result<()> {
            self.alerts_received.lock().unwrap().push(alert.name.clone());
            Ok(())
        }
    }
    
    let manager = AlertManager::new();
    let alerts_received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    
    // Register handler
    manager.register_handler(Box::new(TestAlertHandler {
        alerts_received: alerts_received.clone(),
    })).await;
    
    // Trigger alerts
    let alert_id = manager.trigger_alert(
        "high_error_rate".to_string(),
        AlertSeverity::Critical,
        "Error rate exceeded threshold".to_string(),
        "api_service".to_string(),
    ).await.unwrap();
    
    // Verify alert was handled
    assert_eq!(alerts_received.lock().unwrap().len(), 1);
    
    // Get active alerts
    let active = manager.get_active_alerts().await;
    assert_eq!(active.len(), 1);
    
    // Resolve alert
    manager.resolve_alert(&alert_id).await.unwrap();
    
    // Verify no active alerts
    let active = manager.get_active_alerts().await;
    assert_eq!(active.len(), 0);
}

#[tokio::test]
async fn test_performance_monitoring() {
    let monitor = PerformanceMonitor::new("test_service".to_string());
    
    // Measure successful operation
    let result = monitor.measure_operation("test_op", || async {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok::<_, anyhow::Error>("success")
    }).await;
    
    assert!(result.is_ok());
    
    // Measure failed operation
    let result: Result<String, _> = monitor.measure_operation("test_fail", || async {
        Err(anyhow::anyhow!("Test error"))
    }).await;
    
    assert!(result.is_err());
    
    // Get metrics
    let snapshot = monitor.metrics.get_snapshot().await;
    
    // Should have recorded both success and failure
    let total_ops = snapshot.metrics.iter()
        .filter(|m| m.name == "operations_total")
        .count();
    assert!(total_ops > 0);
}

#[test]
fn test_prometheus_metrics_format() {
    use prometheus::{register_counter, register_gauge, register_histogram};
    
    // Register test metrics
    let counter = register_counter!("test_counter", "Test counter metric").unwrap();
    let gauge = register_gauge!("test_gauge", "Test gauge metric").unwrap();
    let histogram = register_histogram!("test_histogram", "Test histogram metric").unwrap();
    
    // Record values
    counter.inc();
    gauge.set(42.0);
    histogram.observe(0.5);
    
    // Export metrics
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    
    // Verify format
    assert!(output.contains("test_counter"));
    assert!(output.contains("test_gauge 42"));
    assert!(output.contains("test_histogram"));
}