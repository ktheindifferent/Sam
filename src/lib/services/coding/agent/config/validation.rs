//! Configuration validation

use super::*;
use std::collections::HashSet;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a complete configuration
    pub fn validate(config: &CodingAgentConfig) -> anyhow::Result<()> {
        Self::validate_core(&config.core)?;
        Self::validate_providers(&config.providers)?;
        Self::validate_resources(&config.resources)?;
        Self::validate_security(&config.security)?;
        Self::validate_performance(&config.performance)?;
        Ok(())
    }

    /// Validate core configuration
    fn validate_core(core: &CoreConfig) -> anyhow::Result<()> {
        // Validate working directory exists
        if !core.working_directory.exists() {
            return Err(anyhow::anyhow!(
                "Working directory does not exist: {:?}",
                core.working_directory
            ));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&core.log_level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level: {}. Must be one of: {:?}",
                core.log_level,
                valid_levels
            ));
        }

        // Validate context size
        if core.max_context_size == 0 {
            return Err(anyhow::anyhow!("Max context size cannot be 0"));
        }

        if core.max_context_size > 32768 {
            return Err(anyhow::anyhow!(
                "Max context size too large: {}. Maximum is 32768",
                core.max_context_size
            ));
        }

        Ok(())
    }

    /// Validate provider configuration
    fn validate_providers(providers: &ProvidersConfig) -> anyhow::Result<()> {
        // Validate default provider is configured
        let valid_providers = ["ollama", "openai", "local"];
        if !valid_providers.contains(&providers.default_provider.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid default provider: {}. Must be one of: {:?}",
                providers.default_provider,
                valid_providers
            ));
        }

        // Validate Ollama configuration
        if providers.ollama.port == 0 {
            return Err(anyhow::anyhow!("Ollama port cannot be 0"));
        }

        if providers.ollama.context_size > 16384 {
            return Err(anyhow::anyhow!(
                "Ollama context size too large: {}. Maximum is 16384",
                providers.ollama.context_size
            ));
        }

        // Validate OpenAI configuration if in use
        if providers.default_provider == "openai" && providers.openai.api_key.is_none() {
            return Err(anyhow::anyhow!(
                "OpenAI API key required when OpenAI is the default provider"
            ));
        }

        if providers.openai.temperature < 0.0 || providers.openai.temperature > 2.0 {
            return Err(anyhow::anyhow!(
                "OpenAI temperature must be between 0.0 and 2.0, got {}",
                providers.openai.temperature
            ));
        }

        Ok(())
    }

    /// Validate resource limits
    fn validate_resources(resources: &ResourceConfig) -> anyhow::Result<()> {
        if resources.max_memory_mb == 0 {
            return Err(anyhow::anyhow!("Max memory cannot be 0"));
        }

        if resources.max_memory_mb > 65536 {
            // 64GB max
            return Err(anyhow::anyhow!(
                "Max memory too large: {} MB. Maximum is 65536 MB (64GB)",
                resources.max_memory_mb
            ));
        }

        if resources.max_cpu_percent <= 0.0 || resources.max_cpu_percent > 100.0 {
            return Err(anyhow::anyhow!(
                "Max CPU percent must be between 0 and 100, got {}",
                resources.max_cpu_percent
            ));
        }

        if resources.max_concurrent_operations == 0 {
            return Err(anyhow::anyhow!("Max concurrent operations cannot be 0"));
        }

        if resources.max_concurrent_operations > 1000 {
            return Err(anyhow::anyhow!(
                "Max concurrent operations too large: {}. Maximum is 1000",
                resources.max_concurrent_operations
            ));
        }

        if resources.command_timeout.as_secs() == 0 {
            return Err(anyhow::anyhow!("Command timeout cannot be 0"));
        }

        Ok(())
    }

    /// Validate security configuration
    fn validate_security(security: &SecurityConfig) -> anyhow::Result<()> {
        // Check for dangerous allowed commands in sandbox mode
        if security.sandbox_mode {
            let dangerous = ["rm", "sudo", "chmod", "chown", "kill", "pkill"];
            for cmd in &security.allowed_commands {
                for danger in &dangerous {
                    if cmd.contains(danger) {
                        return Err(anyhow::anyhow!(
                            "Dangerous command '{}' in allowed list while sandbox mode is enabled",
                            cmd
                        ));
                    }
                }
            }
        }

        // Check for conflicts between allowed and blocked
        let allowed_set: HashSet<_> = security.allowed_commands.iter().collect();
        let blocked_set: HashSet<_> = security.blocked_commands.iter().collect();

        let conflicts: Vec<_> = allowed_set.intersection(&blocked_set).collect();
        if !conflicts.is_empty() {
            return Err(anyhow::anyhow!(
                "Commands appear in both allowed and blocked lists: {:?}",
                conflicts
            ));
        }

        if security.max_command_depth == 0 {
            return Err(anyhow::anyhow!("Max command depth cannot be 0"));
        }

        Ok(())
    }

    /// Validate performance configuration
    fn validate_performance(performance: &PerformanceConfig) -> anyhow::Result<()> {
        if performance.cache_size_mb > 8192 {
            return Err(anyhow::anyhow!(
                "Cache size too large: {} MB. Maximum is 8192 MB",
                performance.cache_size_mb
            ));
        }

        let rate_limit = &performance.rate_limiting;
        if rate_limit.requests_per_minute == 0 {
            return Err(anyhow::anyhow!(
                "Rate limit cannot be 0 requests per minute"
            ));
        }

        if rate_limit.requests_per_minute > 10000 {
            return Err(anyhow::anyhow!(
                "Rate limit too high: {} requests per minute. Maximum is 10000",
                rate_limit.requests_per_minute
            ));
        }

        if rate_limit.burst_size > rate_limit.requests_per_minute {
            return Err(anyhow::anyhow!(
                "Burst size ({}) cannot exceed requests per minute ({})",
                rate_limit.burst_size,
                rate_limit.requests_per_minute
            ));
        }

        Ok(())
    }

    /// Check for deprecated settings
    pub fn check_deprecated(config: &CodingAgentConfig) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for deprecated model names
        if config.core.default_model.contains("codellama-7b") {
            warnings.push(
                "Model 'codellama-7b' is deprecated. Use 'codellama:7b' instead.".to_string(),
            );
        }

        // Add more deprecation checks as needed

        warnings
    }

    /// Suggest optimizations
    pub fn suggest_optimizations(config: &CodingAgentConfig) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Memory suggestions
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_memory();
        let available_memory = (sys.total_memory() / 1024 / 1024) as usize; // Convert to MB

        if config.resources.max_memory_mb < available_memory / 4 {
            suggestions.push(format!(
                "Consider increasing max_memory_mb to {} MB (25% of available memory)",
                available_memory / 4
            ));
        }

        // Cache suggestions
        if config.performance.cache_size_mb < 256 && config.resources.max_memory_mb >= 4096 {
            suggestions.push(
                "Consider increasing cache_size_mb to at least 256 MB for better performance"
                    .to_string(),
            );
        }

        // Concurrency suggestions
        let cpu_count = num_cpus::get();
        if config.resources.max_concurrent_operations < cpu_count {
            suggestions.push(format!(
                "Consider increasing max_concurrent_operations to {} (number of CPU cores)",
                cpu_count
            ));
        }

        // Feature suggestions
        if !config.features.performance_profiling && config.core.log_level == "debug" {
            suggestions.push(
                "Consider enabling performance_profiling when running in debug mode".to_string(),
            );
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_valid_config() {
        let config = CodingAgentConfig::default();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_invalid_memory() {
        let mut config = CodingAgentConfig::default();
        config.resources.max_memory_mb = 0;
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_invalid_cpu_percent() {
        let mut config = CodingAgentConfig::default();
        config.resources.max_cpu_percent = 150.0;
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_command_conflicts() {
        let mut config = CodingAgentConfig::default();
        config.security.allowed_commands.push("rm".to_string());
        config.security.blocked_commands.push("rm".to_string());
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_dangerous_commands_in_sandbox() {
        let mut config = CodingAgentConfig::default();
        config.security.sandbox_mode = true;
        config
            .security
            .allowed_commands
            .push("sudo rm -rf /".to_string());
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_deprecation_warnings() {
        let mut config = CodingAgentConfig::default();
        config.core.default_model = "codellama-7b".to_string();
        let warnings = ConfigValidator::check_deprecated(&config);
        assert!(!warnings.is_empty());
    }
}
