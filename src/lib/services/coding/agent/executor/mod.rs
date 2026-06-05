//! Improved executor module with modern async patterns

use anyhow::{Context, Result};
use futures::stream::{Stream, StreamExt};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, oneshot, RwLock, Semaphore};
use tokio::time::{timeout, Duration};

pub mod base;
pub mod command;
pub mod compat;
pub mod stream;
pub mod task;

pub use base::*;
pub use command::CommandExecutor;
pub use stream::StreamingExecutor;
pub use task::TaskExecutor;

// Re-export compatibility types
pub use compat::{CodingAgentExecutor, EnhancedContext, UserMessage};

/// Modern async executor with improved patterns
pub struct AsyncExecutor {
    /// Shared state using RwLock for better async performance
    state: Arc<RwLock<ExecutorState>>,
    /// Command channel for async command processing
    command_tx: mpsc::Sender<ExecutorCommand>,
    /// Result channel for async responses
    result_rx: Arc<RwLock<mpsc::Receiver<ExecutorResult>>>,
    /// Semaphore for concurrent execution limiting
    concurrency_limiter: Arc<Semaphore>,
    /// Shutdown signal channel
    shutdown_tx: mpsc::Sender<()>,
}

#[derive(Debug)]
struct ExecutorState {
    running_tasks: Vec<RunningTask>,
    completed_tasks: Vec<CompletedTask>,
    queued_commands: Vec<QueuedCommand>,
    metrics: ExecutorMetrics,
}

#[derive(Debug)]
struct RunningTask {
    id: String,
    name: String,
    started_at: std::time::Instant,
    cancel_tx: oneshot::Sender<()>,
}

#[derive(Debug, Clone)]
struct CompletedTask {
    id: String,
    name: String,
    result: TaskResult,
    duration: Duration,
}

#[derive(Debug, Clone)]
struct QueuedCommand {
    id: String,
    command: String,
    priority: Priority,
    queued_at: std::time::Instant,
}

#[derive(Debug, Clone)]
enum Priority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone)]
struct ExecutorMetrics {
    total_executed: u64,
    successful: u64,
    failed: u64,
    average_duration: Duration,
    peak_concurrency: usize,
}

#[derive(Debug)]
enum ExecutorCommand {
    Execute {
        command: String,
        working_dir: PathBuf,
        response: oneshot::Sender<Result<String>>,
    },
    ExecuteStream {
        command: String,
        working_dir: PathBuf,
        stream_tx: mpsc::Sender<StreamChunk>,
    },
    Cancel {
        task_id: String,
        response: oneshot::Sender<Result<()>>,
    },
    GetStatus {
        response: oneshot::Sender<ExecutorStatus>,
    },
}

#[derive(Debug, Clone)]
enum ExecutorResult {
    Success { task_id: String, output: String },
    Failure { task_id: String, error: String },
    Cancelled { task_id: String },
}

#[derive(Debug, Clone)]
struct ExecutorStatus {
    running: usize,
    queued: usize,
    completed: usize,
    metrics: ExecutorMetrics,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub data: String,
    pub is_stdout: bool,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    Success(String),
    Failure(String),
    Cancelled,
}

impl AsyncExecutor {
    /// Create a new async executor with modern patterns
    pub fn new(max_concurrency: usize) -> Self {
        let (command_tx, mut command_rx) = mpsc::channel::<ExecutorCommand>(100);
        let (result_tx, result_rx) = mpsc::channel::<ExecutorResult>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let state = Arc::new(RwLock::new(ExecutorState {
            running_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            queued_commands: Vec::new(),
            metrics: ExecutorMetrics {
                total_executed: 0,
                successful: 0,
                failed: 0,
                average_duration: Duration::from_secs(0),
                peak_concurrency: 0,
            },
        }));

        let concurrency_limiter = Arc::new(Semaphore::new(max_concurrency));

        // Spawn background task processor
        let state_clone = state.clone();
        let limiter_clone = concurrency_limiter.clone();
        let result_tx_clone = result_tx.clone();

        tokio::spawn(async move {
            loop {
                select! {
                    Some(cmd) = command_rx.recv() => {
                        Self::handle_command(
                            cmd,
                            state_clone.clone(),
                            limiter_clone.clone(),
                            result_tx_clone.clone(),
                        ).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Executor shutdown requested");
                        break;
                    }
                }
            }
        });

        Self {
            state,
            command_tx,
            result_rx: Arc::new(RwLock::new(result_rx)),
            concurrency_limiter,
            shutdown_tx,
        }
    }

    /// Execute a command asynchronously with modern patterns
    pub async fn execute(&self, command: &str, working_dir: &Path) -> Result<String> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(ExecutorCommand::Execute {
                command: command.to_string(),
                working_dir: working_dir.to_path_buf(),
                response: response_tx,
            })
            .await
            .context("Failed to send command")?;

        response_rx.await.context("Failed to receive response")?
    }

    /// Execute with streaming output
    pub async fn execute_stream(
        &self,
        command: &str,
        working_dir: &Path,
    ) -> Result<impl Stream<Item = StreamChunk>> {
        let (stream_tx, stream_rx) = mpsc::channel(100);

        self.command_tx
            .send(ExecutorCommand::ExecuteStream {
                command: command.to_string(),
                working_dir: working_dir.to_path_buf(),
                stream_tx,
            })
            .await
            .context("Failed to send stream command")?;

        Ok(tokio_stream::wrappers::ReceiverStream::new(stream_rx))
    }

    /// Execute with timeout
    pub async fn execute_with_timeout(
        &self,
        command: &str,
        working_dir: &Path,
        duration: Duration,
    ) -> Result<String> {
        timeout(duration, self.execute(command, working_dir))
            .await
            .context("Command timed out")?
    }

    /// Execute multiple commands concurrently
    pub async fn execute_batch(&self, commands: Vec<(&str, &Path)>) -> Vec<Result<String>> {
        let futures = commands
            .into_iter()
            .map(|(cmd, dir)| self.execute(cmd, dir))
            .collect::<Vec<_>>();

        futures::future::join_all(futures).await
    }

    /// Execute with retry logic using exponential backoff
    pub async fn execute_with_retry(
        &self,
        command: &str,
        working_dir: &Path,
        max_retries: u32,
    ) -> Result<String> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            match self.execute(command, working_dir).await {
                Ok(output) => return Ok(output),
                Err(e) if attempts < max_retries => {
                    attempts += 1;
                    warn!(
                        "Command failed (attempt {}/{}): {}",
                        attempts, max_retries, e
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(ExecutorCommand::Cancel {
                task_id: task_id.to_string(),
                response: response_tx,
            })
            .await
            .context("Failed to send cancel command")?;

        response_rx
            .await
            .context("Failed to receive cancel response")?
    }

    /// Get executor status
    pub async fn get_status(&self) -> Result<ExecutorStatus> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(ExecutorCommand::GetStatus {
                response: response_tx,
            })
            .await
            .context("Failed to send status command")?;

        Ok(response_rx.await.context("Failed to receive status")?)
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating executor shutdown");
        let _ = self.shutdown_tx.send(()).await;

        // Wait for running tasks to complete
        let mut attempts = 0;
        while attempts < 30 {
            let state = self.state.read().await;
            if state.running_tasks.is_empty() {
                break;
            }
            drop(state);
            tokio::time::sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        Ok(())
    }

    /// Handle incoming commands (internal)
    async fn handle_command(
        command: ExecutorCommand,
        state: Arc<RwLock<ExecutorState>>,
        limiter: Arc<Semaphore>,
        result_tx: mpsc::Sender<ExecutorResult>,
    ) {
        match command {
            ExecutorCommand::Execute {
                command,
                working_dir,
                response,
            } => {
                let task_id = format!(
                    "task_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );

                let limiter_clone = limiter.clone();
                tokio::spawn(async move {
                    let permit = limiter_clone.acquire().await.unwrap();
                    let result = Self::run_command(&command, &working_dir).await;
                    drop(permit);

                    let _ = response.send(
                        result
                            .as_ref()
                            .map(|s| s.clone())
                            .map_err(|e| anyhow::anyhow!(e.to_string())),
                    );
                    let _ = result_tx
                        .send(match result {
                            Ok(output) => ExecutorResult::Success {
                                task_id: task_id.clone(),
                                output,
                            },
                            Err(e) => ExecutorResult::Failure {
                                task_id: task_id.clone(),
                                error: e.to_string(),
                            },
                        })
                        .await;
                });
            }
            ExecutorCommand::ExecuteStream {
                command,
                working_dir,
                stream_tx,
            } => {
                let limiter = limiter.clone();
                tokio::spawn(async move {
                    let permit = limiter.acquire().await.unwrap();
                    Self::run_command_stream(&command, &working_dir, stream_tx).await;
                    drop(permit);
                });
            }
            ExecutorCommand::Cancel { task_id, response } => {
                let mut state = state.write().await;
                if let Some(pos) = state.running_tasks.iter().position(|t| t.id == task_id) {
                    let _task = state.running_tasks.remove(pos);
                    // Note: cancel_tx is consumed when removed, cancellation happens automatically
                    let _ = response.send(Ok(()));
                } else {
                    let _ = response.send(Err(anyhow::anyhow!("Task not found")));
                }
            }
            ExecutorCommand::GetStatus { response } => {
                let state = state.read().await;
                let _ = response.send(ExecutorStatus {
                    running: state.running_tasks.len(),
                    queued: state.queued_commands.len(),
                    completed: state.completed_tasks.len(),
                    metrics: state.metrics.clone(),
                });
            }
        }
    }

    /// Run a command (internal)
    async fn run_command(command: &str, working_dir: &Path) -> Result<String> {
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .output()
            .await
            .context("Failed to execute command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Run a command with streaming output (internal)
    async fn run_command_stream(
        command: &str,
        working_dir: &Path,
        stream_tx: mpsc::Sender<StreamChunk>,
    ) {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn command");

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let tx_clone = stream_tx.clone();

        // Stream stdout
        let stdout_handle = tokio::spawn(async move {
            let mut lines = stdout_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx_clone
                    .send(StreamChunk {
                        data: line,
                        is_stdout: true,
                        timestamp: std::time::SystemTime::now(),
                    })
                    .await;
            }
        });

        // Stream stderr
        let stderr_handle = tokio::spawn(async move {
            let mut lines = stderr_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = stream_tx
                    .send(StreamChunk {
                        data: line,
                        is_stdout: false,
                        timestamp: std::time::SystemTime::now(),
                    })
                    .await;
            }
        });

        let _ = tokio::join!(stdout_handle, stderr_handle);
        let _ = child.wait().await;
    }
}
