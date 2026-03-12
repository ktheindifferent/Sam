//! Constants and configuration values for the coding agent module.
//!
//! This module centralizes all magic numbers, strings, and configuration
//! constants to improve maintainability and avoid duplication.

// Provider names
pub const PROVIDER_OLLAMA: &str = "ollama";
pub const PROVIDER_OPENAI: &str = "openai";
pub const PROVIDER_LOCAL: &str = "local";

// Default model names
pub const DEFAULT_OLLAMA_MODEL: &str = "codellama:latest";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4";

// Network defaults
pub const DEFAULT_LOCALHOST: &str = "localhost";
pub const DEFAULT_OLLAMA_PORT: u16 = 11434;
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

// Resource limits
pub const MAX_MEMORY_MB: usize = 512;
pub const MAX_MEMORY_BYTES: usize = MAX_MEMORY_MB * 1024 * 1024;
pub const MAX_CPU_SECONDS: u64 = 60;
pub const MAX_CONCURRENT_OPERATIONS: usize = 10;
pub const MAX_OUTPUT_MB: usize = 1;
pub const MAX_OUTPUT_BYTES: usize = MAX_OUTPUT_MB * 1024 * 1024;

// Conversation limits
pub const MAX_CONVERSATION_MESSAGES: usize = 20;
pub const MAX_CONVERSATION_TOKENS: usize = 8192;
pub const MAX_COMMAND_HISTORY: usize = 100;

// UI and display
pub const MAX_DISPLAY_DEPENDENCIES: usize = 10;
pub const MAX_LOG_LINE_LENGTH: usize = 500;
pub const COMMAND_OUTPUT_TRIM_LENGTH: usize = 1000;

// Retry and timeout settings
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_DELAY_SECONDS: u64 = 2;
pub const MAX_RETRY_DELAY_SECONDS: u64 = 60;
pub const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
pub const CIRCUIT_BREAKER_TIMEOUT_SECONDS: u64 = 30;

// Execution settings
pub const COMMAND_EXECUTION_PAUSE_MS: u64 = 200;
pub const SPINNER_UPDATE_INTERVAL_MS: u64 = 100;

// File and path limits
pub const MAX_FILE_SIZE_MB: usize = 10;
pub const MAX_FILE_SIZE_BYTES: usize = MAX_FILE_SIZE_MB * 1024 * 1024;
pub const MAX_PATH_DEPTH: usize = 10;

// Cache settings
pub const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes
pub const MAX_CACHE_ENTRIES: usize = 1000;

// Performance thresholds
pub const HIGH_COMPLEXITY_THRESHOLD: f64 = 10.0;
pub const SLOW_OPERATION_THRESHOLD_MS: u64 = 1000;

// String constants for common patterns
pub const CARGO_NEW_PREFIX: &str = "cargo new ";
pub const CD_COMMAND_PREFIX: &str = "cd ";
pub const HEREDOC_MARKER: &str = "EOF";

// Safe commands (can be moved to config)
pub const DEFAULT_SAFE_COMMANDS: &[&str] = &[
    "echo", "cat", "ls", "pwd", "mkdir", "touch", "cp", "mv", "rm",
    "cargo", "rustc", "git", "npm", "node", "python", "pip",
    "grep", "find", "sed", "awk", "sort", "uniq", "head", "tail",
];