use serde::{Deserialize, Serialize};
use std::fmt;
use super::utils;

/// Execution state for incremental task processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionState {
    Planning,
    GeneratingSteps,
    ExecutingCommand { step: usize, total: usize, command: String },
    WaitingForConfirmation { step: usize, total: usize, command: String },
    Completed,
    Failed { error: String },
}

/// Individual execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub(crate) step_number: usize,
    pub(crate) description: String,
    pub(crate) command: String,
    pub(crate) output: Option<String>,
    pub(crate) success: bool,
    pub(crate) timestamp: u64,
}

impl ExecutionStep {
    /// Create a new execution step
    pub fn new(description: String, command: String, timestamp: u64) -> Self {
        Self {
            step_number: 0,
            description,
            command,
            output: None,
            success: false,
            timestamp,
        }
    }

    /// Create with step number
    pub fn with_step_number(step_number: usize, description: String, command: String, timestamp: u64) -> Self {
        Self {
            step_number,
            description,
            command,
            output: None,
            success: false,
            timestamp,
        }
    }

    /// Get the command
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Get the output if available
    pub fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    /// Check if step was successful
    pub fn is_successful(&self) -> bool {
        self.success
    }

    /// Get the description
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Incremental execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalExecution {
    pub task_description: String,
    pub state: ExecutionState,
    pub steps: Vec<ExecutionStep>,
    pub current_step: usize,
    pub spinner_active: bool,
    pub start_time: u64,
    pub auto_execute: bool,
    pub raw_ai_response: Option<String>,  // Store raw AI output for debugging
}

impl Default for IncrementalExecution {
    fn default() -> Self {
        Self {
            task_description: String::new(),
            state: ExecutionState::Planning,
            steps: Vec::new(),
            current_step: 0,
            spinner_active: false,
            start_time: 0,
            auto_execute: true,
            raw_ai_response: None,
        }
    }
}

impl IncrementalExecution {
    /// Create a new execution context
    pub fn new(task_description: String, auto_execute: bool) -> Self {
        let start_time = utils::unix_timestamp();

        Self {
            task_description,
            state: ExecutionState::Planning,
            steps: Vec::new(),
            current_step: 0,
            spinner_active: true,
            start_time,
            auto_execute,
            raw_ai_response: None,
        }
    }

    /// Update the execution state
    pub fn update_state(&mut self, new_state: ExecutionState) {
        self.state = new_state;
        self.spinner_active = matches!(self.state,
            ExecutionState::Planning |
            ExecutionState::GeneratingSteps |
            ExecutionState::ExecutingCommand { .. }
        );
    }

    /// Add execution steps
    pub fn set_steps(&mut self, steps: Vec<ExecutionStep>) {
        self.steps = steps;
        self.spinner_active = false;
    }

    /// Set the raw AI response for debugging
    pub fn set_raw_response(&mut self, response: String) {
        self.raw_ai_response = Some(response);
    }

    /// Update a specific step with results
    pub fn update_step(&mut self, step_index: usize, output: String, success: bool) {
        if step_index < self.steps.len() {
            self.steps[step_index].output = Some(output);
            self.steps[step_index].success = success;
        }
    }

    /// Move to the next step
    pub fn advance_step(&mut self) {
        self.current_step += 1;
    }

    /// Check if execution is active
    pub fn is_active(&self) -> bool {
        matches!(self.state,
            ExecutionState::Planning |
            ExecutionState::GeneratingSteps |
            ExecutionState::ExecutingCommand { .. } |
            ExecutionState::WaitingForConfirmation { .. }
        )
    }

    /// Get spinner text based on current state
    pub fn get_spinner_text(&self) -> String {
        match &self.state {
            ExecutionState::Planning => "🤖 Planning task...".into(),
            ExecutionState::GeneratingSteps => "🧠 Generating execution steps...".into(),
            ExecutionState::ExecutingCommand { step, total, command } => {
                format!("⚡ [{}/{}] Executing: {}", step, total, Self::get_command_description(command))
            },
            ExecutionState::WaitingForConfirmation { step, total, command } => {
                format!("⏳ [{}/{}] Confirm: {}", step, total, command)
            },
            ExecutionState::Completed => "✅ Task completed!".into(),
            ExecutionState::Failed { error } => format!("❌ Failed: {}", error),
        }
    }

    /// Get a human-readable description for a command
    fn get_command_description(command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return command.into();
        }

        match parts[0] {
            "mkdir" => format!("Creating directory {}", parts.get(1).unwrap_or(&"<name>")),
            "cd" => format!("Changing to directory {}", parts.get(1).unwrap_or(&"<dir>")),
            "cargo" if parts.get(1) == Some(&"new") => {
                format!("Creating new Rust project {}", parts.get(2).unwrap_or(&"<name>"))
            },
            "touch" => format!("Creating file {}", parts.get(1).unwrap_or(&"<file>")),
            "echo" => "Writing content to file".into(),
            "cp" => "Copying files".into(),
            "mv" => "Moving/renaming files".into(),
            "ls" => "Listing directory contents".into(),
            "cat" => "Displaying file contents".into(),
            _ => command.into(),
        }
    }

    /// Get formatted execution log for display
    pub fn get_execution_log(&self) -> Vec<String> {
        let mut log = Vec::new();

        log.push(format!("🎯 Task: {}", self.task_description));
        log.push(String::new());

        for (i, step) in self.steps.iter().enumerate() {
            let status = if i < self.current_step {
                if step.success { "✅" } else { "❌" }
            } else if i == self.current_step {
                "⚡"
            } else {
                "⏳"
            };

            log.push(format!("{} {}", status, step.description));
            log.push(format!("   Command: {}", step.command));

            if let Some(output) = &step.output {
                if !output.is_empty() {
                    let trimmed_output = if output.len() > 200 {
                        format!("{}...", &output[..200])
                    } else {
                        output.clone()
                    };
                    log.push(format!("   Output: {}", trimmed_output));
                }
            }
            log.push(String::new());
        }

        match &self.state {
            ExecutionState::Completed => log.push("🎉 All steps completed successfully!".into()),
            ExecutionState::Failed { error } => log.push(format!("💥 Execution failed: {}", error)),
            _ => {}
        }

        // Add raw AI response for debugging
        if let Some(raw_response) = &self.raw_ai_response {
            log.push(String::new());
            log.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
            log.push("DEBUG: Raw Ollama Output".into());
            log.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
            // Split the response into lines for better formatting
            for line in raw_response.lines() {
                log.push(line.into());
            }
            log.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
        }

        log
    }

    /// Add multiple steps at once
    pub fn add_steps(&mut self, new_steps: Vec<ExecutionStep>) {
        self.steps.extend(new_steps);
    }

    /// Get the next pending step
    pub fn get_next_pending_step(&mut self) -> Option<ExecutionStep> {
        for (index, step) in self.steps.iter().enumerate() {
            if step.output.is_none() {
                self.current_step = index;
                return Some(step.clone());
            }
        }
        None
    }

    /// Update step output by step number
    pub fn update_step_output(&mut self, step_number: usize, result: Result<String, String>) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.step_number == step_number) {
            match result {
                Ok(output) => {
                    step.output = Some(output);
                    step.success = true;
                }
                Err(error) => {
                    step.output = Some(error);
                    step.success = false;
                }
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> ExecutionState {
        self.state.clone()
    }

    /// Get execution summary
    pub fn get_summary(&self) -> String {
        let completed = self.steps.iter().filter(|s| s.output.is_some()).count();
        let total = self.steps.len();
        format!("Execution: {}/{} steps completed", completed, total)
    }

    /// Cancel the execution
    pub fn cancel(&mut self) {
        self.state = ExecutionState::Failed {
            error: "Cancelled by user".into()
        };
        self.spinner_active = false;
    }

    /// Get execution progress as percentage
    pub fn get_progress_percentage(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        (self.current_step as f32 / self.steps.len() as f32) * 100.0
    }

    /// Get execution duration so far
    pub fn get_execution_duration(&self) -> u64 {
        utils::duration_between(self.start_time, utils::unix_timestamp())
    }

    /// Check if all steps are completed successfully
    pub fn is_successful(&self) -> bool {
        matches!(self.state, ExecutionState::Completed) && 
        self.steps.iter().all(|step| step.success)
    }

    /// Get failed steps
    pub fn get_failed_steps(&self) -> Vec<(usize, &ExecutionStep)> {
        self.steps.iter()
            .enumerate()
            .filter(|(_, step)| !step.success && step.output.is_some())
            .collect()
    }

    /// Get successful steps
    pub fn get_successful_steps(&self) -> Vec<(usize, &ExecutionStep)> {
        self.steps.iter()
            .enumerate()
            .filter(|(_, step)| step.success)
            .collect()
    }

    /// Get formatted output for display
    pub fn get_formatted_output(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Add task description
        lines.push(format!("🎯 Task: {}", self.task_description));
        lines.push(String::new());

        // Add steps with their results
        for (i, step) in self.steps.iter().enumerate() {
            let icon = if step.success {
                "✅"
            } else if step.output.is_some() {
                "❌"
            } else {
                "⚡"
            };

            lines.push(format!("{} {}", icon, step.description));
            lines.push(format!("   Command: {}", step.command));

            if let Some(output) = &step.output {
                let trimmed = if output.len() > 200 {
                    format!("{}...", &output[..200])
                } else {
                    output.clone()
                };
                if !trimmed.trim().is_empty() {
                    lines.push(format!("   Output: {}", trimmed));
                }
            }

            lines.push(String::new());
        }

        // Add completion status
        match &self.state {
            ExecutionState::Completed => {
                lines.push("🎉 All steps completed successfully!".into());
            },
            ExecutionState::Failed { error } => {
                lines.push(format!("💥 Execution failed: {}", error));
            },
            _ => {}
        }

        // Add raw AI response for debugging
        if let Some(raw_response) = &self.raw_ai_response {
            lines.push(String::new());
            lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
            lines.push("DEBUG: Raw Ollama Output".into());
            lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
            lines.push(raw_response.clone());
            lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into());
        }

        lines
    }
}