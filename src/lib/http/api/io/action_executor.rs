// SAM Action Executor Module
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use super::command_parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy)]
pub enum ExecutionContext {
    Web,
    Tui,
    Api,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Execute an action command using the existing TUI command infrastructure
pub async fn execute_action(command: &str) -> ActionResult {
    execute_action_with_context(command, ExecutionContext::Web).await
}

/// Execute an action command with specific execution context
pub async fn execute_action_with_context(command: &str, context: ExecutionContext) -> ActionResult {
    // Validate command first
    if !command_parser::validate_command(command) {
        return ActionResult {
            success: false,
            output: String::new(),
            error: Some("Command not allowed for security reasons".to_string()),
        };
    }

    // Check if this command needs context-specific adaptation
    if let Some(adaptation) = get_context_adaptation(command, context) {
        return ActionResult {
            success: true,
            output: adaptation,
            error: None,
        };
    }

    // Create output buffer to capture command results
    let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let current_dir = Arc::new(Mutex::new(
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
    ));

    // Create a command context similar to TUI
    let mut ctx = crate::cli::commands::CommandContext {
        output_lines: &output_lines,
        current_dir: &mut *current_dir.lock().await,
        human_name: "AI Assistant",
        output_height: 25,
        scroll_offset: &mut 0,
    };

    // Execute the command using the existing router
    crate::cli::commands::router::route_command(command, &mut ctx).await;

    // Collect results
    let results = {
        let output = output_lines.lock().await;
        output.clone()
    };

    // Format output
    let output_text = if results.is_empty() {
        "Command executed successfully".to_string()
    } else {
        results.join("\n")
    };

    // Check if there were any error indicators in the output
    let has_error = results.iter().any(|line| {
        line.to_lowercase().contains("error")
            || line.to_lowercase().contains("failed")
            || line.to_lowercase().contains("not found")
            || line.to_lowercase().contains("permission denied")
    });

    ActionResult {
        success: !has_error,
        output: output_text,
        error: if has_error {
            Some("Command execution had errors".to_string())
        } else {
            None
        },
    }
}

/// Execute multiple commands in sequence
pub async fn execute_actions_batch(commands: &[String]) -> Vec<ActionResult> {
    let mut results = Vec::new();

    for command in commands {
        let result = execute_action(command).await;
        results.push(result);

        // If a command fails, we might want to stop execution
        // This can be configurable based on command type
        if results.last().is_some_and(|result| !result.success) {
            // For now, continue executing remaining commands
            // but this could be made configurable
        }
    }

    results
}

/// Execute a command with specific environment or context
pub async fn execute_action_with_env(
    command: &str,
    working_dir: Option<PathBuf>,
    context_vars: Option<std::collections::HashMap<String, String>>,
) -> ActionResult {
    // This is a more advanced version that could handle:
    // - Custom working directories
    // - Environment variables
    // - User-specific contexts

    // For now, use the basic execute_action but this can be extended
    let mut modified_command = command.to_string();

    // Handle working directory changes
    if let Some(dir) = working_dir {
        if command.starts_with("ls") || command.starts_with("find") || command.starts_with("cat") {
            // Modify command to use the specific directory
            // This is a simplified approach - a more robust solution would
            // temporarily change the working directory for the command execution
        }
    }

    execute_action(&modified_command).await
}

/// Convert natural language to command and execute
pub async fn execute_natural_language_action(natural_text: &str) -> ActionResult {
    match command_parser::parse_natural_language(natural_text) {
        Some(command) => {
            log::info!(
                "Parsed natural language '{}' to command '{}'",
                natural_text,
                command
            );
            execute_action(&command).await
        }
        None => ActionResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Could not parse natural language request: {}",
                natural_text
            )),
        },
    }
}

/// Helper function to determine if a command is safe for automatic execution
pub fn is_safe_for_auto_execution(command: &str) -> bool {
    // Define commands that are safe to run automatically
    let safe_commands = vec![
        "ls", "pwd", "cat", "less", "head", "tail", "grep", "find", "echo", "wc", "sort", "date",
        "whoami", "uname", "df", "du", "ps", "top", "status", "help", "clear",
    ];

    let safe_service_commands = vec![
        "redis status",
        "spotify status",
        "lifx status",
        "docker status",
        "crawler status",
        "pg status",
        "postgres status",
    ];

    let first_word = command.split_whitespace().next().unwrap_or("");

    // Check if it's a basic safe command
    if safe_commands.contains(&first_word) {
        return true;
    }

    // Check if it's a safe service status command
    if safe_service_commands
        .iter()
        .any(|safe_cmd| command.starts_with(safe_cmd))
    {
        return true;
    }

    // Be conservative - require explicit approval for other commands
    false
}

/// Context-aware command adaptations
fn get_context_adaptation(command: &str, context: ExecutionContext) -> Option<String> {
    // Extract the base command (first word) for matching
    let base_cmd = command.split_whitespace().next().unwrap_or(command);

    match (base_cmd, context) {
        ("clear" | "cls", ExecutionContext::Web) => Some("WEB_ACTION:CLEAR_SCREEN".to_string()),
        ("clear" | "cls", ExecutionContext::Tui) => Some("TUI_ACTION:CLEAR_SCREEN".to_string()),
        // Future adaptations can be added here:
        // ("top", ExecutionContext::Web) => Some("WEB_ACTION:SCROLL_TO_TOP".to_string()),
        // ("end", ExecutionContext::Web) => Some("WEB_ACTION:SCROLL_TO_BOTTOM".to_string()),
        // ("refresh", ExecutionContext::Web) => Some("WEB_ACTION:REFRESH_PAGE".to_string()),
        _ => None,
    }
}

/// Get command execution confirmation message
pub fn get_execution_confirmation(command: &str) -> String {
    if is_safe_for_auto_execution(command) {
        format!("Executing: {}", command)
    } else {
        format!(
            "⚠️  About to execute: {} - This command will make changes to your system.",
            command
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_safe_command() {
        let result = execute_action("pwd").await;
        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_execute_unsafe_command() {
        let result = execute_action("sudo rm -rf /").await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_is_safe_for_auto_execution() {
        assert!(is_safe_for_auto_execution("ls -la"));
        assert!(is_safe_for_auto_execution("pwd"));
        assert!(is_safe_for_auto_execution("redis status"));
        assert!(!is_safe_for_auto_execution("rm -rf ~/Downloads/*"));
        assert!(!is_safe_for_auto_execution("redis start"));
    }
}
