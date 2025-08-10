// Integration Tests for Service Orchestration
// Tests service lifecycle, health checks, dependency management, and recovery

use sam::sam::services::orchestrator::*;
use sam::sam::services::error_handling::*;
use sam::sam::services::monitoring::*;
use std::time::Duration;
use tokio::time::sleep;
use std::collections::HashMap;

#[tokio::test]
async fn test_service_registration_and_lifecycle() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Register a test service
    let config = ServiceConfig {
        name: ServiceName::Redis,
        enabled: true,
        auto_restart: true,
        max_restarts: 3,
        health_check_interval: Duration::from_secs(5),
        startup_timeout: Duration::from_secs(30),
        shutdown_timeout: Duration::from_secs(10),
        dependencies: vec![],
        environment: HashMap::new(),
    };
    
    orchestrator.register_service(config).await.unwrap();
    
    // Start the service
    orchestrator.start_service(ServiceName::Redis).await.unwrap();
    
    // Check service status
    let health = orchestrator.get_service_health(ServiceName::Redis).await.unwrap();
    assert_eq!(health.status, ServiceStatus::Running);
    
    // Stop the service
    orchestrator.stop_service(ServiceName::Redis).await.unwrap();
    
    // Verify stopped
    let health = orchestrator.get_service_health(ServiceName::Redis).await.unwrap();
    assert_eq!(health.status, ServiceStatus::Stopped);
}

#[tokio::test]
async fn test_service_dependency_management() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Register PostgreSQL (no dependencies)
    let pg_config = ServiceConfig {
        name: ServiceName::PostgreSQL,
        enabled: true,
        dependencies: vec![],
        ..Default::default()
    };
    orchestrator.register_service(pg_config).await.unwrap();
    
    // Register Backup service (depends on PostgreSQL)
    let backup_config = ServiceConfig {
        name: ServiceName::Backup,
        enabled: true,
        dependencies: vec![ServiceName::PostgreSQL],
        ..Default::default()
    };
    orchestrator.register_service(backup_config).await.unwrap();
    
    // Try to start Backup without PostgreSQL running - should fail
    let result = orchestrator.start_service(ServiceName::Backup).await;
    assert!(result.is_err());
    
    // Start PostgreSQL first
    orchestrator.start_service(ServiceName::PostgreSQL).await.unwrap();
    
    // Now Backup should start
    orchestrator.start_service(ServiceName::Backup).await.unwrap();
    
    // Verify both are running
    let pg_health = orchestrator.get_service_health(ServiceName::PostgreSQL).await.unwrap();
    let backup_health = orchestrator.get_service_health(ServiceName::Backup).await.unwrap();
    
    assert_eq!(pg_health.status, ServiceStatus::Running);
    assert_eq!(backup_health.status, ServiceStatus::Running);
}

#[tokio::test]
async fn test_service_auto_restart() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Register service with auto-restart
    let config = ServiceConfig {
        name: ServiceName::Crawler,
        enabled: true,
        auto_restart: true,
        max_restarts: 2,
        health_check_interval: Duration::from_secs(1),
        ..Default::default()
    };
    
    orchestrator.register_service(config).await.unwrap();
    orchestrator.start_service(ServiceName::Crawler).await.unwrap();
    
    // Simulate service failure
    orchestrator.simulate_service_failure(ServiceName::Crawler).await;
    
    // Wait for auto-restart
    sleep(Duration::from_secs(2)).await;
    
    // Check that service was restarted
    let health = orchestrator.get_service_health(ServiceName::Crawler).await.unwrap();
    assert_eq!(health.status, ServiceStatus::Running);
    assert_eq!(health.restart_count, 1);
    
    // Simulate multiple failures to exceed max_restarts
    orchestrator.simulate_service_failure(ServiceName::Crawler).await;
    sleep(Duration::from_secs(2)).await;
    orchestrator.simulate_service_failure(ServiceName::Crawler).await;
    sleep(Duration::from_secs(2)).await;
    
    // Service should now be in failed state
    let health = orchestrator.get_service_health(ServiceName::Crawler).await.unwrap();
    match health.status {
        ServiceStatus::Failed(_) => {},
        _ => panic!("Service should be in failed state after exceeding max restarts"),
    }
}

#[tokio::test]
async fn test_health_check_monitoring() {
    let orchestrator = ServiceOrchestrator::new();
    let health_manager = HealthCheckManager::new("test_orchestrator".to_string());
    
    // Register a custom health check
    struct TestHealthCheck {
        service_name: ServiceName,
        orchestrator: ServiceOrchestrator,
    }
    
    #[async_trait::async_trait]
    impl HealthCheckable for TestHealthCheck {
        async fn check(&self) -> Result<HealthCheck> {
            let health = self.orchestrator.get_service_health(self.service_name.clone()).await?;
            
            Ok(HealthCheck {
                name: format!("{:?}_health", self.service_name),
                status: match health.status {
                    ServiceStatus::Running => HealthStatus::Healthy,
                    ServiceStatus::Degraded(msg) => HealthStatus::Degraded(msg),
                    ServiceStatus::Failed(msg) => HealthStatus::Unhealthy(msg),
                    _ => HealthStatus::Unknown,
                },
                message: None,
                last_check: Utc::now(),
                response_time_ms: 0,
                metadata: HashMap::new(),
            })
        }
        
        fn name(&self) -> String {
            format!("{:?}_health", self.service_name)
        }
    }
    
    // Register services
    orchestrator.register_service(ServiceConfig {
        name: ServiceName::Redis,
        enabled: true,
        ..Default::default()
    }).await.unwrap();
    
    orchestrator.start_service(ServiceName::Redis).await.unwrap();
    
    // Register health check
    health_manager.register_check(Box::new(TestHealthCheck {
        service_name: ServiceName::Redis,
        orchestrator: orchestrator.clone(),
    })).await;
    
    // Run health checks
    let service_health = health_manager.get_health().await;
    assert_eq!(service_health.overall_status, HealthStatus::Healthy);
    assert!(!service_health.checks.is_empty());
}

#[tokio::test]
async fn test_graceful_degradation() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Register primary and fallback services
    let primary_config = ServiceConfig {
        name: ServiceName::OpenAI,
        enabled: true,
        ..Default::default()
    };
    
    let fallback_config = ServiceConfig {
        name: ServiceName::Llama,
        enabled: true,
        ..Default::default()
    };
    
    orchestrator.register_service(primary_config).await.unwrap();
    orchestrator.register_service(fallback_config).await.unwrap();
    
    // Start primary service
    orchestrator.start_service(ServiceName::OpenAI).await.unwrap();
    
    // Simulate primary service degradation
    orchestrator.degrade_service(ServiceName::OpenAI, "High latency detected").await;
    
    // Check that fallback is activated
    orchestrator.start_service(ServiceName::Llama).await.unwrap();
    
    let primary_health = orchestrator.get_service_health(ServiceName::OpenAI).await.unwrap();
    let fallback_health = orchestrator.get_service_health(ServiceName::Llama).await.unwrap();
    
    match primary_health.status {
        ServiceStatus::Degraded(_) => {},
        _ => panic!("Primary service should be degraded"),
    }
    
    assert_eq!(fallback_health.status, ServiceStatus::Running);
}

#[tokio::test]
async fn test_circuit_breaker_integration() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Create circuit breaker for external service
    let circuit_breaker = CircuitBreaker::new(
        "github_api".to_string(),
        3,  // failure threshold
        2,  // success threshold
        Duration::from_secs(5),  // timeout
    );
    
    // Register GitHub service with circuit breaker
    let config = ServiceConfig {
        name: ServiceName::GitHub,
        enabled: true,
        ..Default::default()
    };
    
    orchestrator.register_service(config).await.unwrap();
    orchestrator.start_service(ServiceName::GitHub).await.unwrap();
    
    // Simulate multiple failures
    for _ in 0..3 {
        let result = circuit_breaker.call(|| async {
            Err(anyhow::anyhow!("API timeout"))
        }).await;
        assert!(result.is_err());
    }
    
    // Circuit should now be open
    match circuit_breaker.get_state().await {
        CircuitState::Open { .. } => {},
        _ => panic!("Circuit breaker should be open"),
    }
    
    // Subsequent calls should fail fast
    let result = circuit_breaker.call(|| async {
        Ok("Should not execute")
    }).await;
    
    assert!(matches!(
        result.unwrap_err().downcast_ref::<ServiceError>(),
        Some(ServiceError::CircuitBreakerOpen(_))
    ));
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let mut attempt_count = 0;
    let mut attempt_times = Vec::new();
    
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        exponential_base: 2.0,
        jitter: false,
    };
    
    let start = std::time::Instant::now();
    
    let result = retry_with_backoff(
        || async {
            attempt_count += 1;
            attempt_times.push(start.elapsed());
            
            if attempt_count < 3 {
                Err(anyhow::anyhow!("Temporary failure"))
            } else {
                Ok("Success")
            }
        },
        config,
        "test_operation",
    ).await;
    
    assert!(result.is_ok());
    assert_eq!(attempt_count, 3);
    
    // Verify exponential backoff timing
    assert!(attempt_times[1] >= Duration::from_millis(100));
    assert!(attempt_times[2] >= Duration::from_millis(200));
}

#[tokio::test]
async fn test_service_metrics_collection() {
    let orchestrator = ServiceOrchestrator::new();
    let metrics = MetricsCollector::new("orchestrator".to_string());
    
    // Register and start multiple services
    for service in vec![ServiceName::Redis, ServiceName::PostgreSQL, ServiceName::Docker] {
        orchestrator.register_service(ServiceConfig {
            name: service.clone(),
            enabled: true,
            ..Default::default()
        }).await.unwrap();
        
        let start = std::time::Instant::now();
        orchestrator.start_service(service.clone()).await.unwrap();
        let duration = start.elapsed().as_millis() as f64;
        
        // Record metrics
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), format!("{:?}", service));
        
        metrics.increment_counter("service_starts", labels.clone()).await;
        metrics.record_histogram("service_startup_time_ms", duration, labels).await;
    }
    
    // Get metrics snapshot
    let snapshot = metrics.get_snapshot().await;
    assert!(!snapshot.metrics.is_empty());
    
    // Export as Prometheus format
    let prometheus_output = metrics.export_prometheus().await;
    assert!(prometheus_output.contains("service_starts"));
    assert!(prometheus_output.contains("service_startup_time_ms"));
}

#[tokio::test]
async fn test_resource_monitoring() {
    let orchestrator = ServiceOrchestrator::new();
    
    // Register service
    orchestrator.register_service(ServiceConfig {
        name: ServiceName::Crawler,
        enabled: true,
        ..Default::default()
    }).await.unwrap();
    
    orchestrator.start_service(ServiceName::Crawler).await.unwrap();
    
    // Simulate resource usage updates
    orchestrator.update_service_resources(
        ServiceName::Crawler,
        1024 * 1024 * 50,  // 50MB memory
        25.5,  // 25.5% CPU
    ).await;
    
    let health = orchestrator.get_service_health(ServiceName::Crawler).await.unwrap();
    assert_eq!(health.memory_usage, Some(1024 * 1024 * 50));
    assert_eq!(health.cpu_usage, Some(25.5));
    
    // Test resource limits
    orchestrator.set_resource_limits(
        ServiceName::Crawler,
        1024 * 1024 * 100,  // 100MB memory limit
        50.0,  // 50% CPU limit
    ).await;
    
    // Simulate exceeding limits
    orchestrator.update_service_resources(
        ServiceName::Crawler,
        1024 * 1024 * 150,  // 150MB - exceeds limit
        60.0,  // 60% - exceeds limit
    ).await;
    
    // Service should be degraded or stopped
    let health = orchestrator.get_service_health(ServiceName::Crawler).await.unwrap();
    match health.status {
        ServiceStatus::Degraded(_) | ServiceStatus::Stopped => {},
        _ => panic!("Service should be degraded or stopped when exceeding resource limits"),
    }
}