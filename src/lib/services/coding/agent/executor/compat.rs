//! Compatibility layer for legacy executor

use std::sync::Arc;
use std::path::PathBuf;
use anyhow::Result;
use tokio::sync::{Mutex, mpsc};
use log::info;

use crate::services::coding::agent::service::CodingAgentService;
use crate::services::coding::agent::execution_state::{IncrementalExecution, ExecutionState};
use crate::services::coding::agent::step_parser::StepParser;
use crate::services::coding::agent::command_executor::CommandExecutor;

/// Message queue for user input during execution (legacy)
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

/// Enhanced execution context (legacy)
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

/// Legacy executor that maintains backward compatibility
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
        info!("Starting incremental task execution: {}", task_description);

        // Initialize execution state
        {
            let mut execution = self.execution.lock().await;
            *execution = IncrementalExecution::new(task_description.to_string(), auto_execute);
        }

        // Update context
        {
            let mut context = self.enhanced_context.lock().await;
            context.original_task = task_description.to_string();
            context.working_directory = current_dir.clone();
            context.session_context = session_context.to_vec();
        }

        // Phase 1: Generate execution plan
        info!("Phase 1: Generating execution plan");
        self.update_state(ExecutionState::GeneratingSteps).await;

        let planning_prompt = self.create_planning_prompt(task_description);

        let response = self.coding_agent.generate_response(
            &planning_prompt,
            current_dir,
            session_context,
            None,
        ).await?;

        // Parse and execute steps
        let steps = self.step_parser.parse_execution_steps(&response.response_text, task_description).await;

        {
            let mut execution = self.execution.lock().await;
            execution.add_steps(steps);
        }

        // Execute steps if auto_execute is enabled
        if auto_execute {
            self.execute_all_steps(current_dir).await?;
        }

        Ok(())
    }

    async fn update_state(&self, state: ExecutionState) {
        let mut execution = self.execution.lock().await;
        execution.update_state(state);
    }

    fn create_planning_prompt(&self, task_description: &str) -> String {
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
    }

    async fn execute_all_steps(&self, current_dir: &PathBuf) -> Result<()> {
        loop {
            let step = {
                let mut execution = self.execution.lock().await;
                execution.get_next_pending_step()
            };

            if let Some(step) = step {
                let output = self.command_executor.execute_command(
                    &step.command,
                    current_dir
                ).await;

                let result = Ok(output);  // Wrap in Ok since execute_command doesn't fail
                let mut execution = self.execution.lock().await;
                execution.update_step_output(step.step_number, result);
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Get current execution state
    pub async fn get_state(&self) -> ExecutionState {
        let execution = self.execution.lock().await;
        execution.get_state()
    }

    /// Get execution summary
    pub async fn get_summary(&self) -> String {
        let execution = self.execution.lock().await;
        execution.get_summary()
    }

    /// Submit user message
    pub async fn submit_message(&self, message: String) {
        let msg = UserMessage {
            content: message,
            timestamp: std::time::SystemTime::now(),
        };

        let mut queue = self.message_queue.lock().await;
        queue.push(msg.clone());

        let mut context = self.enhanced_context.lock().await;
        context.user_messages.push(msg);
    }

    /// Get command history
    pub async fn get_command_history(&self) -> Vec<(String, String)> {
        let context = self.enhanced_context.lock().await;
        context.command_history.clone()
    }

    /// Cancel execution
    pub async fn cancel_execution(&self) {
        let mut execution = self.execution.lock().await;
        execution.cancel();
    }

    /// Check if executing
    pub async fn is_executing(&self) -> bool {
        let execution = self.execution.lock().await;
        execution.is_active()
    }

    /// Queue a message
    pub async fn queue_message(&self, message: String) {
        self.submit_message(message).await;
    }

    /// Enable verification mode
    pub async fn enable_verification(&mut self) {
        let mut context = self.enhanced_context.lock().await;
        context.verification_enabled = true;
    }

    /// Setup message channel
    pub fn setup_message_channel(&mut self) -> mpsc::Receiver<UserMessage> {
        let (tx, rx) = mpsc::channel(100);

        let queue = self.message_queue.clone();
        tokio::spawn(async move {
            // Bridge from queue to channel
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let mut q = queue.lock().await;
                while let Some(msg) = q.pop() {
                    let _ = tx.send(msg).await;
                }
            }
        });

        rx
    }

    /// Execute with verification
    pub async fn execute_incremental_task_with_verification(
        &self,
        task_description: &str,
        current_dir: &PathBuf,
        session_context: &[String],
    ) -> Result<()> {
        let mut context = self.enhanced_context.lock().await;
        context.verification_enabled = true;
        drop(context);

        self.execute_incremental_task(task_description, current_dir, session_context, false).await
    }

    /// Get spinner text
    pub async fn get_spinner_text(&self) -> String {
        let execution = self.execution.lock().await;
        execution.get_spinner_text()
    }

    /// Get execution log
    pub async fn get_execution_log(&self) -> Vec<String> {
        let execution = self.execution.lock().await;
        execution.get_execution_log()
    }

    /// Get interactive status
    pub async fn get_interactive_status(&self) -> String {
        let execution = self.execution.lock().await;
        format!("State: {:?}, Steps: {}/{}",
            execution.state,
            execution.current_step,
            execution.steps.len())
    }

    /// Process queued messages
    pub async fn process_queued_messages(&self) -> Vec<UserMessage> {
        let mut queue = self.message_queue.lock().await;
        let messages: Vec<UserMessage> = queue.drain(..).collect();
        messages
    }
}