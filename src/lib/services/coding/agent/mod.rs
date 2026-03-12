//! # Coding Agent Module - Consolidated and Refactored
//!
//! A comprehensive AI-powered coding assistant system that provides intelligent
//! code generation, command execution, and incremental task management.
//!
//! ## Architecture
//!
//! The module is organized into logical groups:
//!
//! ### Core Components
//! - `service`: Main service coordinator
//! - `constants`: System constants
//! - `utils`: Shared utilities
//!
//! ### Data Models & Types
//! - `types`: Legacy type definitions (being migrated)
//! - `models/`: New organized data models
//!   - `analysis`: Code analysis models
//!   - `conversation`: Conversation and messaging
//!   - `debugging`: Debugging-related models
//!   - `metrics`: Performance metrics
//!   - `review`: Code review models
//!   - `security`: Security scan models
//!
//! ### Configuration
//! - `config/`: Unified configuration system
//!   - `mod`: Master configuration
//!   - `base`: Configuration traits
//!   - `builder`: Fluent API builder
//!   - `validation`: Configuration validation
//!
//! ### Error Handling
//! - `errors`: Legacy error types (being migrated)
//! - `error/`: New comprehensive error system
//!   - `mod`: Unified error types
//!   - `context`: Error context and debugging
//!   - `recovery`: Recovery strategies
//!   - `reporting`: Error telemetry
//! - `error_handling`: Error recovery mechanisms
//! - `error_recovery`: Additional recovery logic
//!
//! ### Providers
//! - `providers/`: LLM provider implementations
//!   - `ollama`: Ollama provider
//!   - `openai`: OpenAI provider
//!   - `local`: Local model provider
//!   - `manager`: Provider management
//! - `provider_base`: Base provider implementation
//! - `remote_ollama`: Remote Ollama support
//! - `ollama_config_manager`: Ollama configuration
//! - `ollama_auto_config`: Auto-configuration for Ollama
//!
//! ### Execution & Task Management
//! - `executor/`: Modern async executor
//! - `command_executor`: Command execution
//! - `interactive_executor`: Interactive execution
//! - `execution_context`: Execution context management
//! - `execution_state`: State tracking
//! - `step_parser`: Step parsing for incremental execution
//!
//! ### Resource Management
//! - `resource_limits`: Resource limiting
//! - `resource_manager`: Advanced resource management
//! - `gpu_offload`: GPU offloading support
//!
//! ### Code Intelligence & Analysis
//! - `code_intelligence`: Code analysis and intelligence
//! - `workspace_analyzer`: Workspace analysis
//! - `dependency_analyzer`: Dependency analysis
//! - `code_metrics_dashboard`: Code metrics
//! - `code_flow_visualizer`: Code flow visualization
//! - `bug_predictor`: Bug prediction
//!
//! ### Code Generation & Refactoring
//! - `scaffolding`: Project scaffolding
//! - `refactoring`: Code refactoring
//! - `automated_refactoring`: Automated refactoring
//! - `paradigm_translator`: Paradigm translation
//! - `api_client_generator`: API client generation
//! - `migration`: Code migration
//!
//! ### Testing & Debugging
//! - `testing`: Testing utilities
//! - `advanced_testing`: Advanced testing features
//! - `test_generation`: Test generation
//! - `debugging`: Debugging utilities
//! - `automated_debugging`: Automated debugging
//!
//! ### Documentation & Review
//! - `documentation_generator`: Documentation generation
//! - `code_review`: Code review
//! - `code_explanation`: Code explanation
//!
//! ### Collaboration
//! - `collaboration`: Collaboration features
//! - `pair_programming`: Pair programming
//! - `distributed_collaboration`: Distributed collaboration
//! - `realtime_collaboration`: Real-time collaboration
//!
//! ### Search & Completion
//! - `completion`: Code completion
//! - `intelligent_completion`: Intelligent completion
//! - `ai_code_search`: AI-powered code search
//! - `multi_language_search`: Multi-language search
//!
//! ### Performance & Security
//! - `performance_profiler`: Performance profiling
//! - `performance_optimizer`: Performance optimization
//! - `security_analyzer`: Security analysis
//! - `benchmarking`: Benchmarking utilities
//!
//! ### Machine Learning
//! - `model_training`: Model training
//! - `continuous_learning`: Continuous learning
//!
//! ### Version Control
//! - `git_integration`: Git integration
//!
//! ### Templates & Context
//! - `templates`: Template management
//! - `context`: Context management
//! - `metrics`: Metrics collection
//!
//! ### Traits & Interfaces
//! - `traits/`: Standardized interfaces
//!   - `analyzer`: Analysis traits
//!   - `executor`: Execution traits
//!   - `generator`: Generation traits
//!   - `provider`: Provider traits
//!
//! ### I/O & File Operations
//! - `io/`: Modern async I/O operations

// Core modules
pub mod constants;
pub mod utils;
pub mod service;

// Type definitions (consolidating)
pub mod types;
pub mod models;

// Configuration system
pub mod config;

// Error handling system
pub mod errors; // Legacy - being migrated
pub mod error;
pub mod error_handling;
pub mod error_recovery;

// Provider system
pub mod providers;
pub mod provider_base;
pub mod remote_ollama;
pub mod ollama_config_manager;
pub mod ollama_auto_config;

// Execution and task management
pub mod executor;
pub mod command_executor;
pub mod interactive_executor;
pub mod execution_context;
pub mod execution_state;
pub mod step_parser;

// Resource management
pub mod resource_limits;
pub mod resource_manager;
pub mod gpu_offload;

// Code intelligence and analysis
pub mod code_intelligence;
pub mod workspace_analyzer;
pub mod dependency_analyzer;
pub mod code_metrics_dashboard;
pub mod code_flow_visualizer;
pub mod bug_predictor;

// Code generation and refactoring
pub mod scaffolding;
pub mod refactoring;
pub mod automated_refactoring;
pub mod paradigm_translator;
pub mod api_client_generator;
pub mod migration;

// Testing and debugging
pub mod testing;
pub mod advanced_testing;
pub mod test_generation;
pub mod debugging;
pub mod automated_debugging;

// Documentation and review
pub mod documentation_generator;
pub mod code_review;
pub mod code_explanation;

// Collaboration features
pub mod collaboration;
pub mod pair_programming;
pub mod distributed_collaboration;
pub mod realtime_collaboration;

// Search and completion
pub mod completion;
pub mod intelligent_completion;
pub mod ai_code_search;
pub mod multi_language_search;

// Performance and security
pub mod performance_profiler;
pub mod performance_optimizer;
pub mod security_analyzer;
pub mod benchmarking;

// Machine learning
pub mod model_training;
pub mod continuous_learning;

// Version control
pub mod git_integration;

// Templates and context
pub mod templates;
pub mod context;
pub mod metrics;

// Traits and interfaces
pub mod traits;

// I/O operations
pub mod io;

// ==============================================================================
// PUBLIC API EXPORTS
// ==============================================================================

// Re-export main service types
pub use service::{CodingAgentService, ConversationMessage, ConversationMemory};

// Re-export executor types
pub use executor::{CodingAgentExecutor, UserMessage, EnhancedContext};
pub use interactive_executor::{InteractiveExecutor, ExecutionContext as InteractiveContext};
pub use command_executor::CommandExecutor;

// Re-export common types
pub use types::{
    CodeExecutionRequest,
    CodingAgentResponse,
    CommandHistoryEntry,
    ProjectStructure,
    ProjectType,
    RiskLevel,
    BuildSystem,
};

// Re-export execution state types
pub use execution_state::{
    IncrementalExecution,
    ExecutionState,
    ExecutionStep,
};

// Re-export error types
pub use errors::{CodingAgentError, CodingAgentResult, ErrorSeverity};

// Re-export configuration
pub use config::CodingAgentConfig;

// Re-export provider types
pub use providers::{ProviderManager, OllamaProvider, OpenAIProvider, LocalProvider};

// Re-export resource management
pub use resource_limits::{ResourceLimits, ResourceMonitor};
pub use resource_manager::ResourceManager;

// Re-export code intelligence
pub use code_intelligence::{CodeIntelligence, FileAnalysis, CodeIssue};

// ==============================================================================
// FEATURE FLAGS
// ==============================================================================

/// Initialize the coding agent with default configuration
pub async fn initialize() -> Result<CodingAgentService, CodingAgentError> {
    let config = CodingAgentConfig::default();
    Ok(CodingAgentService::new(config).await)
}

/// Initialize with custom configuration
pub async fn initialize_with_config(config: CodingAgentConfig) -> Result<CodingAgentService, CodingAgentError> {
    Ok(CodingAgentService::new(config).await)
}

/// Get version information
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get module information
pub fn info() -> ModuleInfo {
    ModuleInfo {
        name: "coding_agent".to_string(),
        version: version().to_string(),
        description: "AI-powered coding assistant".to_string(),
        authors: vec!["Anthropic".to_string()],
        features: get_enabled_features(),
    }
}

/// Module information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub features: Vec<String>,
}

fn get_enabled_features() -> Vec<String> {
    vec![
        "core".to_string(),
        "providers".to_string(),
        "execution".to_string(),
        "analysis".to_string(),
        "models".to_string(),
        "traits".to_string(),
        "io".to_string(),
        "error".to_string(),
    ]
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_info() {
        let info = info();
        assert_eq!(info.name, "coding_agent");
        assert!(!info.features.is_empty());
    }

    #[tokio::test]
    async fn test_initialization() {
        let result = initialize().await;
        assert!(result.is_ok());
    }
}