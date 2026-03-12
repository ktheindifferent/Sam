use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::{Mutex, RwLock};
use log::{info, debug, warn, error};
use serde::{Serialize, Deserialize};

use super::config::CodingAgentConfig;
use super::constants::*;
use super::types::{CodingAgentResponse, CommandHistoryEntry, CodeExecutionRequest, RiskLevel};
use super::providers::{ProviderManager, OllamaProvider};
use super::utils;
use crate::services::llms::ollama::{OllamaService, OllamaConfig};
use super::context::ContextManager;
use super::templates::TemplateManager;
use super::metrics::MetricsManager;
use super::code_intelligence::{CodeIssue, IssueSeverity, IssueCategory};
use super::workspace_analyzer::{WorkspaceAnalyzer, WorkspaceAnalysis};
use super::ollama_config_manager::OllamaConfigManager;

/// Conversation message for multi-turn support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String, // "user", "assistant", or "system"
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

/// Conversation memory with context window management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    pub messages: Vec<ConversationMessage>,
    pub max_messages: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
}

/// Main coding agent service that coordinates all functionality
pub struct CodingAgentService {
    config: CodingAgentConfig,
    providers: Arc<Mutex<ProviderManager>>,
    context_manager: Arc<RwLock<ContextManager>>,
    template_manager: Arc<RwLock<TemplateManager>>,
    metrics_manager: Arc<RwLock<MetricsManager>>,
    command_history: Arc<Mutex<Vec<CommandHistoryEntry>>>,
    conversation_memory: Arc<Mutex<ConversationMemory>>,
    streaming_enabled: Arc<RwLock<bool>>,
    configured_model: String,  // Store the model from Ollama config
}

// Manual Debug implementation since we can't derive it with trait objects
impl std::fmt::Debug for CodingAgentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodingAgentService")
            .field("config", &self.config)
            .field("command_history_len", &"<vec>")
            .finish()
    }
}

impl CodingAgentService {
    pub async fn new(config: CodingAgentConfig) -> Self {
        let mut provider_manager = ProviderManager::new();

        // Try to load Ollama configuration from file
        let (ollama_config, configured_model) = if let Ok(config_manager) = OllamaConfigManager::new().await {
            let model = config_manager.get_selected_model().unwrap_or_else(|| DEFAULT_OLLAMA_MODEL).to_string();
            if let Some(endpoint) = config_manager.get_current_endpoint() {
                info!("Using Ollama server from config: {} with model: {}", endpoint, model);
                (OllamaConfig::from_endpoint(&endpoint, config.ollama_timeout_seconds), model)
            } else {
                info!("No Ollama server configured, using defaults");
                (OllamaConfig {
                    host: DEFAULT_LOCALHOST.to_string(),
                    port: DEFAULT_OLLAMA_PORT,
                    timeout_seconds: config.ollama_timeout_seconds,
                    custom_endpoint: None,
                }, config.default_model.clone())
            }
        } else {
            info!("Could not load Ollama config, using defaults");
            (OllamaConfig {
                host: DEFAULT_LOCALHOST.to_string(),
                port: DEFAULT_OLLAMA_PORT,
                timeout_seconds: config.ollama_timeout_seconds,
                custom_endpoint: None,
            }, config.default_model.clone())
        };
        let ollama_service = Arc::new(OllamaService::new(ollama_config));
        let ollama_provider = OllamaProvider::new(ollama_service);
        provider_manager.add_provider(PROVIDER_OLLAMA.to_string(), Box::new(ollama_provider));

        // Set default provider
        provider_manager.set_default_provider(PROVIDER_OLLAMA.to_string());
        info!("Provider manager initialized with ollama as default provider");

        Self {
            config,
            providers: Arc::new(Mutex::new(provider_manager)),
            context_manager: Arc::new(RwLock::new(ContextManager::new())),
            template_manager: Arc::new(RwLock::new(TemplateManager::new())),
            metrics_manager: Arc::new(RwLock::new(MetricsManager::new())),
            command_history: Arc::new(Mutex::new(Vec::new())),
            conversation_memory: Arc::new(Mutex::new(ConversationMemory {
                messages: Vec::new(),
                max_messages: MAX_CONVERSATION_MESSAGES,
                total_tokens: 0,
                max_tokens: MAX_CONVERSATION_TOKENS,
            })),
            streaming_enabled: Arc::new(RwLock::new(false)),
            configured_model,
        }
    }

    pub async fn new_with_defaults() -> Self {
        Self::new(CodingAgentConfig::default()).await
    }

    /// Check if the service is available
    pub async fn is_available(&self) -> bool {
        let providers = self.providers.lock().await;
        providers.is_current_provider_available().await
    }

    /// Generate response using the coding agent
    pub async fn generate_response(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        model_override: Option<&str>,
    ) -> Result<CodingAgentResponse> {
        info!("generate_response called with input length: {}", user_input.len());

        // Check for direct command execution in user input
        let (processed_input, direct_commands) = self.preprocess_user_input(user_input);
        
        if !direct_commands.is_empty() {
            debug!("Direct commands detected: {:?}", direct_commands);
            return Ok(CodingAgentResponse {
                response_text: format!("I'll execute the following commands:\n{}", 
                    direct_commands.iter()
                        .map(|cmd| format!("- {}", cmd.command))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                suggested_commands: direct_commands,
                model_used: "direct".to_string(),
                context_used: 0,
            });
        }

        // Check for file modification requests first
        info!("Checking for file modification patterns");
        if let Some(response) = self.detect_and_handle_file_modification(user_input, current_dir, session_context).await? {
            info!("File modification pattern detected, returning early response");
            return Ok(response);
        }

        // Generate enhanced response using AI
        info!("No file modification pattern, calling generate_enhanced_response_internal");
        self.generate_enhanced_response_internal(user_input, current_dir, session_context, model_override).await
    }

    /// Update context manager with current directory
    async fn update_context(&self, current_dir: &PathBuf) -> Result<()> {
        info!("Updating context manager");
        let mut context_manager = self.context_manager.write().await;
        context_manager.update_workspace_context(current_dir).await?;
        info!("Context updated");
        Ok(())
    }

    /// Call LLM provider with timeout
    async fn call_llm_with_timeout(
        &self,
        prompt: &str,
        model: &str,
        provider_hint: Option<&str>,
    ) -> Result<(String, std::time::Duration)> {
        let start_time = std::time::Instant::now();
        let mut providers = self.providers.lock().await;

        info!("Requesting LLM response with model: {} (timeout: {}s)",
            model, self.config.ollama_timeout_seconds);

        let timeout_duration = std::time::Duration::from_secs(self.config.ollama_timeout_seconds);
        let response_text = match tokio::time::timeout(
            timeout_duration,
            providers.generate_response(prompt, model)
        ).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                error!("LLM provider error: {}", e);
                return Err(anyhow::anyhow!("LLM provider error: {}", e));
            }
            Err(_) => {
                error!("LLM request timed out after {}s", self.config.ollama_timeout_seconds);
                return Err(anyhow::anyhow!("LLM request timed out after {} seconds",
                    self.config.ollama_timeout_seconds));
            }
        };

        drop(providers);
        let response_time = start_time.elapsed();
        Ok((response_text, response_time))
    }

    /// Record AI generation metrics
    async fn record_generation_metrics(&self, model: &str, success: bool, duration: std::time::Duration) {
        let mut metrics = self.metrics_manager.write().await;
        metrics.record_command_execution(
            &format!("generate_response_{}", model),
            success,
            duration,
            "ai_generation"
        );
    }

    /// Generate enhanced response for complex requests (internal, bypasses file detection)
    async fn generate_enhanced_response_internal(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        model_override: Option<&str>,
    ) -> Result<CodingAgentResponse> {
        info!("generate_enhanced_response_internal called");

        // Update context
        self.update_context(current_dir).await?;

        // Build system prompt with enhanced context
        let system_prompt = self.build_system_prompt(current_dir, session_context).await?;

        // Prepare the full prompt
        let full_prompt = format!("{}\n\nUser Request: {}", system_prompt, user_input);

        // Select model - use configured model from Ollama config
        let model_to_use = model_override.unwrap_or(&self.configured_model);

        // Generate response using current provider with timeout
        let (response_text, response_time) = self.call_llm_with_timeout(
            &full_prompt,
            model_to_use,
            Some(PROVIDER_OLLAMA)
        ).await?;

        info!("LLM response received in {:?} (length: {} chars)", response_time, response_text.len());

        // Parse commands from response
        let suggested_commands = self.parse_commands_from_response(&response_text);

        // Record metrics
        self.record_generation_metrics(model_to_use, true, response_time).await;

        Ok(CodingAgentResponse {
            response_text,
            suggested_commands,
            model_used: model_to_use.to_string(),
            context_used: session_context.len(),
        })
    }

    /// Build system prompt with current context
    async fn build_system_prompt(
        &self,
        current_dir: &PathBuf,
        session_lines: &[String]
    ) -> Result<String> {
        let context_manager = self.context_manager.read().await;
        let enhanced_context = context_manager.get_enhanced_context(current_dir, session_lines).await?;
        
        let available_models = {
            let providers = self.providers.lock().await;
            providers.list_available_models().await.unwrap_or_default()
        };

        let session_context = self.build_context(session_lines);
        
        // Replace placeholders in template
        let prompt = self.config.system_prompt_template
            .replace("{current_dir}", &current_dir.display().to_string())
            .replace("{system_info}", &enhanced_context)
            .replace("{available_models}", &available_models.join(", "))
            .replace("{context_lines}", &self.config.max_context_lines.to_string())
            .replace("{session_context}", &session_context)
            .replace("{safe_commands}", &self.config.safe_commands.join(", "));

        Ok(prompt)
    }

    /// Build context from session history
    fn build_context(&self, session_lines: &[String]) -> String {
        let max_lines = self.config.max_context_lines;
        let context_lines = if session_lines.len() > max_lines {
            &session_lines[session_lines.len() - max_lines..]
        } else {
            session_lines
        };
        context_lines.join("\n")
    }

    /// Preprocess user input to handle direct command execution
    fn preprocess_user_input(&self, input: &str) -> (String, Vec<CodeExecutionRequest>) {
        let mut direct_commands = Vec::new();
        let input_lower = input.to_lowercase();

        // Check for direct command patterns
        if input_lower.starts_with("run ") || input_lower.starts_with("execute ") || input_lower.starts_with("exec ") {
            let command = if input_lower.starts_with("run ") {
                input[4..].trim()
            } else if input_lower.starts_with("execute ") {
                input[8..].trim()
            } else {
                input[5..].trim()
            };

            if self.config.is_safe_command(command) {
                direct_commands.push(self.create_execution_request(
                    command.to_string(),
                    format!("Execute: {}", command),
                    self.assess_command_risk(command),
                    Some(30)
                ));
            }
        }

        // Check for backtick-wrapped commands
        if let Ok(regex) = regex::Regex::new(r"`([^`]+)`") {
            for captures in regex.captures_iter(input) {
                if let Some(command_match) = captures.get(1) {
                    let command = command_match.as_str();
                    if self.is_recognizable_command(command) {
                        direct_commands.push(self.create_execution_request(
                            command.to_string(),
                            format!("Execute: {}", command),
                            self.assess_command_risk(command),
                            Some(30)
                        ));
                    }
                }
            }
        }

        (input.to_string(), direct_commands)
    }

    /// Parse agent response for command suggestions
    fn parse_commands_from_response(&self, response: &str) -> Vec<CodeExecutionRequest> {
        let mut commands = Vec::new();

        // Look for backtick-wrapped commands
        if let Ok(regex) = regex::Regex::new(r"`([^`]+)`") {
            for captures in regex.captures_iter(response) {
                if let Some(command_match) = captures.get(1) {
                    let command = command_match.as_str();
                    if self.is_recognizable_command(command) {
                        commands.push(self.create_execution_request(
                            command.to_string(),
                            format!("Suggested: {}", command),
                            self.assess_command_risk(command),
                            None
                        ));
                    }
                }
            }
        }

        // Look for markdown code blocks
        if let Ok(regex) = regex::Regex::new(r"```(?:bash|shell|sh)?\n(.+?)\n```") {
            for captures in regex.captures_iter(response) {
                if let Some(code_block) = captures.get(1) {
                    for line in code_block.as_str().lines() {
                        let command = line.trim();
                        if !command.is_empty() && !command.starts_with('#') && self.is_recognizable_command(command) {
                            commands.push(self.create_execution_request(
                                command.to_string(),
                                format!("Code block: {}", command),
                                self.assess_command_risk(command),
                                None
                            ));
                        }
                    }
                }
            }
        }

        commands
    }

    /// Check if a command is recognizable (not just safe)
    fn is_recognizable_command(&self, command: &str) -> bool {
        let base_cmd = command.split_whitespace().next().unwrap_or("");
        
        // List of recognizable commands (broader than safe commands)
        let recognizable_commands = [
            "ls", "cat", "pwd", "echo", "mkdir", "touch", "cp", "mv", "rm",
            "grep", "find", "head", "tail", "wc", "sort", "uniq", "cargo",
            "git", "tree", "file", "stat", "which", "basename", "dirname",
            "clear", "date", "whoami", "id", "uname", "rustc", "rustfmt",
            "sed", "awk", "curl", "wget", "python", "python3", "npm", "node",
            "yarn", "pip", "pip3", "make", "cmake", "gcc", "clang", "java",
            "javac", "go", "docker", "kubectl", "ssh", "cd", "vim", "nano",
            "code", "open", "chmod", "chown", "ps", "top", "kill", "tar",
            "zip", "unzip", "diff", "patch", "less", "more",
        ];

        recognizable_commands.contains(&base_cmd)
    }

    /// Detect and handle file modification requests
    async fn detect_and_handle_file_modification(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_lines: &[String],
    ) -> Result<Option<CodingAgentResponse>> {
        let input_lower = user_input.to_lowercase();

        // Check for file modification keywords
        let modification_keywords = [
            "edit", "modify", "change", "update", "add to", "append to",
            "insert into", "replace in", "delete from", "remove from"
        ];

        let has_modification_keyword = modification_keywords.iter()
            .any(|keyword| input_lower.contains(keyword));

        if !has_modification_keyword {
            debug!("No modification keywords found in input");
            return Ok(None);
        }

        debug!("Modification keywords detected, looking for target file");

        // Try to detect the target file
        if let Some(target_file) = self.detect_target_file(user_input, current_dir) {
            debug!("Target file detected: {}", target_file);
            // Read the file first
            if let Ok(file_content) = tokio::fs::read_to_string(&target_file).await {
                debug!("File content read successfully, length: {}", file_content.len());
                let enhanced_prompt = format!(
                    "File modification request for: {}\n\nCurrent file content:\n```\n{}\n```\n\nUser request: {}\n\nProvide specific commands to make these changes:",
                    target_file,
                    file_content,
                    user_input
                );

                return Ok(Some(self.generate_enhanced_response_internal(
                    &enhanced_prompt,
                    current_dir,
                    session_lines,
                    None
                ).await?));
            }
        }

        Ok(None)
    }

    /// Detect target file from user input
    fn detect_target_file(&self, user_input: &str, current_dir: &PathBuf) -> Option<String> {
        // Common file extensions to look for
        let file_extensions = [".rs", ".py", ".js", ".ts", ".json", ".toml", ".yaml", ".yml", ".md", ".txt"];
        
        for ext in &file_extensions {
            if let Ok(regex) = regex::Regex::new(&format!(r"(\w+{})", regex::escape(ext))) {
                if let Some(captures) = regex.captures(user_input) {
                    if let Some(filename) = captures.get(1) {
                        let file_path = current_dir.join(filename.as_str());
                        if file_path.exists() {
                            return Some(filename.as_str().to_string());
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Execute a command safely
    pub async fn execute_command(&self, command: &str, require_confirmation: bool) -> Result<String> {
        // Use the current working directory from environment
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        self.execute_command_in_dir(command, &current_dir, require_confirmation).await
    }

    /// Execute a command in a specific directory
    pub async fn execute_command_in_dir(&self, command: &str, working_dir: &std::path::PathBuf, require_confirmation: bool) -> Result<String> {
        // Validate command
        if !self.config.is_safe_command(command) {
            return Err(anyhow::anyhow!("Command '{}' is not in the safe command list", command));
        }

        // Record command in history
        let timestamp = utils::unix_timestamp();

        let start_time = std::time::Instant::now();

        // Actually execute the command using tokio::process with specific working directory
        let output = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .output()
            .await
        {
            Ok(result) => {
                if result.status.success() {
                    String::from_utf8_lossy(&result.stdout).to_string()
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();

                    // Format the error output more cleanly
                    if !stderr.is_empty() && stdout.is_empty() {
                        stderr
                    } else if stderr.is_empty() && !stdout.is_empty() {
                        stdout
                    } else if !stderr.is_empty() && !stdout.is_empty() {
                        format!("{}\n{}", stdout, stderr)
                    } else {
                        format!("Command '{}' failed with exit code: {}", command, result.status.code().unwrap_or(-1))
                    }
                }
            }
            Err(e) => {
                format!("Failed to execute command: {}", e)
            }
        };

        let success = !output.contains("Command failed") && !output.contains("Failed to execute");

        let execution_time = start_time.elapsed();

        // Record in history
        {
            let mut history = self.command_history.lock().await;
            history.push(CommandHistoryEntry {
                command: command.to_string(),
                timestamp,
                success,
                output: output.clone(),
                working_directory: working_dir.to_string_lossy().to_string(),
            });
        }

        // Record metrics
        {
            let mut metrics = self.metrics_manager.write().await;
            metrics.record_command_execution(command, success, execution_time, "user_command");
        }

        Ok(output)
    }

    /// Get service status
    pub async fn get_status(&self) -> HashMap<String, serde_json::Value> {
        let mut status = HashMap::new();

        // Provider status
        {
            let providers = self.providers.lock().await;
            let current_provider = providers.get_current_provider_name().await;
            let available_providers = providers.list_providers().await;
            let provider_status = match current_provider.as_ref() {
                Some(name) => providers.get_provider_status(name).await.unwrap_or(false),
                None => false
            };
            status.insert("providers".to_string(), serde_json::json!({
                "current": current_provider,
                "available": available_providers,
                "status": provider_status
            }));
        }

        // Metrics
        {
            let metrics = self.metrics_manager.read().await;
            status.insert("metrics".to_string(), serde_json::json!(metrics.get_performance_metrics()));
        }

        // Command history count
        {
            let history = self.command_history.lock().await;
            status.insert("command_history_count".to_string(), serde_json::Value::Number(history.len().into()));
        }

        // Configuration
        status.insert("config".to_string(), serde_json::json!({
            "default_model": self.config.default_model,
            "safe_commands_count": self.config.safe_commands.len(),
            "require_confirmation": self.config.require_confirmation,
            "workspace_integration": self.config.workspace_integration
        }));

        status
    }

    /// Check if a command is in the safe list
    pub fn is_safe_command(&self, command: &str) -> bool {
        self.config.is_safe_command(command)
    }

    /// Update configuration
    pub fn update_config(&mut self, new_config: CodingAgentConfig) {
        self.config = new_config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &CodingAgentConfig {
        &self.config
    }

    /// Get command history
    pub async fn get_command_history(&self) -> Vec<CommandHistoryEntry> {
        self.command_history.lock().await.clone()
    }

    /// Get recent command history
    pub async fn get_recent_command_history(&self, limit: usize) -> Vec<CommandHistoryEntry> {
        let history = self.command_history.lock().await;
        if history.len() <= limit {
            history.clone()
        } else {
            history[history.len() - limit..].to_vec()
        }
    }

    /// Clear command history
    pub async fn clear_command_history(&self) {
        self.command_history.lock().await.clear();
    }

    /// Create an execution request with risk assessment
    fn create_execution_request(
        &self,
        command: String,
        explanation: String,
        risk_level: RiskLevel,
        estimated_duration: Option<u32>,
    ) -> CodeExecutionRequest {
        CodeExecutionRequest {
            command: command.clone(),
            require_confirmation: self.config.require_confirmation || matches!(risk_level, RiskLevel::High | RiskLevel::Critical),
            explanation,
            risk_level,
            estimated_duration,
            prerequisites: self.analyze_command_prerequisites(&command),
            expected_outputs: self.predict_command_outputs(&command),
        }
    }

    /// Analyze command prerequisites
    fn analyze_command_prerequisites(&self, command: &str) -> Vec<String> {
        let mut prerequisites = Vec::new();
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        
        if cmd_parts.is_empty() {
            return prerequisites;
        }

        match cmd_parts[0] {
            "cargo" => {
                prerequisites.push("Rust toolchain installed".to_string());
                if cmd_parts.get(1) == Some(&"build") || cmd_parts.get(1) == Some(&"run") {
                    prerequisites.push("Cargo.toml file present".to_string());
                }
            }
            "npm" | "yarn" => {
                prerequisites.push("Node.js installed".to_string());
                if cmd_parts.get(1) == Some(&"install") || cmd_parts.get(1) == Some(&"run") {
                    prerequisites.push("package.json file present".to_string());
                }
            }
            "git" => {
                prerequisites.push("Git installed".to_string());
                if cmd_parts.get(1) != Some(&"init") {
                    prerequisites.push("Git repository initialized".to_string());
                }
            }
            "docker" => {
                prerequisites.push("Docker installed and running".to_string());
            }
            _ => {}
        }

        prerequisites
    }

    /// Predict command outputs
    fn predict_command_outputs(&self, command: &str) -> Vec<String> {
        let mut outputs = Vec::new();
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        
        if cmd_parts.is_empty() {
            return outputs;
        }

        match cmd_parts[0] {
            "ls" => outputs.push("Directory listing".to_string()),
            "cat" => outputs.push("File contents".to_string()),
            "mkdir" => outputs.push("Directory created".to_string()),
            "cargo" => {
                match cmd_parts.get(1) {
                    Some(&"build") => outputs.push("Compilation results".to_string()),
                    Some(&"run") => outputs.push("Program output".to_string()),
                    Some(&"test") => outputs.push("Test results".to_string()),
                    _ => outputs.push("Cargo command output".to_string()),
                }
            }
            "git" => {
                match cmd_parts.get(1) {
                    Some(&"status") => outputs.push("Repository status".to_string()),
                    Some(&"log") => outputs.push("Commit history".to_string()),
                    _ => outputs.push("Git command output".to_string()),
                }
            }
            _ => outputs.push("Command output".to_string()),
        }

        outputs
    }

    /// Assess command risk level
    fn assess_command_risk(&self, command: &str) -> RiskLevel {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        
        if cmd_parts.is_empty() {
            return RiskLevel::Safe;
        }

        match cmd_parts[0] {
            "rm" => {
                if cmd_parts.contains(&"-rf") || cmd_parts.contains(&"-r") {
                    RiskLevel::Critical
                } else {
                    RiskLevel::High
                }
            }
            "sudo" | "chmod" | "chown" => RiskLevel::Critical,
            "mv" | "cp" => {
                if cmd_parts.len() > 2 {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                }
            }
            "cargo" => {
                match cmd_parts.get(1) {
                    Some(&"install") => RiskLevel::Medium,
                    Some(&"build") | Some(&"run") | Some(&"test") => RiskLevel::Low,
                    _ => RiskLevel::Safe,
                }
            }
            "npm" | "yarn" => {
                match cmd_parts.get(1) {
                    Some(&"install") => RiskLevel::Medium,
                    _ => RiskLevel::Low,
                }
            }
            "git" => {
                match cmd_parts.get(1) {
                    Some(&"push") | Some(&"pull") => RiskLevel::Medium,
                    Some(&"reset") | Some(&"rebase") => RiskLevel::High,
                    _ => RiskLevel::Low,
                }
            }
            "ls" | "cat" | "pwd" | "echo" | "grep" | "find" => RiskLevel::Safe,
            _ => RiskLevel::Low,
        }
    }

    // ===== Conversation Memory Management =====

    /// Add a message to conversation memory
    pub async fn add_to_conversation(&self, role: &str, content: &str) {
        let mut memory = self.conversation_memory.lock().await;

        let message = ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now(),
        };

        memory.messages.push(message);

        // Manage context window - keep only recent messages
        if memory.messages.len() > memory.max_messages {
            let excess = memory.messages.len() - memory.max_messages;
            memory.messages.drain(0..excess);
        }

        // Estimate tokens (rough approximation: 1 token ≈ 4 chars)
        memory.total_tokens = memory.messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();
    }

    /// Get conversation history for context
    pub async fn get_conversation_context(&self) -> Vec<ConversationMessage> {
        self.conversation_memory.lock().await.messages.clone()
    }

    /// Clear conversation memory
    pub async fn clear_conversation(&self) {
        let mut memory = self.conversation_memory.lock().await;
        memory.messages.clear();
        memory.total_tokens = 0;
    }

    /// Enable or disable streaming mode
    pub async fn set_streaming_mode(&self, enabled: bool) {
        *self.streaming_enabled.write().await = enabled;
    }

    /// Check if streaming is enabled
    pub async fn is_streaming_enabled(&self) -> bool {
        *self.streaming_enabled.read().await
    }

    /// Generate response with conversation context
    pub async fn generate_contextual_response(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        model_override: Option<&str>,
    ) -> Result<CodingAgentResponse> {
        // Add user message to conversation
        self.add_to_conversation("user", user_input).await;

        // Build context from conversation history
        let conversation = self.get_conversation_context().await;
        let mut context_lines: Vec<String> = Vec::new();

        for msg in &conversation {
            if msg.role != "system" {
                context_lines.push(format!("{}: {}", msg.role, msg.content));
            }
        }

        // Generate response
        let response = self.generate_response(
            user_input,
            current_dir,
            &context_lines,
            model_override
        ).await?;

        // Add assistant response to conversation
        self.add_to_conversation("assistant", &response.response_text).await;

        Ok(response)
    }

    /// Generate streaming response (returns a channel for real-time updates)
    pub async fn generate_streaming_response(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        model_override: Option<&str>,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Clone necessary data for the async task
        let input = user_input.to_string();
        let dir = current_dir.clone();
        let model = model_override.map(|s| s.to_string());
        let providers = self.providers.clone();
        let context_manager = self.context_manager.clone();
        let config = self.config.clone();

        // Spawn async task for streaming
        tokio::spawn(async move {
            // Update context
            if let Ok(mut ctx) = context_manager.try_write() {
                let _ = ctx.update_workspace_context(&dir).await;
            }

            // Build prompt
            let prompt = format!("User Request: {}", input);
            let model_to_use = model.as_deref().unwrap_or(&config.default_model);

            // Stream response chunks
            if let Ok(mut provs) = providers.try_lock() {
                // For now, send complete response in chunks (can be enhanced with actual streaming)
                match provs.generate_response(&prompt, model_to_use).await {
                    Ok(response) => {
                        // Simulate streaming by sending response in chunks
                        let chunk_size = 50; // Send 50 chars at a time
                        for chunk in response.chars().collect::<Vec<_>>().chunks(chunk_size) {
                            let chunk_str: String = chunk.iter().collect();
                            if tx.send(chunk_str).await.is_err() {
                                break; // Receiver dropped
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Error: {}", e)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Manage conversation context window to stay within token limits
    pub async fn compress_conversation_if_needed(&self) {
        let mut memory = self.conversation_memory.lock().await;

        if memory.total_tokens > memory.max_tokens * 80 / 100 {
            // Keep system messages and recent messages
            let keep_recent = 10;
            if memory.messages.len() > keep_recent {
                let mut new_messages = Vec::new();

                // Keep system messages
                for msg in &memory.messages {
                    if msg.role == "system" {
                        new_messages.push(msg.clone());
                    }
                }

                // Keep recent messages
                let start_idx = memory.messages.len().saturating_sub(keep_recent);
                for msg in &memory.messages[start_idx..] {
                    if msg.role != "system" {
                        new_messages.push(msg.clone());
                    }
                }

                memory.messages = new_messages;

                // Recalculate tokens
                memory.total_tokens = memory.messages.iter()
                    .map(|m| m.content.len() / 4)
                    .sum();
            }
        }
    }

    // ===== Error Recovery and Fallback Mechanisms =====

    /// Generate response with automatic fallback on errors
    pub async fn generate_response_with_fallback(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        model_override: Option<&str>,
    ) -> Result<CodingAgentResponse> {
        let models_to_try = if let Some(model) = model_override {
            vec![model.to_string()]
        } else {
            // Try multiple models in order of preference
            vec![
                self.config.default_model.clone(),
                "codellama:latest".to_string(),
                "llama3.2:latest".to_string(),
                "mistral:latest".to_string(),
            ]
        };

        let mut last_error = None;

        for model in &models_to_try {
            info!("Attempting to generate response with model: {}", model);

            match self.generate_response(
                user_input,
                current_dir,
                session_context,
                Some(model)
            ).await {
                Ok(response) => {
                    info!("Successfully generated response with model: {}", model);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Failed with model {}: {}", model, e);
                    last_error = Some(e);

                    // Try to pull the model if it's not available
                    if let Ok(providers) = self.providers.try_lock() {
                        if let Some(provider) = providers.get_provider("ollama") {
                            // Check if we can access the Ollama service
                            if let Some(ollama_provider) = provider.as_any()
                                .downcast_ref::<OllamaProvider>() {
                                let ollama_service = ollama_provider.get_service();

                                // Try to pull the model
                                info!("Attempting to pull model: {}", model);
                                if let Err(pull_err) = ollama_service.pull_model(model).await {
                                    warn!("Failed to pull model {}: {}", model, pull_err);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If all models failed, return a helpful error message
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("All models failed. Please ensure Ollama is running and at least one model is installed.")
        }))
    }

    /// Retry logic with exponential backoff
    pub async fn generate_response_with_retry(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        model_override: Option<&str>,
        max_retries: u32,
    ) -> Result<CodingAgentResponse> {
        let mut retry_count = 0;
        let mut backoff_ms = 100;

        loop {
            match self.generate_response(
                user_input,
                current_dir,
                session_context,
                model_override
            ).await {
                Ok(response) => return Ok(response),
                Err(e) if retry_count < max_retries => {
                    retry_count += 1;
                    warn!("Attempt {} failed: {}. Retrying in {}ms...", retry_count, e, backoff_ms);

                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2; // Exponential backoff
                    backoff_ms = backoff_ms.min(5000); // Cap at 5 seconds
                }
                Err(e) => {
                    error!("All {} retry attempts failed", max_retries);
                    return Err(e);
                }
            }
        }
    }

    /// Health check with auto-recovery
    pub async fn ensure_service_health(&self) -> Result<()> {
        if !self.is_available().await {
            info!("Service not available, attempting recovery...");

            // Try to start Ollama service
            if let Ok(providers) = self.providers.try_lock() {
                if let Some(provider) = providers.get_provider("ollama") {
                    if let Some(ollama_provider) = provider.as_any()
                        .downcast_ref::<OllamaProvider>() {
                        let ollama_service = ollama_provider.get_service();
                        // Check if Ollama is installed
                        if !ollama_service.is_installed().await {
                            warn!("Ollama is not installed. Please install it manually.");
                            return Err(anyhow::anyhow!("Ollama not installed"));
                        }

                        // Try to start Ollama
                        if !ollama_service.is_running().await {
                            info!("Ollama service is not running. Please start it manually.");
                            // Note: Arc<OllamaService> doesn't expose start() method
                            // The service should be started externally
                        }
                    }
                }
            }

            // Verify service is now available
            if !self.is_available().await {
                return Err(anyhow::anyhow!("Service still not available after recovery attempt"));
            }
        }

        Ok(())
    }

    /// Generate response with comprehensive error handling
    pub async fn safe_generate_response(
        &self,
        user_input: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        model_override: Option<&str>,
    ) -> Result<CodingAgentResponse> {
        // Ensure service health first
        self.ensure_service_health().await?;

        // Compress conversation if needed
        self.compress_conversation_if_needed().await;

        // Try with fallback and retry
        let result = self.generate_response_with_retry(
            user_input,
            current_dir,
            session_context,
            model_override,
            3 // Max retries
        ).await;

        match result {
            Ok(response) => Ok(response),
            Err(_) => {
                // If retry fails, try with fallback models
                self.generate_response_with_fallback(
                    user_input,
                    current_dir,
                    session_context,
                    model_override
                ).await
            }
        }
    }
}

// Convenience functions
impl CodingAgentService {
    /// Quick code question without command execution
    pub async fn ask_question(
        &self,
        question: &str,
        current_dir: &PathBuf,
        session_context: &[String],
    ) -> Result<String> {
        let response = self.generate_response(question, current_dir, session_context, None).await?;
        Ok(response.response_text)
    }

    // ===== Intelligent Code Analysis =====

    /// Analyze code in a file and provide intelligent suggestions
    pub async fn analyze_code_file(
        &self,
        file_path: &PathBuf,
        current_dir: &PathBuf,
    ) -> Result<CodeAnalysisReport> {
        // Read the file content
        let content = tokio::fs::read_to_string(file_path).await?;
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Determine language from extension
        let language = self.detect_language(file_path);

        // Analyze code structure
        let structure_analysis = self.analyze_code_structure(&content, &language);

        // Find potential issues
        let issues = self.detect_code_issues(&content, &language);

        // Generate improvement suggestions
        let suggestions = self.generate_improvement_suggestions(&content, &language, &issues).await?;

        // Calculate metrics
        let metrics = self.calculate_code_metrics(&content);

        Ok(CodeAnalysisReport {
            file_path: file_path.clone(),
            language,
            metrics,
            issues,
            suggestions,
            structure: structure_analysis,
        })
    }

    /// Detect programming language from file extension
    fn detect_language(&self, path: &PathBuf) -> String {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "rs" => "rust",
            "py" => "python",
            "js" | "mjs" => "javascript",
            "ts" | "tsx" => "typescript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" => "c",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "scala" => "scala",
            "sh" | "bash" => "bash",
            "sql" => "sql",
            "html" | "htm" => "html",
            "css" | "scss" | "sass" => "css",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "md" | "markdown" => "markdown",
            _ => "unknown",
        }.to_string()
    }

    /// Analyze code structure
    fn analyze_code_structure(&self, content: &str, language: &str) -> CodeStructure {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let non_empty_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();
        let comment_lines = self.count_comment_lines(&lines, language);

        // Find functions/methods
        let functions = self.find_functions(content, language);

        // Find classes/structs
        let classes = self.find_classes(content, language);

        // Find imports/dependencies
        let imports = self.find_imports(content, language);

        CodeStructure {
            total_lines,
            code_lines: non_empty_lines - comment_lines,
            comment_lines,
            blank_lines: total_lines - non_empty_lines,
            functions,
            classes,
            imports,
        }
    }

    /// Count comment lines based on language
    fn count_comment_lines(&self, lines: &[&str], language: &str) -> usize {
        let single_line_comment = match language {
            "rust" | "javascript" | "typescript" | "go" | "java" | "cpp" | "c" => "//",
            "python" | "ruby" | "bash" => "#",
            "sql" => "--",
            _ => "//",
        };

        lines.iter().filter(|l| l.trim().starts_with(single_line_comment)).count()
    }

    /// Find function definitions
    fn find_functions(&self, content: &str, language: &str) -> Vec<String> {
        let mut functions = Vec::new();

        let pattern = match language {
            "rust" => r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)",
            "python" => r"def\s+(\w+)\s*\(",
            "javascript" | "typescript" => r"(?:function\s+(\w+)|const\s+(\w+)\s*=\s*(?:async\s*)?\()",
            "go" => r"func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(",
            "java" | "cpp" | "c" => r"(?:public|private|protected)?\s*(?:static)?\s*\w+\s+(\w+)\s*\(",
            _ => return functions,
        };

        if let Ok(regex) = regex::Regex::new(pattern) {
            for cap in regex.captures_iter(content) {
                if let Some(name) = cap.get(1).or(cap.get(2)) {
                    functions.push(name.as_str().to_string());
                }
            }
        }

        functions
    }

    /// Find class/struct definitions
    fn find_classes(&self, content: &str, language: &str) -> Vec<String> {
        let mut classes = Vec::new();

        let pattern = match language {
            "rust" => r"(?:pub\s+)?(?:struct|enum|trait)\s+(\w+)",
            "python" => r"class\s+(\w+)(?:\(|:)",
            "javascript" | "typescript" => r"class\s+(\w+)",
            "go" => r"type\s+(\w+)\s+(?:struct|interface)",
            "java" | "cpp" => r"(?:public\s+)?class\s+(\w+)",
            _ => return classes,
        };

        if let Ok(regex) = regex::Regex::new(pattern) {
            for cap in regex.captures_iter(content) {
                if let Some(name) = cap.get(1) {
                    classes.push(name.as_str().to_string());
                }
            }
        }

        classes
    }

    /// Find imports/dependencies
    fn find_imports(&self, content: &str, language: &str) -> Vec<String> {
        let mut imports = Vec::new();

        let pattern = match language {
            "rust" => r"use\s+([\w:]+)",
            "python" => r"(?:from\s+(\S+)\s+)?import\s+(\S+)",
            "javascript" | "typescript" => r#"import\s+.*\s+from\s+['"]([^'"]+)['"]"#,
            "go" => r#"import\s+(?:\([\s\S]*?\)|"([^"]+)")"#,
            "java" => r"import\s+([\w\.]+);",
            _ => return imports,
        };

        if let Ok(regex) = regex::Regex::new(pattern) {
            for cap in regex.captures_iter(content) {
                if let Some(name) = cap.get(1).or(cap.get(2)) {
                    imports.push(name.as_str().to_string());
                }
            }
        }

        imports
    }

    /// Detect common code issues
    fn detect_code_issues(&self, content: &str, language: &str) -> Vec<CodeIssue> {
        let mut issues = Vec::new();

        // Check for long lines
        for (i, line) in content.lines().enumerate() {
            if line.len() > 100 {
                issues.push(CodeIssue {
                    severity: IssueSeverity::Info,
                    category: IssueCategory::Style,
                    message: format!("Line exceeds 100 characters ({})", line.len()),
                    file: PathBuf::new(),
                    line: i + 1,
                    column: 100,
                    suggestion: Some("Consider breaking this line into multiple lines".to_string()),
                    auto_fixable: false,
                });
            }
        }

        // Check for TODO/FIXME comments
        for (i, line) in content.lines().enumerate() {
            if line.contains("TODO") || line.contains("FIXME") || line.contains("XXX") {
                issues.push(CodeIssue {
                    severity: IssueSeverity::Hint,
                    category: IssueCategory::Maintainability,
                    message: "Found TODO/FIXME comment".to_string(),
                    file: PathBuf::new(),
                    line: i + 1,
                    column: 0,
                    suggestion: Some("Consider addressing this technical debt".to_string()),
                    auto_fixable: false,
                });
            }
        }

        // Language-specific checks
        match language {
            "rust" => {
                // Check for unwrap() usage
                for (i, line) in content.lines().enumerate() {
                    if line.contains(".unwrap()") {
                        issues.push(CodeIssue {
                            severity: IssueSeverity::Warning,
                            category: IssueCategory::Reliability,
                            message: "Using unwrap() can cause panics".to_string(),
                            file: PathBuf::new(),
                            line: i + 1,
                            column: line.find(".unwrap()").unwrap_or_default(),
                            suggestion: Some("Consider using ? operator or proper error handling".to_string()),
                            auto_fixable: true,
                        });
                    }
                }
            }
            "python" => {
                // Check for bare except
                for (i, line) in content.lines().enumerate() {
                    if line.trim() == "except:" {
                        issues.push(CodeIssue {
                            severity: IssueSeverity::Warning,
                            category: IssueCategory::BestPractice,
                            message: "Bare except clause catches all exceptions".to_string(),
                            file: PathBuf::new(),
                            line: i + 1,
                            column: 0,
                            suggestion: Some("Specify the exception type to catch".to_string()),
                            auto_fixable: false,
                        });
                    }
                }
            }
            _ => {}
        }

        issues
    }

    /// Calculate code metrics
    fn calculate_code_metrics(&self, content: &str) -> CodeMetrics {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Simple cyclomatic complexity (count decision points)
        let decision_keywords = ["if", "else", "match", "while", "for", "loop", "?", "&&", "||"];
        let mut complexity = 1;
        for keyword in &decision_keywords {
            complexity += content.matches(keyword).count() as u32;
        }

        // Calculate nesting depth
        let mut max_nesting = 0;
        let mut current_nesting: usize = 0;
        for char in content.chars() {
            match char {
                '{' | '(' | '[' => {
                    current_nesting += 1;
                    max_nesting = max_nesting.max(current_nesting);
                }
                '}' | ')' | ']' => {
                    current_nesting = current_nesting.saturating_sub(1);
                }
                _ => {}
            }
        }

        CodeMetrics {
            total_lines,
            cyclomatic_complexity: complexity,
            max_nesting_depth: max_nesting,
            average_line_length: if total_lines > 0 {
                content.len() / total_lines
            } else {
                0
            },
        }
    }

    /// Generate improvement suggestions using AI
    async fn generate_improvement_suggestions(
        &self,
        content: &str,
        language: &str,
        issues: &[CodeIssue],
    ) -> Result<Vec<String>> {
        let issue_summary = issues.iter()
            .map(|i| format!("- {} (line {}): {}",
                match i.severity {
                    IssueSeverity::Error => "ERROR",
                    IssueSeverity::Warning => "WARNING",
                    IssueSeverity::Info => "INFO",
                    IssueSeverity::Hint => "HINT",
                },
                i.line,
                i.message
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Analyze this {} code and provide 3-5 specific improvement suggestions:\n\n\
            Code Issues Found:\n{}\n\n\
            Code:\n```{}\n{}\n```\n\n\
            Provide actionable suggestions for improving code quality, performance, and maintainability.",
            language, issue_summary, language, content
        );

        let response = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        // Parse suggestions from response
        Ok(response.lines()
            .filter(|l| l.starts_with("- ") || l.starts_with("* ") || l.starts_with("• "))
            .map(|l| l.trim_start_matches(&['-', '*', '•', ' '][..]).to_string())
            .collect())
    }

    /// Generate test cases for a function
    pub async fn generate_tests(
        &self,
        function_code: &str,
        language: &str,
        current_dir: &PathBuf,
    ) -> Result<String> {
        let prompt = format!(
            "Generate comprehensive test cases for this {} function:\n\n```{}\n{}\n```\n\n\
            Include:\n\
            1. Normal cases\n\
            2. Edge cases\n\
            3. Error cases\n\
            4. Performance considerations\n\n\
            Use the appropriate testing framework for {}.",
            language, language, function_code, language
        );

        self.ask_question(&prompt, current_dir, &[]).await
    }

    /// Suggest refactoring for code
    pub async fn suggest_refactoring(
        &self,
        code: &str,
        language: &str,
        refactoring_type: &str,
    ) -> Result<RefactoringSuggestion> {
        let prompt = format!(
            "Suggest {} refactoring for this {} code:\n\n```{}\n{}\n```\n\n\
            Provide:\n\
            1. The refactored code\n\
            2. Explanation of changes\n\
            3. Benefits of the refactoring",
            refactoring_type, language, language, code
        );

        let response = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        Ok(RefactoringSuggestion {
            original_code: code.to_string(),
            refactored_code: self.extract_code_block(&response),
            explanation: response,
            benefits: vec![
                "Improved readability".to_string(),
                "Better maintainability".to_string(),
                "Enhanced performance".to_string(),
            ],
        })
    }

    /// Extract code block from markdown response
    fn extract_code_block(&self, text: &str) -> String {
        if let Some(start) = text.find("```") {
            let code_start = text[start + 3..].find('\n').map(|i| start + 3 + i + 1).unwrap_or(start + 3);
            if let Some(end) = text[code_start..].find("```") {
                return text[code_start..code_start + end].trim().to_string();
            }
        }
        text.to_string()
    }

    /// Provide debugging assistance
    pub async fn debug_assistance(
        &self,
        error_message: &str,
        code_context: &str,
        language: &str,
    ) -> Result<DebuggingHelp> {
        let prompt = format!(
            "Help debug this {} error:\n\n\
            Error: {}\n\n\
            Code context:\n```{}\n{}\n```\n\n\
            Provide:\n\
            1. Root cause analysis\n\
            2. Step-by-step debugging approach\n\
            3. Potential fixes\n\
            4. Prevention strategies",
            language, error_message, language, code_context
        );

        let response = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        Ok(DebuggingHelp {
            error: error_message.to_string(),
            root_cause: self.extract_section(&response, "Root cause"),
            debugging_steps: self.extract_list_items(&response, "debugging"),
            potential_fixes: self.extract_list_items(&response, "fix"),
            prevention: self.extract_section(&response, "Prevention"),
        })
    }

    /// Extract section from response
    fn extract_section(&self, text: &str, section: &str) -> String {
        let lower_text = text.to_lowercase();
        let lower_section = section.to_lowercase();

        if let Some(start) = lower_text.find(&lower_section) {
            let section_text = &text[start..];
            if let Some(end) = section_text.find("\n\n") {
                return section_text[..end].to_string();
            }
            return section_text.to_string();
        }

        text.lines().next().unwrap_or("").to_string()
    }

    /// Extract list items containing keyword
    fn extract_list_items(&self, text: &str, keyword: &str) -> Vec<String> {
        text.lines()
            .filter(|l| {
                let line_lower = l.to_lowercase();
                let keyword_lower = keyword.to_lowercase();
                (l.starts_with("- ") || l.starts_with("* ") || l.starts_with("• ") || l.starts_with(char::is_numeric))
                    && line_lower.contains(&keyword_lower)
            })
            .map(|l| l.trim_start_matches(&['-', '*', '•', ' ', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', ' '][..]).to_string())
            .collect()
    }

    /// Interactive code review
    pub async fn review_code(
        &self,
        code: &str,
        language: &str,
        focus_areas: Vec<&str>,
    ) -> Result<CodeReview> {
        let focus = if focus_areas.is_empty() {
            "general quality, security, performance, and best practices".to_string()
        } else {
            focus_areas.join(", ")
        };

        let prompt = format!(
            "Perform a thorough code review of this {} code, focusing on {}:\n\n\
            ```{}\n{}\n```\n\n\
            Provide:\n\
            1. Overall assessment\n\
            2. Specific issues found\n\
            3. Security concerns\n\
            4. Performance optimizations\n\
            5. Best practice violations\n\
            6. Suggested improvements with examples",
            language, focus, language, code
        );

        let response = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        Ok(CodeReview {
            overall_score: 7.5, // This could be parsed from the response
            summary: self.extract_section(&response, "assessment"),
            issues: self.parse_review_issues(&response),
            suggestions: self.extract_list_items(&response, "suggest"),
            security_concerns: self.extract_list_items(&response, "security"),
            performance_notes: self.extract_list_items(&response, "performance"),
        })
    }

    /// Parse review issues from response
    fn parse_review_issues(&self, text: &str) -> Vec<ReviewIssue> {
        let mut issues = Vec::new();

        for line in text.lines() {
            if line.contains("issue") || line.contains("problem") || line.contains("concern") {
                let severity = if line.to_lowercase().contains("critical") || line.to_lowercase().contains("error") {
                    "high"
                } else if line.to_lowercase().contains("warning") || line.to_lowercase().contains("moderate") {
                    "medium"
                } else {
                    "low"
                };

                issues.push(ReviewIssue {
                    severity: severity.to_string(),
                    description: line.trim_start_matches(&['-', '*', '•', ' '][..]).to_string(),
                    line_number: None, // Could be extracted if present
                    suggestion: None,
                });
            }
        }

        issues
    }

    // ===== Workspace Analysis and Automated Fixes =====

    /// Analyze entire workspace/project
    pub async fn analyze_workspace(&self, workspace_path: &PathBuf) -> Result<WorkspaceAnalysis> {
        info!("Analyzing workspace at: {:?}", workspace_path);

        let mut analyzer = WorkspaceAnalyzer::new(workspace_path.clone());
        let analysis = analyzer.analyze().await?;

        // Generate AI-powered insights
        let insights = self.generate_workspace_insights(&analysis).await?;

        info!("Workspace analysis complete");
        Ok(analysis)
    }

    /// Generate AI-powered insights from workspace analysis
    async fn generate_workspace_insights(&self, analysis: &WorkspaceAnalysis) -> Result<String> {
        let prompt = format!(
            "Based on this workspace analysis, provide actionable insights:\n\n\
            Project Type: {:?}\n\
            Total Files: {}\n\
            Total Lines: {}\n\
            Test Coverage: {:?}\n\
            Security Issues: {}\n\
            Dependencies: {}\n\n\
            Provide:\n\
            1. Key strengths of the codebase\n\
            2. Critical issues to address\n\
            3. Quick wins for improvement\n\
            4. Long-term recommendations",
            analysis.project_structure.project_type,
            analysis.statistics.total_files,
            analysis.statistics.total_lines,
            analysis.code_health.test_coverage,
            analysis.security_issues.len(),
            analysis.dependencies.direct_dependencies.len()
        );

        self.ask_question(&prompt, &PathBuf::new(), &[]).await
    }

    /// Apply automated fixes to code issues
    pub async fn apply_automated_fixes(
        &self,
        file_path: &PathBuf,
        issues: &[CodeIssue],
    ) -> Result<Vec<AppliedFix>> {
        let mut applied_fixes = Vec::new();
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut modified_content = content.clone();

        for issue in issues {
            if issue.auto_fixable {
                match self.generate_fix_for_issue(issue, &modified_content).await {
                    Ok(fix) => {
                        // Apply the fix
                        modified_content = self.apply_fix_to_content(&modified_content, &fix);
                        applied_fixes.push(AppliedFix {
                            issue: issue.clone(),
                            fix_applied: fix.replacement,
                            success: true,
                            message: "Fix applied successfully".to_string(),
                        });
                    }
                    Err(e) => {
                        applied_fixes.push(AppliedFix {
                            issue: issue.clone(),
                            fix_applied: String::new(),
                            success: false,
                            message: format!("Failed to generate fix: {}", e),
                        });
                    }
                }
            }
        }

        // Write the modified content back to file if any fixes were applied
        if applied_fixes.iter().any(|f| f.success) {
            // Create backup first
            let backup_path = file_path.with_extension("bak");
            tokio::fs::copy(file_path, &backup_path).await?;

            // Write fixed content
            tokio::fs::write(file_path, modified_content).await?;
        }

        Ok(applied_fixes)
    }

    /// Generate fix for a specific issue
    async fn generate_fix_for_issue(&self, issue: &CodeIssue, content: &str) -> Result<CodeFix> {
        // Extract the problematic line
        let lines: Vec<&str> = content.lines().collect();
        let problem_line = if issue.line > 0 && issue.line <= lines.len() {
            lines[issue.line - 1]
        } else {
            return Err(anyhow::anyhow!("Invalid line number"));
        };

        let prompt = format!(
            "Generate a fix for this code issue:\n\n\
            Issue: {}\n\
            Category: {:?}\n\
            Line {}: {}\n\
            Suggestion: {}\n\n\
            Provide ONLY the fixed line of code, nothing else.",
            issue.message,
            issue.category,
            issue.line,
            problem_line,
            issue.suggestion.as_ref().unwrap_or(&"Apply best practices".to_string())
        );

        let fixed_line = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        Ok(CodeFix {
            line_number: issue.line,
            original: problem_line.to_string(),
            replacement: fixed_line.trim().to_string(),
        })
    }

    /// Apply a fix to content
    fn apply_fix_to_content(&self, content: &str, fix: &CodeFix) -> String {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if fix.line_number > 0 && fix.line_number <= lines.len() {
            lines[fix.line_number - 1] = fix.replacement.clone();
        }

        lines.join("\n")
    }

    /// Generate comprehensive documentation for code
    pub async fn generate_documentation(
        &self,
        code: &str,
        language: &str,
        doc_type: DocumentationType,
    ) -> Result<String> {
        let doc_style = match doc_type {
            DocumentationType::Api => "API documentation with parameter descriptions and return values",
            DocumentationType::Tutorial => "tutorial-style documentation with examples",
            DocumentationType::Reference => "reference documentation with detailed specifications",
            DocumentationType::Comments => "inline code comments",
        };

        let prompt = format!(
            "Generate {} for this {} code:\n\n```{}\n{}\n```\n\n\
            Follow the standard documentation conventions for {}.",
            doc_style, language, language, code, language
        );

        self.ask_question(&prompt, &PathBuf::new(), &[]).await
    }

    /// Visualize code complexity
    pub async fn visualize_complexity(&self, file_path: &PathBuf) -> Result<ComplexityVisualization> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let language = self.detect_language(file_path);

        // Analyze functions and their complexity
        let functions = self.find_functions(&content, &language);
        let mut function_complexities = Vec::new();

        for function_name in &functions {
            // Find function body (simplified)
            let function_body = self.extract_function_body(&content, function_name, &language);
            let complexity = self.calculate_complexity(&function_body);

            function_complexities.push(FunctionComplexity {
                name: function_name.clone(),
                cyclomatic_complexity: complexity,
                lines: function_body.lines().count(),
                nesting_depth: self.calculate_max_nesting(&function_body),
            });
        }

        // Sort by complexity (highest first)
        function_complexities.sort_by_key(|f| std::cmp::Reverse(f.cyclomatic_complexity));

        let hotspots = self.identify_complexity_hotspots(&function_complexities);

        Ok(ComplexityVisualization {
            file: file_path.clone(),
            total_complexity: function_complexities.iter().map(|f| f.cyclomatic_complexity).sum(),
            functions: function_complexities,
            hotspots,
        })
    }

    /// Extract function body from content
    fn extract_function_body(&self, content: &str, function_name: &str, language: &str) -> String {
        // This is a simplified implementation
        // In production, you'd use a proper parser

        if let Some(start) = content.find(function_name) {
            let from_function = &content[start..];

            // Find the opening brace
            if let Some(open_brace) = from_function.find('{') {
                let mut brace_count = 1;
                let mut end_pos = open_brace + 1;

                for (i, ch) in from_function[open_brace + 1..].char_indices() {
                    match ch {
                        '{' => brace_count += 1,
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                end_pos = open_brace + 1 + i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                return from_function[..end_pos].to_string();
            }
        }

        String::new()
    }

    /// Calculate complexity of code
    fn calculate_complexity(&self, code: &str) -> u32 {
        let decision_points = [
            "if ", "else", "match", "while", "for", "loop",
            "?", "&&", "||", "case", "when", "catch", "except"
        ];

        let mut complexity = 1; // Base complexity

        for point in &decision_points {
            complexity += code.matches(point).count() as u32;
        }

        complexity
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting(&self, code: &str) -> u32 {
        let mut max_depth = 0;
        let mut current_depth: u32 = 0;

        for ch in code.chars() {
            match ch {
                '{' | '(' | '[' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' | ')' | ']' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }

    /// Identify complexity hotspots
    fn identify_complexity_hotspots(&self, functions: &[FunctionComplexity]) -> Vec<String> {
        functions.iter()
            .filter(|f| f.cyclomatic_complexity > 10)
            .map(|f| format!("{} (complexity: {})", f.name, f.cyclomatic_complexity))
            .collect()
    }

    /// Scan for security vulnerabilities
    pub async fn scan_security_vulnerabilities(
        &self,
        file_path: &PathBuf,
    ) -> Result<SecurityScanReport> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let language = self.detect_language(file_path);

        let mut vulnerabilities = Vec::new();

        // Check for common security issues
        let security_patterns = vec![
            ("password\\s*=\\s*[\"']", "Hardcoded password", "Critical"),
            ("api_key\\s*=\\s*[\"']", "Hardcoded API key", "Critical"),
            ("eval\\(", "Use of eval() function", "High"),
            ("exec\\(", "Use of exec() function", "High"),
            ("os\\.system\\(", "Command injection risk", "High"),
            ("SELECT.*\\+", "Potential SQL injection", "Critical"),
            ("innerHTML\\s*=", "XSS vulnerability risk", "High"),
            ("disable.*ssl.*verif", "SSL verification disabled", "High"),
            ("md5\\(", "Weak cryptographic hash", "Medium"),
            ("Math\\.random\\(", "Weak random number generation", "Medium"),
        ];

        for (pattern, description, severity) in security_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for mat in regex.find_iter(&content) {
                    let line_num = content[..mat.start()].lines().count() + 1;

                    vulnerabilities.push(SecurityVulnerability {
                        severity: severity.to_string(),
                        category: "Security".to_string(),
                        description: description.to_string(),
                        file: file_path.clone(),
                        line: line_num,
                        recommendation: self.get_security_recommendation(description),
                    });
                }
            }
        }

        // Get AI-powered security analysis
        let ai_analysis = self.ai_security_analysis(&content, &language, &vulnerabilities).await?;
        let risk_score = self.calculate_risk_score(&vulnerabilities);

        Ok(SecurityScanReport {
            file: file_path.clone(),
            vulnerabilities,
            risk_score,
            ai_analysis,
        })
    }

    /// Get security recommendation for issue
    fn get_security_recommendation(&self, issue: &str) -> String {
        match issue {
            "Hardcoded password" => "Use environment variables or secure key management systems",
            "Hardcoded API key" => "Store API keys in environment variables or secure vaults",
            "Use of eval() function" => "Parse and validate input instead of using eval()",
            "Command injection risk" => "Use parameterized commands or subprocess with shell=False",
            "Potential SQL injection" => "Use parameterized queries or prepared statements",
            "XSS vulnerability risk" => "Sanitize user input and use textContent instead of innerHTML",
            "SSL verification disabled" => "Enable SSL certificate verification for production",
            "Weak cryptographic hash" => "Use SHA-256 or stronger hashing algorithms",
            "Weak random number generation" => "Use cryptographically secure random number generators",
            _ => "Review and apply security best practices",
        }.to_string()
    }

    /// Calculate risk score from vulnerabilities
    fn calculate_risk_score(&self, vulnerabilities: &[SecurityVulnerability]) -> f32 {
        let mut score = 0.0;

        for vuln in vulnerabilities {
            score += match vuln.severity.as_str() {
                "Critical" => 10.0,
                "High" => 7.0,
                "Medium" => 4.0,
                "Low" => 1.0,
                _ => 0.0,
            };
        }

        f32::min(score / 10.0, 10.0) // Normalize to 0-10 scale
    }

    /// AI-powered security analysis
    async fn ai_security_analysis(
        &self,
        content: &str,
        language: &str,
        found_vulnerabilities: &[SecurityVulnerability],
    ) -> Result<String> {
        let vuln_summary = found_vulnerabilities.iter()
            .map(|v| format!("- {} ({}): {}", v.severity, v.line, v.description))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Analyze this {} code for security vulnerabilities:\n\n\
            Found Issues:\n{}\n\n\
            Code (first 100 lines):\n```{}\n{}\n```\n\n\
            Provide:\n\
            1. Overall security assessment\n\
            2. Most critical issues to fix\n\
            3. Security best practices for this code\n\
            4. Recommended security tools and libraries",
            language,
            if vuln_summary.is_empty() { "None detected automatically" } else { &vuln_summary },
            language,
            content.lines().take(100).collect::<Vec<_>>().join("\n")
        );

        self.ask_question(&prompt, &PathBuf::new(), &[]).await
    }

    /// Generate performance profiling suggestions
    pub async fn suggest_performance_improvements(
        &self,
        code: &str,
        language: &str,
    ) -> Result<PerformanceSuggestions> {
        let prompt = format!(
            "Analyze this {} code for performance improvements:\n\n```{}\n{}\n```\n\n\
            Identify:\n\
            1. Performance bottlenecks\n\
            2. Optimization opportunities\n\
            3. Better algorithms or data structures\n\
            4. Caching opportunities\n\
            5. Parallelization potential",
            language, language, code
        );

        let analysis = self.ask_question(&prompt, &PathBuf::new(), &[]).await?;

        // Parse the response to extract specific suggestions
        let suggestions = self.extract_list_items(&analysis, "");

        Ok(PerformanceSuggestions {
            bottlenecks: self.extract_list_items(&analysis, "bottleneck"),
            optimizations: self.extract_list_items(&analysis, "optimi"),
            algorithm_improvements: self.extract_list_items(&analysis, "algorithm"),
            caching_opportunities: self.extract_list_items(&analysis, "cach"),
            parallelization: self.extract_list_items(&analysis, "parallel"),
            overall_analysis: analysis,
        })
    }
}

// Supporting structures for code analysis

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisReport {
    pub file_path: PathBuf,
    pub language: String,
    pub metrics: CodeMetrics,
    pub issues: Vec<CodeIssue>,
    pub suggestions: Vec<String>,
    pub structure: CodeStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub functions: Vec<String>,
    pub classes: Vec<String>,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub total_lines: usize,
    pub cyclomatic_complexity: u32,
    pub max_nesting_depth: usize,
    pub average_line_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub original_code: String,
    pub refactored_code: String,
    pub explanation: String,
    pub benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggingHelp {
    pub error: String,
    pub root_cause: String,
    pub debugging_steps: Vec<String>,
    pub potential_fixes: Vec<String>,
    pub prevention: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReview {
    pub overall_score: f32,
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<String>,
    pub security_concerns: Vec<String>,
    pub performance_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: String,
    pub description: String,
    pub line_number: Option<usize>,
    pub suggestion: Option<String>,
}

// New supporting structures for advanced features

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFix {
    pub issue: CodeIssue,
    pub fix_applied: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFix {
    pub line_number: usize,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentationType {
    Api,
    Tutorial,
    Reference,
    Comments,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityVisualization {
    pub file: PathBuf,
    pub total_complexity: u32,
    pub functions: Vec<FunctionComplexity>,
    pub hotspots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub cyclomatic_complexity: u32,
    pub lines: usize,
    pub nesting_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanReport {
    pub file: PathBuf,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub risk_score: f32,
    pub ai_analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub file: PathBuf,
    pub line: usize,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSuggestions {
    pub bottlenecks: Vec<String>,
    pub optimizations: Vec<String>,
    pub algorithm_improvements: Vec<String>,
    pub caching_opportunities: Vec<String>,
    pub parallelization: Vec<String>,
    pub overall_analysis: String,
}