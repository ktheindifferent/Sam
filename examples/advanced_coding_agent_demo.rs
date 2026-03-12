// Advanced Coding Agent Demo
// This example shows more advanced features like:
// - Code analysis and intelligence
// - Refactoring suggestions
// - Debugging assistance
// - Code review

use libsam::services::coding::agent::{
    CodingAgentService, CodingAgentConfig
};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== SAM Advanced Coding Agent Demo ===\n");

    // Create the coding agent with custom configuration
    let mut config = CodingAgentConfig::default();
    config.resources.max_memory_mb = 2048;
    config.core.max_context_size = 10000;

    let agent = CodingAgentService::new(config).await;
    let current_dir = std::env::current_dir()?;

    // Demo 1: Code analysis
    println!("Demo 1: Code Analysis Request");
    println!("-----------------------------");

    let analysis = agent.generate_response(
        "Analyze this code for potential improvements: fn add(a: i32, b: i32) -> i32 { a + b }",
        &current_dir,
        &["Focus on: performance, safety, documentation".to_string()],
        None
    ).await?;

    println!("Analysis: {}\n", analysis.response_text);

    // Demo 2: Refactoring suggestions
    println!("Demo 2: Refactoring Suggestions");
    println!("-------------------------------");

    let refactor = agent.generate_response(
        "How would you refactor a function with 10 parameters?",
        &current_dir,
        &["Language: Rust".to_string(), "Context: Web API handler".to_string()],
        None
    ).await?;

    println!("Suggestions: {}\n", refactor.response_text);

    // Demo 3: Debugging assistance
    println!("Demo 3: Debugging Assistance");
    println!("----------------------------");

    let debug_help = agent.generate_response(
        "My Rust program panics with 'index out of bounds'. How do I debug this?",
        &current_dir,
        &["Using: Vec operations".to_string(), "No external debugger available".to_string()],
        None
    ).await?;

    println!("Debug advice: {}\n", debug_help.response_text);

    // Demo 4: Code review
    println!("Demo 4: Code Review Request");
    println!("---------------------------");

    let review = agent.generate_response(
        "Review this error handling: result.unwrap_or_else(|e| panic!(\"Error: {}\", e))",
        &current_dir,
        &["Production code".to_string(), "High reliability required".to_string()],
        None
    ).await?;

    println!("Review: {}\n", review.response_text);

    // Demo 5: Best practices
    println!("Demo 5: Best Practices");
    println!("----------------------");

    let practices = agent.generate_response(
        "What are the best practices for async Rust programming?",
        &current_dir,
        &["Using: Tokio".to_string(), "Building: REST API server".to_string()],
        None
    ).await?;

    println!("Best practices: {}\n", practices.response_text);

    println!("=== Advanced Demo Complete ===");
    println!("The coding agent can help with:");
    println!("  ✓ Code analysis and review");
    println!("  ✓ Refactoring suggestions");
    println!("  ✓ Debugging assistance");
    println!("  ✓ Best practices guidance");
    println!("  ✓ Performance optimization");

    Ok(())
}