//! Conversation and messaging models

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Conversation message for multi-turn support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String, // "user", "assistant", or "system"
    pub content: String,
    pub timestamp: SystemTime,
}

/// Conversation memory with context window management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    pub messages: Vec<ConversationMessage>,
    pub max_messages: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
}

impl Default for ConversationMemory {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            max_messages: 50,
            total_tokens: 0,
            max_tokens: 8192,
        }
    }
}

impl ConversationMemory {
    /// Add a message to the conversation
    pub fn add_message(&mut self, role: String, content: String) {
        let message = ConversationMessage {
            role,
            content: content.clone(),
            timestamp: SystemTime::now(),
        };

        self.messages.push(message);

        // Estimate tokens (rough approximation)
        self.total_tokens += content.len() / 4;

        // Trim if necessary
        self.trim_to_limits();
    }

    /// Trim messages to stay within limits
    pub fn trim_to_limits(&mut self) {
        // Remove oldest messages if exceeding max_messages
        while self.messages.len() > self.max_messages {
            if let Some(removed) = self.messages.first() {
                self.total_tokens = self.total_tokens.saturating_sub(removed.content.len() / 4);
            }
            self.messages.remove(0);
        }

        // Remove messages if exceeding token limit
        while self.total_tokens > self.max_tokens && !self.messages.is_empty() {
            if let Some(removed) = self.messages.first() {
                self.total_tokens = self.total_tokens.saturating_sub(removed.content.len() / 4);
            }
            self.messages.remove(0);
        }
    }

    /// Get the last N messages
    pub fn get_last_messages(&self, n: usize) -> Vec<ConversationMessage> {
        let start = self.messages.len().saturating_sub(n);
        self.messages[start..].to_vec()
    }

    /// Clear the conversation
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_tokens = 0;
    }

    /// Get conversation as prompt format
    pub fn to_prompt(&self) -> String {
        self.messages
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub session_id: String,
    pub user_id: Option<String>,
    pub project_context: Option<String>,
    pub working_directory: String,
    pub active_files: Vec<String>,
    pub variables: std::collections::HashMap<String, String>,
}
