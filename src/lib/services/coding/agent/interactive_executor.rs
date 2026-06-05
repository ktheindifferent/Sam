use anyhow::Result;
use log;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

use super::command_executor::CommandExecutor;
use super::execution_state::IncrementalExecution;
use super::service::CodingAgentService;
use super::step_parser::StepParser;

/// Message queue for user input during execution
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

/// Execution context that preserves state
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub original_task: String,
    pub execution_log: Vec<String>,
    pub command_history: Vec<(String, String)>, // (command, output)
    pub user_messages: Vec<UserMessage>,
    pub working_directory: PathBuf,
    pub session_context: Vec<String>,
}

/// Interactive executor with verification and message queuing
#[derive(Clone)]
pub struct InteractiveExecutor {
    coding_agent: Arc<CodingAgentService>,
    execution: Arc<Mutex<IncrementalExecution>>,
    step_parser: Arc<StepParser>,
    command_executor: Arc<CommandExecutor>,
    context: Arc<Mutex<ExecutionContext>>,
    message_queue: Arc<Mutex<Vec<UserMessage>>>,
    message_receiver: Arc<Mutex<Option<mpsc::Receiver<UserMessage>>>>,
}

impl InteractiveExecutor {
    pub fn new(coding_agent: Arc<CodingAgentService>) -> Self {
        let safe_commands = coding_agent.get_config().safe_commands.clone();
        let step_parser = Arc::new(StepParser::new(safe_commands));
        let command_executor = Arc::new(CommandExecutor::new(coding_agent.clone()));

        Self {
            coding_agent,
            execution: Arc::new(Mutex::new(IncrementalExecution::default())),
            step_parser,
            command_executor,
            context: Arc::new(Mutex::new(ExecutionContext {
                original_task: String::new(),
                execution_log: Vec::new(),
                command_history: Vec::new(),
                user_messages: Vec::new(),
                working_directory: PathBuf::from("."),
                session_context: Vec::new(),
            })),
            message_queue: Arc::new(Mutex::new(Vec::new())),
            message_receiver: Arc::new(Mutex::new(None)),
        }
    }

    /// Set up message channel for user input
    pub async fn setup_message_channel(&self) -> mpsc::Sender<UserMessage> {
        let (tx, rx) = mpsc::channel::<UserMessage>(100);
        let mut receiver = self.message_receiver.lock().await;
        *receiver = Some(rx);
        tx
    }

    /// Queue a user message without interrupting execution
    pub async fn queue_message(&self, message: String) {
        let user_msg = UserMessage {
            content: message,
            timestamp: std::time::SystemTime::now(),
        };

        let mut queue = self.message_queue.lock().await;
        queue.push(user_msg.clone());

        let mut context = self.context.lock().await;
        context.user_messages.push(user_msg);

        if let Some(last_message) = context.user_messages.last() {
            log::info!("Queued user message: {}", last_message.content);
        }
    }

    /// Process queued messages without losing context
    async fn process_queued_messages(&self) -> Vec<String> {
        let mut queue = self.message_queue.lock().await;
        let messages: Vec<String> = queue.iter().map(|m| m.content.clone()).collect();
        queue.clear();
        messages
    }

    /// Execute task with interactive verification and correction loop
    pub async fn execute_with_verification(
        &self,
        task_description: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        max_correction_attempts: u32,
    ) -> Result<()> {
        log::info!(
            "Starting interactive execution with verification: {}",
            task_description
        );

        // Initialize context
        {
            let mut context = self.context.lock().await;
            context.original_task = task_description.to_string();
            context.working_directory = current_dir.clone();
            context.session_context = session_context.to_vec();
            context.execution_log.clear();
            context.command_history.clear();
        }

        // Initial execution
        let mut attempt = 0;
        let mut needs_correction = true;

        while needs_correction && attempt < max_correction_attempts {
            attempt += 1;
            log::info!("Execution attempt {}/{}", attempt, max_correction_attempts);

            // Generate or regenerate execution plan
            let plan = self
                .generate_execution_plan(
                    task_description,
                    current_dir,
                    session_context,
                    attempt > 1,
                )
                .await?;

            // Parse steps
            let steps = self
                .step_parser
                .parse_execution_steps(&plan, task_description)
                .await;

            {
                let mut execution = self.execution.lock().await;
                execution.set_steps(steps.clone());
                execution.set_raw_response(plan.clone());
            }

            // Execute steps interactively
            let execution_result = self.execute_steps_interactively(current_dir).await?;

            // Verify execution against original task
            needs_correction = self
                .verify_execution_with_ollama(task_description, &execution_result, current_dir)
                .await?;

            if needs_correction {
                log::info!("Execution needs correction, attempt {}", attempt);
                println!("🔄 Execution needs adjustment. Regenerating plan...");
            } else {
                log::info!("Execution verified successfully!");
                println!("✅ Task completed successfully and verified!");
            }

            // Process any queued user messages
            let user_messages = self.process_queued_messages().await;
            if !user_messages.is_empty() {
                log::info!("Processing {} queued user messages", user_messages.len());
                for msg in user_messages {
                    println!("📝 Processing queued message: {}", msg);
                    // Incorporate user feedback into next iteration
                    let mut context = self.context.lock().await;
                    context
                        .session_context
                        .push(format!("User feedback: {}", msg));
                }
            }
        }

        if needs_correction && attempt >= max_correction_attempts {
            log::warn!("Max correction attempts reached without full success");
            println!("⚠️  Maximum correction attempts reached. Manual intervention may be needed.");
        }

        Ok(())
    }

    /// Generate execution plan (initial or corrected)
    async fn generate_execution_plan(
        &self,
        task_description: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        is_correction: bool,
    ) -> Result<String> {
        let context = self.context.lock().await;

        let prompt = if is_correction {
            let history_str = context
                .command_history
                .iter()
                .map(|(cmd, output)| format!("Command: {}\nOutput: {}\n", cmd, output))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"Task: {}

Previous execution history:
{}

The task was not fully completed or had errors. Please provide CORRECTED commands to complete the task.
Focus on fixing any errors and completing any missing steps.

CRITICAL FORMATTING REQUIREMENTS:
1. Output ONLY a markdown code block with bash commands
2. ONE COMMAND PER LINE - no combining with && or ;
3. Use ONLY these allowed commands: {}
4. NO explanatory text before or after the code block

Generate the corrected sequence of commands:"#,
                task_description,
                history_str,
                self.coding_agent.get_config().safe_commands.join(", ")
            )
        } else {
            format!(
                r#"Task: {}

You are a shell command generator. Convert the task above into a sequence of individual shell commands.

CRITICAL FORMATTING REQUIREMENTS:
1. Output ONLY a markdown code block with bash commands
2. ONE COMMAND PER LINE - no combining with && or ;
3. Use ONLY these allowed commands: {}
4. NO explanatory text before or after the code block

Generate the minimal sequence of commands needed:"#,
                task_description,
                self.coding_agent.get_config().safe_commands.join(", ")
            )
        };

        let response = self
            .coding_agent
            .generate_response(&prompt, current_dir, session_context, None)
            .await?;

        Ok(response.response_text)
    }

    /// Execute steps interactively with per-command verification
    async fn execute_steps_interactively(&self, current_dir: &PathBuf) -> Result<String> {
        let step_count = {
            let execution = self.execution.lock().await;
            execution.steps.len()
        };

        let mut working_dir = current_dir.clone();
        let mut full_execution_log = Vec::new();

        for i in 0..step_count {
            let command = {
                let execution = self.execution.lock().await;
                if i >= execution.steps.len() {
                    break;
                }
                execution.steps[i].command.clone()
            };

            // Show interactive progress
            println!("\n📋 Step {}/{}: {}", i + 1, step_count, command);
            println!("   Working directory: {}", working_dir.display());

            // Check for user input before executing
            if let Ok(messages) = self.message_queue.try_lock() {
                if !messages.is_empty() {
                    println!("   📨 You have {} queued messages", messages.len());
                }
            }

            // Execute command
            let result = self
                .execute_command_with_validation(&command, &working_dir, i)
                .await;

            // Update working directory if needed
            if command.starts_with("cargo new ") {
                let project_name = command.strip_prefix("cargo new ").unwrap_or("").trim();
                if !project_name.is_empty() && result.success {
                    let new_dir = working_dir.join(project_name);
                    if new_dir.exists() {
                        working_dir = new_dir.clone();
                        std::env::set_current_dir(&new_dir).ok();
                        println!("   📁 Changed to: {}", working_dir.display());
                    }
                }
            } else if command.starts_with("cd ") && result.success {
                let target = command.strip_prefix("cd ").unwrap_or("").trim();
                if !target.is_empty() {
                    let new_dir = if target.starts_with("/") {
                        PathBuf::from(target)
                    } else {
                        working_dir.join(target)
                    };
                    if new_dir.exists() {
                        working_dir = new_dir.clone();
                        println!("   📁 Changed to: {}", working_dir.display());
                    }
                }
            }

            // Store command history
            {
                let mut context = self.context.lock().await;
                context
                    .command_history
                    .push((command.clone(), result.output.clone()));
                context.execution_log.push(format!(
                    "Step {}: {} -> {}",
                    i + 1,
                    command,
                    if result.success { "✅" } else { "❌" }
                ));
            }

            full_execution_log.push(format!(
                "Command: {}\nOutput: {}\nSuccess: {}\n",
                command, result.output, result.success
            ));

            // Show result with cleaner formatting
            if result.success {
                if !result.output.trim().is_empty() {
                    // For very long outputs (like ls -la), show more content
                    let output_len = result.output.trim().len();
                    if output_len > 200 {
                        // Show first few lines for long directory listings
                        let lines: Vec<&str> = result.output.lines().take(5).collect();
                        println!("   ✅ {}", lines.join("\n   "));
                        if result.output.lines().count() > 5 {
                            println!("   ... ({} more lines)", result.output.lines().count() - 5);
                        }
                    } else {
                        println!("   ✅ {}", self.format_output(&result.output, 120));
                    }
                } else {
                    println!("   ✅ Success");
                }
            } else {
                // Format the error output more cleanly
                let clean_output = self.format_error_output(&result.output, &command);
                println!("   ❌ {}", clean_output);

                // Ask Ollama for suggestions on failure
                if let Some(suggestion) = self
                    .get_correction_suggestion(&command, &result.output)
                    .await
                {
                    println!("   💡 Suggestion: {}", suggestion);
                }
            }

            // Brief pause for readability
            sleep(Duration::from_millis(300)).await;
        }

        Ok(full_execution_log.join("\n"))
    }

    /// Execute a single command with validation
    async fn execute_command_with_validation(
        &self,
        command: &str,
        working_dir: &PathBuf,
        step_index: usize,
    ) -> CommandResult {
        let output = match self
            .command_executor
            .execute_validated_command(command, working_dir)
            .await
        {
            Ok(out) => out,
            Err(err) => err,
        };

        // Improve success detection by considering expected errors
        let success = if command.starts_with("cat ") && output.contains("Is a directory") {
            // cat on a directory is a failure but expected
            false
        } else if command.starts_with("grep ") && output.contains("Is a directory") {
            // grep on a directory is a failure but expected
            false
        } else if output.contains("No such file or directory") {
            false
        } else if output.contains("Permission denied") {
            false
        } else if output.contains("command not found") {
            false
        } else if output.contains("Command '") && output.contains("failed with exit code") {
            false
        } else {
            // For other cases, consider it success if no critical errors
            !output.to_lowercase().contains("error:")
                && !output.to_lowercase().contains("failed:")
                && !output.to_lowercase().contains("fatal:")
        };

        // Update execution state
        {
            let mut execution = self.execution.lock().await;
            if step_index < execution.steps.len() {
                execution.update_step(
                    step_index,
                    self.command_executor.trim_output(&output, command),
                    success,
                );
                execution.advance_step();
            }
        }

        CommandResult { output, success }
    }

    /// Get correction suggestion from Ollama for failed command
    async fn get_correction_suggestion(&self, command: &str, error: &str) -> Option<String> {
        let prompt = format!(
            r#"Command failed: {}
Error: {}

Provide a BRIEF (one line) suggestion to fix this error. Be specific and actionable.
If the error is expected (like 'file already exists'), say 'No action needed'.
Response:"#,
            command, error
        );

        match self
            .coding_agent
            .generate_response(&prompt, &PathBuf::from("."), &[], None)
            .await
        {
            Ok(response) => {
                let suggestion = response.response_text.trim().to_string();
                if !suggestion.is_empty() && suggestion != "No action needed" {
                    Some(suggestion)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Verify execution results against original task using Ollama
    async fn verify_execution_with_ollama(
        &self,
        original_task: &str,
        execution_log: &str,
        current_dir: &PathBuf,
    ) -> Result<bool> {
        let prompt = format!(
            r#"Original Task: {}

Execution Log:
{}

Please verify if the task was completed successfully.
Check if:
1. All requirements from the original task were met
2. The commands executed successfully
3. The expected output/files were created

Respond with ONLY one of these:
- "SUCCESS" if the task is fully completed
- "NEEDS_CORRECTION" if there are issues or missing steps

Response:"#,
            original_task, execution_log
        );

        let response = self
            .coding_agent
            .generate_response(&prompt, current_dir, &[], None)
            .await?;

        let verdict = response.response_text.trim().to_uppercase();
        Ok(verdict.contains("NEEDS_CORRECTION"))
    }

    /// Get current execution state for UI updates
    pub async fn get_execution_state(&self) -> IncrementalExecution {
        self.execution.lock().await.clone()
    }

    /// Get execution context for debugging
    pub async fn get_context(&self) -> ExecutionContext {
        self.context.lock().await.clone()
    }

    /// Clear execution history while preserving user messages
    pub async fn clear_history_preserve_messages(&self) {
        let mut context = self.context.lock().await;
        context.execution_log.clear();
        context.command_history.clear();
        // Keep user_messages and session_context
    }

    /// Format output for display, truncating if necessary (UTF-8 safe)
    fn format_output(&self, output: &str, max_len: usize) -> String {
        let trimmed = output.trim();
        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            // Use char_indices to safely truncate at character boundaries
            let mut truncate_at = max_len;
            for (idx, _) in trimmed.char_indices() {
                if idx >= max_len {
                    truncate_at = idx;
                    break;
                }
            }
            // If we still haven't found a safe boundary, use chars() iterator
            if truncate_at >= trimmed.len() {
                let safe_str: String = trimmed.chars().take(max_len).collect();
                format!("{}...", safe_str)
            } else {
                format!("{}...", &trimmed[..truncate_at])
            }
        }
    }

    /// Format error output more cleanly
    fn format_error_output(&self, output: &str, command: &str) -> String {
        let trimmed = output.trim();

        // Handle specific error patterns for cleaner display
        if command.starts_with("cat ") && trimmed.contains("Is a directory") {
            if let Some(target) = command.strip_prefix("cat ").map(|s| s.trim()) {
                return format!("{} is a directory", target);
            }
        }

        if command.starts_with("grep ") && trimmed.contains("Is a directory") {
            if let Some(parts) = command.strip_prefix("grep ") {
                let parts: Vec<&str> = parts.split_whitespace().collect();
                if parts.len() > 1 {
                    return format!("{} is a directory", parts.last().unwrap_or(&"target"));
                }
            }
        }

        // Remove redundant "Command failed:" prefix if present
        if trimmed.starts_with("Command failed:") {
            return trimmed
                .strip_prefix("Command failed:")
                .unwrap_or(trimmed)
                .trim()
                .to_string();
        }

        // For other errors, return as-is but truncated if too long
        self.format_output(trimmed, 80)
    }
}

#[derive(Debug, Clone)]
struct CommandResult {
    output: String,
    success: bool,
}
