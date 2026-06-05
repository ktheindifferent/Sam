//! Base configuration traits and types

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait for all configuration types
pub trait Configuration: Debug + Clone + Serialize + for<'de> Deserialize<'de> {
    /// Validate the configuration
    fn validate(&self) -> anyhow::Result<()>;

    /// Get default configuration
    fn default_config() -> Self
    where
        Self: Sized + Default,
    {
        Self::default()
    }

    /// Merge with another configuration
    fn merge(&mut self, other: &Self)
    where
        Self: Sized;
}

/// Trait for configurations that can be loaded from environment
pub trait EnvironmentConfig: Configuration {
    /// Load from environment variables
    fn from_env() -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Get environment variable prefix
    fn env_prefix() -> &'static str;
}

/// Trait for configurations with hot-reload support
pub trait HotReloadConfig: Configuration {
    /// Check if configuration has changed
    fn has_changed(&self, other: &Self) -> bool;

    /// Apply changes without restart
    fn apply_changes(&mut self, other: Self) -> anyhow::Result<()>;
}

/// Configuration source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigSource {
    File(String),
    Environment,
    Default,
    Merged(Vec<ConfigSource>),
}

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub source: ConfigSource,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub checksum: Option<String>,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            source: ConfigSource::Default,
            version: "1.0.0".to_string(),
            created_at: now,
            modified_at: now,
            checksum: None,
        }
    }
}

/// Base configuration wrapper with metadata
#[derive(Debug, Clone)]
pub struct ConfigWithMetadata<T: Configuration> {
    pub config: T,
    pub metadata: ConfigMetadata,
}

impl<T: Configuration> ConfigWithMetadata<T> {
    pub fn new(config: T) -> Self {
        Self {
            config,
            metadata: ConfigMetadata::default(),
        }
    }

    pub fn with_source(mut self, source: ConfigSource) -> Self {
        self.metadata.source = source;
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.config.validate()
    }
}
