use crate::services::coding::agent::types::CodeExecutionRequest;
use crate::services::coding::CodingAgentExecutor;

/// Ring buffer for sparkline history data
#[derive(Debug, Clone)]
pub struct RingBuffer {
    pub data: Vec<f64>,
    pub capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.data.len() >= self.capacity {
            self.data.remove(0);
        }
        self.data.push(value);
    }

    pub fn as_u64_vec(&self) -> Vec<u64> {
        self.data.iter().map(|v| *v as u64).collect()
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(60) // 60 samples = 2 min at 2s intervals
    }
}

/// Enhanced service status with comprehensive monitoring
#[derive(Debug, Default, Clone)]
pub struct ServiceStatus {
    pub crawler: String,
    pub redis: String,
    pub docker: String,
    pub sms: String,
    pub postgres: String,
    pub lifx: String,
    pub http_server: String,
    pub ollama: String,
    pub tts: String,
    pub stt: String,
    pub ssh_server: String,
    pub media: String,
    pub snapcast: String,
    pub memory_usage: String,
    pub cpu_usage: String,
    pub disk_usage: String,
    pub update_count: u64,
    // Sparkline history
    pub cpu_history: RingBuffer,
    pub memory_history: RingBuffer,
}

#[derive(Debug, Clone, Copy)]
pub struct ServiceCatalogEntry {
    pub key: &'static str,
    pub label: &'static str,
}

pub const SERVICE_CATALOG: [ServiceCatalogEntry; 13] = [
    ServiceCatalogEntry {
        key: "crawler",
        label: "Crawler",
    },
    ServiceCatalogEntry {
        key: "redis",
        label: "Redis",
    },
    ServiceCatalogEntry {
        key: "docker",
        label: "Docker",
    },
    ServiceCatalogEntry {
        key: "sms",
        label: "SMS",
    },
    ServiceCatalogEntry {
        key: "postgres",
        label: "PostgreSQL",
    },
    ServiceCatalogEntry {
        key: "lifx",
        label: "LIFX",
    },
    ServiceCatalogEntry {
        key: "http_server",
        label: "HTTP Server",
    },
    ServiceCatalogEntry {
        key: "ollama",
        label: "Ollama AI",
    },
    ServiceCatalogEntry {
        key: "tts",
        label: "TTS",
    },
    ServiceCatalogEntry {
        key: "stt",
        label: "STT",
    },
    ServiceCatalogEntry {
        key: "ssh_server",
        label: "SSH Server",
    },
    ServiceCatalogEntry {
        key: "media",
        label: "Media Center",
    },
    ServiceCatalogEntry {
        key: "snapcast",
        label: "Snapcast",
    },
];

pub fn is_healthy_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "running" | "connected" | "online"
    )
}

impl ServiceStatus {
    pub fn status_for(&self, key: &str) -> &str {
        match key {
            "crawler" => &self.crawler,
            "redis" => &self.redis,
            "docker" => &self.docker,
            "sms" => &self.sms,
            "postgres" => &self.postgres,
            "lifx" => &self.lifx,
            "http_server" => &self.http_server,
            "ollama" => &self.ollama,
            "tts" => &self.tts,
            "stt" => &self.stt,
            "ssh_server" => &self.ssh_server,
            "media" => &self.media,
            "snapcast" => &self.snapcast,
            _ => "unknown",
        }
    }

    pub fn service_rows(&self) -> Vec<(ServiceCatalogEntry, &str)> {
        SERVICE_CATALOG
            .iter()
            .copied()
            .map(|entry| (entry, self.status_for(entry.key)))
            .collect()
    }

    pub fn healthy_service_count(&self) -> usize {
        SERVICE_CATALOG
            .iter()
            .filter(|entry| is_healthy_status(self.status_for(entry.key)))
            .count()
    }
}

/// Notification toast
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: std::time::Instant,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Command palette state
#[derive(Debug, Clone, Default)]
pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub actions: Vec<PaletteAction>,
}

#[derive(Debug, Clone)]
pub struct PaletteAction {
    pub label: String,
    pub description: String,
    pub mode: Option<TuiMode>,
}

/// Navigation state for TUI
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TuiMode {
    #[default]
    Command,
    Services,
    Logs,
    SystemInfo,
    Database,
    Files,
    Help,
    CodingAgent,
}

/// Main TUI state
#[derive(Debug, Clone)]
pub struct TuiState {
    pub mode: TuiMode,
    pub selected_service: usize,
    pub log_filter_level: String,
    pub log_scroll_offset: u16,
    pub log_filter_text: String,
    pub log_input_mode: bool,
    pub help_scroll: u16,
    pub file_browser_path: std::path::PathBuf,
    pub selected_file: usize,
    pub db_table_list: Vec<String>,
    pub selected_table: usize,
    // Coding agent state
    pub coding_agent_input: String,
    pub coding_agent_input_mode: bool,
    pub coding_agent_response: String,
    pub coding_agent_model: String,
    pub coding_agent_pending_commands: Vec<CodeExecutionRequest>,
    pub coding_agent_selected_command: usize,
    pub coding_agent_scroll_offset: u16,
    pub coding_agent_executor: Option<CodingAgentExecutor>,
    pub coding_agent_execution_log: Vec<String>,
    pub coding_agent_spinner_text: String,
    pub coding_agent_history: Vec<String>,
    pub coding_agent_history_index: usize,
    pub coding_agent_context: Vec<String>,
    pub coding_agent_working_dir: String,
    pub coding_agent_execution_steps: Vec<String>,
    pub coding_agent_current_step: usize,
    pub coding_agent_auto_execute: bool,
    pub coding_agent_verify_mode: bool,
    pub coding_agent_panel_focus: usize,
    pub coding_agent_show_help: bool,
    // Command history (F1 mode)
    pub command_history: Vec<String>,
    pub history_search_mode: bool,
    pub history_search_query: String,
    // Notifications
    pub notifications: Vec<Notification>,
    // Command palette
    pub command_palette: CommandPalette,
    // Vim-style keybindings
    pub vim_mode: bool,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            mode: TuiMode::default(),
            selected_service: 0,
            log_filter_level: String::new(),
            log_scroll_offset: 0,
            log_filter_text: String::new(),
            log_input_mode: false,
            help_scroll: 0,
            file_browser_path: std::path::PathBuf::from("."),
            selected_file: 0,
            db_table_list: Vec::new(),
            selected_table: 0,
            coding_agent_input: String::new(),
            coding_agent_input_mode: false,
            coding_agent_response: String::new(),
            coding_agent_model: String::from("llama3.2:3b"),
            coding_agent_pending_commands: Vec::new(),
            coding_agent_selected_command: 0,
            coding_agent_scroll_offset: 0,
            coding_agent_executor: None,
            coding_agent_execution_log: Vec::new(),
            coding_agent_spinner_text: String::new(),
            coding_agent_history: Vec::new(),
            coding_agent_history_index: 0,
            coding_agent_context: Vec::new(),
            coding_agent_working_dir: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            coding_agent_execution_steps: Vec::new(),
            coding_agent_current_step: 0,
            coding_agent_auto_execute: false,
            coding_agent_verify_mode: false,
            coding_agent_panel_focus: 0,
            coding_agent_show_help: false,
            command_history: Vec::new(),
            history_search_mode: false,
            history_search_query: String::new(),
            notifications: Vec::new(),
            command_palette: CommandPalette::default(),
            vim_mode: false,
        }
    }
}
