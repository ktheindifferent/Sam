use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use libsam::services::coding::agent::{CodingAgentExecutor, CodingAgentService};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Get task from command line args
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <task>", args[0]);
        std::process::exit(1);
    }

    let task = args[1..].join(" ");
    let current_dir = PathBuf::from(".");

    println!("🚀 Starting Coding Agent");
    println!("📋 Task: {}", task);
    println!();

    // Initialize coding agent
    let coding_agent = Arc::new(CodingAgentService::new_with_defaults().await);
    let executor = CodingAgentExecutor::new(coding_agent);

    // Execute task
    match executor
        .execute_incremental_task(&task, &current_dir, &[], true)
        .await
    {
        Ok(_) => {
            println!("\n✅ Task completed!");

            // Get and display execution log
            let execution_log = executor.get_execution_log().await;
            if !execution_log.is_empty() {
                println!("\n📜 Incremental Execution Log:");
                for entry in execution_log {
                    println!("{}", entry);
                }
            }
        }
        Err(e) => {
            eprintln!("\n❌ Task failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
