use std::sync::Arc;
use std::path::PathBuf;
use anyhow::Result;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{sleep, Duration};
use log;

use super::service::CodingAgentService;
use super::execution_state::{IncrementalExecution, ExecutionState, ExecutionStep};
use super::step_parser::StepParser;
use super::command_executor::CommandExecutor;

/// Message queue for user input during execution
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

/// Enhanced execution context
#[derive(Debug, Clone)]
pub struct EnhancedContext {
    pub original_task: String,
    pub command_history: Vec<(String, String)>, // (command, output)
    pub user_messages: Vec<UserMessage>,
    pub working_directory: PathBuf,
    pub session_context: Vec<String>,
    pub verification_enabled: bool,
    pub max_correction_attempts: u32,
}

/// Main executor that coordinates incremental task execution
#[derive(Clone, Debug)]
pub struct CodingAgentExecutor {
    coding_agent: Arc<CodingAgentService>,
    execution: Arc<Mutex<IncrementalExecution>>,
    step_parser: Arc<StepParser>,
    command_executor: Arc<CommandExecutor>,
    enhanced_context: Arc<Mutex<EnhancedContext>>,
    message_queue: Arc<Mutex<Vec<UserMessage>>>,
    message_receiver: Arc<Mutex<Option<mpsc::Receiver<UserMessage>>>>,
}

impl CodingAgentExecutor {
    pub fn new(coding_agent: Arc<CodingAgentService>) -> Self {
        // Get safe commands from the coding agent config
        let safe_commands = coding_agent.get_config().safe_commands.clone();
        let step_parser = Arc::new(StepParser::new(safe_commands));
        let command_executor = Arc::new(CommandExecutor::new(coding_agent.clone()));

        Self {
            coding_agent,
            execution: Arc::new(Mutex::new(IncrementalExecution::default())),
            step_parser,
            command_executor,
            enhanced_context: Arc::new(Mutex::new(EnhancedContext {
                original_task: String::new(),
                command_history: Vec::new(),
                user_messages: Vec::new(),
                working_directory: PathBuf::from("."),
                session_context: Vec::new(),
                verification_enabled: false,
                max_correction_attempts: 3,
            })),
            message_queue: Arc::new(Mutex::new(Vec::new())),
            message_receiver: Arc::new(Mutex::new(None)),
        }
    }

    /// Start incremental execution of a complex task
    pub async fn execute_incremental_task(
        &self,
        task_description: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        auto_execute: bool,
    ) -> Result<()> {
        log::info!("Starting incremental task execution: {}", task_description);

        // Initialize execution state
        {
            let mut execution = self.execution.lock().await;
            *execution = IncrementalExecution::new(task_description.to_string(), auto_execute);
        }

        // Phase 1: Generate execution plan
        log::info!("Phase 1: Generating execution plan");
        self.update_state(ExecutionState::GeneratingSteps).await;

        let planning_prompt = format!(
            r#"Task: {}

You are a shell command generator. Convert the task above into a sequence of individual shell commands.

CRITICAL FORMATTING REQUIREMENTS:
1. Output ONLY a markdown code block with bash commands
2. ONE COMMAND PER LINE - no combining with && or ;
3. Use ONLY these allowed commands: {}
4. NO explanatory text before or after the code block

IMPORTANT: After 'cargo new PROJECT_NAME', you are INSIDE the project directory!
- Do NOT use 'PROJECT_NAME/' prefixes in paths
- Do NOT use '--manifest-path'
- Files like src/main.rs are relative to the current directory
- Just use 'cargo add rand' not 'cargo add rand --manifest-path ...'
- Just use 'cat > src/main.rs' not 'cat > PROJECT_NAME/src/main.rs'

CORRECT EXAMPLE:
```bash
cargo new my_project
cargo add rand
cat > src/main.rs << 'EOF'
use rand::Rng;
fn main() {{
    let mut rng = rand::thread_rng();
    let n: u32 = rng.gen_range(1..=100);
    println!("Random number: {{}}", n);
}}
EOF
cargo build
cargo run
```

TASK ANALYSIS FOR: '{}'
Generate the minimal sequence of commands needed:"#,
            task_description,
            self.coding_agent.get_config().safe_commands.join(", "),
            task_description
        );

        log::info!("Calling generate_response with prompt length: {}", planning_prompt.len());
        let response = self.coding_agent.generate_response(
            &planning_prompt,
            current_dir,
            session_context,
            None,
        ).await?;
        log::info!("Received response from generate_response");

        // Store the raw AI response for debugging
        {
            let mut execution = self.execution.lock().await;
            execution.set_raw_response(response.response_text.clone());
        }

        // Parse the response into execution steps
        let steps = self.step_parser.parse_execution_steps(&response.response_text, task_description).await;

        {
            let mut execution = self.execution.lock().await;
            execution.set_steps(steps);
        }

        // Phase 2: Execute steps incrementally
        if auto_execute {
            self.execute_all_steps(current_dir).await?;
        }

        Ok(())
    }

    /// Execute all steps automatically
    async fn execute_all_steps(&self, current_dir: &PathBuf) -> Result<()> {
        let step_count = {
            let execution = self.execution.lock().await;
            execution.steps.len()
        };

        // Keep track of the actual working directory as we execute commands
        let mut working_dir = current_dir.clone();

        for i in 0..step_count {
            // Update state to show current step
            let command = {
                let execution = self.execution.lock().await;
                if i >= execution.steps.len() {
                    break;
                }
                execution.steps[i].command.clone()
            };

            self.update_state(ExecutionState::ExecutingCommand {
                step: i + 1,
                total: step_count,
                command: command.clone(),
            }).await;

            // Small delay for visual feedback
            sleep(Duration::from_millis(500)).await;

            // Check if this is a cargo new command and update working directory
            if command.starts_with("cargo new ") {
                let project_name = command.strip_prefix("cargo new ").unwrap_or("").trim();
                if !project_name.is_empty() {
                    // After cargo new, change into the created directory
                    let new_dir = working_dir.join(project_name);
                    log::info!("Detected cargo new {}, will use directory: {}", project_name, new_dir.display());

                    // Execute the cargo new command first
                    let result = self.execute_single_command(&command, &working_dir).await;
                    let success = self.determine_command_success(&result, &command);

                    // Update step with result
                    let display_output = self.command_executor.trim_output(&result, &command);
                    {
                        let mut execution = self.execution.lock().await;
                        execution.update_step(i, display_output.clone(), success);
                        execution.advance_step();
                    }

                    // If cargo new succeeded, update working directory AND change process working directory
                    if success && new_dir.exists() {
                        working_dir = new_dir.clone();
                        // Actually change the process working directory
                        if let Err(e) = std::env::set_current_dir(&new_dir) {
                            log::error!("Failed to change process working directory: {}", e);
                        } else {
                            log::info!("Changed process working directory to: {}", new_dir.display());
                        }
                    }

                    continue; // Skip the normal execution below
                }
            }

            // Check for cd commands and update working directory
            if command.starts_with("cd ") {
                // Execute cd command through the command executor
                let result = self.execute_single_command(&command, &working_dir).await;
                let success = self.determine_command_success(&result, &command);

                // Update step with result
                let display_output = self.command_executor.trim_output(&result, &command);
                {
                    let mut execution = self.execution.lock().await;
                    execution.update_step(i, display_output.clone(), success);
                    execution.advance_step();
                }

                // If cd succeeded, update working directory
                if success && result.contains("Changed directory to") {
                    let target = command.strip_prefix("cd ").unwrap_or("").trim();
                    if !target.is_empty() {
                        let new_dir = if target.starts_with("/") {
                            PathBuf::from(target)
                        } else {
                            working_dir.join(target)
                        };

                        if new_dir.exists() && new_dir.is_dir() {
                            working_dir = new_dir.clone();
                            log::info!("Executor: Updated working directory to: {}", working_dir.display());
                        }
                    }
                }

                continue;
            }

            // Execute the command in the current working directory
            let result = self.execute_single_command(&command, &working_dir).await;
            let success = self.determine_command_success(&result, &command);

            // Trim output for display
            let display_output = self.command_executor.trim_output(&result, &command);

            // Update step with result
            {
                let mut execution = self.execution.lock().await;
                execution.update_step(i, display_output.clone(), success);
                execution.advance_step();
            }

            // Provide user feedback and recovery suggestions on failure
            if !success {
                // Get suggestions for failed command
                let suggestions = self.command_executor.suggest_alternatives(&command, &result);

                if self.command_executor.is_critical_command(&command) {
                    let mut error_msg = format!("Critical command failed: {}\nOutput: {}", command, display_output);
                    if !suggestions.is_empty() {
                        error_msg.push_str(&format!("\n\nSuggestions:\n{}", suggestions.join("\n")));
                    }

                    self.update_state(ExecutionState::Failed {
                        error: error_msg
                    }).await;
                    return Ok(());
                } else {
                    // For non-critical commands, log the failure but continue
                    if !suggestions.is_empty() {
                        println!("⚠️  Command failed but continuing: {}", command);
                        println!("💡 Suggestions: {}", suggestions.join(", "));
                    }
                }
            }

            // Brief pause between commands
            sleep(Duration::from_millis(200)).await;
        }

        self.update_state(ExecutionState::Completed).await;
        Ok(())
    }

    /// Execute a single command
    async fn execute_single_command(&self, command: &str, current_dir: &PathBuf) -> String {
        match self.command_executor.execute_validated_command(command, current_dir).await {
            Ok(output) => output,
            Err(error) => error,
        }
    }

    /// Determine if a command execution was successful
    fn determine_command_success(&self, output: &str, command: &str) -> bool {
        // Check for explicit error indicators
        let output_lower = output.to_lowercase();

        // Special case: mkdir with "File exists" is actually okay
        if command.starts_with("mkdir") && output_lower.contains("file exists") {
            return true; // Directory already exists, that's fine
        }

        let error_indicators = [
            "error:", "failed:", "command failed", "validation failed",
            "cannot", "no such file", "permission denied", "command not found",
            "syntax error", "parse error", "invalid", "unexpected eof",
            "line 0:", "line 1:", "unexpected end of file"
        ];

        for indicator in &error_indicators {
            if output_lower.contains(indicator) {
                return false;
            }
        }

        // Command-specific success checking
        if command.starts_with("cargo") {
            if output_lower.contains("compilation failed") ||
               output_lower.contains("build failed") ||
               output_lower.contains("could not compile") {
                return false;
            }
        }

        if command.starts_with("git") {
            if output_lower.contains("fatal:") ||
               output_lower.contains("not a git repository") {
                return false;
            }
        }

        if command.starts_with("mkdir") {
            if output_lower.contains("cannot create directory") ||
               output_lower.contains("file exists") {
                // For mkdir, "file exists" is actually success if the directory already exists
                return !output_lower.contains("cannot create directory");
            }
        }

        // If no error indicators found, assume success
        true
    }

    /// Update execution state
    async fn update_state(&self, new_state: ExecutionState) {
        let mut execution = self.execution.lock().await;
        execution.update_state(new_state);
    }

    /// Get current execution state (for UI updates)
    pub async fn get_execution_state(&self) -> IncrementalExecution {
        self.execution.lock().await.clone()
    }

    /// Check if execution is active
    pub async fn is_executing(&self) -> bool {
        let execution = self.execution.lock().await;
        execution.is_active()
    }

    /// Get spinner text based on current state
    pub async fn get_spinner_text(&self) -> String {
        let execution = self.execution.lock().await;
        execution.get_spinner_text()
    }

    /// Cancel current execution
    pub async fn cancel_execution(&self) {
        let mut execution = self.execution.lock().await;
        execution.cancel();
    }

    /// Get formatted execution log for display
    pub async fn get_execution_log(&self) -> Vec<String> {
        let execution = self.execution.lock().await;
        execution.get_execution_log()
    }

    /// Execute a single step manually (for manual mode)
    pub async fn execute_step(&self, step_index: usize, current_dir: &PathBuf) -> Result<()> {
        let command = {
            let execution = self.execution.lock().await;
            if step_index >= execution.steps.len() {
                return Err(anyhow::anyhow!("Step index out of bounds"));
            }
            execution.steps[step_index].command.clone()
        };

        self.update_state(ExecutionState::ExecutingCommand {
            step: step_index + 1,
            total: {
                let execution = self.execution.lock().await;
                execution.steps.len()
            },
            command: command.clone(),
        }).await;

        let result = self.execute_single_command(&command, current_dir).await;
        let success = !result.contains("Error:") && !result.contains("Command failed") && !result.contains("error:");

        {
            let mut execution = self.execution.lock().await;
            execution.update_step(step_index, self.command_executor.trim_output(&result, &command), success);
            if step_index >= execution.current_step {
                execution.current_step = step_index + 1;
            }
        }

        // Check if all steps are completed
        let (all_executed, total_steps) = {
            let execution = self.execution.lock().await;
            (execution.current_step >= execution.steps.len(), execution.steps.len())
        };

        if all_executed {
            self.update_state(ExecutionState::Completed).await;
        }

        Ok(())
    }

    /// Skip a step
    pub async fn skip_step(&self, step_index: usize) -> Result<()> {
        let mut execution = self.execution.lock().await;
        
        if step_index >= execution.steps.len() {
            return Err(anyhow::anyhow!("Step index out of bounds"));
        }

        execution.update_step(step_index, "Skipped by user".to_string(), true);
        if step_index >= execution.current_step {
            execution.current_step = step_index + 1;
        }

        // Check if all steps are completed
        if execution.current_step >= execution.steps.len() {
            execution.update_state(ExecutionState::Completed);
        }

        Ok(())
    }

    /// Retry a failed step
    pub async fn retry_step(&self, step_index: usize, current_dir: &PathBuf) -> Result<()> {
        self.execute_step(step_index, current_dir).await
    }

    /// Add a new step to the execution
    pub async fn add_step(&self, description: String, command: String) -> Result<()> {
        // Validate the command first
        if let Err(error) = self.command_executor.validate_command(&command) {
            return Err(anyhow::anyhow!("Invalid command: {}", error));
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_step = ExecutionStep {
            description,
            command,
            output: None,
            success: false,
            timestamp,
        };

        let mut execution = self.execution.lock().await;
        execution.steps.push(new_step);

        Ok(())
    }

    /// Modify an existing step
    pub async fn modify_step(&self, step_index: usize, new_command: String) -> Result<()> {
        // Validate the command first
        if let Err(error) = self.command_executor.validate_command(&new_command) {
            return Err(anyhow::anyhow!("Invalid command: {}", error));
        }

        let mut execution = self.execution.lock().await;
        
        if step_index >= execution.steps.len() {
            return Err(anyhow::anyhow!("Step index out of bounds"));
        }

        execution.steps[step_index].command = new_command.clone();
        execution.steps[step_index].description = format!("Modified: {}", new_command);
        execution.steps[step_index].output = None;
        execution.steps[step_index].success = false;

        Ok(())
    }

    /// Get execution progress as percentage
    pub async fn get_progress(&self) -> f32 {
        let execution = self.execution.lock().await;
        execution.get_progress_percentage()
    }

    /// Get execution duration
    pub async fn get_duration(&self) -> u64 {
        let execution = self.execution.lock().await;
        execution.get_execution_duration()
    }

    /// Check if execution was successful
    pub async fn is_successful(&self) -> bool {
        let execution = self.execution.lock().await;
        execution.is_successful()
    }

    /// Get failed steps
    pub async fn get_failed_steps(&self) -> Vec<(usize, String, String)> {
        let execution = self.execution.lock().await;
        execution.get_failed_steps()
            .into_iter()
            .map(|(index, step)| (index, step.command.clone(), step.output.clone().unwrap_or_default()))
            .collect()
    }

    /// Get execution summary
    pub async fn get_execution_summary(&self) -> HashMap<String, serde_json::Value> {
        let execution = self.execution.lock().await;
        let mut summary = HashMap::new();

        summary.insert("task_description".to_string(), serde_json::Value::String(execution.task_description.clone()));
        summary.insert("total_steps".to_string(), serde_json::Value::Number(execution.steps.len().into()));
        summary.insert("completed_steps".to_string(), serde_json::Value::Number(execution.current_step.into()));
        summary.insert("progress_percentage".to_string(), serde_json::json!(execution.get_progress_percentage()));
        summary.insert("duration_seconds".to_string(), serde_json::Value::Number(execution.get_execution_duration().into()));
        summary.insert("state".to_string(), serde_json::json!(execution.state));
        summary.insert("successful".to_string(), serde_json::Value::Bool(execution.is_successful()));

        let failed_count = execution.get_failed_steps().len();
        summary.insert("failed_steps_count".to_string(), serde_json::Value::Number(failed_count.into()));

        summary
    }

    /// Set execution to manual mode (wait for user confirmation for each step)
    pub async fn set_manual_mode(&self, manual: bool) {
        let mut execution = self.execution.lock().await;
        execution.auto_execute = !manual;
    }

    /// Check if execution is in auto mode
    pub async fn is_auto_mode(&self) -> bool {
        let execution = self.execution.lock().await;
        execution.auto_execute
    }

    // ============= Enhanced Interactive Methods =============

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

        let mut context = self.enhanced_context.lock().await;
        context.user_messages.push(user_msg);
        log::info!("Queued user message: {}", &context.user_messages.last().unwrap().content);
    }

    /// Process queued messages without losing context
    pub async fn process_queued_messages(&self) -> Vec<String> {
        let mut queue = self.message_queue.lock().await;
        let messages: Vec<String> = queue.iter().map(|m| m.content.clone()).collect();
        queue.clear();
        messages
    }

    /// Enable verification mode
    pub async fn enable_verification(&self, max_attempts: u32) {
        let mut context = self.enhanced_context.lock().await;
        context.verification_enabled = true;
        context.max_correction_attempts = max_attempts;
    }

    /// Execute task with interactive verification and correction loop
    pub async fn execute_incremental_task_with_verification(
        &self,
        task_description: &str,
        current_dir: &PathBuf,
        session_context: &[String],
        auto_execute: bool,
    ) -> Result<()> {
        log::info!("Starting incremental task with verification: {}", task_description);

        // Check if verification is enabled
        let (verification_enabled, max_attempts) = {
            let context = self.enhanced_context.lock().await;
            (context.verification_enabled, context.max_correction_attempts)
        };

        // Initialize context
        {
            let mut context = self.enhanced_context.lock().await;
            context.original_task = task_description.to_string();
            context.working_directory = current_dir.clone();
            context.session_context = session_context.to_vec();
            context.command_history.clear();
        }

        if verification_enabled {
            // Execute with verification loop
            let mut attempt = 0;
            let mut needs_correction = true;

            while needs_correction && attempt < max_attempts {
                attempt += 1;
                log::info!("Execution attempt {}/{}", attempt, max_attempts);

                // Execute the task
                self.execute_incremental_task(task_description, current_dir, session_context, auto_execute).await?;

                // Get execution results
                let execution_log = self.get_execution_log().await.join("\n");

                // Verify execution against original task
                needs_correction = self.verify_execution(task_description, &execution_log, current_dir).await?;

                if needs_correction {
                    log::info!("Execution needs correction, attempt {}", attempt);

                    // Get correction suggestions
                    if let Some(correction) = self.get_correction_plan(task_description, &execution_log).await {
                        log::info!("Applying correction: {}", correction);

                        // Update task description with corrections
                        let corrected_task = format!("{}\n\nCorrection needed:\n{}", task_description, correction);

                        // Clear previous execution
                        {
                            let mut execution = self.execution.lock().await;
                            *execution = IncrementalExecution::default();
                        }
                    }
                }

                // Process any queued user messages
                let user_messages = self.process_queued_messages().await;
                if !user_messages.is_empty() {
                    log::info!("Processing {} queued user messages", user_messages.len());
                    for msg in user_messages {
                        let mut context = self.enhanced_context.lock().await;
                        context.session_context.push(format!("User feedback: {}", msg));
                    }
                }
            }

            if needs_correction && attempt >= max_attempts {
                log::warn!("Max correction attempts reached without full success");
            }
        } else {
            // Execute without verification (original behavior)
            self.execute_incremental_task(task_description, current_dir, session_context, auto_execute).await?;
        }

        Ok(())
    }

    /// Verify execution results against original task
    async fn verify_execution(&self, original_task: &str, execution_log: &str, current_dir: &PathBuf) -> Result<bool> {
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
            original_task,
            execution_log
        );

        let response = self.coding_agent.generate_response(
            &prompt,
            current_dir,
            &[],
            None,
        ).await?;

        let verdict = response.response_text.trim().to_uppercase();
        Ok(verdict.contains("NEEDS_CORRECTION"))
    }

    /// Get correction plan from the AI
    async fn get_correction_plan(&self, original_task: &str, execution_log: &str) -> Option<String> {
        let context = self.enhanced_context.lock().await;
        let history_str = context.command_history.iter()
            .map(|(cmd, output)| format!("Command: {}\nOutput: {}\n", cmd, output))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"Task: {}

Previous execution history:
{}

The task was not fully completed or had errors.
Provide a BRIEF correction plan (2-3 lines max) describing what needs to be fixed.
Be specific and actionable.

Response:"#,
            original_task,
            history_str
        );

        match self.coding_agent.generate_response(
            &prompt,
            &PathBuf::from("."),
            &[],
            None,
        ).await {
            Ok(response) => {
                let plan = response.response_text.trim().to_string();
                if !plan.is_empty() {
                    Some(plan)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Get enhanced context for debugging
    pub async fn get_enhanced_context(&self) -> EnhancedContext {
        self.enhanced_context.lock().await.clone()
    }

    /// Update command history during execution
    pub async fn update_command_history(&self, command: String, output: String) {
        let mut context = self.enhanced_context.lock().await;
        context.command_history.push((command, output));
    }

    /// Clear execution history while preserving user messages
    pub async fn clear_history_preserve_messages(&self) {
        let mut context = self.enhanced_context.lock().await;
        context.command_history.clear();
        // Keep user_messages and session_context
    }

    /// Get interactive execution status for TUI display
    pub async fn get_interactive_status(&self) -> String {
        let context = self.enhanced_context.lock().await;
        let execution = self.execution.lock().await;

        if context.verification_enabled {
            format!("Verification: ON | Max Attempts: {} | Messages: {}",
                context.max_correction_attempts,
                context.user_messages.len())
        } else {
            format!("Steps: {}/{} | Auto: {}",
                execution.current_step,
                execution.steps.len(),
                execution.auto_execute)
        }
    }
}

use std::collections::HashMap;