// Coding Agent Demo - Showcasing new features
// This example demonstrates the enhanced coding agent capabilities:
// - Streaming responses
// - Conversation memory
// - Multi-turn conversations
// - Error recovery and fallback mechanisms

use libsam::services::coding::agent::{CodingAgentConfig, CodingAgentExecutor, CodingAgentService};
use std::path::PathBuf;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== SAM Coding Agent Demo ===\n");

    // Create the coding agent with default configuration
    let config = CodingAgentConfig::default();
    let agent = CodingAgentService::new(config).await;

    // Get current directory
    let current_dir = std::env::current_dir()?;

    // Demo 1: Basic response generation
    println!("Demo 1: Basic Response Generation");
    println!("---------------------------------");

    let response = agent
        .generate_response(
            "What is the best way to handle errors in Rust?",
            &current_dir,
            &[],
            None,
        )
        .await?;

    println!("Agent response: {}\n", response.response_text);

    // Demo 2: Response with context
    println!("Demo 2: Contextual Response");
    println!("---------------------------");

    let response2 = agent
        .generate_response(
            "Write a simple hello world function in Rust",
            &current_dir,
            &["This is for a beginner tutorial".to_string()],
            None,
        )
        .await?;

    println!("Agent response: {}\n", response2.response_text);

    // Demo 3: Using the executor
    println!("Demo 3: Task Executor");
    println!("---------------------");

    let agent_arc = Arc::new(agent);
    let mut executor = CodingAgentExecutor::new(agent_arc);

    println!("Created executor for incremental task execution.");
    println!("The executor can:");
    println!("  - Execute tasks incrementally");
    println!("  - Enable verification mode");
    println!("  - Track command history");
    println!("  - Handle user messages\n");

    // Demo 4: Get execution summary
    let summary = executor.get_summary().await;
    println!("Executor status: {}\n", summary);

    println!("=== Demo Complete ===");
    println!("For interactive usage, see the coding_agent_interactive binary.");

    Ok(())
}
