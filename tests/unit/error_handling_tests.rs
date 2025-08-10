// Comprehensive Unit Tests for Error Handling Module

use sam::sam::services::error_handling::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();
    
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_secs(1),
        exponential_base: 2.0,
        jitter: false,
    };
    
    let result = retry_with_backoff(
        || {
            let count = attempt_count_clone.clone();
            async move {
                let attempts = count.fetch_add(1, Ordering::SeqCst);
                if attempts < 2 {
                    Err(anyhow::anyhow!("Temporary failure"))
                } else {
                    Ok("Success")
                }
            }
        },
        config,
        "test_operation",
    ).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_exhaustion() {
    let config = RetryConfig {
        max_attempts: 2,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_secs(1),
        exponential_base: 2.0,
        jitter: false,
    };
    
    let result = retry_with_backoff(
        || async {
            Err(anyhow::anyhow!("Persistent failure"))
        },
        config,
        "failing_operation",
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_circuit_breaker_open_state() {
    let breaker = CircuitBreaker::new(
        3,  // failure_threshold
        Duration::from_millis(100),  // timeout_duration
        Duration::from_millis(50),   // half_open_duration
    );
    
    // Trigger failures to open the circuit
    for _ in 0..3 {
        breaker.record_failure().await;
    }
    
    assert_eq!(breaker.get_state().await, CircuitState::Open);
    
    // Attempt should fail immediately when open
    let can_proceed = breaker.can_proceed().await;
    assert!(!can_proceed);
}

#[tokio::test]
async fn test_circuit_breaker_half_open_recovery() {
    let breaker = CircuitBreaker::new(
        2,  // failure_threshold
        Duration::from_millis(50),   // timeout_duration
        Duration::from_millis(100),  // half_open_duration
    );
    
    // Open the circuit
    for _ in 0..2 {
        breaker.record_failure().await;
    }
    
    // Wait for timeout to transition to half-open
    sleep(Duration::from_millis(60)).await;
    
    assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
    
    // Record success to close the circuit
    breaker.record_success().await;
    
    assert_eq!(breaker.get_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_error_aggregation() {
    let aggregator = ErrorAggregator::new(Duration::from_secs(1));
    
    // Add various error types
    aggregator.add_error(ServiceError::ConnectionError("DB connection failed".to_string())).await;
    aggregator.add_error(ServiceError::TimeoutError(Duration::from_secs(30))).await;
    aggregator.add_error(ServiceError::ConnectionError("Redis connection failed".to_string())).await;
    
    let summary = aggregator.get_summary().await;
    
    assert_eq!(summary.total_errors, 3);
    assert_eq!(summary.errors_by_type.get("ConnectionError"), Some(&2));
    assert_eq!(summary.errors_by_type.get("TimeoutError"), Some(&1));
}

#[tokio::test]
async fn test_fallback_handler() {
    let handler = FallbackHandler::new();
    
    let result = handler.execute_with_fallback(
        || async {
            Err(anyhow::anyhow!("Primary failed"))
        },
        || async {
            Ok("Fallback value")
        },
    ).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Fallback value");
}

#[tokio::test]
async fn test_error_recovery_strategies() {
    let strategy = RecoveryStrategy::ExponentialBackoff {
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_secs(1),
        multiplier: 2.0,
    };
    
    let delay = strategy.calculate_delay(3);
    assert!(delay >= Duration::from_millis(80) && delay <= Duration::from_millis(100));
}

#[tokio::test]
async fn test_graceful_degradation() {
    let degradation_manager = GracefulDegradationManager::new();
    
    // Enable degraded mode
    degradation_manager.enable_degraded_mode("database", "Connection pool exhausted").await;
    
    assert!(degradation_manager.is_degraded("database").await);
    
    let status = degradation_manager.get_degradation_status().await;
    assert_eq!(status.degraded_services.len(), 1);
    assert!(status.degraded_services.contains_key("database"));
    
    // Restore normal operation
    degradation_manager.restore_normal_mode("database").await;
    
    assert!(!degradation_manager.is_degraded("database").await);
}

#[tokio::test]
async fn test_error_context_propagation() {
    let error = ServiceError::DatabaseError("Connection failed".to_string());
    
    let with_context = error
        .add_context("Service", "PostgreSQL")
        .add_context("Host", "localhost:5432")
        .add_context("Retry", "3/3");
    
    let context = with_context.get_context();
    assert_eq!(context.get("Service"), Some(&"PostgreSQL".to_string()));
    assert_eq!(context.get("Host"), Some(&"localhost:5432".to_string()));
    assert_eq!(context.get("Retry"), Some(&"3/3".to_string()));
}

#[tokio::test]
async fn test_cascading_failure_prevention() {
    let failure_detector = CascadingFailureDetector::new(
        3,  // max_failures_per_window
        Duration::from_secs(60),  // window_duration
    );
    
    // Record failures
    for _ in 0..2 {
        failure_detector.record_failure("service_a").await;
    }
    
    // Should not trigger cascade prevention yet
    assert!(!failure_detector.should_isolate("service_a").await);
    
    // One more failure should trigger isolation
    failure_detector.record_failure("service_a").await;
    assert!(failure_detector.should_isolate("service_a").await);
}