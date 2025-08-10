// Comprehensive Unit Tests for Monitoring Module

use sam::sam::services::monitoring::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;

#[tokio::test]
async fn test_metrics_collector_counter() {
    let collector = MetricsCollector::new("test_service".to_string());
    
    let mut labels = HashMap::new();
    labels.insert("endpoint".to_string(), "/api/v1/users".to_string());
    labels.insert("method".to_string(), "GET".to_string());
    
    // Increment counter multiple times
    for _ in 0..5 {
        collector.increment_counter("http_requests_total", labels.clone()).await;
    }
    
    let snapshot = collector.get_snapshot().await;
    
    let counter_metric = snapshot.metrics.iter()
        .find(|m| m.name == "http_requests_total")
        .expect("Counter metric not found");
    
    assert_eq!(counter_metric.value, 5.0);
    assert_eq!(counter_metric.metric_type, MetricType::Counter);
}

#[tokio::test]
async fn test_metrics_collector_gauge() {
    let collector = MetricsCollector::new("test_service".to_string());
    
    let labels = HashMap::new();
    
    // Set gauge values
    collector.set_gauge("memory_usage_bytes", 1024.0 * 1024.0 * 256.0, labels.clone()).await;
    collector.set_gauge("cpu_usage_percent", 45.5, labels.clone()).await;
    
    let snapshot = collector.get_snapshot().await;
    
    let memory_metric = snapshot.metrics.iter()
        .find(|m| m.name == "memory_usage_bytes")
        .expect("Memory gauge not found");
    
    assert_eq!(memory_metric.value, 268435456.0);
    assert_eq!(memory_metric.metric_type, MetricType::Gauge);
}

#[tokio::test]
async fn test_metrics_collector_histogram() {
    let collector = MetricsCollector::new("test_service".to_string());
    
    let labels = HashMap::new();
    
    // Record multiple values for histogram
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 100.0, 200.0];
    for value in values {
        collector.record_histogram("response_time_ms", value, labels.clone()).await;
    }
    
    let percentiles = collector.get_histogram_percentiles("response_time_ms", vec![50.0, 90.0, 99.0]).await;
    
    assert!(percentiles.get(&50.0).unwrap() >= &30.0 && percentiles.get(&50.0).unwrap() <= &40.0);
    assert!(percentiles.get(&90.0).unwrap() >= &100.0);
}

#[tokio::test]
async fn test_health_check_system() {
    let health_monitor = HealthMonitor::new("test_service".to_string());
    
    // Register health checks
    health_monitor.register_check("database", Box::new(|| {
        Box::pin(async {
            // Simulate database check
            Ok(HealthStatus::Healthy)
        })
    })).await;
    
    health_monitor.register_check("redis", Box::new(|| {
        Box::pin(async {
            // Simulate Redis check with degradation
            Ok(HealthStatus::Degraded("High latency detected".to_string()))
        })
    })).await;
    
    // Run health checks
    let service_health = health_monitor.check_health().await;
    
    assert_eq!(service_health.service_name, "test_service");
    assert_eq!(service_health.overall_status, HealthStatus::Degraded("One or more checks degraded".to_string()));
    
    let db_check = service_health.checks.iter()
        .find(|c| c.name == "database")
        .expect("Database check not found");
    assert_eq!(db_check.status, HealthStatus::Healthy);
    
    let redis_check = service_health.checks.iter()
        .find(|c| c.name == "redis")
        .expect("Redis check not found");
    assert!(matches!(redis_check.status, HealthStatus::Degraded(_)));
}

#[tokio::test]
async fn test_alert_manager() {
    let alert_manager = AlertManager::new();
    
    // Configure alert rules
    let rule = AlertRule {
        name: "high_error_rate".to_string(),
        condition: AlertCondition::ThresholdExceeded {
            metric: "error_rate".to_string(),
            threshold: 0.05,
            duration: Duration::from_secs(60),
        },
        severity: AlertSeverity::Critical,
        actions: vec![
            AlertAction::LogError,
            AlertAction::SendNotification("ops-team".to_string()),
        ],
    };
    
    alert_manager.add_rule(rule).await;
    
    // Trigger alert condition
    alert_manager.evaluate_metric("error_rate", 0.10).await;
    
    let active_alerts = alert_manager.get_active_alerts().await;
    assert_eq!(active_alerts.len(), 1);
    assert_eq!(active_alerts[0].rule_name, "high_error_rate");
    assert_eq!(active_alerts[0].severity, AlertSeverity::Critical);
}

#[tokio::test]
async fn test_distributed_tracing() {
    let tracer = DistributedTracer::new("test_service".to_string());
    
    // Start a trace
    let trace_id = tracer.start_trace("user_request").await;
    
    // Add spans
    let span1 = tracer.start_span(&trace_id, "database_query").await;
    sleep(Duration::from_millis(50)).await;
    tracer.end_span(&span1).await;
    
    let span2 = tracer.start_span(&trace_id, "cache_lookup").await;
    sleep(Duration::from_millis(10)).await;
    tracer.end_span(&span2).await;
    
    // Get trace summary
    let trace_summary = tracer.get_trace(&trace_id).await.unwrap();
    
    assert_eq!(trace_summary.spans.len(), 2);
    assert!(trace_summary.total_duration >= Duration::from_millis(60));
}

#[tokio::test]
async fn test_resource_monitoring() {
    let resource_monitor = ResourceMonitor::new();
    
    // Start monitoring
    resource_monitor.start_monitoring(Duration::from_millis(100)).await;
    
    // Wait for some samples
    sleep(Duration::from_millis(300)).await;
    
    let resources = resource_monitor.get_current_usage().await;
    
    assert!(resources.cpu_percent >= 0.0 && resources.cpu_percent <= 100.0);
    assert!(resources.memory_bytes > 0);
    assert!(resources.disk_io_bytes >= 0);
    assert!(resources.network_io_bytes >= 0);
}

#[tokio::test]
async fn test_metrics_aggregation() {
    let aggregator = MetricsAggregator::new(Duration::from_secs(60));
    
    // Add metrics over time
    for i in 0..10 {
        aggregator.add_metric(Metric {
            name: "requests".to_string(),
            metric_type: MetricType::Counter,
            value: i as f64,
            labels: HashMap::new(),
            timestamp: Utc::now(),
            unit: Some("count".to_string()),
            description: None,
        }).await;
    }
    
    let aggregated = aggregator.aggregate().await;
    
    assert_eq!(aggregated.count, 10);
    assert_eq!(aggregated.sum, 45.0);
    assert_eq!(aggregated.average, 4.5);
    assert_eq!(aggregated.min, 0.0);
    assert_eq!(aggregated.max, 9.0);
}

#[tokio::test]
async fn test_prometheus_exporter() {
    let exporter = PrometheusExporter::new();
    
    let metrics = vec![
        Metric {
            name: "http_requests_total".to_string(),
            metric_type: MetricType::Counter,
            value: 1234.0,
            labels: {
                let mut labels = HashMap::new();
                labels.insert("method".to_string(), "GET".to_string());
                labels.insert("status".to_string(), "200".to_string());
                labels
            },
            timestamp: Utc::now(),
            unit: None,
            description: Some("Total HTTP requests".to_string()),
        },
    ];
    
    let prometheus_format = exporter.export(metrics).await;
    
    assert!(prometheus_format.contains("# HELP http_requests_total Total HTTP requests"));
    assert!(prometheus_format.contains("# TYPE http_requests_total counter"));
    assert!(prometheus_format.contains("http_requests_total{method=\"GET\",status=\"200\"} 1234"));
}

#[tokio::test]
async fn test_custom_metrics() {
    let custom_metrics = CustomMetricsRegistry::new();
    
    // Register custom metric
    custom_metrics.register(
        "business_transactions_total",
        MetricType::Counter,
        "Total business transactions processed",
    ).await;
    
    // Record custom metric
    custom_metrics.record("business_transactions_total", 1.0, {
        let mut labels = HashMap::new();
        labels.insert("type".to_string(), "purchase".to_string());
        labels.insert("region".to_string(), "us-west".to_string());
        labels
    }).await;
    
    let metric = custom_metrics.get("business_transactions_total").await.unwrap();
    assert_eq!(metric.value, 1.0);
}

#[tokio::test]
async fn test_sla_monitoring() {
    let sla_monitor = SlaMonitor::new();
    
    // Configure SLA targets
    sla_monitor.set_target("availability", 99.9).await;
    sla_monitor.set_target("response_time_p99", 500.0).await;
    
    // Record measurements
    for _ in 0..1000 {
        sla_monitor.record_success("availability").await;
    }
    sla_monitor.record_failure("availability").await;
    
    sla_monitor.record_value("response_time_p99", 450.0).await;
    
    let compliance = sla_monitor.check_compliance().await;
    
    assert!(compliance.get("availability").unwrap().is_compliant);
    assert!(compliance.get("response_time_p99").unwrap().is_compliant);
}