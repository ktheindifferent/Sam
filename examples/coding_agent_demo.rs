// Coding Agent Demo - Showcasing new features
// This example demonstrates the enhanced coding agent capabilities:
// - Streaming responses
// - Conversation memory
// - Multi-turn conversations
// - Error recovery and fallback mechanisms

use sam::lib::services::coding::agent::{
    CodingAgentService, CodingAgentConfig, CodingAgentExecutor
};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== SAM Coding Agent Demo ===\n");

    // Create the coding agent with default configuration
    let config = CodingAgentConfig::default();
    let agent = CodingAgentService::new(config);

    // Get current directory
    let current_dir = std::env::current_dir()?;

    // Demo 1: Basic conversation
    println!("Demo 1: Basic Conversation");
    println!("--------------------------");

    let response = agent.ask_question(
        "What is the best way to handle errors in Rust?",
        &current_dir,
        &[]
    ).await?;

    println!("Agent: {}\n", response);

    // Demo 2: Multi-turn conversation with context
    println!("Demo 2: Multi-turn Conversation");
    println!("--------------------------------");

    // First turn
    let response1 = agent.generate_contextual_response(
        "Can you help me create a simple web server in Rust?",
        &current_dir,
        None
    ).await?;

    println!("User: Can you help me create a simple web server in Rust?");
    println!("Agent: {}\n", response1.response_text);

    // Second turn (uses conversation memory)
    let response2 = agent.generate_contextual_response(
        "How do I add routing to it?",
        &current_dir,
        None
    ).await?;

    println!("User: How do I add routing to it?");
    println!("Agent: {}\n", response2.response_text);

    // Demo 3: Streaming response
    println!("Demo 3: Streaming Response");
    println!("--------------------------");

    agent.set_streaming_mode(true).await;

    let mut receiver = agent.generate_streaming_response(
        "Write a function to calculate fibonacci numbers",
        &current_dir,
        None
    ).await?;

    print!("Agent (streaming): ");
    while let Some(chunk) = receiver.recv().await {
        print!("{}", chunk);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; // Simulate real-time
    }
    println!("\n");

    // Demo 4: Error recovery with fallback
    println!("Demo 4: Error Recovery");
    println!("----------------------");

    // This will try multiple models if the first one fails
    let safe_response = agent.safe_generate_response(
        "Explain async/await in Rust",
        &current_dir,
        &[],
        Some("nonexistent-model") // Will fallback to working models
    ).await?;

    println!("Agent (with fallback): {}\n", safe_response.response_text);

    // Demo 5: Command execution with retry
    println!("Demo 5: Command Execution");
    println!("-------------------------");

    let executor = CodingAgentExecutor::new(agent);

    let execution_result = executor.execute_incremental_task(
        "Create a new Rust project called 'demo' and add a hello world function",
        &current_dir
    ).await?;

    println!("Execution completed: {:?}\n", execution_result.state);

    // Demo 6: Conversation management
    println!("Demo 6: Conversation Management");
    println!("-------------------------------");

    // Get conversation history
    let history = agent.get_conversation_context().await;
    println!("Conversation history: {} messages", history.len());

    for (i, msg) in history.iter().enumerate() {
        println!("  {}. [{}] {}", i + 1, msg.role,
            if msg.content.len() > 50 {
                format!("{}...", &msg.content[..50])
            } else {
                msg.content.clone()
            }
        );
    }

    // Clear conversation
    agent.clear_conversation().await;
    println!("\nConversation cleared!");

    println!("\n=== Demo Complete ===");

    Ok(())
}