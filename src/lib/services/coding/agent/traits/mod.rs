//! Common traits and interfaces for the coding agent
//!
//! This module defines traits that components can implement to ensure
//! consistency and reduce coupling between modules.

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub mod analyzer;
pub mod executor;
pub mod generator;
pub mod provider;

pub use analyzer::*;
pub use executor::*;
pub use generator::*;
pub use provider::*;

/// Base trait for all services
#[async_trait]
pub trait Service: Send + Sync {
    /// Service name
    fn name(&self) -> &str;

    /// Initialize the service
    async fn initialize(&mut self) -> Result<()>;

    /// Check if service is healthy
    async fn health_check(&self) -> Result<ServiceHealth>;

    /// Shutdown the service
    async fn shutdown(&mut self) -> Result<()>;
}

/// Service health status
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub metrics: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Trait for components that can be cached
pub trait Cacheable {
    /// Get cache key
    fn cache_key(&self) -> String;

    /// Check if cached value is still valid
    fn is_cache_valid(&self, cached_at: std::time::SystemTime) -> bool;

    /// Serialize for caching
    fn to_cache_value(&self) -> Result<Vec<u8>>;

    /// Deserialize from cache
    fn from_cache_value(data: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

/// Trait for components with configuration
pub trait Configurable {
    type Config;

    /// Get current configuration
    fn config(&self) -> &Self::Config;

    /// Update configuration
    fn update_config(&mut self, config: Self::Config) -> Result<()>;

    /// Validate configuration
    fn validate_config(config: &Self::Config) -> Result<()>;
}

/// Trait for components that support serialization
pub trait Persistable {
    /// Save state to file
    fn save_to_file(&self, path: &Path) -> Result<()>;

    /// Load state from file
    fn load_from_file(path: &Path) -> Result<Self>
    where
        Self: Sized;

    /// Export as JSON
    fn to_json(&self) -> Result<serde_json::Value>;

    /// Import from JSON
    fn from_json(json: serde_json::Value) -> Result<Self>
    where
        Self: Sized;
}

/// Trait for components with metrics
#[async_trait]
pub trait Measurable {
    type Metrics;

    /// Collect current metrics
    async fn collect_metrics(&self) -> Result<Self::Metrics>;

    /// Reset metrics
    async fn reset_metrics(&mut self) -> Result<()>;

    /// Export metrics in Prometheus format
    async fn export_prometheus(&self) -> Result<String>;
}

/// Trait for pluggable components
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize plugin
    async fn on_load(&mut self) -> Result<()>;

    /// Clean up plugin
    async fn on_unload(&mut self) -> Result<()>;

    /// Handle events
    async fn handle_event(&mut self, event: PluginEvent) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    FileChanged(PathBuf),
    CommandExecuted(String),
    AnalysisCompleted(String),
    Custom(String, serde_json::Value),
}

/// Trait for components that support streaming
#[async_trait]
pub trait Streamable {
    type Item;

    /// Start streaming
    async fn start_stream(&mut self) -> Result<()>;

    /// Get next item from stream
    async fn next_item(&mut self) -> Result<Option<Self::Item>>;

    /// Stop streaming
    async fn stop_stream(&mut self) -> Result<()>;
}

/// Trait for components with lifecycle hooks
#[async_trait]
pub trait Lifecycle {
    /// Called before starting
    async fn on_before_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called after starting
    async fn on_after_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called before stopping
    async fn on_before_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called after stopping
    async fn on_after_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called on error
    async fn on_error(&mut self, error: anyhow::Error) -> Result<()> {
        log::error!("Lifecycle error: {}", error);
        Ok(())
    }
}
