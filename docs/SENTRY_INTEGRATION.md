# Sentry Integration Guide

## Overview
Sentry has been integrated into the SAM project for error tracking, performance monitoring, and application observability.

## Configuration

### Environment Variables
- `SENTRY_DSN`: Override the default DSN (optional)
- `ENVIRONMENT`: Set the environment (development/staging/production)

### Default DSN
```
http://2f7ca9e40bcc42589eb9c01e0a8696ea@sentry.alpha.opensam.foundation/5
```

## Features Implemented

### 1. Automatic Error Tracking
- Panic handler integration captures all panics
- Errors are automatically reported with stack traces
- Environment-based filtering (dev/staging/prod)

### 2. Enhanced Error Reporting
- WebSocket errors with security implications are auto-reported
- Critical system errors are tracked
- Service-specific error context is captured

### 3. Performance Monitoring
- Transaction tracing (30% sample rate)
- Performance spans for critical operations
- Automatic performance metrics collection

### 4. Privacy Protection
- Sensitive data filtering before sending
- PII (Personally Identifiable Information) is not sent
- Authentication tokens and passwords are filtered

## Usage Examples

### Basic Error Reporting
```rust
use sentry;

// Capture any error that implements std::error::Error
sentry::capture_error(&error);

// Capture a message
sentry::capture_message("Something important happened", sentry::Level::Info);
```

### Service Error Reporting
```rust
use sam::monitoring::{report_service_error, report_critical_error};
use std::collections::BTreeMap;

// Report a service error with context
let mut context = BTreeMap::new();
context.insert("user_id".to_string(), "123".to_string());
report_service_error("redis", &error, Some(context));

// Report a critical error
report_critical_error(&error, "database");
```

### Performance Monitoring
```rust
use sam::monitoring::PerformanceSpan;

// Start a performance span
let span = PerformanceSpan::new("database_query", "select");
// ... perform operation ...
span.finish(); // Or it will auto-finish when dropped
```

### Adding Breadcrumbs
```rust
use sam::monitoring::add_breadcrumb;

// Add debugging breadcrumbs
add_breadcrumb("User clicked button".to_string(), Some("ui".to_string()));
```

### Using the Macro Helper
```rust
use sam::with_sentry_context;

// Automatically report errors from a Result
let result = some_operation();
with_sentry_context!(result, "service_name");
```

## Integration Points

1. **Main Application** (`src/main.rs`)
   - Sentry initialized early in application startup
   - Panic handler configured to report to Sentry

2. **WebSocket Module** (`src/sam/websocket/error.rs`)
   - Security errors auto-reported
   - Configuration and unexpected errors tracked

3. **Monitoring Module** (`src/sam/monitoring.rs`)
   - Central module for all Sentry operations
   - Helper functions and performance tracking

## Monitoring Dashboard

View your errors and performance metrics at:
- Sentry Dashboard: `sentry.alpha.opensam.foundation`

## Testing Sentry Integration

To test that Sentry is working:

```rust
// Add this temporary code to trigger a test error
sentry::capture_message("SAM Sentry Integration Test", sentry::Level::Info);

// Or trigger a panic (in development only!)
panic!("Test panic for Sentry");
```

## Best Practices

1. **Use appropriate error levels:**
   - `Fatal`: System crashes, critical failures
   - `Error`: Operation failures, service errors
   - `Warning`: Degraded performance, recoverable issues
   - `Info`: Important state changes
   - `Debug`: Detailed debugging information

2. **Add context to errors:**
   - Include service names
   - Add relevant user/request context
   - Include operation parameters (without sensitive data)

3. **Performance monitoring:**
   - Wrap expensive operations in performance spans
   - Monitor database queries and external API calls
   - Track background job execution

4. **Privacy considerations:**
   - Never log passwords, tokens, or API keys
   - Be careful with user data
   - Use the filtering mechanisms provided

## Troubleshooting

### Errors not appearing in Sentry
1. Check the DSN is correct
2. Verify network connectivity to sentry.alpha.opensam.foundation
3. Check the environment variable is set correctly
4. Look for local logs about Sentry connection issues

### Performance issues
- Reduce `traces_sample_rate` if too much data is being sent
- Use sampling for high-volume error scenarios

## Future Enhancements
- [ ] Add custom context processors
- [ ] Implement release tracking
- [ ] Add source map support for web dashboard
- [ ] Configure alert rules
- [ ] Add custom performance metrics
- [ ] Implement distributed tracing