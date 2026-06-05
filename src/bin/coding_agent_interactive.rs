use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use libsam::services::coding::agent::{CodingAgentExecutor, CodingAgentService, UserMessage};

// Simple CLI args structure without clap
struct Cli {
    command: Option<Commands>,
    task: Option<String>,
    dir: PathBuf,
    max_corrections: u32,
    interactive: bool,
    verbose: bool,
}

impl Cli {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();

        // Simple argument parsing without clap
        let mut cli = Cli {
            command: None,
            task: None,
            dir: PathBuf::from("."),
            max_corrections: 3,
            interactive: false,
            verbose: false,
        };

        // Parse basic flags
        for (i, arg) in args.iter().enumerate() {
            match arg.as_str() {
                "-d" | "--dir" => {
                    if let Some(dir) = args.get(i + 1) {
                        cli.dir = PathBuf::from(dir);
                    }
                }
                "-c" | "--max-corrections" => {
                    if let Some(val) = args.get(i + 1) {
                        cli.max_corrections = val.parse().unwrap_or(3);
                    }
                }
                "-i" | "--interactive" => cli.interactive = true,
                "-v" | "--verbose" => cli.verbose = true,
                "execute" => {
                    if let Some(task) = args.get(i + 1) {
                        cli.command = Some(Commands::Execute { task: task.clone() });
                    }
                }
                "session" | "interactive" => cli.command = Some(Commands::Interactive),
                _ => {
                    // If not a flag and not the program name, treat as task
                    if i > 0
                        && !arg.starts_with('-')
                        && args.get(i - 1).map(|a| !a.starts_with('-')).unwrap_or(true)
                    {
                        cli.task = Some(arg.clone());
                    }
                }
            }
        }

        cli
    }
}

enum Commands {
    Execute {
        task: String,
    },
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
        execute_task(
            task_desc,
            cli.dir,
            cli.max_corrections,
            cli.interactive,
            cli.verbose,
        )
        .await?;
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
    let mut executor = CodingAgentExecutor::new(coding_agent);

    // Set up message channel if interactive mode is enabled
    if interactive {
        println!("💬 Interactive mode enabled. You can type messages during execution.");
        println!("   Messages will be queued and processed without interrupting execution.");
        println!();

        let mut _receiver = executor.setup_message_channel();

        // Spawn task to read user input
        let executor_clone = executor.clone();
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
                            executor_clone.queue_message(msg.to_string()).await;
                            println!("📨 Message queued: {}", msg);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        break;
                    }
                }
            }
        });
    }

    // Execute task with verification
    if max_corrections > 0 {
        // Enable verification mode
        executor.enable_verification().await;
    }

    match executor
        .execute_incremental_task_with_verification(
            &task,
            &working_dir,
            &[], // session context
        )
        .await
    {
        Ok(_) => {
            println!("\n✅ Task completed successfully!");

            // Show final context if verbose
            if verbose {
                let summary = executor.get_summary().await;
                let history = executor.get_command_history().await;

                println!("\n📊 Execution Summary:");
                println!("   {}", summary);
                println!("   Commands executed: {}", history.len());

                if !history.is_empty() {
                    println!("\n📜 Command History:");
                    for (cmd, output) in &history {
                        println!("   > {}", cmd);
                        if !output.is_empty() {
                            let preview = if output.len() > 100 {
                                format!("{}...", &output[..100])
                            } else {
                                output.clone()
                            };
                            println!("     {}", preview);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("\n❌ Task failed: {}", e);

            // Show command history on failure for debugging
            let history = executor.get_command_history().await;
            if !history.is_empty() {
                println!("\n📜 Command History:");
                for (i, (cmd, output)) in history.iter().enumerate() {
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
    let mut executor = CodingAgentExecutor::new(coding_agent);

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
                    // Clear history
                    println!("History cleared");
                    session_context.clear();
                    continue;
                }

                if task == "context" {
                    let summary = executor.get_summary().await;
                    let history = executor.get_command_history().await;
                    println!("Current Context:");
                    println!("  Working dir: {}", working_dir.display());
                    println!("  {}", summary);
                    println!("  Commands in history: {}", history.len());
                    continue;
                }

                // Execute task
                println!("\n🚀 Executing: {}", task);
                session_context.push(format!("Previous task: {}", task));

                if max_corrections > 0 {
                    executor.enable_verification().await;
                }

                match executor
                    .execute_incremental_task_with_verification(
                        task,
                        &working_dir,
                        &session_context,
                    )
                    .await
                {
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
