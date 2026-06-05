//! Application monitoring and observability module
//!
//! Provides integration with Sentry for error tracking, performance monitoring,
//! and application observability.

use sentry::{protocol::Event, Level};
use std::collections::BTreeMap;

/// Initialize Sentry with enhanced configuration
pub fn init_sentry() -> sentry::ClientInitGuard {
    sentry::init((
        std::env::var("SENTRY_DSN").unwrap_or_else(|_| {
            "http://2f7ca9e40bcc42589eb9c01e0a8696ea@sentry.alpha.opensam.foundation/5".to_string()
        }),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(
                std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string())
                    .into(),
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
        },
    ))
}

/// Report a service error to Sentry with context
pub fn report_service_error(
    service: &str,
    error: &dyn std::fmt::Display,
    context: Option<BTreeMap<String, String>>,
) {
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

    event
        .tags
        .insert("service".to_string(), service.to_string());
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

/// Helper macro for easily adding Sentry context to errors
#[macro_export]
macro_rules! with_sentry_context {
    ($result:expr, $service:literal) => {
        $result.map_err(|e| {
            $crate::monitoring::report_service_error($service, &e, None);
            e
        })
    };
    ($result:expr, $service:literal, $context:expr) => {
        $result.map_err(|e| {
            $crate::monitoring::report_service_error($service, &e, Some($context));
            e
        })
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_span() {
        let span = PerformanceSpan::new("test", "test_operation");
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(10));
        span.finish();
    }
}
