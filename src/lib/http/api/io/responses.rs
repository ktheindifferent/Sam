// SAM Responses Module
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/// This module handles response formatting and templating for the IO system
/// It provides consistent response patterns for different types of actions
use super::ExecutedAction;

/// Generate a response that includes action execution results
pub fn format_response_with_actions(
    original_response: &str,
    executed_actions: &[ExecutedAction],
) -> String {
    let mut formatted_response = original_response.to_string();

    if !executed_actions.is_empty() {
        formatted_response.push_str("\n\n");
        formatted_response.push_str("**Actions executed:**\n");

        for action in executed_actions {
            if action.success {
                formatted_response.push_str(&format!("✅ {}\n", action.command));
                if !action.result.is_empty() {
                    formatted_response.push_str(&format!("   Result: {}\n", action.result));
                }
            } else {
                formatted_response.push_str(&format!("❌ {} (failed)\n", action.command));
                if !action.result.is_empty() {
                    formatted_response.push_str(&format!("   Error: {}\n", action.result));
                }
            }
        }
    }

    formatted_response
}

/// Generate confirmation messages for potentially destructive actions
pub fn get_action_confirmation_message(command: &str) -> Option<String> {
    if command.starts_with("rm ") {
        Some(format!(
            "⚠️ This will delete files/directories: {}",
            command
        ))
    } else if command.starts_with("mv ") {
        Some(format!("ℹ️ This will move/rename: {}", command))
    } else if command.contains("start") || command.contains("stop") {
        Some(format!("🔧 This will control a service: {}", command))
    } else {
        None
    }
}

/// Generate contextual help for commands
pub fn get_command_help(command: &str) -> Option<String> {
    let first_word = command.split_whitespace().next().unwrap_or("");

    match first_word {
        "rm" => {
            Some("Remove files and directories. Use -f for force, -r for recursive.".to_string())
        }
        "cp" => Some("Copy files and directories. Use -r for recursive copying.".to_string()),
        "mv" => Some("Move/rename files and directories.".to_string()),
        "ls" => Some("List directory contents. Use -la for detailed listing.".to_string()),
        "cat" => Some("Display file contents.".to_string()),
        "grep" => Some("Search for patterns in files. Use -i for case-insensitive.".to_string()),
        "find" => Some("Search for files and directories.".to_string()),
        "redis" => {
            Some("Control Redis service. Commands: start, stop, status, install.".to_string())
        }
        "spotify" => Some(
            "Control Spotify. Commands: start, stop, play, pause, shuffle, status.".to_string(),
        ),
        "lifx" => Some("Control LIFX lights. Commands: start, stop, status.".to_string()),
        _ => None,
    }
}

/// Generate smart suggestions based on command context
pub fn get_smart_suggestions(command: &str, success: bool) -> Vec<String> {
    let mut suggestions = Vec::new();

    if !success {
        // Suggest alternatives for failed commands
        if command.starts_with("ls") {
            suggestions.push("Try 'ls -la' for detailed listing".to_string());
        } else if command.starts_with("cat") {
            suggestions.push("File might not exist. Try 'ls' to see available files".to_string());
        } else if command.contains("permission denied") {
            suggestions.push("You might need different permissions for this operation".to_string());
        }
    } else {
        // Suggest next logical steps for successful commands
        if command.starts_with("cd") {
            suggestions.push("Use 'ls' to see contents of the new directory".to_string());
        } else if command.starts_with("ls") {
            suggestions.push("Use 'cat <filename>' to view file contents".to_string());
        } else if command.contains("redis start") {
            suggestions.push("Use 'redis status' to verify it's running".to_string());
        }
    }

    suggestions
}

/// Template responses for common scenarios
pub struct ResponseTemplates;

impl ResponseTemplates {
    pub fn file_operation_success(operation: &str, target: &str) -> String {
        match operation {
            "delete" | "rm" => format!("✅ Successfully deleted {}", target),
            "copy" | "cp" => format!("✅ Successfully copied {}", target),
            "move" | "mv" => format!("✅ Successfully moved {}", target),
            "create" | "mkdir" => format!("✅ Successfully created {}", target),
            _ => format!("✅ Successfully performed {} on {}", operation, target),
        }
    }

    pub fn service_operation_success(service: &str, operation: &str) -> String {
        match operation {
            "start" => format!("✅ {} service started successfully", service),
            "stop" => format!("✅ {} service stopped successfully", service),
            "restart" => format!("✅ {} service restarted successfully", service),
            "status" => format!("ℹ️ {} service status checked", service),
            _ => format!("✅ {} service {} completed", service, operation),
        }
    }

    pub fn error_response(command: &str, error: &str) -> String {
        format!("❌ Command '{}' failed: {}", command, error)
    }

    pub fn safety_warning(command: &str) -> String {
        format!("⚠️ Command '{}' was blocked for security reasons", command)
    }

    pub fn natural_language_success(intent: &str, executed_command: &str) -> String {
        format!(
            "✅ Understood '{}' and executed: {}",
            intent, executed_command
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_response_with_actions() {
        let actions = vec![ExecutedAction {
            command: "ls -la".to_string(),
            result: "file1.txt  file2.txt".to_string(),
            success: true,
        }];

        let formatted = format_response_with_actions("Here are your files:", &actions);
        assert!(formatted.contains("✅ ls -la"));
        assert!(formatted.contains("file1.txt  file2.txt"));
    }

    #[test]
    fn test_response_templates() {
        assert_eq!(
            ResponseTemplates::file_operation_success("delete", "test.txt"),
            "✅ Successfully deleted test.txt"
        );

        assert_eq!(
            ResponseTemplates::service_operation_success("redis", "start"),
            "✅ redis service started successfully"
        );
    }
}
