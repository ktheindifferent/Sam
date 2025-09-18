use super::{commands, helpers};
// use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    Terminal,
};
use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

use ratatui::widgets::{Block, Borders, Paragraph};
// use std::io::BufRead;
// use std::io::Read;
use std::sync::mpsc::{self, Sender};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState, TuiWidgetEvent};
// Add this import for catch_unwind on async blocks
// use futures::FutureExt;

// Add this struct for a custom Write implementation
struct PipeWriter {
    sender: Sender<String>,
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            for line in s.lines() {
                let _ = self.sender.send(line.to_string());
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Enhanced service status with more comprehensive monitoring
#[derive(Debug, Default, Clone)]
struct ServiceStatus {
    crawler: String,
    redis: String,
    docker: String,
    sms: String,
    postgres: String,
    lifx: String,
    http_server: String,
    ollama: String,
    tts: String,
    stt: String,
    ssh_server: String,
    memory_usage: String,
    cpu_usage: String,
    disk_usage: String,
    update_count: u64,
}

// Navigation state for TUI
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
enum TuiMode {
    #[default]
    Command,    // Default command input mode
    Services,   // Service management view
    Logs,       // Log viewer mode
    SystemInfo, // System information view
    Database,   // Database management view
    Files,      // File browser mode
    Help,       // Help screen
    CodingAgent, // AI coding assistant mode
}

#[derive(Debug, Clone)]
struct TuiState {
    mode: TuiMode,
    selected_service: usize,
    log_filter_level: String,
    log_scroll_offset: u16,
    log_filter_text: String,
    log_input_mode: bool, // whether we're in filter input mode
    help_scroll: u16,
    file_browser_path: std::path::PathBuf,
    db_table_list: Vec<String>,
    selected_table: usize,
    // Coding agent state
    coding_agent_input: String,
    coding_agent_input_mode: bool,
    coding_agent_response: String,
    coding_agent_model: String,
    coding_agent_pending_commands: Vec<crate::services::coding::agent::types::CodeExecutionRequest>,
    coding_agent_selected_command: usize,
    coding_agent_scroll_offset: u16,
    coding_agent_executor: Option<crate::services::coding::CodingAgentExecutor>,
    coding_agent_execution_log: Vec<String>,
    coding_agent_spinner_text: String,
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
        }
    }
}

// Global flag to track terminal state
static TERMINAL_NEEDS_RESTORE: AtomicBool = AtomicBool::new(false);

// Terminal state restoration functions
#[cfg(unix)]
fn restore_terminal_state() {
    if TERMINAL_NEEDS_RESTORE.load(Ordering::SeqCst) {
        let _ = execute!(
            io::stdout(),
            crossterm::cursor::Show,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
        TERMINAL_NEEDS_RESTORE.store(false, Ordering::SeqCst);
    }
}

#[cfg(not(unix))]
fn restore_terminal_state() {
    if TERMINAL_NEEDS_RESTORE.load(Ordering::SeqCst) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
        TERMINAL_NEEDS_RESTORE.store(false, Ordering::SeqCst);
    }
}

// Signal handlers for Unix systems
#[cfg(unix)]
extern "C" fn handle_suspend(_sig: i32) {
    restore_terminal_state();
}

#[cfg(unix)]
extern "C" fn handle_continue(_sig: i32) {
    // Re-initialize terminal when resumed
    let _ = enable_raw_mode();
    let _ = execute!(
        io::stdout(),
        EnterAlternateScreen,
        crossterm::cursor::Hide
    );
    TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_resize(_sig: i32) {
    // Terminal size changed - force a redraw
    // The main loop will handle the actual resize
}

// Function to detect if terminal state has been corrupted
fn check_terminal_corruption() -> bool {
    // Simple check: try to get terminal size
    // If this fails, terminal state may be corrupted
    crossterm::terminal::size().is_err()
}

// Function to force terminal refresh/restore
fn force_terminal_refresh(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    if check_terminal_corruption() {
        log::warn!("Terminal corruption detected, attempting refresh");
        restore_terminal_state();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let _ = enable_raw_mode();
        let _ = execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide);
        TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);
        terminal.clear()?;
    }
    Ok(())
}

/// Starts the interactive command prompt
///
/// This function checks for required Postgres environment variables,
/// initializes the TUI logger, and launches the TUI event loop.
pub async fn start_prompt() {
    log::info!("[sam cli] start_prompt() called");
    helpers::check_postgres_env();
    // Initialize tui-logger (new crate) - only if not already initialized
    let _ = tui_logger::init_logger(log::LevelFilter::Debug);
    tui_logger::set_default_level(log::LevelFilter::Debug);

    // Set up log file in a more robust way that handles sudo permissions
    let log_file_path = if let Ok(temp_dir) = std::env::var("TMPDIR") {
        // Use TMPDIR if available (macOS)
        std::path::PathBuf::from(temp_dir).join("sam_output.log")
    } else if std::path::Path::new("/tmp").exists() {
        // Fallback to /tmp (Linux/Unix)
        std::path::PathBuf::from("/tmp/sam_output.log")
    } else {
        // Final fallback - try /opt/sam if it exists, otherwise skip file logging
        let opt_sam = std::path::Path::new("/opt/sam");
        if opt_sam.exists() && opt_sam.is_dir() {
            opt_sam.join("output.log")
        } else {
            // Skip file logging if no suitable directory is found
            log::warn!("No suitable directory found for TUI log file, skipping file logging");
            std::path::PathBuf::new()
        }
    };
    
    // Only set up file logging if we have a valid path
    if !log_file_path.as_os_str().is_empty() {
        // Test if we can write to the log file
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_file_path)
        {
            Ok(_) => {
                log::debug!("Setting TUI log file to: {:?}", log_file_path);
                let file_options = tui_logger::TuiLoggerFile::new(log_file_path.to_str().unwrap())
                    .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
                    .output_file(true)
                    .output_separator(':');
                tui_logger::set_log_file(file_options);
            },
            Err(e) => {
                log::warn!("Cannot write to log file {:?}: {}. TUI logging will be memory-only.", log_file_path, e);
            }
        }
    }

    if let Err(e) = run_tui().await {
        log::info!("TUI error: {:?}", e);
    }
}

/// Render the command mode interface
fn render_command_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
    input: &str,
    show_cursor: bool,
    output_lines: &[String],
    scroll_offset: u16,
    output_height: &mut usize,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::Paragraph,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // status block (made larger)
            Constraint::Min(3),    // output
            Constraint::Length(3), // input
        ])
        .split(area);

    *output_height = chunks[1].height.max(1) as usize;

    let cursor_char = if show_cursor { "_" } else { " " };
    let input_display = format!("{input}{cursor_char}");

    // Enhanced service status block
    let status_lines = vec![
        Line::from(vec![Span::styled(
            "Services: ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from(vec![
            Span::styled(
                "Crawler: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.crawler, get_status_color(&status.crawler)),
            Span::raw("  "),
            Span::styled(
                "Redis: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.redis, get_status_color(&status.redis)),
            Span::raw("  "),
            Span::styled(
                "PostgreSQL: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.postgres, get_status_color(&status.postgres)),
            Span::raw("  "),
            Span::styled(
                "Ollama: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.ollama, get_status_color(&status.ollama)),
        ]),
        Line::from(vec![
            Span::styled(
                "Docker: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.docker, get_status_color(&status.docker)),
            Span::raw("  "),
            Span::styled(
                "TTS: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.tts, get_status_color(&status.tts)),
            Span::raw("  "),
            Span::styled(
                "STT: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.stt, get_status_color(&status.stt)),
            Span::raw("  "),
            Span::styled(
                "SSH: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.ssh_server, get_status_color(&status.ssh_server)),
            Span::raw("  "),
            Span::styled(
                "LIFX: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.lifx, get_status_color(&status.lifx)),
        ]),
        Line::from(vec![
            Span::styled(
                "System: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
            ),
            Span::styled(
                "CPU: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                &status.cpu_usage,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
            Span::raw("  "),
            Span::styled(
                "Memory: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                &status.memory_usage,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
    ];

    let status_widget = Paragraph::new(status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("System Status"),
    );

    let output: Vec<Line> = output_lines
        .iter()
        .map(|l| Line::from(Span::raw(l)))
        .collect();

    let output_widget = Paragraph::new(output)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Output"),
        )
        .scroll((scroll_offset, 0))
        .wrap(ratatui::widgets::Wrap { trim: false });

    let input_widget = Paragraph::new(input_display).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Command Input"),
    );


    f.render_widget(status_widget, chunks[0]);
    f.render_widget(output_widget, chunks[1]);
    f.render_widget(input_widget, chunks[2]);
}

/// Render the services management mode
fn render_services_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
    selected: usize,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::{List, ListItem, ListState, Paragraph},
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Service list
    let services = [("Crawler", &status.crawler),
        ("Redis", &status.redis),
        ("Docker", &status.docker),
        ("SMS", &status.sms),
        ("PostgreSQL", &status.postgres),
        ("LIFX", &status.lifx),
        ("HTTP Server", &status.http_server),
        ("Ollama AI", &status.ollama),
        ("TTS", &status.tts),
        ("STT", &status.stt),
        ("SSH Server", &status.ssh_server)];

    let service_items: Vec<ListItem> = services
        .iter()
        .enumerate()
        .map(|(i, (name, status_val))| {
            let style = if i == selected {
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::Yellow)
                    .fg(ratatui::style::Color::Black)
            } else {
                ratatui::style::Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", name), style),
                Span::styled(*status_val, get_status_color(status_val).patch(style)),
            ]))
        })
        .collect();

    let service_list = List::new(service_items)
        .block(Block::default().borders(Borders::ALL).title("Services"))
        .highlight_style(
            ratatui::style::Style::default()
                .bg(ratatui::style::Color::Yellow)
                .fg(ratatui::style::Color::Black),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    // Service details
    let unknown_status = String::from("Unknown");
    let (selected_name, selected_status) = if selected < services.len() {
        let (name, status) = services[selected];
        (name, status)
    } else {
        ("Unknown", &unknown_status)
    };

    let details_lines = vec![
        Line::from(vec![
            Span::styled(
                "Service: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                selected_name,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Status: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(selected_status, get_status_color(selected_status)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from("  [Space] Start/Stop Service"),
        Line::from("  [R] Restart Service"),
        Line::from("  [L] View Logs"),
        Line::from("  [Enter] Service Details"),
    ];

    let details_widget = Paragraph::new(details_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Service Details"),
    );

    f.render_stateful_widget(service_list, chunks[0], &mut list_state);
    f.render_widget(details_widget, chunks[1]);
}

/// Render the system information mode
fn render_system_info_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::Paragraph,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // System metrics
    let system_lines = vec![
        Line::from(vec![Span::styled(
            "System Information",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "CPU Usage: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                &status.cpu_usage,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Memory Usage: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                &status.memory_usage,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Disk Usage: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                &status.disk_usage,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Update Count: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                status.update_count.to_string(),
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
    ];

    let system_widget = Paragraph::new(system_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("System Metrics"),
    );

    // Service overview
    let service_lines = vec![
        Line::from(vec![Span::styled(
            "Service Overview",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Crawler: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.crawler, get_status_color(&status.crawler)),
        ]),
        Line::from(vec![
            Span::styled(
                "Redis: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.redis, get_status_color(&status.redis)),
        ]),
        Line::from(vec![
            Span::styled(
                "PostgreSQL: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.postgres, get_status_color(&status.postgres)),
        ]),
        Line::from(vec![
            Span::styled(
                "HTTP Server: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.http_server, get_status_color(&status.http_server)),
        ]),
        Line::from(vec![
            Span::styled(
                "Ollama AI: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.ollama, get_status_color(&status.ollama)),
        ]),
        Line::from(vec![
            Span::styled(
                "Docker: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.docker, get_status_color(&status.docker)),
        ]),
        Line::from(vec![
            Span::styled(
                "TTS: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.tts, get_status_color(&status.tts)),
        ]),
        Line::from(vec![
            Span::styled(
                "STT: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.stt, get_status_color(&status.stt)),
        ]),
        Line::from(vec![
            Span::styled(
                "SSH Server: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(&status.ssh_server, get_status_color(&status.ssh_server)),
        ]),
    ];

    let service_widget = Paragraph::new(service_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Service Status"),
    );

    f.render_widget(system_widget, chunks[0]);
    f.render_widget(service_widget, chunks[1]);
}

/// Render the logs mode with scrolling and filtering support
fn render_logs_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
    show_cursor: bool,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::Paragraph,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter input
            Constraint::Min(3),    // Log output
            Constraint::Length(2), // Status/help line
        ])
        .split(area);

    // Filter input box
    let cursor_char = if show_cursor && state.log_input_mode { "_" } else { " " };
    let filter_display = format!("{}{}", state.log_filter_text, cursor_char);
    let filter_title = if state.log_input_mode {
        "Filter (Type to filter, ESC to exit, Enter to apply)"
    } else {
        "Filter (Press / to edit, Use Up/Down to scroll)"
    };
    
    let filter_widget = Paragraph::new(filter_display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(filter_title),
        );

    // Create appropriate widget state based on filter
    let effective_widget_state = if !state.log_filter_text.is_empty() {
        let filter_lower = state.log_filter_text.to_lowercase();

        // Check if filtering by log level
        if filter_lower == "error" || filter_lower == "err" {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Error)
        } else if filter_lower == "warn" || filter_lower == "warning" {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Warn)
        } else if filter_lower == "info" {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Info)
        } else if filter_lower == "debug" {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Debug)
        } else if filter_lower == "trace" {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Trace)
        } else {
            // Apply target-based filtering for other text
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Off)
                .set_level_for_target(&state.log_filter_text, log::LevelFilter::Trace)
        }
    } else {
        // Default state showing all logs at debug level
        TuiWidgetState::new()
            .set_default_display_level(log::LevelFilter::Debug)
    };

    // Create a custom log widget with scrolling and filtering
    let log_title = if !state.log_filter_text.is_empty() {
        format!("System Logs - Filter: '{}' | Scroll: {}", state.log_filter_text, state.log_scroll_offset)
    } else {
        format!("System Logs - Scroll: {}", state.log_scroll_offset)
    };

    let tui_logger_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(log_title),
        )
        .output_separator('|')
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .style_error(ratatui::style::Style::default().fg(ratatui::style::Color::Red))
        .style_warn(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
        .style_info(ratatui::style::Style::default().fg(ratatui::style::Color::Blue))
        .style_debug(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
        .style_trace(ratatui::style::Style::default().fg(ratatui::style::Color::Gray))
        .state(&effective_widget_state);

    // Status/help line
    let help_text = if state.log_input_mode {
        "ESC: Exit filter | Enter: Apply filter | Backspace: Delete | Type: error/warn/info/debug/trace or target name"
    } else {
        "↑/↓: Scroll | PageUp/Down: Page | /: Filter | c: Clear | +/-: Log Level | Space: Toggle | ←/→: Display Level"
    };
    
    let help_widget = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Controls"),
        );

    f.render_widget(filter_widget, chunks[0]);
    f.render_widget(tui_logger_widget, chunks[1]);
    f.render_widget(help_widget, chunks[2]);
}

fn render_database_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    _tables: &[String],
    _selected: usize,
) {
    let placeholder = Paragraph::new("Database management mode\n\nFeatures coming soon:\n- Table browser\n- Query executor\n- Schema viewer")
        .block(Block::default().borders(Borders::ALL).title("Database Management"));
    f.render_widget(placeholder, area);
}

fn render_files_mode(f: &mut ratatui::Frame, area: ratatui::layout::Rect, _path: &std::path::Path) {
    let placeholder = Paragraph::new("File browser mode\n\nFeatures coming soon:\n- Directory navigation\n- File operations\n- Quick file viewer")
        .block(Block::default().borders(Borders::ALL).title("File Browser"));
    f.render_widget(placeholder, area);
}

fn render_help_mode(f: &mut ratatui::Frame, area: ratatui::layout::Rect, _scroll: u16) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "S.A.M. TUI Help",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )]),
        Line::from("  F1 - Command Mode (default)"),
        Line::from("  F2 - Services Management"),
        Line::from("  F3 - System Logs (with scrolling & filtering)"),
        Line::from("  F4 - System Information"),
        Line::from("  F5 - Database Management"),
        Line::from("  F6 - File Browser"),
        Line::from("  F7 - This Help Screen"),
        Line::from("  F8 - AI Coding Agent (Interactive)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Commands:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )]),
        Line::from("  Ctrl+C - Exit application"),
        Line::from("  Page Up/Down - Scroll output"),
        Line::from("  Up/Down - Navigate lists"),
        Line::from("  Enter - Execute command/action"),
        Line::from("  Tab - Auto-complete (in command mode)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Service Management:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )]),
        Line::from("  Space - Start/Stop service"),
        Line::from("  R - Restart service"),
        Line::from("  L - View service logs"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Log Viewer (F3):",
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )]),
        Line::from("  Up/Down - Scroll logs"),
        Line::from("  Page Up/Down - Fast scroll"),
        Line::from("  / - Enter filter mode"),
        Line::from("  c - Clear filter"),
        Line::from("  ESC - Exit filter mode"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tips:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Green),
        )]),
        Line::from("  - Use arrow keys to navigate"),
        Line::from("  - Status colors: Green=running, Red=stopped, Gray=unknown"),
        Line::from("  - System metrics update every 2 seconds"),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help & Documentation"),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(help_widget, area);
}

/// Get spinner character for animation
fn get_spinner_char() -> char {
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let index = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() / 100) as usize % spinner_chars.len();
    spinner_chars[index]
}

/// Render the coding agent mode
fn render_coding_agent_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
    show_cursor: bool,
    output_lines: &[String],
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::{List, ListItem, Paragraph},
    };

    // Simpler layout - just status, output, and input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Status line
            Constraint::Min(10),   // Main output area
            Constraint::Length(3), // Input box
        ])
        .split(area);

    // Simplified status line
    let spinner_char = if !state.coding_agent_spinner_text.is_empty() {
        format!("{} ", get_spinner_char())
    } else {
        String::new()
    };

    let status_text = if !state.coding_agent_spinner_text.is_empty() {
        format!("{}{}", spinner_char, state.coding_agent_spinner_text)
    } else {
        format!("Ready | Model: gpt-oss | pwd: {}",
            std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| "?".to_string()))
    };

    let status_widget = Paragraph::new(status_text)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
        .block(Block::default().borders(Borders::NONE));

    // Main output area - unified view
    let content_lines: Vec<Line> = if !state.coding_agent_execution_log.is_empty() {
        state.coding_agent_execution_log
            .iter()
            .map(|l| {
                if l.starts_with("✅") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::Green)))
                } else if l.starts_with("❌") || l.contains("Error:") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::Red)))
                } else if l.starts_with("⏳") || l.starts_with("🤖") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)))
                } else if l.starts_with("🎯") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)))
                } else if l.starts_with("   Command:") || l.starts_with("   Output:") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)))
                } else if l.starts_with("```") || l.starts_with("DEBUG:") {
                    Line::from(Span::styled(l, ratatui::style::Style::default().fg(ratatui::style::Color::Gray)))
                } else {
                    Line::from(Span::raw(l))
                }
            })
            .collect()
    } else if !state.coding_agent_response.is_empty() {
        vec![Line::from(Span::raw(&state.coding_agent_response))]
    } else {
        vec![Line::from(Span::styled(
            "Type a command or task. I'll help you execute it.",
            ratatui::style::Style::default().fg(ratatui::style::Color::Gray)
        ))]
    };

    let output_widget = Paragraph::new(content_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output"),
        )
        .scroll((state.coding_agent_scroll_offset, 0))
        .wrap(ratatui::widgets::Wrap { trim: false });

    // Input box - simplified
    let cursor_char = if show_cursor && state.coding_agent_input_mode { "_" } else { " " };
    let input_display = format!("{}{}", state.coding_agent_input, cursor_char);

    let input_title = if state.coding_agent_executor.is_some() {
        "Task (Enter to queue message | Ctrl+C to cancel)"
    } else {
        "Task (Enter to execute | 'verify:' prefix for verification | ESC to exit)"
    };

    let input_widget = Paragraph::new(input_display).block(
        Block::default()
            .borders(Borders::ALL)
            .title(input_title),
    );

    f.render_widget(status_widget, chunks[0]);
    f.render_widget(output_widget, chunks[1]);
    f.render_widget(input_widget, chunks[2]);
}

/// Get color style based on status string
fn get_status_color(status: &str) -> ratatui::style::Style {
    match status {
        "running" | "connected" | "online" => {
            ratatui::style::Style::default().fg(ratatui::style::Color::Green)
        }
        "stopped" | "disconnected" | "offline" => {
            ratatui::style::Style::default().fg(ratatui::style::Color::Red)
        }
        "not installed" | "disabled" => {
            ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
        }
        "error" => ratatui::style::Style::default().fg(ratatui::style::Color::Magenta),
        _ => ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
    }
}

/// Run the TUI event loop
///
/// Handles user input, command execution, and UI rendering.
async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Enhanced service status with comprehensive monitoring
    let service_status = Arc::new(Mutex::new(ServiceStatus {
        crawler: "unknown".to_string(),
        redis: "unknown".to_string(),
        docker: "unknown".to_string(),
        sms: "unknown".to_string(),
        postgres: "unknown".to_string(),
        lifx: "unknown".to_string(),
        http_server: "unknown".to_string(),
        ollama: "unknown".to_string(),
        tts: "unknown".to_string(),
        stt: "unknown".to_string(),
        ssh_server: "unknown".to_string(),
        memory_usage: "0 MB".to_string(),
        cpu_usage: "0%".to_string(),
        disk_usage: "0%".to_string(),
        update_count: 0,
    }));

    // Initialize TUI state
    let tui_state = Arc::new(Mutex::new(TuiState {
        mode: TuiMode::Command,
        selected_service: 0,
        log_filter_level: "INFO".to_string(),
        log_scroll_offset: 0,
        log_filter_text: String::new(),
        log_input_mode: false,
        help_scroll: 0,
        file_browser_path: std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from(".")),
        db_table_list: Vec::new(),
        selected_table: 0,
        // Initialize coding agent state
        coding_agent_input: String::new(),
        coding_agent_input_mode: true, // Start in input mode
        coding_agent_response: String::new(),
        coding_agent_model: "gpt-oss".to_string(),
        coding_agent_pending_commands: Vec::new(),
        coding_agent_selected_command: 0,
        coding_agent_scroll_offset: 0,
        coding_agent_executor: None,
        coding_agent_execution_log: Vec::new(),
        coding_agent_spinner_text: String::new(),
    }));

    let service_status_clone = service_status.clone();
    tokio::spawn(async move {
        let mut count = 0u64;
        let mut sys = sysinfo::System::new_all();

        loop {
            // Update system information with proper refresh cycle
            sys.refresh_cpu_all();
            sys.refresh_memory();
            sys.refresh_all();

            // Wait for CPU usage to stabilize
            tokio::time::sleep(Duration::from_millis(200)).await;
            sys.refresh_cpu_all();

            // Service statuses with better error handling
            let crawler = match std::panic::catch_unwind(|| {
                crate::services::crawler::service_status()
            }) {
                Ok(status) => {
                    log::debug!("Crawler service status: {}", status);
                    status.to_string()
                }
                Err(e) => {
                    log::error!("Failed to get crawler status: {:?}", e);
                    "error".to_string()
                }
            };

            let redis = match tokio::time::timeout(
                Duration::from_millis(500),
                crate::services::redis::status()
            ).await {
                Ok(status) => status.to_string(),
                Err(_) => {
                    log::debug!("Redis status check timed out");
                    "timeout".to_string()
                }
            };

            // Docker status - check if not on CapRover
            let docker = if std::env::var("CAPROVER").is_ok() {
                "disabled (CapRover)".to_string()
            } else {
                match tokio::time::timeout(
                    Duration::from_millis(1000),
                    crate::services::docker::is_running_async()
                ).await {
                    Ok(Ok(true)) => "running".to_string(),
                    Ok(Ok(false)) => "stopped".to_string(),
                    Ok(Err(_)) => "error".to_string(),
                    Err(_) => "timeout".to_string(),
                }
            };

            let sms = match std::panic::catch_unwind(|| {
                crate::services::sms::status()
            }) {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string()
            };

            // PostgreSQL status - quick connection test
            let postgres = match tokio::time::timeout(
                Duration::from_millis(500),
                crate::services::pg::health_check()
            ).await {
                Ok(Ok(_)) => "connected".to_string(),
                Ok(Err(_)) => "disconnected".to_string(),
                Err(_) => "timeout".to_string(),
            };

            // LIFX service status - check if service is available
            let lifx = match std::panic::catch_unwind(|| {
                // For now, assume unknown until we implement proper status check
                "unknown"
            }) {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            // TTS service status - check if TTS is available
            let tts = match std::panic::catch_unwind(|| {
                // For now, assume unknown until we implement proper status check
                "unknown"
            }) {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            // STT service status - check if STT is available
            let stt = match std::panic::catch_unwind(|| {
                // For now, assume unknown until we implement proper status check
                "unknown"
            }) {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            // Ollama service with timeout
            let ollama = match tokio::time::timeout(
                Duration::from_millis(1000),
                async {
                    let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
                    if service.is_installed().await {
                        if service.is_running().await {
                            "running"
                        } else {
                            "stopped"
                        }
                    } else {
                        "not installed"
                    }
                }
            ).await {
                Ok(status) => status.to_string(),
                Err(_) => "timeout".to_string(),
            };

            // Check HTTP server (assume running if we got here)
            let http_server = "running".to_string();

            // Check SSH server
            let ssh_server = if crate::services::ssh::server::is_ssh_server_running().await {
                "running"
            } else {
                "stopped"
            }.to_string();

            // System metrics with better error handling
            let memory_usage = {
                let total = sys.total_memory();
                let used = sys.used_memory();
                if total > 0 {
                    let total_mb = total as f64 / 1024.0 / 1024.0;
                    let used_mb = used as f64 / 1024.0 / 1024.0;
                    let percent = (used as f64 / total as f64) * 100.0;
                    format!("{:.0}/{:.0} MB ({:.1}%)", used_mb, total_mb, percent)
                } else {
                    "N/A".to_string()
                }
            };

            let cpu_usage = {
                let cpu_percent = sys.global_cpu_usage();
                if cpu_percent.is_finite() && cpu_percent >= 0.0 {
                    format!("{:.1}%", cpu_percent)
                } else {
                    "N/A".to_string()
                }
            };

            let disk_usage = {
                let disks = sysinfo::Disks::new_with_refreshed_list();
                let mut total_space = 0u64;
                let mut available_space = 0u64;

                for disk in disks.list() {
                    total_space += disk.total_space();
                    available_space += disk.available_space();
                }

                if total_space > 0 {
                    let used_space = total_space - available_space;
                    let usage_percent = (used_space as f64 / total_space as f64) * 100.0;
                    format!(
                        "{:.1}% ({:.1}/{:.1} GB)",
                        usage_percent,
                        used_space as f64 / (1024.0 * 1024.0 * 1024.0),
                        total_space as f64 / (1024.0 * 1024.0 * 1024.0)
                    )
                } else {
                    "N/A".to_string()
                }
            };

            // Use lock with timeout to avoid deadlocks
            match tokio::time::timeout(
                std::time::Duration::from_millis(100),
                service_status_clone.lock()
            ).await {
                Ok(mut status) => {
                    status.crawler = crawler;
                    status.redis = redis;
                    status.docker = docker;
                    status.sms = sms;
                    status.postgres = postgres;
                    status.lifx = lifx;
                    status.http_server = http_server;
                    status.ollama = ollama;
                    status.tts = tts;
                    status.stt = stt;
                    status.ssh_server = ssh_server;
                    status.memory_usage = memory_usage;
                    status.cpu_usage = cpu_usage;
                    status.disk_usage = disk_usage;
                    status.update_count = count;
                    count += 1;
                }
                Err(_) => {
                    log::debug!("Service status update timed out, will retry");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // Enhanced panic hook with better terminal restoration
    std::panic::set_hook(Box::new(|info| {
        log::error!("\nSAM TUI PANIC: {info}");
        restore_terminal_state();
        eprintln!("\rTUI encountered an error and has been reset. {}", info);
    }));

    // Install signal handlers for proper terminal restoration
    #[cfg(unix)]
    {
        unsafe {
            libc::signal(libc::SIGTSTP, handle_suspend as libc::sighandler_t);
            libc::signal(libc::SIGCONT, handle_continue as libc::sighandler_t);
            libc::signal(libc::SIGWINCH, handle_resize as libc::sighandler_t);
        }
    }

    let backend = CrosstermBackend::new(io::stdout());
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide)?;
    TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);

    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Enhanced terminal restoration guard
    struct TerminalRestoreGuard;
    impl Drop for TerminalRestoreGuard {
        fn drop(&mut self) {
            restore_terminal_state();
        }
    }
    let _guard = TerminalRestoreGuard;

    let mut input = String::new();
    let output_lines = Arc::new(Mutex::new(vec![
        "Welcome to the SAM Command Interface!".to_string(),
        "Type 'help' to see available commands.".to_string(),
        "Press Ctrl+C or type 'exit' to quit.".to_string(),
    ]));

    // --- Add: Redirect stdout/stderr to output_lines ---
    let (tx, rx) = mpsc::channel::<String>();
    {
        let tx_out = tx.clone();
        let tx_err = tx.clone();

        // Redirect stdout
        let _stdout_writer = PipeWriter { sender: tx_out };
        // let _ = std::io::set_print(Some(Box::new(stdout_writer)));

        // Redirect stderr
        let _stderr_writer = PipeWriter { sender: tx_err };
        // let _ = std::io::set_panic(Some(Box::new(stderr_writer)));
    }
    // Spawn a thread to forward lines from rx to output_lines
    {
        let output_lines_clone = output_lines.clone();
        tokio::spawn(async move {
            while let Ok(line) = rx.recv() {
                let output_lines = output_lines_clone.clone();
                let line = line.clone();
                // Use tokio runtime if available, otherwise block
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    rt.spawn(async move {
                        helpers::append_line(&output_lines, line).await;
                    });
                } else {
                    futures::executor::block_on(helpers::append_line(&output_lines, line));
                }
            }
        });
    }
    // --- End Add ---

    let human_name = helpers::get_human_name();
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut scroll_offset: u16 = 0;
    let mut output_height: usize = 10;

    // Blinking cursor state
    let mut show_cursor = true;
    let mut cursor_tick: u8 = 0;
    let mut last_known_tui_state = TuiState::default();

    loop {
        // Fetch service status and TUI state for display with timeout to prevent deadlocks
        let (status, current_tui_state) = match tokio::time::timeout(
            std::time::Duration::from_millis(50),
            async {
                let guard = service_status.lock().await;
                let state_guard = tui_state.lock().await;
                (guard.clone(), state_guard.clone())
            }
        ).await {
            Ok(result) => {
                last_known_tui_state = result.1.clone();
                result
            },
            Err(_) => {
                log::debug!("Timeout fetching status for display, using defaults");
                // Use default/previous values if timeout occurs
                (ServiceStatus::default(), last_known_tui_state.clone())
            }
        };

        // FIX: Acquire output_lines lock asynchronously and clone before draw
        let output_lines_snapshot = {
            let lines = output_lines.lock().await;
            lines.clone()
        };

        let draw_result = catch_unwind(AssertUnwindSafe(|| {
            let mut local_output_height = output_height;
            let input_ref = &input;
            let status = status.clone();
            let output_lines_guard = &output_lines_snapshot;

            terminal.draw(|f| {
                let tui_state_local = &current_tui_state;
                let size = f.area();

                // Create navigation bar at top
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3), // Navigation bar
                        Constraint::Min(0),    // Main content
                    ])
                    .split(size);

                // Navigation bar
                let nav_items = vec![
                    ("F1", "Command", tui_state_local.mode == TuiMode::Command),
                    ("F2", "Services", tui_state_local.mode == TuiMode::Services),
                    ("F3", "Logs", tui_state_local.mode == TuiMode::Logs),
                    ("F4", "System", tui_state_local.mode == TuiMode::SystemInfo),
                    ("F5", "Database", tui_state_local.mode == TuiMode::Database),
                    ("F6", "Files", tui_state_local.mode == TuiMode::Files),
                    ("F7", "Help", tui_state_local.mode == TuiMode::Help),
                    ("F8", "AI Code", tui_state_local.mode == TuiMode::CodingAgent),
                ];

                let nav_line = Line::from(
                    nav_items
                        .iter()
                        .flat_map(|(key, name, active)| {
                            let style = if *active {
                                ratatui::style::Style::default()
                                    .fg(ratatui::style::Color::Black)
                                    .bg(ratatui::style::Color::Yellow)
                            } else {
                                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                            };
                            vec![
                                Span::styled(*key, style),
                                Span::styled(format!(" {} ", name), style),
                                Span::raw(" "),
                            ]
                        })
                        .collect::<Vec<_>>(),
                );

                let nav_widget = Paragraph::new(nav_line)
                    .block(Block::default().borders(Borders::ALL).title("Navigation"));
                f.render_widget(nav_widget, main_chunks[0]);

                // Render content based on current mode
                match tui_state_local.mode {
                    TuiMode::Command => render_command_mode(
                        f,
                        main_chunks[1],
                        &status,
                        input_ref,
                        show_cursor,
                        output_lines_guard,
                        scroll_offset,
                        &mut local_output_height,
                    ),
                    TuiMode::Services => render_services_mode(
                        f,
                        main_chunks[1],
                        &status,
                        tui_state_local.selected_service,
                    ),
                    TuiMode::Logs => {
                        render_logs_mode(f, main_chunks[1], tui_state_local, show_cursor)
                    }
                    TuiMode::SystemInfo => render_system_info_mode(f, main_chunks[1], &status),
                    TuiMode::Database => render_database_mode(
                        f,
                        main_chunks[1],
                        &tui_state_local.db_table_list,
                        tui_state_local.selected_table,
                    ),
                    TuiMode::Files => {
                        render_files_mode(f, main_chunks[1], &tui_state_local.file_browser_path)
                    }
                    TuiMode::Help => {
                        render_help_mode(f, main_chunks[1], tui_state_local.help_scroll)
                    }
                    TuiMode::CodingAgent => {
                        render_coding_agent_mode(f, main_chunks[1], &tui_state_local, show_cursor, output_lines_guard)
                    }
                }
            })?;
            output_height = local_output_height;
            Ok::<(), std::io::Error>(())
        }));

        if let Err(e) = draw_result {
            log::error!("TUI draw error (recovering): {:?}", e);
            // Try to recover from terminal corruption
            if let Err(refresh_err) = force_terminal_refresh(&mut terminal) {
                log::error!("Failed to refresh corrupted terminal: {:?}", refresh_err);
            }
            // Add a small delay to avoid rapid error loops
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        cursor_tick = cursor_tick.wrapping_add(1);
        if cursor_tick >= 5 {
            show_cursor = !show_cursor;
            cursor_tick = 0;
        }

        let poll_result = catch_unwind(AssertUnwindSafe(|| {
            event::poll(std::time::Duration::from_millis(100))
        }));

        if let Err(e) = poll_result {
            let mut lines = output_lines.lock().await;
            lines.push(format!("TUI poll panic: {e:?}"));
            log::error!("TUI poll panic: {:?}", e);
            break;
        }

        if let Ok(Ok(true)) = poll_result {
            let read_result = catch_unwind(AssertUnwindSafe(event::read));
            if let Err(e) = read_result {
                let mut lines = futures::executor::block_on(output_lines.lock());
                lines.push(format!("TUI read panic: {e:?}"));
                log::error!("TUI read panic: {:?}", e);
                break;
            }
            if let Ok(Ok(Event::Key(key))) = read_result {
                // On Windows, only handle key presses (not releases)
                #[cfg(windows)]
                {
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                }
                match key.code {
                    KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break
                    }
                    // Function keys for mode switching
                    KeyCode::F(1) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Command;
                    }
                    KeyCode::F(2) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Services;
                    }
                    KeyCode::F(3) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Logs;
                    }
                    KeyCode::F(4) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::SystemInfo;
                    }
                    KeyCode::F(5) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Database;
                    }
                    KeyCode::F(6) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Files;
                    }
                    KeyCode::F(7) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::Help;
                    }
                    KeyCode::F(8) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::CodingAgent;
                    }

                    // Handle different modes
                    _ => {
                        let current_mode = {
                            let state = tui_state.lock().await;
                            state.mode.clone()
                        };

                        match current_mode {
                            TuiMode::Command => match key.code {
                                KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                                KeyCode::PageDown => {
                                    scroll_offset = scroll_offset.saturating_add(5)
                                }
                                KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                                KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                                KeyCode::Enter => {
                                    let cmd = input.trim().to_string();
                                    if cmd == "exit" || cmd == "quit" {
                                        break;
                                    }
                                    if !cmd.is_empty() {
                                        helpers::append_line(
                                            &output_lines,
                                            format!("┌─[{human_name}]─> {cmd}"),
                                        )
                                        .await;
                                        commands::handle_command(
                                            &cmd,
                                            &output_lines,
                                            &mut current_dir,
                                            &human_name,
                                            output_height,
                                            &mut scroll_offset,
                                        )
                                        .await;
                                        
                                        // Check if TUI restart is needed (e.g., after SSH session)
                                        {
                                            let mut lines = output_lines.lock().await;
                                            if lines.iter().any(|line| line.contains("__TUI_RESTART_NEEDED__")) {
                                                // Remove the marker
                                                lines.retain(|line| !line.contains("__TUI_RESTART_NEEDED__"));
                                                drop(lines); // Release lock before terminal operations
                                                
                                                // Force complete TUI reinitialization
                                                restore_terminal_state();
                                                std::thread::sleep(std::time::Duration::from_millis(100));

                                                // Reinitialize the terminal
                                                let _ = enable_raw_mode();
                                                let _ = execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide);
                                                TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);
                                                let _ = terminal.clear();
                                            }
                                        }
                                    }
                                    input.clear();
                                }
                                KeyCode::Char(c) => input.push(c),
                                KeyCode::Backspace => {
                                    input.pop();
                                }
                                _ => {}
                            },

                            TuiMode::Services => {
                                match key.code {
                                    KeyCode::Up => {
                                        let mut state = tui_state.lock().await;
                                        if state.selected_service > 0 {
                                            state.selected_service -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        let mut state = tui_state.lock().await;
                                        if state.selected_service < 10 {
                                            // 11 services (0-10)
                                            state.selected_service += 1;
                                        }
                                    }
                                    KeyCode::Char(' ') => {
                                        // TODO: Start/Stop service
                                        helpers::append_line(
                                            &output_lines,
                                            "Service start/stop functionality coming soon"
                                                .to_string(),
                                        )
                                        .await;
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        // TODO: Restart service
                                        helpers::append_line(
                                            &output_lines,
                                            "Service restart functionality coming soon".to_string(),
                                        )
                                        .await;
                                    }
                                    KeyCode::Enter => {
                                        // TODO: Show service details
                                        helpers::append_line(
                                            &output_lines,
                                            "Service details view coming soon".to_string(),
                                        )
                                        .await;
                                    }
                                    _ => {}
                                }
                            }

                            TuiMode::Logs => {
                                let current_input_mode = {
                                    let state = tui_state.lock().await;
                                    state.log_input_mode
                                };

                                if current_input_mode {
                                    // Handle filter input mode
                                    match key.code {
                                        KeyCode::Esc => {
                                            let mut state = tui_state.lock().await;
                                            state.log_input_mode = false;
                                        }
                                        KeyCode::Enter => {
                                            let mut state = tui_state.lock().await;
                                            state.log_input_mode = false;
                                            // Filter text is already stored in log_filter_text
                                            // The TuiLoggerWidget will need to be enhanced to support filtering
                                        }
                                        KeyCode::Char(c) => {
                                            let mut state = tui_state.lock().await;
                                            state.log_filter_text.push(c);
                                        }
                                        KeyCode::Backspace => {
                                            let mut state = tui_state.lock().await;
                                            state.log_filter_text.pop();
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Handle navigation mode
                                    match key.code {
                                        KeyCode::Up => {
                                            // For now, just track scroll offset since we don't have access to log_widget_state here
                                            let mut state = tui_state.lock().await;
                                            state.log_scroll_offset = state.log_scroll_offset.saturating_sub(1);
                                        }
                                        KeyCode::Down => {
                                            let mut state = tui_state.lock().await;
                                            state.log_scroll_offset = state.log_scroll_offset.saturating_add(1);
                                        }
                                        KeyCode::PageUp => {
                                            let mut state = tui_state.lock().await;
                                            state.log_scroll_offset = state.log_scroll_offset.saturating_sub(10);
                                        }
                                        KeyCode::PageDown => {
                                            let mut state = tui_state.lock().await;
                                            state.log_scroll_offset = state.log_scroll_offset.saturating_add(10);
                                        }
                                        KeyCode::Char('/') => {
                                            let mut state = tui_state.lock().await;
                                            state.log_input_mode = true;
                                        }
                                        KeyCode::Char('c') => {
                                            // Clear filter
                                            let mut state = tui_state.lock().await;
                                            state.log_filter_text.clear();
                                            state.log_scroll_offset = 0;
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            TuiMode::Help => match key.code {
                                KeyCode::Up => {
                                    let mut state = tui_state.lock().await;
                                    state.help_scroll = state.help_scroll.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    let mut state = tui_state.lock().await;
                                    state.help_scroll = state.help_scroll.saturating_add(1);
                                }
                                _ => {}
                            },

                            TuiMode::CodingAgent => {
                                let current_input_mode = {
                                    let state = tui_state.lock().await;
                                    state.coding_agent_input_mode
                                };

                                if current_input_mode {
                                    // Handle input mode
                                    match key.code {
                                        KeyCode::Esc => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_input_mode = false;
                                        }
                                        KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                            // Cancel current execution
                                            let mut state = tui_state.lock().await;
                                            if let Some(executor) = &state.coding_agent_executor {
                                                let executor_clone = executor.clone();
                                                tokio::spawn(async move {
                                                    executor_clone.cancel_execution().await;
                                                });
                                                state.coding_agent_spinner_text = "❌ Execution cancelled".to_string();
                                            }
                                        }
                                        KeyCode::Enter => {
                                            // Check if executor is already running
                                            let is_executing = {
                                                let state = tui_state.lock().await;
                                                if let Some(ref executor) = state.coding_agent_executor {
                                                    executor.is_executing().await
                                                } else {
                                                    false
                                                }
                                            };

                                            // If already executing, queue the message instead
                                            if is_executing {
                                                let message = {
                                                    let mut state = tui_state.lock().await;
                                                    let msg = state.coding_agent_input.clone();
                                                    state.coding_agent_input.clear();
                                                    msg
                                                };

                                                if !message.trim().is_empty() {
                                                    let state = tui_state.lock().await;
                                                    if let Some(ref executor) = state.coding_agent_executor {
                                                        executor.queue_message(message.clone()).await;
                                                        helpers::append_line(
                                                            &output_lines,
                                                            format!("💬 Message queued: {}", message),
                                                        ).await;
                                                    }
                                                }
                                                continue;
                                            }

                                            // Check if it's a complex multi-step task
                                            let (input, current_dir, should_auto_execute) = {
                                                let mut state = tui_state.lock().await;
                                                let input = state.coding_agent_input.clone();
                                                state.coding_agent_input.clear();
                                                state.coding_agent_input_mode = false;

                                                // Detect complex tasks that should use incremental execution
                                                let is_complex_task = input.contains("and") ||
                                                    input.contains("then") ||
                                                    input.to_lowercase().contains("create") ||
                                                    input.to_lowercase().contains("make") ||
                                                    input.to_lowercase().contains("build") ||
                                                    input.to_lowercase().contains("setup");

                                                (input, current_dir.clone(), is_complex_task)
                                            };

                                            if !input.trim().is_empty() {
                                                // Add user input to output_lines for context
                                                helpers::append_line(
                                                    &output_lines,
                                                    format!("🤖 User: {}", input),
                                                ).await;

                                                if should_auto_execute {
                                                    // Use incremental executor for complex tasks
                                                    let coding_agent = Arc::new(crate::services::coding::CodingAgentService::new_with_defaults().await);
                                                    let executor = crate::services::coding::CodingAgentExecutor::new(coding_agent);

                                                    // Check for verification mode (can be triggered with "verify:" prefix)
                                                    let enable_verification = input.starts_with("verify:");
                                                    let actual_input = if enable_verification {
                                                        input.strip_prefix("verify:").unwrap_or(&input).trim().to_string()
                                                    } else {
                                                        input.clone()
                                                    };

                                                    if enable_verification {
                                                        executor.enable_verification(3).await; // Max 3 correction attempts
                                                    }

                                                    // Initialize executor in state
                                                    {
                                                        let mut state = tui_state.lock().await;
                                                        state.coding_agent_executor = Some(executor.clone());
                                                        state.coding_agent_execution_log.clear();
                                                        state.coding_agent_spinner_text = "🤖 Planning task...".to_string();
                                                    }

                                                    // Get session context for the AI
                                                    let session_context = {
                                                        let lines = output_lines.lock().await;
                                                        lines.clone()
                                                    };

                                                    // Start incremental execution in background
                                                    let executor_clone = executor.clone();
                                                    let input_clone = actual_input.clone();
                                                    let current_dir_clone = current_dir.clone();
                                                    let session_context_clone = session_context.clone();
                                                    let tui_state_clone = tui_state.clone();
                                                    let output_lines_clone = output_lines.clone();

                                                    // Set up message channel for interactive feedback
                                                    let message_sender = executor.setup_message_channel().await;
                                                    {
                                                        let mut state = tui_state.lock().await;
                                                        // Store the message sender in TUI state for later use
                                                        // Note: We'll need to add this field to TuiState
                                                    }

                                                    tokio::spawn(async move {
                                                        // Execute with or without verification based on mode
                                                        let result = if enable_verification {
                                                            executor_clone.execute_incremental_task_with_verification(
                                                                &actual_input,
                                                                &current_dir_clone,
                                                                &session_context_clone,
                                                                true, // auto_execute
                                                            ).await
                                                        } else {
                                                            executor_clone.execute_incremental_task(
                                                                &actual_input,
                                                                &current_dir_clone,
                                                                &session_context_clone,
                                                                true, // auto_execute
                                                            ).await
                                                        };

                                                        if let Err(e) = result {
                                                            let mut state = tui_state_clone.lock().await;
                                                            state.coding_agent_response = format!("Execution failed: {}", e);
                                                            state.coding_agent_spinner_text.clear();
                                                        }

                                                        // Update UI periodically during execution
                                                        loop {
                                                            if !executor_clone.is_executing().await {
                                                                break;
                                                            }

                                                            let spinner_text = executor_clone.get_spinner_text().await;
                                                            let execution_log = executor_clone.get_execution_log().await;
                                                            let interactive_status = executor_clone.get_interactive_status().await;

                                                            {
                                                                let mut state = tui_state_clone.lock().await;
                                                                state.coding_agent_spinner_text = format!("{} | {}", spinner_text, interactive_status);
                                                                state.coding_agent_execution_log = execution_log;
                                                            }

                                                            // Process any queued messages
                                                            let queued_messages = executor_clone.process_queued_messages().await;
                                                            for msg in queued_messages {
                                                                helpers::append_line(
                                                                    &output_lines_clone,
                                                                    format!("💬 Queued feedback: {}", msg),
                                                                ).await;
                                                            }

                                                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                                        }

                                                        // Final update when done
                                                        let execution_log = executor_clone.get_execution_log().await;
                                                        {
                                                            let mut state = tui_state_clone.lock().await;
                                                            state.coding_agent_spinner_text.clear();
                                                            state.coding_agent_execution_log = execution_log.clone();
                                                        }

                                                        // Add final results to output_lines
                                                        for log_line in execution_log {
                                                            if log_line.starts_with("✅") || log_line.starts_with("❌") {
                                                                helpers::append_line(
                                                                    &output_lines_clone,
                                                                    format!("🤖 {}", log_line),
                                                                ).await;
                                                            }
                                                        }
                                                    });

                                                } else {
                                                    // Use traditional single-response mode for simple queries
                                                    let coding_agent = crate::services::coding::CodingAgentService::new_with_defaults().await;

                                                    // Get session context for the AI
                                                    let session_context = {
                                                        let lines = output_lines.lock().await;
                                                        lines.clone()
                                                    };

                                                    match coding_agent.generate_response(
                                                        &input,
                                                        &current_dir,
                                                        &session_context,
                                                        None,
                                                    ).await {
                                                        Ok(response) => {
                                                            let mut state = tui_state.lock().await;
                                                            state.coding_agent_response = response.response_text.clone();
                                                            state.coding_agent_pending_commands = response.suggested_commands;
                                                            state.coding_agent_selected_command = 0;
                                                            state.coding_agent_execution_log.clear(); // Clear log for normal responses

                                                            // Add AI response to output_lines for context
                                                            helpers::append_line(
                                                                &output_lines,
                                                                format!("🤖 AI ({}): {}", response.model_used, response.response_text),
                                                            ).await;
                                                        }
                                                        Err(e) => {
                                                            let mut state = tui_state.lock().await;
                                                            state.coding_agent_response = format!("Error: {}", e);
                                                            state.coding_agent_pending_commands.clear();
                                                            state.coding_agent_execution_log.clear();

                                                            helpers::append_line(
                                                                &output_lines,
                                                                format!("🤖 Error: {}", e),
                                                            ).await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        KeyCode::Char(c) => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_input.push(c);
                                        }
                                        KeyCode::Backspace => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_input.pop();
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Handle command navigation mode
                                    match key.code {
                                        KeyCode::Enter => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_input_mode = true;
                                        }
                                        KeyCode::Up => {
                                            let mut state = tui_state.lock().await;
                                            if !state.coding_agent_pending_commands.is_empty() && state.coding_agent_selected_command > 0 {
                                                state.coding_agent_selected_command -= 1;
                                            }
                                            // Also scroll response
                                            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_sub(1);
                                        }
                                        KeyCode::Down => {
                                            let mut state = tui_state.lock().await;
                                            if state.coding_agent_selected_command < state.coding_agent_pending_commands.len().saturating_sub(1) {
                                                state.coding_agent_selected_command += 1;
                                            }
                                            // Also scroll response
                                            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_add(1);
                                        }
                                        KeyCode::PageUp => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_sub(5);
                                        }
                                        KeyCode::PageDown => {
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_add(5);
                                        }
                                        KeyCode::Char(' ') => {
                                            // Execute selected command
                                            let (selected_cmd, require_confirmation) = {
                                                let state = tui_state.lock().await;
                                                if state.coding_agent_selected_command < state.coding_agent_pending_commands.len() {
                                                    let cmd = &state.coding_agent_pending_commands[state.coding_agent_selected_command];
                                                    (Some(cmd.command.clone()), cmd.require_confirmation)
                                                } else {
                                                    (None, false)
                                                }
                                            };

                                            if let Some(command) = selected_cmd {
                                                // For now, just execute safe commands directly
                                                // TODO: Add confirmation dialog for unsafe commands
                                                helpers::append_line(
                                                    &output_lines,
                                                    format!("🤖 Executing: {}", command),
                                                ).await;

                                                let coding_agent = crate::services::coding::CodingAgentService::new_with_defaults().await;
                                                match coding_agent.execute_command(&command, require_confirmation).await {
                                                    Ok(result) => {
                                                        helpers::append_line(
                                                            &output_lines,
                                                            format!("🤖 Result:\n{}", result),
                                                        ).await;
                                                    }
                                                    Err(e) => {
                                                        helpers::append_line(
                                                            &output_lines,
                                                            format!("🤖 Command failed: {}", e),
                                                        ).await;
                                                    }
                                                }
                                            }
                                        }
                                        KeyCode::Char('c') => {
                                            // Clear commands
                                            let mut state = tui_state.lock().await;
                                            state.coding_agent_pending_commands.clear();
                                            state.coding_agent_selected_command = 0;
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            // Other modes - basic navigation for now
                            _ => {
                                match key.code {
                                    KeyCode::Up
                                    | KeyCode::Down
                                    | KeyCode::PageUp
                                    | KeyCode::PageDown => {
                                        // Handle navigation for other modes
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // DropGuard will restore terminal state here
    Ok(())
}

/// Take over the terminal for an interactive SSH session by using system exec
/// This function completely exits TUI mode and runs SSH natively
/// Returns true if TUI needs to be restarted (always true after SSH)
#[cfg(unix)]
pub fn tui_takeover_ssh_session(ssh_command: &str) -> bool {
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    use std::io::{self, Write};

    // Exit TUI mode completely using our restoration system
    restore_terminal_state();
    
    println!("\r[Starting SSH session: {}]", ssh_command);
    println!("[The TUI will return when SSH session ends]");
    io::stdout().flush().unwrap();
    
    // Run SSH using system call - this gives us real TTY behavior
    let exit_status = std::process::Command::new("sh")
        .arg("-c")
        .arg(ssh_command)
        .status();

    // Show result
    match exit_status {
        Ok(status) if status.success() => {
            println!("\r[SSH session completed successfully]");
        }
        Ok(status) => {
            println!("\r[SSH session ended with exit code: {:?}]", status.code());
        }
        Err(e) => {
            println!("\r[SSH session failed: {}]", e);
        }
    }
    
    println!("[Press Enter to return to TUI...]");
    io::stdout().flush().unwrap();
    
    // Wait for user to press Enter before returning to TUI
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    
    // Don't try to restore here - let the main loop handle it
    // Return true to signal TUI needs complete reinitialization
    true
}
