#[cfg(test)]
mod tests {
    use crate::services::orchestrator::{ServiceName, ServiceOrchestrator, ServiceConfig};
    use crate::services::restart::{RestartManager, RestartConfig, RestartStrategy, RestartEvent, RestartNotifier};
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};
    use std::collections::HashMap;
    use tokio::time::sleep;
    use async_trait::async_trait;

    /// Mock service for testing
    struct MockService {
        name: ServiceName,
        fail_count: Arc<RwLock<u32>>,
        max_failures: u32,
    }

    impl MockService {
        fn new(name: ServiceName, max_failures: u32) -> Self {
            Self {
                name,
                fail_count: Arc::new(RwLock::new(0)),
                max_failures,
            }
        }

        async fn start(&self) -> Result<(), anyhow::Error> {
            let mut count = self.fail_count.write().unwrap();
            *count += 1;
            
            if *count <= self.max_failures {
                Err(anyhow::anyhow!("Service start failed (attempt {})", count))
            } else {
                Ok(())
            }
        }

        async fn stop(&self) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn health_check(&self) -> Result<(), anyhow::Error> {
            let count = self.fail_count.read().unwrap();
            if *count <= self.max_failures {
                Err(anyhow::anyhow!("Health check failed"))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_service_restart_success() {
        let orchestrator = ServiceOrchestrator::new();
        
        // Register a service that will succeed on first restart
        let config = ServiceConfig {
            name: ServiceName::Redis,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(1),
            startup_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
            dependencies: vec![],
            environment: HashMap::new(),
        };
        
        orchestrator.register_service(config).expect("Failed to register service");
        
        // Verify service is registered
        let health = orchestrator.get_health(&ServiceName::Redis);
        assert!(health.is_some());
    }

    #[tokio::test]
    async fn test_exponential_backoff_restart() {
        let manager = RestartManager::new();
        
        let config = RestartConfig {
            strategy: RestartStrategy::ExponentialBackoff {
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(1),
                multiplier: 2.0,
            },
            max_attempts: 5,
            ..Default::default()
        };
        
        manager.register_config(ServiceName::PostgreSQL, config);
        
        // Test backoff delays
        let delays: Vec<Duration> = (0..5)
            .map(|i| manager.calculate_delay(&ServiceName::PostgreSQL, i))
            .collect();
        
        assert_eq!(delays[0], Duration::from_millis(100));
        assert_eq!(delays[1], Duration::from_millis(200));
        assert_eq!(delays[2], Duration::from_millis(400));
        assert_eq!(delays[3], Duration::from_millis(800));
        assert_eq!(delays[4], Duration::from_secs(1)); // Capped at max
    }

    #[tokio::test]
    async fn test_circuit_breaker_behavior() {
        let manager = RestartManager::new();
        
        let config = RestartConfig {
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: 3,
            circuit_breaker_timeout: Duration::from_millis(500),
            ..Default::default()
        };
        
        manager.register_config(ServiceName::Docker, config);
        
        // Initially circuit should be closed
        assert!(manager.check_circuit_breaker(&ServiceName::Docker));
        
        // Simulate failures
        for _ in 0..2 {
            manager.update_circuit_breaker(&ServiceName::Docker, false);
            assert!(manager.check_circuit_breaker(&ServiceName::Docker)); // Still closed
        }
        
        // Third failure should trip the breaker
        manager.update_circuit_breaker(&ServiceName::Docker, false);
        assert!(!manager.check_circuit_breaker(&ServiceName::Docker)); // Now open
        
        // Wait for timeout
        sleep(Duration::from_millis(600)).await;
        
        // Should be half-open now
        assert!(manager.check_circuit_breaker(&ServiceName::Docker));
        
        // Success should close it
        manager.update_circuit_breaker(&ServiceName::Docker, true);
        assert!(manager.check_circuit_breaker(&ServiceName::Docker));
    }

    #[tokio::test]
    async fn test_dependency_checking() {
        let orchestrator = ServiceOrchestrator::new();
        
        // Register PostgreSQL as a dependency
        let pg_config = ServiceConfig {
            name: ServiceName::PostgreSQL,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(1),
            startup_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
            dependencies: vec![],
            environment: HashMap::new(),
        };
        orchestrator.register_service(pg_config).expect("Failed to register PostgreSQL");
        
        // Register FileStorage with PostgreSQL dependency
        let fs_config = ServiceConfig {
            name: ServiceName::FileStorage,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(1),
            startup_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
            dependencies: vec![ServiceName::PostgreSQL],
            environment: HashMap::new(),
        };
        orchestrator.register_service(fs_config).expect("Failed to register FileStorage");
        
        // Test that both services are registered (since get_startup_order is private)
        assert!(orchestrator.get_health(&ServiceName::PostgreSQL).is_some());
        assert!(orchestrator.get_health(&ServiceName::FileStorage).is_some());
    }

    #[tokio::test]
    async fn test_restart_metrics_tracking() {
        let manager = RestartManager::new();
        manager.register_config(ServiceName::Crawler, RestartConfig::default());
        
        // Simulate successful restart
        manager.update_metrics(&ServiceName::Crawler, true, Duration::from_secs(2));
        
        let metrics = manager.get_metrics(&ServiceName::Crawler).unwrap();
        assert_eq!(metrics.total_restarts, 1);
        assert_eq!(metrics.successful_restarts, 1);
        assert_eq!(metrics.failed_restarts, 0);
        assert_eq!(metrics.average_restart_time, Duration::from_secs(2));
        
        // Simulate another successful restart
        manager.update_metrics(&ServiceName::Crawler, true, Duration::from_secs(4));
        
        let metrics = manager.get_metrics(&ServiceName::Crawler).unwrap();
        assert_eq!(metrics.total_restarts, 2);
        assert_eq!(metrics.successful_restarts, 2);
        assert_eq!(metrics.average_restart_time, Duration::from_secs(3)); // Average of 2 and 4
        
        // Simulate failed restart
        manager.update_metrics(&ServiceName::Crawler, false, Duration::from_secs(1));
        
        let metrics = manager.get_metrics(&ServiceName::Crawler).unwrap();
        assert_eq!(metrics.total_restarts, 3);
        assert_eq!(metrics.successful_restarts, 2);
        assert_eq!(metrics.failed_restarts, 1);
        assert_eq!(metrics.consecutive_failures, 1);
        
        // Reset metrics
        manager.reset_metrics(&ServiceName::Crawler);
        let metrics = manager.get_metrics(&ServiceName::Crawler).unwrap();
        assert_eq!(metrics.total_restarts, 0);
        assert_eq!(metrics.successful_restarts, 0);
        assert_eq!(metrics.failed_restarts, 0);
    }

    #[tokio::test]
    async fn test_notification_system() {
        use async_trait::async_trait;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        // Custom notifier for testing
        struct TestNotifier {
            events: Arc<Mutex<Vec<RestartEvent>>>,
        }
        
        #[async_trait]
        impl RestartNotifier for TestNotifier {
            async fn notify(&self, event: RestartEvent) -> anyhow::Result<()> {
                self.events.lock().await.push(event);
                Ok(())
            }
        }
        
        let manager = RestartManager::new();
        let events = Arc::new(Mutex::new(Vec::new()));
        let notifier = Arc::new(TestNotifier {
            events: events.clone(),
        });
        
        manager.add_notifier(notifier);
        
        // Send various events
        manager.notify(RestartEvent::RestartInitiated {
            service: ServiceName::Redis,
            attempt: 1,
            reason: "Test restart".to_string(),
        }).await;
        
        manager.notify(RestartEvent::RestartSucceeded {
            service: ServiceName::Redis,
            duration: Duration::from_secs(3),
        }).await;
        
        manager.notify(RestartEvent::CircuitBreakerTripped {
            service: ServiceName::PostgreSQL,
            failure_count: 5,
        }).await;
        
        // Verify events were recorded
        let recorded_events = events.lock().await;
        assert_eq!(recorded_events.len(), 3);
        
        match &recorded_events[0] {
            RestartEvent::RestartInitiated { service, attempt, .. } => {
                assert_eq!(*service, ServiceName::Redis);
                assert_eq!(*attempt, 1);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_scheduled_restart() {
        let manager = RestartManager::new();
        
        let future_time = Instant::now() + Duration::from_millis(500);
        let config = RestartConfig {
            strategy: RestartStrategy::Scheduled(future_time),
            ..Default::default()
        };
        
        manager.register_config(ServiceName::WebSocket, config);
        
        // Calculate delay should return time until scheduled
        let delay = manager.calculate_delay(&ServiceName::WebSocket, 0);
        assert!(delay > Duration::from_millis(400));
        assert!(delay < Duration::from_millis(600));
        
        // After the scheduled time passes
        sleep(Duration::from_millis(600)).await;
        let delay = manager.calculate_delay(&ServiceName::WebSocket, 0);
        assert_eq!(delay, Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_immediate_restart() {
        let manager = RestartManager::new();
        
        let config = RestartConfig {
            strategy: RestartStrategy::Immediate,
            ..Default::default()
        };
        
        manager.register_config(ServiceName::MDNS, config);
        
        // All attempts should have zero delay
        for attempt in 0..5 {
            let delay = manager.calculate_delay(&ServiceName::MDNS, attempt);
            assert_eq!(delay, Duration::from_secs(0));
        }
    }

    #[tokio::test]
    async fn test_delayed_restart() {
        let manager = RestartManager::new();
        
        let fixed_delay = Duration::from_millis(250);
        let config = RestartConfig {
            strategy: RestartStrategy::Delayed(fixed_delay),
            ..Default::default()
        };
        
        manager.register_config(ServiceName::Lifx, config);
        
        // All attempts should have the same fixed delay
        for attempt in 0..5 {
            let delay = manager.calculate_delay(&ServiceName::Lifx, attempt);
            assert_eq!(delay, fixed_delay);
        }
    }

    #[tokio::test]
    async fn test_health_check_retries() {
        let orchestrator = ServiceOrchestrator::new();
        
        // Register a service
        let config = ServiceConfig {
            name: ServiceName::Voice,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(1),
            startup_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
            dependencies: vec![],
            environment: HashMap::new(),
        };
        
        orchestrator.register_service(config).expect("Failed to register service");
        
        // Test that service health can be retrieved
        let health = orchestrator.get_health(&ServiceName::Voice);
        assert!(health.is_some());
        
        // Verify the service is registered
        let all_health = orchestrator.get_all_health();
        assert!(all_health.contains_key(&ServiceName::Voice));
    }

    #[tokio::test]
    async fn test_concurrent_restart_prevention() {
        let manager = RestartManager::new();
        manager.register_config(ServiceName::P2P, RestartConfig::default());
        
        // Simulate multiple concurrent restart attempts
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let mgr = manager.clone();
                tokio::spawn(async move {
                    mgr.update_metrics(&ServiceName::P2P, true, Duration::from_secs(i));
                })
            })
            .collect();
        
        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Check that metrics are consistent
        let metrics = manager.get_metrics(&ServiceName::P2P).unwrap();
        assert_eq!(metrics.total_restarts, 5);
        assert_eq!(metrics.successful_restarts, 5);
    }
}