use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use anyhow::Result;

/// Enhanced execution context that maintains state across commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Current working directory
    pub working_directory: PathBuf,

    /// Environment variables (persisted across commands)
    pub environment: HashMap<String, String>,

    /// Shell aliases and functions
    pub aliases: HashMap<String, String>,

    /// Command execution timeout
    pub timeout: Duration,

    /// Shell type (bash, zsh, fish, etc.)
    pub shell: ShellType,

    /// Session ID for tracking
    pub session_id: String,

    /// Creation timestamp
    pub created_at: SystemTime,

    /// Last command timestamp
    pub last_command_at: Option<SystemTime>,

    /// Virtual display for GUI apps (X11/Wayland)
    pub display: Option<String>,

    /// GUI rendering mode
    pub gui_mode: GuiMode,

    /// Persistent shell process ID (if using persistent shell)
    pub shell_pid: Option<u32>,

    /// Command history for this context
    pub command_history: Vec<ContextCommand>,

    /// Open file descriptors/handles
    pub open_files: HashMap<String, FileHandle>,

    /// Active background processes
    pub background_processes: Vec<ProcessInfo>,
}

/// Shell type for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Sh,
    PowerShell,
    Cmd,
}

/// GUI rendering mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuiMode {
    /// No GUI support
    None,
    /// X11 forwarding
    X11,
    /// Wayland
    Wayland,
    /// Terminal-based GUI rendering (term.everything)
    TerminalGui {
        /// Width in characters
        width: u32,
        /// Height in characters
        height: u32,
        /// Color depth
        colors: ColorDepth,
    },
}

/// Color depth for terminal GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorDepth {
    Monochrome,
    Colors16,
    Colors256,
    TrueColor,
}

/// Command executed in this context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCommand {
    pub command: String,
    pub executed_at: SystemTime,
    pub working_directory: PathBuf,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

/// File handle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHandle {
    pub path: PathBuf,
    pub mode: FileMode,
    pub opened_at: SystemTime,
    pub size: u64,
}

/// File access mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileMode {
    Read,
    Write,
    Append,
    ReadWrite,
}

/// Background process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub started_at: SystemTime,
    pub status: ProcessStatus,
}

/// Process status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Completed(i32),
    Failed(String),
}

impl Default for ExecutionContext {
    fn default() -> Self {
        // Only include essential environment variables, not the entire process environment
        let mut essential_env = HashMap::new();

        // Only copy essential environment variables
        for key in &["PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "LC_ALL"] {
            if let Ok(value) = std::env::var(key) {
                essential_env.insert(key.to_string(), value);
            }
        }

        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            environment: essential_env,  // Use only essential env vars
            aliases: HashMap::new(),
            timeout: Duration::from_secs(120),
            shell: ShellType::Bash,
            session_id: uuid::Uuid::new_v4().to_string(),
            created_at: SystemTime::now(),
            last_command_at: None,
            display: std::env::var("DISPLAY").ok(),
            gui_mode: GuiMode::None,
            shell_pid: None,
            command_history: Vec::new(),
            open_files: HashMap::new(),
            background_processes: Vec::new(),
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with GUI support
    pub fn with_gui_support(width: u32, height: u32) -> Self {
        let mut context = Self::default();
        context.gui_mode = GuiMode::TerminalGui {
            width,
            height,
            colors: ColorDepth::TrueColor,
        };
        // Set up virtual display for term.everything
        context.environment.insert("TERM_GUI".to_string(), "1".to_string());
        context.environment.insert("TERM_GUI_WIDTH".to_string(), width.to_string());
        context.environment.insert("TERM_GUI_HEIGHT".to_string(), height.to_string());
        context
    }

    /// Update working directory
    pub fn set_working_directory(&mut self, path: PathBuf) -> Result<(), String> {
        if path.exists() && path.is_dir() {
            self.working_directory = path;
            Ok(())
        } else {
            Err(format!("Directory does not exist: {:?}", path))
        }
    }

    /// Add environment variable
    pub fn set_env(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }

    /// Add command to history
    pub fn add_command(&mut self, command: ContextCommand) {
        self.last_command_at = Some(SystemTime::now());
        self.command_history.push(command);

        // Limit history to last 1000 commands
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }
    }

    /// Get command for execution with proper context
    pub fn prepare_command(&self, command: &str) -> String {
        // For simple commands, just return them as-is
        // Don't export the entire environment which can cause issues
        if command.trim().split_whitespace().count() <= 3 {
            // Simple commands like "mkdir foo" or "cargo new project"
            return command.to_string();
        }

        let mut prepared = String::new();

        // Only set up explicitly added environment variables (not inherited ones)
        // Skip this for now to avoid command length issues
        // for (key, value) in &self.environment {
        //     prepared.push_str(&format!("export {}=\"{}\"; ", key, value));
        // }

        // Set up aliases only if we have any custom ones
        if !self.aliases.is_empty() {
            for (alias, expansion) in &self.aliases {
                prepared.push_str(&format!("alias {}='{}'; ", alias, expansion));
            }
        }

        // Don't change directory in the command itself - let the executor handle that
        // prepared.push_str(&format!("cd {:?} && ", self.working_directory));

        // Add the actual command
        if prepared.is_empty() {
            command.to_string()
        } else {
            prepared.push_str(command);
            prepared
        }
    }

    /// Check if GUI is supported
    pub fn supports_gui(&self) -> bool {
        !matches!(self.gui_mode, GuiMode::None)
    }

    /// Prepare command for GUI execution
    pub fn prepare_gui_command(&self, command: &str) -> String {
        match &self.gui_mode {
            GuiMode::TerminalGui { width, height, .. } => {
                format!(
                    "term.everything --width {} --height {} -- {}",
                    width, height, command
                )
            }
            GuiMode::X11 | GuiMode::Wayland => {
                // Use regular display
                command.to_string()
            }
            GuiMode::None => command.to_string(),
        }
    }

    /// Clean up background processes
    pub fn cleanup_processes(&mut self) {
        self.background_processes.retain(|p| {
            matches!(p.status, ProcessStatus::Running | ProcessStatus::Stopped)
        });
    }

    /// Get context summary
    pub fn summary(&self) -> String {
        format!(
            "Session: {}\nDirectory: {:?}\nShell: {:?}\nCommands executed: {}\nBackground processes: {}\nGUI Mode: {:?}",
            self.session_id,
            self.working_directory,
            self.shell,
            self.command_history.len(),
            self.background_processes.len(),
            self.gui_mode
        )
    }

    /// Export context for persistence
    pub fn export(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import context from persistence
    pub fn import(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }
}

/// Execution context manager for handling multiple execution contexts
#[derive(Debug, Clone)]
pub struct ExecutionContextManager {
    contexts: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    active_context: Arc<RwLock<Option<String>>>,
}

impl ExecutionContextManager {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            active_context: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new context
    pub async fn create_context(&self, name: String) -> ExecutionContext {
        let context = ExecutionContext::new();
        let mut contexts = self.contexts.write().await;
        contexts.insert(name.clone(), context.clone());

        // Set as active if no active context
        let mut active = self.active_context.write().await;
        if active.is_none() {
            *active = Some(name);
        }

        context
    }

    /// Get active context
    pub async fn get_active_context(&self) -> Option<ExecutionContext> {
        let active = self.active_context.read().await;
        if let Some(name) = &*active {
            let contexts = self.contexts.read().await;
            contexts.get(name).cloned()
        } else {
            None
        }
    }

    /// Update active context
    pub async fn update_active_context<F>(&self, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut ExecutionContext),
    {
        let active = self.active_context.read().await;
        if let Some(name) = &*active {
            let mut contexts = self.contexts.write().await;
            if let Some(context) = contexts.get_mut(name) {
                updater(context);
                Ok(())
            } else {
                Err("Active context not found".to_string())
            }
        } else {
            Err("No active context".to_string())
        }
    }

    /// Switch active context
    pub async fn switch_context(&self, name: String) -> Result<(), String> {
        let contexts = self.contexts.read().await;
        if contexts.contains_key(&name) {
            let mut active = self.active_context.write().await;
            *active = Some(name);
            Ok(())
        } else {
            Err(format!("Context '{}' not found", name))
        }
    }

    /// List all contexts
    pub async fn list_contexts(&self) -> Vec<String> {
        let contexts = self.contexts.read().await;
        contexts.keys().cloned().collect()
    }

    /// Remove a context
    pub async fn remove_context(&self, name: &str) -> Result<(), String> {
        let mut contexts = self.contexts.write().await;
        if contexts.remove(name).is_some() {
            // If this was the active context, clear it
            let mut active = self.active_context.write().await;
            if active.as_ref() == Some(&name.to_string()) {
                *active = None;
            }
            Ok(())
        } else {
            Err(format!("Context '{}' not found", name))
        }
    }

    /// Save all contexts to disk
    pub async fn save_contexts(&self, path: &Path) -> Result<()> {
        let contexts = self.contexts.read().await;
        let data = serde_json::to_string_pretty(&*contexts)?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    /// Load contexts from disk
    pub async fn load_contexts(&self, path: &Path) -> Result<()> {
        let data = tokio::fs::read_to_string(path).await?;
        let loaded: HashMap<String, ExecutionContext> = serde_json::from_str(&data)?;
        let mut contexts = self.contexts.write().await;
        *contexts = loaded;
        Ok(())
    }
}