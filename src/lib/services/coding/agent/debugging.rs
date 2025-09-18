use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Debugging engine for interactive debugging support
pub struct DebuggingEngine {
    debuggers: HashMap<String, Box<dyn Debugger>>,
    sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
    breakpoint_manager: BreakpointManager,
}

/// Trait for language-specific debuggers
#[async_trait::async_trait]
pub trait Debugger: Send + Sync {
    fn name(&self) -> &str;
    fn supported_languages(&self) -> Vec<&str>;

    async fn start_session(&self, config: &DebugConfig) -> Result<String>;
    async fn attach_to_process(&self, pid: u32) -> Result<String>;
    async fn set_breakpoint(&self, session_id: &str, breakpoint: &Breakpoint) -> Result<()>;
    async fn remove_breakpoint(&self, session_id: &str, breakpoint_id: &str) -> Result<()>;
    async fn continue_execution(&self, session_id: &str) -> Result<DebugEvent>;
    async fn step_over(&self, session_id: &str) -> Result<DebugEvent>;
    async fn step_into(&self, session_id: &str) -> Result<DebugEvent>;
    async fn step_out(&self, session_id: &str) -> Result<DebugEvent>;
    async fn evaluate(&self, session_id: &str, expression: &str) -> Result<Value>;
    async fn get_stack_trace(&self, session_id: &str) -> Result<Vec<StackFrame>>;
    async fn get_variables(&self, session_id: &str, frame_id: usize) -> Result<Vec<Variable>>;
    async fn stop_session(&self, session_id: &str) -> Result<()>;
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_directory: PathBuf,
    pub stop_on_entry: bool,
    pub debug_adapter: DebugAdapter,
}

/// Debug adapter protocol support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugAdapter {
    Lldb,           // Rust, C/C++, Swift
    Gdb,            // C/C++, Go
    Delve,          // Go
    Pdb,            // Python
    NodeDebug,      // JavaScript/TypeScript
    JavaDebug,      // Java
    NetCoreDebug,   // C#/.NET
}

/// Debug session
#[derive(Debug, Clone)]
pub struct DebugSession {
    pub id: String,
    pub config: DebugConfig,
    pub state: DebugState,
    pub current_frame: Option<usize>,
    pub breakpoints: Vec<Breakpoint>,
    pub watch_expressions: Vec<WatchExpression>,
    pub output_buffer: Vec<String>,
}

/// Debug state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugState {
    Starting,
    Running,
    Paused(PauseReason),
    Terminated,
}

/// Reason for pause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PauseReason {
    Breakpoint(String),
    Step,
    Exception(String),
    Pause,
    Entry,
}

/// Breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: String,
    pub file: PathBuf,
    pub line: usize,
    pub condition: Option<String>,
    pub hit_count: usize,
    pub log_message: Option<String>,
    pub enabled: bool,
}

/// Watch expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
    pub id: String,
    pub expression: String,
    pub value: Option<Value>,
}

/// Debug event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugEvent {
    Stopped(StoppedEvent),
    Continued,
    Exited(i32),
    Output(OutputEvent),
    Breakpoint(BreakpointEvent),
}

/// Stopped event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoppedEvent {
    pub reason: PauseReason,
    pub thread_id: Option<u32>,
    pub all_threads_stopped: bool,
    pub description: Option<String>,
}

/// Output event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEvent {
    pub category: OutputCategory,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
    Telemetry,
}

/// Breakpoint event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointEvent {
    pub reason: BreakpointReason,
    pub breakpoint: Breakpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakpointReason {
    New,
    Changed,
    Removed,
}

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub source: Option<Source>,
    pub line: usize,
    pub column: usize,
    pub module_id: Option<String>,
}

/// Source file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub path: PathBuf,
    pub name: String,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: Value,
    pub type_name: Option<String>,
    pub reference: Option<usize>,
    pub indexed_variables: Option<usize>,
    pub named_variables: Option<usize>,
}

/// Value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Null,
    Reference(usize),
}

/// Breakpoint manager
pub struct BreakpointManager {
    breakpoints: Arc<RwLock<HashMap<String, Vec<Breakpoint>>>>,
    next_id: Arc<RwLock<usize>>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            breakpoints: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    pub async fn add_breakpoint(&self, file: PathBuf, line: usize) -> String {
        let mut next_id = self.next_id.write().await;
        let id = format!("bp_{}", *next_id);
        *next_id += 1;

        let breakpoint = Breakpoint {
            id: id.clone(),
            file: file.clone(),
            line,
            condition: None,
            hit_count: 0,
            log_message: None,
            enabled: true,
        };

        let mut breakpoints = self.breakpoints.write().await;
        breakpoints.entry(file.to_string_lossy().to_string())
            .or_insert_with(Vec::new)
            .push(breakpoint);

        id
    }

    pub async fn remove_breakpoint(&self, id: &str) -> Result<()> {
        let mut breakpoints = self.breakpoints.write().await;

        for (_, bps) in breakpoints.iter_mut() {
            if let Some(pos) = bps.iter().position(|bp| bp.id == id) {
                bps.remove(pos);
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Breakpoint {} not found", id))
    }

    pub async fn get_breakpoints(&self, file: &Path) -> Vec<Breakpoint> {
        let breakpoints = self.breakpoints.read().await;
        breakpoints.get(&file.to_string_lossy().to_string())
            .cloned()
            .unwrap_or_default()
    }

    pub async fn toggle_breakpoint(&self, id: &str) -> Result<()> {
        let mut breakpoints = self.breakpoints.write().await;

        for (_, bps) in breakpoints.iter_mut() {
            if let Some(bp) = bps.iter_mut().find(|bp| bp.id == id) {
                bp.enabled = !bp.enabled;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Breakpoint {} not found", id))
    }
}

impl DebuggingEngine {
    pub fn new() -> Self {
        Self {
            debuggers: HashMap::new(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            breakpoint_manager: BreakpointManager::new(),
        }
    }

    /// Register a debugger
    pub fn register_debugger(&mut self, name: String, debugger: Box<dyn Debugger>) {
        self.debuggers.insert(name, debugger);
    }

    /// Start a debug session
    pub async fn start_session(&self, config: DebugConfig) -> Result<String> {
        let debugger = self.select_debugger(&config)?;
        let session_id = debugger.start_session(&config).await?;

        let session = DebugSession {
            id: session_id.clone(),
            config,
            state: DebugState::Starting,
            current_frame: None,
            breakpoints: Vec::new(),
            watch_expressions: Vec::new(),
            output_buffer: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Set a breakpoint
    pub async fn set_breakpoint(
        &self,
        session_id: &str,
        file: PathBuf,
        line: usize,
    ) -> Result<String> {
        let bp_id = self.breakpoint_manager.add_breakpoint(file.clone(), line).await;

        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let debugger = self.select_debugger(&session.config)?;

            let breakpoint = Breakpoint {
                id: bp_id.clone(),
                file,
                line,
                condition: None,
                hit_count: 0,
                log_message: None,
                enabled: true,
            };

            debugger.set_breakpoint(session_id, &breakpoint).await?;
        }

        Ok(bp_id)
    }

    /// Continue execution
    pub async fn continue_execution(&self, session_id: &str) -> Result<DebugEvent> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        let event = debugger.continue_execution(session_id).await?;

        // Update session state
        drop(sessions);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            match &event {
                DebugEvent::Stopped(stopped) => {
                    session.state = DebugState::Paused(stopped.reason.clone());
                }
                DebugEvent::Continued => {
                    session.state = DebugState::Running;
                }
                DebugEvent::Exited(_) => {
                    session.state = DebugState::Terminated;
                }
                _ => {}
            }
        }

        Ok(event)
    }

    /// Step over
    pub async fn step_over(&self, session_id: &str) -> Result<DebugEvent> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        debugger.step_over(session_id).await
    }

    /// Step into
    pub async fn step_into(&self, session_id: &str) -> Result<DebugEvent> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        debugger.step_into(session_id).await
    }

    /// Get stack trace
    pub async fn get_stack_trace(&self, session_id: &str) -> Result<Vec<StackFrame>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        debugger.get_stack_trace(session_id).await
    }

    /// Evaluate expression
    pub async fn evaluate(&self, session_id: &str, expression: &str) -> Result<Value> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        debugger.evaluate(session_id, expression).await
    }

    /// Add watch expression
    pub async fn add_watch(&self, session_id: &str, expression: String) -> Result<String> {
        let watch_id = format!("watch_{}", uuid::Uuid::new_v4());

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.watch_expressions.push(WatchExpression {
                id: watch_id.clone(),
                expression,
                value: None,
            });
        }

        Ok(watch_id)
    }

    /// Update watch expressions
    pub async fn update_watches(&self, session_id: &str) -> Result<Vec<WatchExpression>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        let mut updated_watches = Vec::new();

        for watch in &session.watch_expressions {
            let value = debugger.evaluate(session_id, &watch.expression).await.ok();
            updated_watches.push(WatchExpression {
                id: watch.id.clone(),
                expression: watch.expression.clone(),
                value,
            });
        }

        // Update session with new values
        drop(sessions);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.watch_expressions = updated_watches.clone();
        }

        Ok(updated_watches)
    }

    /// Stop debug session
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let debugger = self.select_debugger(&session.config)?;
        debugger.stop_session(session_id).await?;

        drop(sessions);
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);

        Ok(())
    }

    /// Select appropriate debugger for config
    fn select_debugger(&self, config: &DebugConfig) -> Result<&Box<dyn Debugger>> {
        let debugger_name = match &config.debug_adapter {
            DebugAdapter::Lldb => "lldb",
            DebugAdapter::Gdb => "gdb",
            DebugAdapter::Delve => "delve",
            DebugAdapter::Pdb => "pdb",
            DebugAdapter::NodeDebug => "node",
            DebugAdapter::JavaDebug => "java",
            DebugAdapter::NetCoreDebug => "netcore",
        };

        self.debuggers.get(debugger_name)
            .ok_or_else(|| anyhow::anyhow!("Debugger '{}' not available", debugger_name))
    }

    /// Get all active sessions
    pub async fn get_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Get session info
    pub async fn get_session_info(&self, session_id: &str) -> Option<DebugSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }
}