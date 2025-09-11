// SAM Context Manager Module
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use super::ExecutedAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub current_directory: String,
    pub preferences: UserPreferences,
    pub conversation_history: Vec<ConversationEntry>,
    pub recent_commands: Vec<CommandHistory>,
    pub system_state: SystemState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPreferences {
    pub preferred_editor: String,
    pub default_file_browser: String,
    pub auto_execute_safe_commands: bool,
    pub confirmation_required_commands: Vec<String>,
    pub favorite_directories: Vec<String>,
    pub custom_aliases: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub sam_response: String,
    pub executed_actions: Vec<ExecutedAction>,
    pub sentiment: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandHistory {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub success: bool,
    pub execution_time_ms: Option<u64>,
    pub working_directory: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemState {
    pub last_known_directories: Vec<String>,
    pub running_services: Vec<String>,
    pub recent_files_accessed: Vec<String>,
    pub active_projects: Vec<ProjectContext>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectContext {
    pub name: String,
    pub path: String,
    pub project_type: String, // "rust", "python", "node", etc.
    pub last_accessed: DateTime<Utc>,
}

impl Default for UserContext {
    fn default() -> Self {
        Self {
            user_id: "localuser".to_string(),
            current_directory: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            preferences: UserPreferences::default(),
            conversation_history: Vec::new(),
            recent_commands: Vec::new(),
            system_state: SystemState::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preferred_editor: "nano".to_string(),
            default_file_browser: "ls".to_string(),
            auto_execute_safe_commands: true,
            confirmation_required_commands: vec![
                "rm".to_string(),
                "mv".to_string(),
                "cp".to_string(),
                "chmod".to_string(),
            ],
            favorite_directories: vec![
                "~/Downloads".to_string(),
                "~/Documents".to_string(),
                "~/Desktop".to_string(),
            ],
            custom_aliases: HashMap::new(),
        }
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            last_known_directories: Vec::new(),
            running_services: Vec::new(),
            recent_files_accessed: Vec::new(),
            active_projects: Vec::new(),
        }
    }
}

/// Load user context from storage
pub async fn load_user_context(user_id: &str) -> UserContext {
    let context_path = get_context_file_path(user_id);
    
    match fs::read_to_string(&context_path).await {
        Ok(content) => {
            match serde_json::from_str::<UserContext>(&content) {
                Ok(mut context) => {
                    context.updated_at = Utc::now();
                    context
                }
                Err(e) => {
                    log::warn!("Failed to parse user context for {}: {}", user_id, e);
                    create_default_context(user_id)
                }
            }
        }
        Err(_) => {
            // Context file doesn't exist, create default
            create_default_context(user_id)
        }
    }
}

/// Save user context to storage
pub async fn save_user_context(user_id: &str, context: &UserContext) {
    let context_path = get_context_file_path(user_id);
    
    // Ensure the context directory exists
    if let Some(parent) = context_path.parent() {
        if let Err(e) = fs::create_dir_all(parent).await {
            log::error!("Failed to create context directory: {}", e);
            return;
        }
    }
    
    let mut updated_context = context.clone();
    updated_context.updated_at = Utc::now();
    
    match serde_json::to_string_pretty(&updated_context) {
        Ok(json) => {
            if let Err(e) = fs::write(&context_path, json).await {
                log::error!("Failed to save user context for {}: {}", user_id, e);
            }
        }
        Err(e) => {
            log::error!("Failed to serialize user context for {}: {}", user_id, e);
        }
    }
}

/// Update context based on conversation and executed actions
pub fn update_context(
    context: &mut UserContext, 
    user_input: &str, 
    sam_response: &str, 
    executed_actions: &[ExecutedAction]
) {
    // Add to conversation history
    let conversation_entry = ConversationEntry {
        timestamp: Utc::now(),
        user_input: user_input.to_string(),
        sam_response: sam_response.to_string(),
        executed_actions: executed_actions.to_vec(),
        sentiment: analyze_sentiment(user_input), // Optional sentiment analysis
    };
    
    context.conversation_history.push(conversation_entry);
    
    // Limit conversation history to last 100 entries
    if context.conversation_history.len() > 100 {
        context.conversation_history.drain(0..context.conversation_history.len() - 100);
    }
    
    // Update command history
    for action in executed_actions {
        let command_entry = CommandHistory {
            timestamp: Utc::now(),
            command: action.command.clone(),
            success: action.success,
            execution_time_ms: None, // Could be added if we track execution time
            working_directory: context.current_directory.clone(),
        };
        context.recent_commands.push(command_entry);
    }
    
    // Limit command history to last 50 commands
    if context.recent_commands.len() > 50 {
        context.recent_commands.drain(0..context.recent_commands.len() - 50);
    }
    
    // Update system state based on executed commands
    update_system_state(context, executed_actions);
    
    // Update current directory if cd command was executed
    for action in executed_actions {
        if action.command.starts_with("cd ") && action.success {
            if let Some(new_dir) = extract_cd_target(&action.command) {
                context.current_directory = new_dir;
            }
        }
    }
    
    context.updated_at = Utc::now();
}

/// Serialize context for passing to brain.py
pub fn serialize_context(context: &UserContext) -> String {
    let summary = ContextSummary {
        current_directory: context.current_directory.clone(),
        recent_commands: context.recent_commands.iter()
            .take(5)
            .map(|cmd| cmd.command.clone())
            .collect(),
        favorite_directories: context.preferences.favorite_directories.clone(),
        running_services: context.system_state.running_services.clone(),
        conversation_context: extract_conversation_context(&context.conversation_history),
    };
    
    serde_json::to_string(&summary).unwrap_or_default()
}

#[derive(Serialize, Deserialize)]
struct ContextSummary {
    current_directory: String,
    recent_commands: Vec<String>,
    favorite_directories: Vec<String>,
    running_services: Vec<String>,
    conversation_context: String,
}

fn create_default_context(user_id: &str) -> UserContext {
    let mut context = UserContext::default();
    context.user_id = user_id.to_string();
    context
}

fn get_context_file_path(user_id: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("sam_contexts");
    path.push(format!("{}.json", user_id));
    path
}

fn analyze_sentiment(text: &str) -> Option<String> {
    // Simple sentiment analysis based on keywords
    let positive_words = ["thanks", "thank you", "great", "awesome", "good", "excellent", "perfect"];
    let negative_words = ["error", "problem", "issue", "wrong", "bad", "terrible", "awful"];
    
    let text_lower = text.to_lowercase();
    
    let positive_count = positive_words.iter().filter(|word| text_lower.contains(*word)).count();
    let negative_count = negative_words.iter().filter(|word| text_lower.contains(*word)).count();
    
    if positive_count > negative_count {
        Some("positive".to_string())
    } else if negative_count > positive_count {
        Some("negative".to_string())
    } else {
        Some("neutral".to_string())
    }
}

fn update_system_state(context: &mut UserContext, executed_actions: &[ExecutedAction]) {
    for action in executed_actions {
        // Track service commands
        if action.command.contains("start") && action.success {
            let service = extract_service_name(&action.command);
            if let Some(service_name) = service {
                if !context.system_state.running_services.contains(&service_name) {
                    context.system_state.running_services.push(service_name);
                }
            }
        }
        
        if action.command.contains("stop") && action.success {
            let service = extract_service_name(&action.command);
            if let Some(service_name) = service {
                context.system_state.running_services.retain(|s| s != &service_name);
            }
        }
        
        // Track file access
        if action.command.starts_with("cat ") || 
           action.command.starts_with("less ") || 
           action.command.starts_with("nano ") {
            if let Some(filename) = extract_filename(&action.command) {
                context.system_state.recent_files_accessed.push(filename);
                
                // Limit to last 20 files
                if context.system_state.recent_files_accessed.len() > 20 {
                    context.system_state.recent_files_accessed.remove(0);
                }
            }
        }
    }
}

fn extract_service_name(command: &str) -> Option<String> {
    let services = ["redis", "spotify", "lifx", "docker", "crawler", "postgres", "pg"];
    for service in services {
        if command.contains(service) {
            return Some(service.to_string());
        }
    }
    None
}

fn extract_filename(command: &str) -> Option<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() > 1 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

fn extract_cd_target(command: &str) -> Option<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() > 1 && parts[0] == "cd" {
        Some(parts[1].to_string())
    } else {
        None
    }
}

fn extract_conversation_context(history: &[ConversationEntry]) -> String {
    // Extract the last few relevant conversation points
    history.iter()
        .rev()
        .take(3)
        .map(|entry| format!("User: {} | Sam: {}", entry.user_input, entry.sam_response))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_context() {
        let context = UserContext::default();
        assert_eq!(context.user_id, "localuser");
        assert!(!context.current_directory.is_empty());
    }
    
    #[test]
    fn test_analyze_sentiment() {
        assert_eq!(analyze_sentiment("Thanks for the help!"), Some("positive".to_string()));
        assert_eq!(analyze_sentiment("This is an error"), Some("negative".to_string()));
        assert_eq!(analyze_sentiment("Hello world"), Some("neutral".to_string()));
    }
    
    #[test]
    fn test_extract_service_name() {
        assert_eq!(extract_service_name("redis start"), Some("redis".to_string()));
        assert_eq!(extract_service_name("spotify pause"), Some("spotify".to_string()));
        assert_eq!(extract_service_name("unknown command"), None);
    }
}