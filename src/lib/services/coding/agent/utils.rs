//! Utility functions for the coding agent module.
//!
//! This module provides common helper functions for timestamp operations,
//! string manipulation, safe arithmetic, and other shared functionality.

use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Get current Unix timestamp in seconds
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Get current Unix timestamp in milliseconds
pub fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Calculate duration between two timestamps safely
pub fn duration_between(start: u64, end: u64) -> u64 {
    end.saturating_sub(start)
}

/// Get duration since a given SystemTime
pub fn duration_since(start: SystemTime) -> Duration {
    SystemTime::now()
        .duration_since(start)
        .unwrap_or(Duration::ZERO)
}

/// Format duration in human-readable format
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m {}s", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    }
}

/// Safe division for f64 values that might be NaN or infinite
pub fn safe_division(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 || denominator.is_nan() || numerator.is_nan() {
        0.0
    } else {
        let result = numerator / denominator;
        if result.is_infinite() {
            0.0
        } else {
            result
        }
    }
}

/// Safe comparison for partial ordering of floats
pub fn safe_partial_cmp(a: &f64, b: &f64) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
}

/// Truncate string to specified length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Convert Option<String> to String with default
pub fn option_string_or_default(opt: Option<String>, default: &str) -> String {
    opt.unwrap_or_else(|| default.to_string())
}

/// Convert Result to Option, logging errors
pub fn result_to_option_with_log<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> Option<T> {
    match result {
        Ok(val) => Some(val),
        Err(e) => {
            log::warn!("{}: {}", context, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_timestamp() {
        let ts = unix_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_duration_between() {
        assert_eq!(duration_between(100, 200), 100);
        assert_eq!(duration_between(200, 100), 0); // saturating sub
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3665), "1h 1m 5s");
    }

    #[test]
    fn test_safe_division() {
        assert_eq!(safe_division(10.0, 2.0), 5.0);
        assert_eq!(safe_division(10.0, 0.0), 0.0);
        assert_eq!(safe_division(f64::NAN, 2.0), 0.0);
        assert_eq!(safe_division(10.0, f64::NAN), 0.0);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 2), "hi");
    }
}