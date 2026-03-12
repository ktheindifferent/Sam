//! Configuration builder for fluent API

use super::*;
use std::path::PathBuf;

/// Configuration builder with fluent API
pub struct ConfigBuilder {
    config: CodingAgentConfig,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: CodingAgentConfig::default(),
        }
    }

    /// Set working directory
    pub fn working_directory(mut self, path: PathBuf) -> Self {
        self.config.core.working_directory = path;
        self
    }

    /// Set default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.config.core.default_model = model.into();
        self
    }

    /// Set session timeout
    pub fn session_timeout(mut self, timeout: Duration) -> Self {
        self.config.core.session_timeout = timeout;
        self
    }

    /// Configure Ollama provider
    pub fn ollama_endpoint(mut self, endpoint: impl Into<String>, port: u16) -> Self {
        self.config.providers.ollama.endpoint = endpoint.into();
        self.config.providers.ollama.port = port;
        self
    }

    /// Add Ollama model
    pub fn add_ollama_model(mut self, model: impl Into<String>) -> Self {
        self.config.providers.ollama.models.push(model.into());
        self
    }

    /// Configure OpenAI
    pub fn openai_api_key(mut self, key: impl Into<String>) -> Self {
        self.config.providers.openai.api_key = Some(key.into());
        self
    }

    /// Set OpenAI model
    pub fn openai_model(mut self, model: impl Into<String>) -> Self {
        self.config.providers.openai.model = model.into();
        self
    }

    /// Set default provider
    pub fn default_provider(mut self, provider: impl Into<String>) -> Self {
        self.config.providers.default_provider = provider.into();
        self
    }

    /// Configure memory limit
    pub fn max_memory_mb(mut self, limit: usize) -> Self {
        self.config.resources.max_memory_mb = limit;
        self
    }

    /// Configure CPU limit
    pub fn max_cpu_percent(mut self, percent: f32) -> Self {
        self.config.resources.max_cpu_percent = percent;
        self
    }

    /// Configure concurrent operations
    pub fn max_concurrent_operations(mut self, max: usize) -> Self {
        self.config.resources.max_concurrent_operations = max;
        self
    }

    /// Configure command timeout
    pub fn command_timeout(mut self, timeout: Duration) -> Self {
        self.config.resources.command_timeout = timeout;
        self
    }

    /// Enable/disable sandbox mode
    pub fn sandbox_mode(mut self, enabled: bool) -> Self {
        self.config.security.sandbox_mode = enabled;
        self
    }

    /// Add allowed command
    pub fn allow_command(mut self, command: impl Into<String>) -> Self {
        self.config.security.allowed_commands.push(command.into());
        self
    }

    /// Add blocked command
    pub fn block_command(mut self, command: impl Into<String>) -> Self {
        self.config.security.blocked_commands.push(command.into());
        self
    }

    /// Require confirmation for commands
    pub fn require_confirmation(mut self, required: bool) -> Self {
        self.config.security.require_confirmation = required;
        self
    }

    /// Enable audit logging
    pub fn audit_logging(mut self, enabled: bool) -> Self {
        self.config.security.audit_logging = enabled;
        self
    }

    /// Configure cache size
    pub fn cache_size_mb(mut self, size: usize) -> Self {
        self.config.performance.cache_size_mb = size;
        self
    }

    /// Enable batch processing
    pub fn batch_processing(mut self, enabled: bool) -> Self {
        self.config.performance.batch_processing = enabled;
        self
    }

    /// Configure rate limiting
    pub fn rate_limit(mut self, requests_per_minute: usize) -> Self {
        self.config.performance.rate_limiting.requests_per_minute = requests_per_minute;
        self
    }

    /// Enable feature
    pub fn enable_feature(mut self, feature: Feature) -> Self {
        match feature {
            Feature::CodeIntelligence => self.config.features.code_intelligence = true,
            Feature::AutoCompletion => self.config.features.auto_completion = true,
            Feature::GitIntegration => self.config.features.git_integration = true,
            Feature::Debugging => self.config.features.debugging_tools = true,
            Feature::PerformanceProfiling => self.config.features.performance_profiling = true,
            Feature::SecurityScanning => self.config.features.security_scanning = true,
            Feature::DocumentationGeneration => self.config.features.documentation_generation = true,
            Feature::TestGeneration => self.config.features.test_generation = true,
            Feature::RefactoringSuggestions => self.config.features.refactoring_suggestions = true,
        }
        self
    }

    /// Disable feature
    pub fn disable_feature(mut self, feature: Feature) -> Self {
        match feature {
            Feature::CodeIntelligence => self.config.features.code_intelligence = false,
            Feature::AutoCompletion => self.config.features.auto_completion = false,
            Feature::GitIntegration => self.config.features.git_integration = false,
            Feature::Debugging => self.config.features.debugging_tools = false,
            Feature::PerformanceProfiling => self.config.features.performance_profiling = false,
            Feature::SecurityScanning => self.config.features.security_scanning = false,
            Feature::DocumentationGeneration => self.config.features.documentation_generation = false,
            Feature::TestGeneration => self.config.features.test_generation = false,
            Feature::RefactoringSuggestions => self.config.features.refactoring_suggestions = false,
        }
        self
    }

    /// Load from file and merge
    pub fn from_file(mut self, path: &PathBuf) -> anyhow::Result<Self> {
        let file_config = CodingAgentConfig::from_file(path)?;
        self.config = self.config.merge(file_config);
        Ok(self)
    }

    /// Use environment preset
    pub fn environment(mut self, env: &str) -> Self {
        self.config = CodingAgentConfig::for_environment(env);
        self
    }

    /// Add custom metadata
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.metadata.insert(key.into(), value);
        self
    }

    /// Build and validate the configuration
    pub fn build(self) -> anyhow::Result<CodingAgentConfig> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Build without validation
    pub fn build_unchecked(self) -> CodingAgentConfig {
        self.config
    }
}

/// Feature enumeration for easy toggling
pub enum Feature {
    CodeIntelligence,
    AutoCompletion,
    GitIntegration,
    Debugging,
    PerformanceProfiling,
    SecurityScanning,
    DocumentationGeneration,
    TestGeneration,
    RefactoringSuggestions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let config = ConfigBuilder::new().build().unwrap();
        assert_eq!(config.providers.default_provider, "ollama");
        assert!(config.security.sandbox_mode);
    }

    #[test]
    fn test_builder_fluent_api() {
        let config = ConfigBuilder::new()
            .default_model("codellama:34b")
            .max_memory_mb(8192)
            .sandbox_mode(false)
            .enable_feature(Feature::PerformanceProfiling)
            .build()
            .unwrap();

        assert_eq!(config.core.default_model, "codellama:34b");
        assert_eq!(config.resources.max_memory_mb, 8192);
        assert!(!config.security.sandbox_mode);
        assert!(config.features.performance_profiling);
    }

    #[test]
    fn test_environment_presets() {
        let dev_config = ConfigBuilder::new()
            .environment("development")
            .build()
            .unwrap();

        assert_eq!(dev_config.core.log_level, "debug");
        assert!(!dev_config.security.sandbox_mode);

        let prod_config = ConfigBuilder::new()
            .environment("production")
            .build()
            .unwrap();

        assert_eq!(prod_config.core.log_level, "warn");
        assert!(prod_config.security.sandbox_mode);
    }
}