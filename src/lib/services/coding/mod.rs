// Coding Services Module
// This module provides AI-powered coding assistance and automation services.

pub mod agent;

// Re-export the main coding agent functionality
pub use agent::{CodingAgentConfig, CodingAgentExecutor, CodingAgentService};
