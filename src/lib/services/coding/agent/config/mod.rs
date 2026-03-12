//! Consolidated configuration management for coding agent
//!
//! This module provides a unified configuration system that replaces
//! scattered config structs throughout the codebase.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub mod base;
pub mod builder;
pub mod validation;

pub use base::*;
pub use builder::ConfigBuilder;
pub use validation::ConfigValidator;

/// Master configuration for the entire coding agent system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentConfig {
    /// Core agent settings
    pub core: CoreConfig,

    /// LLM provider configurations
    pub providers: ProvidersConfig,

    /// Resource limits and management
    pub resources: ResourceConfig,

    /// Security and safety settings
    pub security: SecurityConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,

    /// Feature flags
    pub features: FeaturesConfig,

    /// Safe commands (backward compatibility)
    pub safe_commands: Vec<String>,

    /// Ollama timeout (backward compatibility)
    pub ollama_timeout_seconds: u64,

    /// Default model (backward compatibility)
    pub default_model: String,

    /// System prompt template (backward compatibility)
    pub system_prompt_template: String,

    /// Max context lines (backward compatibility)
    pub max_context_lines: usize,

    /// Require confirmation (backward compatibility)
    pub require_confirmation: bool,

    /// Workspace integration (backward compatibility)
    pub workspace_integration: bool,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CodingAgentConfig {
    /// Check if a command is safe to execute
    pub fn is_safe_command(&self, command: &str) -> bool {
        // Check against safe commands list
        if self.safe_commands.iter().any(|safe| command.starts_with(safe)) {
            return true;
        }

        // Also check security config allowed commands
        if self.security.allowed_commands.iter().any(|allowed| command.starts_with(allowed)) {
            return true;
        }

        // Check if it's blocked
        if self.security.blocked_commands.iter().any(|blocked| command.contains(blocked)) {
            return false;
        }

        // In sandbox mode, only explicitly allowed commands are safe
        !self.security.sandbox_mode
    }
}

impl Default for CodingAgentConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            providers: ProvidersConfig::default(),
            resources: ResourceConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeaturesConfig::default(),
            safe_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "cd".to_string(),
            ],
            ollama_timeout_seconds: 120,
            default_model: "codellama:13b".to_string(),
            system_prompt_template: r#"You are SAM, an expert coding assistant.

IMPORTANT RULES FOR DEBUGGING TASKS:
1. When asked to "debug" something, first check if it's a file or directory using `ls -la`
2. For directories: Use `ls -la <dir>` to list contents, then examine relevant files inside
3. For files: Use `cat <file>` to read contents
4. NEVER use `cat` on a directory - it will fail
5. NEVER use `grep` on a directory without the -r flag
6. When searching in directories, use: `grep -r "pattern" directory/`

COMMAND GUIDELINES:
- Always verify targets exist before operating on them
- Use `ls -la` to check if something is a file or directory
- For exploring directories: `ls -la`, `tree`, `find`
- For reading files: `cat`, `head`, `tail`
- For searching: `grep -r` for directories, `grep` for files

When given a vague task like "debug X", interpret it as:
1. First explore what X is (file or directory)
2. If directory: List contents and examine relevant files
3. If file: Read and analyze its contents
4. Look for errors, issues, or problems to fix"#.to_string(),
            max_context_lines: 100,
            require_confirmation: false,
            workspace_integration: false,
            metadata: HashMap::new(),
        }
    }
}

/// Core configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub working_directory: PathBuf,
    pub session_timeout: Duration,
    pub max_context_size: usize,
    pub default_model: String,
    pub log_level: String,
    pub enable_telemetry: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            session_timeout: Duration::from_secs(3600),
            max_context_size: 8192,
            default_model: "codellama:13b".to_string(),
            log_level: "info".to_string(),
            enable_telemetry: false,
        }
    }
}

/// Provider-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub ollama: OllamaProviderConfig,
    pub openai: OpenAIProviderConfig,
    pub local: LocalProviderConfig,
    pub default_provider: String,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            ollama: OllamaProviderConfig::default(),
            openai: OpenAIProviderConfig::default(),
            local: LocalProviderConfig::default(),
            default_provider: "ollama".to_string(),
        }
    }
}

/// Ollama provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaProviderConfig {
    pub endpoint: String,
    pub port: u16,
    pub timeout: Duration,
    pub models: Vec<String>,
    pub gpu_layers: Option<u32>,
    pub context_size: usize,
}

impl Default for OllamaProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost".to_string(),
            port: 11434,
            timeout: Duration::from_secs(120),
            models: vec!["codellama:13b".to_string()],
            gpu_layers: None,
            context_size: 4096,
        }
    }
}

/// OpenAI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderConfig {
    pub api_key: Option<String>,
    pub organization: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
}

impl Default for OpenAIProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            organization: None,
            base_url: None,
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Local provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProviderConfig {
    pub model_path: Option<PathBuf>,
    pub runtime: String, // "llama.cpp", "ggml", "onnx"
    pub threads: usize,
    pub batch_size: usize,
}

impl Default for LocalProviderConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            runtime: "llama.cpp".to_string(),
            threads: num_cpus::get(),
            batch_size: 512,
        }
    }
}

/// Resource limits and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f32,
    pub max_concurrent_operations: usize,
    pub max_file_size_mb: usize,
    pub command_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 4096,
            max_cpu_percent: 80.0,
            max_concurrent_operations: 10,
            max_file_size_mb: 100,
            command_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub sandbox_mode: bool,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub max_command_depth: usize,
    pub require_confirmation: bool,
    pub audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox_mode: true,
            allowed_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "echo".to_string(),
            ],
            blocked_commands: vec![
                "rm -rf".to_string(),
                "sudo".to_string(),
                "chmod 777".to_string(),
            ],
            max_command_depth: 5,
            require_confirmation: true,
            audit_logging: true,
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub cache_size_mb: usize,
    pub batch_processing: bool,
    pub async_operations: bool,
    pub prefetch_context: bool,
    pub compression: bool,
    pub rate_limiting: RateLimitConfig,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_size_mb: 512,
            batch_processing: true,
            async_operations: true,
            prefetch_context: true,
            compression: true,
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: usize,
    pub burst_size: usize,
    pub cooldown_period: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            cooldown_period: Duration::from_secs(60),
        }
    }
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub code_intelligence: bool,
    pub auto_completion: bool,
    pub syntax_highlighting: bool,
    pub git_integration: bool,
    pub debugging_tools: bool,
    pub performance_profiling: bool,
    pub security_scanning: bool,
    pub documentation_generation: bool,
    pub test_generation: bool,
    pub refactoring_suggestions: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            code_intelligence: true,
            auto_completion: true,
            syntax_highlighting: true,
            git_integration: true,
            debugging_tools: true,
            performance_profiling: false,
            security_scanning: true,
            documentation_generation: true,
            test_generation: true,
            refactoring_suggestions: true,
        }
    }
}

/// Configuration loading and management
impl CodingAgentConfig {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(self, other: Self) -> Self {
        // This is a simple merge - in production, use a proper merge strategy
        other
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Add validation logic here
        if self.resources.max_memory_mb == 0 {
            return Err(anyhow::anyhow!("Max memory cannot be 0"));
        }

        if self.resources.max_concurrent_operations == 0 {
            return Err(anyhow::anyhow!("Max concurrent operations cannot be 0"));
        }

        Ok(())
    }

    /// Get environment-specific configuration
    pub fn for_environment(env: &str) -> Self {
        match env {
            "development" => Self::development(),
            "production" => Self::production(),
            "test" => Self::test(),
            _ => Self::default(),
        }
    }

    /// Development configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.core.log_level = "debug".to_string();
        config.security.sandbox_mode = false;
        config.security.require_confirmation = false;
        config
    }

    /// Production configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.core.log_level = "warn".to_string();
        config.security.sandbox_mode = true;
        config.security.audit_logging = true;
        config.performance.cache_size_mb = 2048;
        config
    }

    /// Test configuration
    pub fn test() -> Self {
        let mut config = Self::default();
        config.core.session_timeout = Duration::from_secs(10);
        config.resources.max_concurrent_operations = 1;
        config
    }
}