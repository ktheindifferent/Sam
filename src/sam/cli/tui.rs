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
use tokio::sync::Mutex;

use ratatui::widgets::{Block, Borders, Paragraph};
// use std::io::BufRead;
// use std::io::Read;
use std::sync::mpsc::{self, Sender};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};
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
    memory_usage: String,
    cpu_usage: String,
    disk_usage: String,
    update_count: u64,
}

// Navigation state for TUI
#[derive(Debug, Clone, PartialEq)]
enum TuiMode {
    Command,    // Default command input mode
    Services,   // Service management view
    Logs,       // Log viewer mode
    SystemInfo, // System information view
    Database,   // Database management view
    Files,      // File browser mode
    Help,       // Help screen
}

#[derive(Debug, Default, Clone)]
struct TuiState {
    mode: TuiMode,
    selected_service: usize,
    log_filter_level: String,
    help_scroll: u16,
    file_browser_path: std::path::PathBuf,
    db_table_list: Vec<String>,
    selected_table: usize,
}

impl Default for TuiMode {
    fn default() -> Self {
        TuiMode::Command
    }
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

    // Only set log file if /opt/sam exists
    let log_dir = std::path::Path::new("/opt/sam");
    if log_dir.exists() && log_dir.is_dir() {
        let log_file_path = log_dir.join("output.log");
        let file_options = tui_logger::TuiLoggerFile::new(log_file_path.to_str().unwrap())
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_file(true)
            .output_separator(':');
        tui_logger::set_log_file(file_options);
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
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // status block (made larger)
            Constraint::Min(3),    // output
            Constraint::Length(3), // input
        ])
        .split(chunks[0]);

    *output_height = left_chunks[1].height.max(1) as usize;

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

    let tui_logger_widget = TuiLoggerWidget::default()
        .block(Block::default().borders(Borders::ALL).title("System Logs"))
        .output_separator('|')
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_timestamp(Some("%H:%M:%S".to_string()));

    f.render_widget(status_widget, left_chunks[0]);
    f.render_widget(output_widget, left_chunks[1]);
    f.render_widget(input_widget, left_chunks[2]);
    f.render_widget(tui_logger_widget, chunks[1]);
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
    let services = vec![
        ("Crawler", &status.crawler),
        ("Redis", &status.redis),
        ("Docker", &status.docker),
        ("SMS", &status.sms),
        ("PostgreSQL", &status.postgres),
        ("LIFX", &status.lifx),
        ("HTTP Server", &status.http_server),
    ];

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
    ];

    let service_widget = Paragraph::new(service_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Service Status"),
    );

    f.render_widget(system_widget, chunks[0]);
    f.render_widget(service_widget, chunks[1]);
}

/// Render other modes (placeholder implementations)
fn render_logs_mode(f: &mut ratatui::Frame, area: ratatui::layout::Rect, _filter_level: &str) {
    let tui_logger_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("System Logs - Full View"),
        )
        .output_separator('|')
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_timestamp(Some("%H:%M:%S".to_string()));

    f.render_widget(tui_logger_widget, area);
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
        Line::from("  F3 - System Logs"),
        Line::from("  F4 - System Information"),
        Line::from("  F5 - Database Management"),
        Line::from("  F6 - File Browser"),
        Line::from("  F7 - This Help Screen"),
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
        help_scroll: 0,
        file_browser_path: std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from(".")),
        db_table_list: Vec::new(),
        selected_table: 0,
    }));

    let service_status_clone = service_status.clone();
    tokio::spawn(async move {
        let mut count = 0u64;
        let mut sys = sysinfo::System::new_all();

        loop {
            // Update system information
            sys.refresh_all();

            // Service statuses
            let crawler = std::panic::catch_unwind(|| {
                let status = crate::sam::services::crawler::service_status();
                log::debug!("Crawler service status: {}", status);
                status.to_string()
            })
            .unwrap_or_else(|e| {
                log::error!("Failed to get crawler status: {:?}", e);
                "error".to_string()
            });

            let redis_status_result = crate::sam::services::redis::status().await;
            let redis = std::panic::catch_unwind(|| redis_status_result.to_string())
                .unwrap_or_else(|_| "error".to_string());

            let docker =
                std::panic::catch_unwind(|| crate::sam::services::docker::status().to_string())
                    .unwrap_or_else(|_| "error".to_string());

            let sms = std::panic::catch_unwind(|| crate::sam::services::sms::status().to_string())
                .unwrap_or_else(|_| "error".to_string());

            // Check PostgreSQL connection
            let postgres = match crate::sam::memory::config::Config::client() {
                Ok(_) => "connected".to_string(),
                Err(_) => "disconnected".to_string(),
            };

            // Check LIFX service
            let lifx = "unknown".to_string(); // TODO: Add LIFX status check

            // Check HTTP server (assume running if we got here)
            let http_server = "running".to_string();

            // System metrics
            let memory_usage = {
                let total = sys.total_memory() as f64 / 1024.0 / 1024.0; // MB
                let used = sys.used_memory() as f64 / 1024.0 / 1024.0; // MB
                format!(
                    "{:.0}/{:.0} MB ({:.1}%)",
                    used,
                    total,
                    (used / total) * 100.0
                )
            };

            let cpu_usage = format!("{:.1}%", sys.global_cpu_usage());

            let disk_usage = {
                let mut total_space = 0u64;
                let mut available_space = 0u64;
                let disks = sysinfo::Disks::new_with_refreshed_list();
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
                        used_space as f64 / 1024.0 / 1024.0 / 1024.0,
                        total_space as f64 / 1024.0 / 1024.0 / 1024.0
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

    // Set a panic hook to print panics to stderr
    std::panic::set_hook(Box::new(|info| {
        log::error!("\nSAM TUI PANIC: {info}");
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        // Try to restore terminal state
        let _ = disable_raw_mode();

        // Flush to ensure message is visible
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
    }));

    let backend = CrosstermBackend::new(io::stdout());
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    // Ensure terminal is restored even if panic or error
    struct DropGuard;
    impl Drop for DropGuard {
        fn drop(&mut self) {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();

            let _ = io::stdout().flush();
            let _ = io::stderr().flush();
        }
    }
    let _guard = DropGuard;

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
            let tui_state_local = current_tui_state.clone();
            let output_lines_guard = &output_lines_snapshot;

            terminal.draw(|f| {
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
                        render_logs_mode(f, main_chunks[1], &tui_state_local.log_filter_level)
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
                }
            })?;
            output_height = local_output_height;
            Ok::<(), std::io::Error>(())
        }));

        if let Err(e) = draw_result {
            log::error!("TUI draw error (recovering): {:?}", e);
            // Instead of breaking, try to recover by continuing
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
                                        if state.selected_service < 6 {
                                            // 7 services (0-6)
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

/// Take over the terminal for an interactive SSH session
#[cfg(unix)]
pub fn tui_takeover_ssh_session<In, Out>(mut send_input: In, mut read_output: Out)
where
    In: FnMut(&[u8]) + Send + 'static,
    Out: FnMut() -> Option<Vec<u8>> + Send + 'static,
{
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io::{self, Write};

    // Leave TUI alternate screen and raw mode
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    println!("[SSH session started. Press Ctrl+D or exit to return to TUI.]");

    // Set terminal to raw mode for direct input
    let _ = enable_raw_mode();
    let mut stdout = io::stdout();
    loop {
        // Print any available SSH output
        if let Some(data) = read_output() {
            let _ = stdout.write_all(&data);
            let _ = stdout.flush();
        }
        // Poll for user input
        if event::poll(std::time::Duration::from_millis(30)).unwrap_or(false) {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('d')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        // Ctrl+D: send EOF and break
                        send_input(&[4]);
                        break;
                    }
                    KeyCode::Char(c) => {
                        let mut buf = [0u8; 4];
                        let n = c.encode_utf8(&mut buf).len();
                        send_input(&buf[..n]);
                    }
                    KeyCode::Enter => send_input(&[b'\n']),
                    KeyCode::Tab => send_input(&[b'\t']),
                    KeyCode::Backspace => send_input(&[8]),
                    KeyCode::Esc => send_input(&[27]),
                    _ => {}
                }
            }
        }
        // End if SSH session output is closed
        if read_output().is_none() {
            break;
        }
    }
    // Restore TUI alternate screen and raw mode
    let _ = execute!(io::stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();
    println!("[SSH session ended. Returning to TUI...]");
}
