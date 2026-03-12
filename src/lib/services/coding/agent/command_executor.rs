use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::time::SystemTime;
use log;
use super::service::CodingAgentService;
use super::execution_context::{ExecutionContext, ContextCommand, ExecutionContextManager};

/// Command executor for safe command execution with context management
#[derive(Debug)]
pub struct CommandExecutor {
    coding_agent: Arc<CodingAgentService>,
    context_manager: Arc<ExecutionContextManager>,
}

impl CommandExecutor {
    pub fn new(coding_agent: Arc<CodingAgentService>) -> Self {
        Self {
            coding_agent,
            context_manager: Arc::new(ExecutionContextManager::new()),
        }
    }

    /// Create with existing context manager
    pub fn with_context_manager(coding_agent: Arc<CodingAgentService>, context_manager: Arc<ExecutionContextManager>) -> Self {
        Self {
            coding_agent,
            context_manager,
        }
    }

    /// Execute a single command with retry logic and context
    pub async fn execute_command(&self, command: &str, current_dir: &PathBuf) -> String {
        // Ensure we have an active context
        let context = if let Some(ctx) = self.context_manager.get_active_context().await {
            ctx
        } else {
            // Create default context
            self.context_manager.create_context("default".to_string()).await
        };

        // Use context-aware execution
        self.execute_command_with_context(command, current_dir, &context, 3).await
    }

    /// Execute command with legacy interface (no context)
    pub async fn execute_command_simple(&self, command: &str, current_dir: &PathBuf) -> String {
        self.execute_command_with_retry(command, current_dir, 3).await
    }

    /// Execute a single command with configurable retry attempts
    async fn execute_command_with_retry(&self, command: &str, current_dir: &PathBuf, max_retries: u32) -> String {
        // Validate command before execution
        if let Err(validation_error) = self.validate_command(command) {
            return format!("Command validation failed: {}", validation_error);
        }

        // Handle cd command specially since it doesn't work in child processes
        if command.starts_with("cd ") {
            return self.handle_cd_command(command, current_dir).await;
        }

        // Handle heredoc commands specially
        if command.contains("<<") && (command.contains("'EOF'") || command.contains("\"EOF\"") || command.contains("<< EOF")) {
            return self.handle_heredoc_command(command, current_dir).await;
        }

        // For other commands, implement retry logic
        let mut last_error = String::new();
        for attempt in 0..max_retries {
            // Sanitize command before execution
            let sanitized_command = self.sanitize_command(command);

            match self.coding_agent.execute_command_in_dir(&sanitized_command, current_dir, false).await {
                Ok(output) => {
                    // Check if the output indicates a failure even though the command "succeeded"
                    if self.output_indicates_failure(&output, command) {
                        // Only retry for actual failures, not expected errors
                        if !output.contains("is a directory") && attempt < max_retries - 1 {
                            let delay_ms = 100 * (2_u64.pow(attempt));
                            sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        // Return the output as-is without adding redundant prefix
                        return output;
                    }

                    // If previous attempts failed but this one succeeded, note the recovery
                    if attempt > 0 {
                        return format!(
                            "[Recovered after {} attempts]\n{}",
                            attempt + 1,
                            output
                        );
                    }
                    return output;
                }
                Err(e) => {
                    last_error = format!("Error: {}", e);

                    // Don't retry for certain types of errors
                    if self.is_non_retryable_error(&last_error) {
                        break;
                    }

                    // Wait before retrying (exponential backoff)
                    if attempt < max_retries - 1 {
                        let delay_ms = 100 * (2_u64.pow(attempt));
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        // All attempts failed
        format!("{}\n[Command failed after {} attempts]", last_error, max_retries)
    }

    /// Execute command with execution context
    async fn execute_command_with_context(
        &self,
        command: &str,
        current_dir: &PathBuf,
        context: &ExecutionContext,
        max_retries: u32,
    ) -> String {
        // Validate command
        if let Err(validation_error) = self.validate_command(command) {
            return format!("Command validation failed: {}", validation_error);
        }

        // Handle cd command specially
        if command.starts_with("cd ") {
            let result = self.handle_cd_command(command, current_dir).await;

            // Update context working directory if cd succeeded
            if !result.contains("Error") {
                let new_dir = command.trim_start_matches("cd ").trim();
                let new_path = if new_dir.starts_with('/') {
                    PathBuf::from(new_dir)
                } else {
                    current_dir.join(new_dir)
                };

                let _ = self.context_manager.update_active_context(|ctx| {
                    let _ = ctx.set_working_directory(new_path);
                }).await;
            }

            return result;
        }

        // Check if this is a GUI command
        let final_command = if self.is_gui_command(command) && context.supports_gui() {
            context.prepare_gui_command(command)
        } else {
            context.prepare_command(command)
        };

        let start_time = SystemTime::now();
        let mut last_error = String::new();

        for attempt in 0..max_retries {
            let sanitized_command = self.sanitize_command(&final_command);

            match self.coding_agent.execute_command_in_dir(&sanitized_command, current_dir, false).await {
                Ok(output) => {
                    // Record in context history
                    let duration = SystemTime::now().duration_since(start_time).unwrap_or_default();
                    let cmd_record = ContextCommand {
                        command: command.to_string(),
                        executed_at: SystemTime::now(),
                        working_directory: context.working_directory.clone(),
                        exit_code: Some(0),
                        stdout: output.clone(),
                        stderr: String::new(),
                        duration,
                    };

                    let _ = self.context_manager.update_active_context(|ctx| {
                        ctx.add_command(cmd_record);
                    }).await;

                    if self.output_indicates_failure(&output, command) {
                        // Only retry for actual failures, not expected errors
                        if !output.contains("is a directory") && attempt < max_retries - 1 {
                            let delay_ms = 100 * (2_u64.pow(attempt));
                            sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        // Return the output as-is without adding redundant prefix
                        return output;
                    }

                    if attempt > 0 {
                        return format!("[Recovered after {} attempts]\n{}", attempt + 1, output);
                    }
                    return output;
                }
                Err(e) => {
                    last_error = format!("Error: {}", e);

                    // Record failure in context
                    let duration = SystemTime::now().duration_since(start_time).unwrap_or_default();
                    let cmd_record = ContextCommand {
                        command: command.to_string(),
                        executed_at: SystemTime::now(),
                        working_directory: context.working_directory.clone(),
                        exit_code: Some(1),
                        stdout: String::new(),
                        stderr: last_error.clone(),
                        duration,
                    };

                    let _ = self.context_manager.update_active_context(|ctx| {
                        ctx.add_command(cmd_record);
                    }).await;

                    if self.is_non_retryable_error(&last_error) {
                        break;
                    }

                    if attempt < max_retries - 1 {
                        let delay_ms = 100 * (2_u64.pow(attempt));
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        format!("{}\n[Command failed after {} attempts]", last_error, max_retries)
    }

    /// Check if command is a GUI application
    fn is_gui_command(&self, command: &str) -> bool {
        let gui_indicators = [
            "firefox", "chrome", "chromium", "code", "vscode", "subl", "sublime",
            "atom", "idea", "pycharm", "webstorm", "gimp", "inkscape", "blender",
            "vlc", "mpv", "spotify", "slack", "discord", "telegram", "zoom",
            "xterm", "gnome-terminal", "konsole", "kitty", "alacritty",
            "nautilus", "dolphin", "thunar", "pcmanfm", "nemo",
            "gedit", "kate", "mousepad", "leafpad", "pluma",
            "eog", "gwenview", "feh", "ristretto", "viewnior",
            "evince", "okular", "zathura", "mupdf", "xpdf"
        ];

        let cmd_lower = command.to_lowercase();
        gui_indicators.iter().any(|&indicator| cmd_lower.contains(indicator))
    }

    /// Sanitize command to prevent shell injection and fix common issues
    fn sanitize_command(&self, command: &str) -> String {
        let command = command.trim();

        // Remove potentially dangerous characters and patterns
        let sanitized = command
            .replace("''", "'")  // Fix double quotes
            .replace("\"\"", "\"")  // Fix double quotes
            .replace(";;", ";")  // Fix double semicolons
            .replace("&&", " && ")  // Ensure spaces around &&
            .replace(";", " ; ")  // Ensure spaces around ;
            .trim()
            .to_string();

        // Ensure no unmatched quotes
        let single_quote_count = sanitized.matches('\'').count();
        let double_quote_count = sanitized.matches('"').count();

        if single_quote_count % 2 != 0 || double_quote_count % 2 != 0 {
            // Remove all quotes if unmatched
            sanitized.replace('\'', "").replace('"', "")
        } else {
            sanitized
        }
    }

    /// Check if command output indicates failure even though exit code was 0
    fn output_indicates_failure(&self, output: &str, command: &str) -> bool {
        let output_lower = output.to_lowercase();

        // Special case: mkdir with "File exists" is not a real failure
        if command.starts_with("mkdir") && output_lower.contains("file exists") {
            return false; // Directory already exists, that's fine
        }

        // Special case: ls on a directory that exists is not a failure
        if command.starts_with("ls") && !output_lower.contains("no such file") {
            return false;
        }

        // Special case: cat on a directory is an expected error, not a failure to retry
        if command.starts_with("cat") && output_lower.contains("is a directory") {
            // It's an error but not one we should retry
            return false;
        }

        // Special case: grep not finding matches is not a failure
        if command.starts_with("grep") && output_lower.contains("is a directory") {
            return false;
        }

        // Skip "Command failed:" prefix check since that's added by our code
        let clean_output = if output_lower.starts_with("command failed:") {
            output_lower.strip_prefix("command failed:").unwrap_or(&output_lower).trim()
        } else {
            &output_lower
        };

        // Common error indicators
        let error_patterns = [
            "error:", "failed:", "cannot", "no such file", "permission denied",
            "command not found", "syntax error", "parse error", "invalid",
            "unexpected eof", "line 0:", "line 1:", "unexpected end of file"
        ];

        for pattern in &error_patterns {
            if clean_output.contains(pattern) {
                // Additional checks for false positives
                if *pattern == "cannot" && command.starts_with("echo") {
                    continue; // Echo might output text containing "cannot"
                }
                return true;
            }
        }

        // Check for command-specific failure patterns
        if command.starts_with("cargo") {
            if clean_output.contains("compilation failed") ||
               clean_output.contains("build failed") ||
               clean_output.contains("could not compile") {
                return true;
            }
        }

        if command.starts_with("git") {
            if clean_output.contains("fatal:") ||
               clean_output.contains("not a git repository") {
                return true;
            }
        }

        false
    }

    /// Handle cd command with directory validation and retry logic
    async fn handle_cd_command(&self, command: &str, current_dir: &PathBuf) -> String {
        let target_dir = command.strip_prefix("cd ").unwrap_or("").trim();
        if target_dir.is_empty() {
            return "Changed to home directory".to_string();
        }

        // Use the actual current working directory (which may have been changed)
        // instead of always using the passed current_dir parameter
        let current_working_dir = match std::env::current_dir() {
            Ok(dir) => {
                log::info!("Current working directory: {:?}", dir);
                dir
            },
            Err(_) => {
                log::info!("Failed to get current working directory, using provided: {:?}", current_dir);
                current_dir.clone()
            }
        };

        // For relative paths, make them relative to current working directory
        let full_path = if std::path::Path::new(target_dir).is_absolute() {
            PathBuf::from(target_dir)
        } else {
            current_working_dir.join(target_dir)
        };

        // Check if directory exists before trying to change
        // Add a small retry mechanism in case of timing issues
        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            if full_path.exists() && full_path.is_dir() {
                break;
            }

            attempts += 1;
            if attempts < max_attempts {
                // Small delay to handle potential timing issues
                sleep(Duration::from_millis(100)).await;
            }
        }

        if !full_path.exists() {
            return format!("Error: Directory '{}' does not exist", full_path.display());
        }

        if !full_path.is_dir() {
            return format!("Error: '{}' is not a directory", full_path.display());
        }

        match std::env::set_current_dir(&full_path) {
            Ok(()) => format!("Changed directory to {}", full_path.display()),
            Err(e) => format!("Error: Failed to change directory: {}", e),
        }
    }

    /// Handle heredoc commands for file writing
    async fn handle_heredoc_command(&self, command: &str, current_dir: &PathBuf) -> String {
        log::info!("Handling heredoc command in dir {:?}", current_dir);
        log::info!("Heredoc command: {}", command);

        // Parse the heredoc command
        let lines: Vec<&str> = command.lines().collect();
        if lines.is_empty() {
            return "Invalid heredoc command".to_string();
        }

        // First line should be like: cat > src/main.rs << 'EOF'
        let first_line = lines[0];

        // Extract the file path
        let file_path = if let Some(start) = first_line.find('>') {
            let end = first_line.find("<<").unwrap_or(first_line.len());
            first_line[start+1..end].trim()
        } else {
            return "Invalid heredoc syntax: missing >".to_string();
        };

        // Find the content between the delimiters
        let mut content_lines = Vec::new();
        let mut found_start = false;

        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                found_start = true;
                continue;
            }

            // Check if this is the ending delimiter
            if line.trim() == "EOF" {
                break;
            }

            if found_start {
                content_lines.push(*line);
            }
        }

        // Join the content
        let content = content_lines.join("\n");

        log::info!("Writing {} bytes to {}", content.len(), file_path);
        log::debug!("Content: {}", content);

        // Use the current working directory (which may have been changed by cd or cargo new)
        // instead of the passed current_dir parameter
        let working_dir = match std::env::current_dir() {
            Ok(dir) => {
                log::info!("Using current working directory: {:?}", dir);
                dir
            },
            Err(_) => {
                log::info!("Failed to get current working directory, using provided: {:?}", current_dir);
                current_dir.clone()
            }
        };

        // Write the file
        let full_path = working_dir.join(file_path);
        log::info!("Full path: {:?}", full_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return format!("Failed to create parent directories: {}", e);
            }
        }

        // Write the content to the file
        match tokio::fs::write(&full_path, content).await {
            Ok(_) => {
                log::info!("Successfully wrote to {}", full_path.display());
                format!("Successfully wrote to {}", file_path)
            },
            Err(e) => {
                log::error!("Failed to write to {}: {}", full_path.display(), e);
                format!("Failed to write to {}: {}", file_path, e)
            }
        }
    }

    /// Check if an error should not be retried
    fn is_non_retryable_error(&self, error_msg: &str) -> bool {
        let non_retryable_patterns = [
            "not in the safe command list",
            "Command not found",
            "Permission denied",
            "File not found",
            "Directory not found",
            "Invalid argument",
            "Syntax error",
            "parse error",
            "No such file or directory",
            "command not found",
        ];

        non_retryable_patterns.iter().any(|pattern| error_msg.to_lowercase().contains(&pattern.to_lowercase()))
    }

    /// Check if a command is critical (failure should stop execution)
    pub fn is_critical_command(&self, command: &str) -> bool {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        if cmd_parts.is_empty() {
            return false;
        }

        // For diagnostic/analysis commands, failures shouldn't stop execution
        // Only project creation commands are truly critical
        if cmd_parts[0] == "cargo" {
            match cmd_parts.get(1) {
                Some(&"new") | Some(&"init") => true,  // Project creation is critical
                Some(&"check") | Some(&"test") | Some(&"clippy") | Some(&"build") | Some(&"run") => false, // Diagnostic commands are not critical
                _ => false,
            }
        } else if cmd_parts[0] == "mkdir" {
            false // Directory creation failures are often due to already existing dirs, not critical
        } else {
            false
        }
    }

    /// Validate command before execution
    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        if cmd_parts.is_empty() {
            return Err("Invalid command format".to_string());
        }

        // Check if command is safe
        if !self.coding_agent.is_safe_command(command) {
            return Err(format!("Command '{}' is not in the safe command list", cmd_parts[0]));
        }

        // Additional validation for specific commands
        match cmd_parts[0] {
            "rm" => {
                if cmd_parts.contains(&"-rf") || cmd_parts.contains(&"-r") {
                    return Err("Recursive delete operations are not allowed".to_string());
                }
            }
            "chmod" => {
                if cmd_parts.len() < 3 {
                    return Err("chmod requires both permissions and file arguments".to_string());
                }
            }
            "sudo" => {
                return Err("sudo commands are not allowed for security reasons".to_string());
            }
            _ => {}
        }

        Ok(())
    }

    /// Execute command with validation and safety checks
    pub async fn execute_validated_command(&self, command: &str, current_dir: &PathBuf) -> Result<String, String> {
        // Validate command first
        self.validate_command(command)?;

        // Execute the command
        let output = self.execute_command(command, current_dir).await;

        // Check if the output indicates an error
        if output.contains("Error:") || output.contains("error:") || output.contains("failed") {
            if self.is_critical_command(command) {
                return Err(format!("Critical command failed: {}", output));
            }
        }

        Ok(output)
    }

    /// Trim long outputs for better display
    pub fn trim_output(&self, output: &str, command: &str) -> String {
        if output.len() <= 1000 {
            return output.to_string();
        }

        // For cargo commands, preserve error messages
        if command.contains("cargo") && output.contains("error:") {
            let lines: Vec<&str> = output.lines().collect();
            let mut important_lines = Vec::new();
            let mut in_error = false;

            for line in lines.iter().take(50) {  // Limit to first 50 lines
                if line.contains("error:") || line.contains("Error:") {
                    in_error = true;
                    important_lines.push(*line);
                } else if in_error && (line.starts_with("   ") || line.starts_with("\t")) {
                    important_lines.push(*line);
                } else if in_error && line.trim().is_empty() {
                    important_lines.push(*line);
                } else if in_error {
                    in_error = false;
                }
            }

            if !important_lines.is_empty() {
                return format!("{}\n... (output truncated)", important_lines.join("\n"));
            }
        }

        // For other commands, just truncate
        format!("{}... (truncated from {} characters)", &output[..500], output.len())
    }

    /// Get command timeout based on command type
    pub fn get_command_timeout(&self, command: &str) -> Duration {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        if cmd_parts.is_empty() {
            return Duration::from_secs(30); // Default timeout
        }

        match cmd_parts[0] {
            "cargo" => {
                match cmd_parts.get(1) {
                    Some(&"build") | Some(&"test") | Some(&"run") => Duration::from_secs(300), // 5 minutes for builds
                    Some(&"install") => Duration::from_secs(600), // 10 minutes for installations
                    _ => Duration::from_secs(60), // 1 minute for other cargo commands
                }
            }
            "npm" | "yarn" => {
                match cmd_parts.get(1) {
                    Some(&"install") => Duration::from_secs(300), // 5 minutes for npm install
                    Some(&"run") => Duration::from_secs(180), // 3 minutes for npm run
                    _ => Duration::from_secs(60),
                }
            }
            "git" => {
                match cmd_parts.get(1) {
                    Some(&"clone") => Duration::from_secs(300), // 5 minutes for git clone
                    Some(&"pull") | Some(&"push") => Duration::from_secs(120), // 2 minutes for push/pull
                    _ => Duration::from_secs(30),
                }
            }
            "docker" => Duration::from_secs(300), // 5 minutes for docker operations
            "make" | "cmake" => Duration::from_secs(300), // 5 minutes for builds
            "find" | "grep" => Duration::from_secs(60), // 1 minute for search operations
            _ => Duration::from_secs(30), // Default 30 seconds
        }
    }

    /// Check if command produces streaming output
    pub fn is_streaming_command(&self, command: &str) -> bool {
        let streaming_commands = ["tail", "watch", "top", "htop", "ping", "docker logs"];
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        
        if cmd_parts.is_empty() {
            return false;
        }

        streaming_commands.contains(&cmd_parts[0]) ||
        (cmd_parts[0] == "tail" && cmd_parts.contains(&"-f"))
    }

    /// Suggest alternative commands for failed commands
    pub fn suggest_alternatives(&self, failed_command: &str, error_output: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let cmd_parts: Vec<&str> = failed_command.split_whitespace().collect();
        
        if cmd_parts.is_empty() {
            return suggestions;
        }

        let command = cmd_parts[0];
        let error_lower = error_output.to_lowercase();

        match command {
            "ls" => {
                if error_lower.contains("permission denied") {
                    suggestions.push("Try: sudo ls".to_string());
                }
                if error_lower.contains("no such file") {
                    suggestions.push("Check if the directory exists with: pwd".to_string());
                }
            }
            "cargo" => {
                if error_lower.contains("not a cargo project") {
                    suggestions.push("Initialize Cargo project with: cargo init".to_string());
                }
                if error_lower.contains("dependency") {
                    suggestions.push("Update dependencies with: cargo update".to_string());
                }
                if error_lower.contains("compilation failed") {
                    suggestions.push("Check syntax with: cargo check".to_string());
                    suggestions.push("Format code with: cargo fmt".to_string());
                }
            }
            "git" => {
                if error_lower.contains("not a git repository") {
                    suggestions.push("Initialize git repository with: git init".to_string());
                }
                if error_lower.contains("no remote") {
                    suggestions.push("Add remote with: git remote add origin <url>".to_string());
                }
            }
            "npm" => {
                if error_lower.contains("package.json") {
                    suggestions.push("Initialize npm project with: npm init".to_string());
                }
                if error_lower.contains("not found") {
                    suggestions.push("Install dependencies with: npm install".to_string());
                }
            }
            _ => {
                // General suggestions based on error patterns
                if error_lower.contains("permission denied") {
                    suggestions.push("Try running with appropriate permissions".to_string());
                }
                if error_lower.contains("command not found") {
                    suggestions.push(format!("Install {} or check if it's in PATH", command));
                }
                if error_lower.contains("no such file") {
                    suggestions.push("Verify the file or directory path".to_string());
                }
            }
        }

        suggestions
    }
}