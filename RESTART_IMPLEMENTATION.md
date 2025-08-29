# Service Restart Logic Implementation

## Overview
Comprehensive service restart capability has been implemented for the orchestrator at `src/sam/services/orchestrator.rs:585` with the following features:

## Implemented Features

### 1. Exponential Backoff for Restart Attempts
- Configurable base delay and maximum delay
- Exponential multiplier for increasing delays between attempts
- Prevents overwhelming the system with rapid restart attempts

### 2. Configurable Max Restart Attempts
- Each service can have its own maximum restart limit
- Restart counter tracks attempts per service
- Automatic failure state when max attempts exceeded

### 3. Restart Strategies
Three distinct restart strategies implemented:
- **Immediate**: Restart without delay
- **Delayed**: Fixed delay before restart
- **Scheduled**: Restart at a specific time
- **Exponential Backoff**: Progressive delay increase (default)

### 4. Service Dependency Checking
- Validates all dependencies are running before restart
- Prevents cascading failures
- Allows degraded dependencies with warning

### 5. Health Check Validation
- Post-restart health verification with retries
- Service-specific health check implementations
- Configurable timeout and retry attempts

### 6. Notification System
- Event-based notifications for all restart activities
- Pluggable notification handlers
- Default logging notifier included
- Events include:
  - RestartInitiated
  - RestartSucceeded
  - RestartFailed
  - CircuitBreakerTripped
  - DependencyCheckFailed
  - HealthCheckFailed

### 7. Restart Metrics
Comprehensive metrics tracking:
- Total restart attempts
- Success/failure counts
- Average restart duration
- Consecutive failure tracking
- Circuit breaker trip count
- Last restart/success/failure timestamps

### 8. Circuit Breaker Pattern
- Prevents continuous restart attempts for failing services
- Three states: Closed, Open, Half-Open
- Configurable failure threshold
- Automatic reset after timeout period
- Testing in half-open state

## Architecture

### Core Components

1. **RestartManager** (`src/sam/services/restart.rs`)
   - Central management of restart configurations
   - Metrics collection and reporting
   - Circuit breaker state management
   - Notification dispatch

2. **ServiceOrchestrator** (`src/sam/services/orchestrator.rs`)
   - Integration with RestartManager
   - Service lifecycle management
   - Dependency resolution
   - Health check coordination

### Key Methods

- `ServiceOrchestrator::restart_service()` - Main restart entry point at line 667
- `RestartManager::calculate_delay()` - Backoff calculation
- `RestartManager::check_circuit_breaker()` - Circuit state validation
- `RestartManager::update_metrics()` - Statistics tracking

## Usage Example

```rust
// Configure service with restart settings
let config = ServiceConfig {
    name: ServiceName::Redis,
    enabled: true,
    auto_restart: true,
    max_restarts: 3,
    health_check_interval: Duration::from_secs(30),
    startup_timeout: Duration::from_secs(60),
    shutdown_timeout: Duration::from_secs(30),
    dependencies: vec![],
    environment: HashMap::new(),
};

orchestrator.register_service(config)?;

// Service will automatically restart on failure with exponential backoff
// Circuit breaker prevents excessive restart attempts
// Metrics available via orchestrator.get_restart_metrics()
```

## Test Coverage

Comprehensive test suite includes:
- Service restart success scenarios
- Exponential backoff calculation
- Circuit breaker state transitions
- Dependency checking logic
- Health check validation
- Metrics tracking accuracy
- Notification system operation
- All restart strategies
- Concurrent restart prevention

## Integration Points

The restart logic integrates with:
- Docker service management
- PostgreSQL health checks
- Redis health checks
- Service dependency graph
- Health monitoring system
- Metrics collection
- Event notification system

## Configuration

Default configuration provides:
- Exponential backoff: 1s base, 60s max, 2x multiplier
- Max attempts: 3 (configurable per service)
- Health check: 30s timeout, 3 retries
- Circuit breaker: 5 failure threshold, 5min timeout
- Full dependency checking enabled
- Notifications for all events

## Files Modified

- `src/sam/services/orchestrator.rs` - Main integration and restart logic
- `src/sam/services/restart.rs` - Core restart management implementation
- `src/sam/services/restart_test.rs` - Comprehensive test suite
- `src/sam/services/mod.rs` - Module registration

## Critical Implementation at Line 585

The TODO at line 585 has been replaced with a complete restart implementation that:
1. Spawns an async task for the restart
2. Integrates with RestartManager for strategy and metrics
3. Performs dependency and health validation
4. Handles circuit breaker logic
5. Sends appropriate notifications

This implementation provides enterprise-grade resilience and self-healing capabilities for the SAM orchestrator system.