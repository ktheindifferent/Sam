use std::sync::Arc;
use std::path::PathBuf;
use sam::services::coding::{CodingAgentService, CodingAgentExecutor};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    println!("Testing Coding Agent...");

    // Create the coding agent service
    let coding_agent = Arc::new(CodingAgentService::new_with_defaults());
    let executor = CodingAgentExecutor::new(coding_agent);

    // Test task
    let task = "create a simple hello world file";
    let current_dir = PathBuf::from(".");
    let session_context = vec![];

    println!("Executing task: {}", task);

    // Execute the task
    match executor.execute_incremental_task(
        task,
        &current_dir,
        &session_context,
        true,  // auto_execute
    ).await {
        Ok(_) => println!("Task completed successfully!"),
        Err(e) => eprintln!("Task failed: {}", e),
    }
}