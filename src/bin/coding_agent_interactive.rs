use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use libsam::services::coding::agent::{
    CodingAgentService,
    InteractiveExecutor,
    UserMessage,
};

#[derive(Parser)]
#[command(
    name = "coding_agent_interactive",
    about = "Interactive AI-powered coding agent with verification and correction"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Task to execute (if not using subcommand)
    task: Option<String>,

    /// Working directory
    #[arg(short = 'd', long, default_value = ".")]
    dir: PathBuf,

    /// Maximum correction attempts
    #[arg(short = 'c', long, default_value = "3")]
    max_corrections: u32,

    /// Enable interactive mode (allows typing during execution)
    #[arg(short = 'i', long)]
    interactive: bool,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a task with interactive verification
    Execute {
        /// Task description
        task: String,
    },
    /// Start an interactive session
    Interactive,
    /// Show help for available commands
    Help,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    // Get task from either positional argument or subcommand
    let task = match &cli.command {
        Some(Commands::Execute { task }) => Some(task.clone()),
        Some(Commands::Interactive) => {
            println!("🤖 Interactive Coding Agent");
            println!("Type your task or 'quit' to exit");
            None
        }
        Some(Commands::Help) | None => cli.task.clone(),
    };

    if let Some(task_desc) = task {
        execute_task(task_desc, cli.dir, cli.max_corrections, cli.interactive, cli.verbose).await?;
    } else if matches!(cli.command, Some(Commands::Interactive)) {
        run_interactive_session(cli.dir, cli.max_corrections, cli.verbose).await?;
    } else {
        println!("Please provide a task to execute.");
        println!("Usage: coding_agent_interactive <TASK>");
        println!("   or: coding_agent_interactive execute <TASK>");
        println!("   or: coding_agent_interactive interactive");
    }

    Ok(())
}

async fn execute_task(
    task: String,
    working_dir: PathBuf,
    max_corrections: u32,
    interactive: bool,
    verbose: bool,
) -> Result<()> {
    println!("🚀 Starting Interactive Coding Agent");
    println!("📋 Task: {}", task);
    println!("📁 Working directory: {}", working_dir.display());
    println!("🔄 Max correction attempts: {}", max_corrections);
    println!();

    // Initialize coding agent
    let coding_agent = Arc::new(CodingAgentService::new_with_defaults().await);
    let executor = InteractiveExecutor::new(coding_agent);

    // Set up message channel if interactive mode is enabled
    let message_sender = if interactive {
        println!("💬 Interactive mode enabled. You can type messages during execution.");
        println!("   Messages will be queued and processed without interrupting execution.");
        println!();

        let sender = executor.setup_message_channel().await;

        // Spawn task to read user input
        let executor_clone = executor.clone();
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            let stdin = io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let msg = line.trim();
                        if !msg.is_empty() {
                            let user_msg = UserMessage {
                                content: msg.to_string(),
                                timestamp: std::time::SystemTime::now(),
                            };
                            if sender_clone.send(user_msg.clone()).await.is_ok() {
                                executor_clone.queue_message(msg.to_string()).await;
                                println!("📨 Message queued: {}", msg);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        break;
                    }
                }
            }
        });

        Some(sender)
    } else {
        None
    };

    // Execute task with verification
    match executor.execute_with_verification(
        &task,
        &working_dir,
        &[],
        max_corrections
    ).await {
        Ok(_) => {
            println!("\n✅ Task completed successfully!");

            // Show final context if verbose
            if verbose {
                let context = executor.get_context().await;
                println!("\n📊 Execution Summary:");
                println!("   Commands executed: {}", context.command_history.len());
                println!("   User messages: {}", context.user_messages.len());

                if !context.execution_log.is_empty() {
                    println!("\n📜 Execution Log:");
                    for entry in &context.execution_log {
                        println!("   {}", entry);
                    }
                }
            }
        }
        Err(e) => {
            println!("\n❌ Task failed: {}", e);

            // Show context on failure for debugging
            let context = executor.get_context().await;
            if !context.command_history.is_empty() {
                println!("\n📜 Command History:");
                for (i, (cmd, output)) in context.command_history.iter().enumerate() {
                    println!("   {}. {}", i + 1, cmd);
                    if verbose && !output.trim().is_empty() {
                        println!("      Output: {}", output.lines().next().unwrap_or(""));
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_interactive_session(
    working_dir: PathBuf,
    max_corrections: u32,
    verbose: bool,
) -> Result<()> {
    let coding_agent = Arc::new(CodingAgentService::new_with_defaults().await);
    let executor = InteractiveExecutor::new(coding_agent);

    println!("🤖 Interactive Coding Agent Session");
    println!("📁 Working directory: {}", working_dir.display());
    println!("Type your tasks or commands. Type 'quit' to exit.\n");

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    let mut session_context = Vec::new();

    loop {
        print!("> ");
        io::stdout().flush().await?;

        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let task = line.trim();

                if task.is_empty() {
                    continue;
                }

                if task == "quit" || task == "exit" {
                    println!("Goodbye!");
                    break;
                }

                if task == "clear" {
                    executor.clear_history_preserve_messages().await;
                    println!("History cleared (messages preserved)");
                    continue;
                }

                if task == "context" {
                    let context = executor.get_context().await;
                    println!("Current Context:");
                    println!("  Working dir: {}", context.working_directory.display());
                    println!("  Commands executed: {}", context.command_history.len());
                    println!("  User messages: {}", context.user_messages.len());
                    continue;
                }

                // Execute task
                println!("\n🚀 Executing: {}", task);
                session_context.push(format!("Previous task: {}", task));

                match executor.execute_with_verification(
                    task,
                    &working_dir,
                    &session_context,
                    max_corrections
                ).await {
                    Ok(_) => {
                        println!("✅ Task completed!");
                    }
                    Err(e) => {
                        println!("❌ Error: {}", e);
                    }
                }
                println!();
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    Ok(())
}