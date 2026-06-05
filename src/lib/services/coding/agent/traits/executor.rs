//! Executor traits

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

/// Trait for command execution
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a command
    async fn execute(&self, command: &str, working_dir: &Path) -> Result<ExecutionResult>;

    /// Execute with timeout
    async fn execute_with_timeout(
        &self,
        command: &str,
        working_dir: &Path,
        timeout: Duration,
    ) -> Result<ExecutionResult>;

    /// Execute in background
    async fn execute_background(&self, command: &str, working_dir: &Path) -> Result<ProcessHandle>;

    /// Check if command is safe
    async fn is_safe_command(&self, command: &str) -> Result<bool>;
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ProcessHandle {
    pub pid: u32,
    pub command: String,
    pub started_at: std::time::SystemTime,
}

impl ProcessHandle {
    /// Check if process is still running
    pub async fn is_running(&self) -> bool {
        // Implementation would check process status
        true
    }

    /// Kill the process
    pub async fn kill(&self) -> Result<()> {
        // Implementation would kill the process
        Ok(())
    }

    /// Wait for process to complete
    pub async fn wait(&self) -> Result<ExecutionResult> {
        // Implementation would wait for process
        Ok(ExecutionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            duration: Duration::from_secs(0),
        })
    }
}

/// Trait for task execution
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task
    async fn execute_task(&self, task: Task) -> Result<TaskResult>;

    /// Execute tasks in parallel
    async fn execute_parallel(&self, tasks: Vec<Task>) -> Result<Vec<TaskResult>>;

    /// Execute tasks in sequence
    async fn execute_sequential(&self, tasks: Vec<Task>) -> Result<Vec<TaskResult>>;

    /// Cancel a running task
    async fn cancel_task(&self, task_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<String>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    Command(String),
    Function(String),
    Pipeline(Vec<Task>),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

/// Trait for script execution
#[async_trait]
pub trait ScriptExecutor: Send + Sync {
    /// Execute a script file
    async fn execute_script(&self, script_path: &Path) -> Result<ExecutionResult>;

    /// Execute inline script
    async fn execute_inline(&self, script: &str, language: &str) -> Result<ExecutionResult>;

    /// Validate script syntax
    async fn validate_script(&self, script: &str, language: &str) -> Result<ValidationResult>;

    /// Get supported script languages
    fn supported_languages(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub warning_code: Option<String>,
}
