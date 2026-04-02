pub mod state;
pub mod terminal;
pub mod status_updater;
pub mod events;
pub mod render;

#[cfg(unix)]
pub use terminal::tui_takeover_ssh_session;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;
use tui_logger::TuiLoggerLevelOutput;

use state::{ServiceStatus, TuiMode, TuiState};
use terminal::{TERMINAL_NEEDS_RESTORE, TerminalRestoreGuard};

use super::helpers;

const MAX_HISTORY_LINES: usize = 1000;

/// Load command history from `~/.sam/history`
fn load_command_history() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let history_path = std::path::PathBuf::from(home).join(".sam").join("history");

    match std::fs::read_to_string(&history_path) {
        Ok(content) => {
            let lines: Vec<String> = content
                .lines()
                .map(|l| l.to_string())
                .collect();
            log::debug!("Loaded {} history entries from {:?}", lines.len(), history_path);
            lines
        }
        Err(_) => Vec::new(),
    }
}

/// Save command history to `~/.sam/history` (max MAX_HISTORY_LINES)
fn save_command_history(history: &[String]) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sam_dir = std::path::PathBuf::from(home).join(".sam");

    if let Err(e) = std::fs::create_dir_all(&sam_dir) {
        log::warn!("Failed to create ~/.sam for history: {}", e);
        return;
    }

    let history_path = sam_dir.join("history");
    let start = if history.len() > MAX_HISTORY_LINES {
        history.len() - MAX_HISTORY_LINES
    } else {
        0
    };
    let content = history[start..].join("\n");

    if let Err(e) = std::fs::write(&history_path, content) {
        log::warn!("Failed to write command history: {}", e);
    } else {
        log::debug!("Saved {} history entries to {:?}", history.len().min(MAX_HISTORY_LINES), history_path);
    }
}

/// Starts the interactive command prompt
///
/// Initializes the TUI logger, configures logging, and launches the TUI event loop.
pub async fn start_prompt() {
    log::info!("[sam cli] start_prompt() called");
    helpers::check_postgres_env();
    let _ = tui_logger::init_logger(log::LevelFilter::Debug);
    tui_logger::set_default_level(log::LevelFilter::Debug);

    let log_file_path = resolve_log_file_path();

    if !log_file_path.as_os_str().is_empty() {
        if let Ok(_) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_file_path)
        {
            log::info!("Setting TUI log file to: {:?}", log_file_path);
            let file_options = tui_logger::TuiLoggerFile::new(log_file_path.to_str().unwrap())
                .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
                .output_file(true)
                .output_separator(':');
            tui_logger::set_log_file(file_options);
            log::info!("TUI Logger initialized - file logging active to: {:?}", log_file_path);
        } else {
            log::warn!("Cannot write to log file {:?}. TUI logging will be memory-only.", log_file_path);
        }
    } else {
        log::warn!("No valid log file path available. TUI logging will be memory-only.");
    }

    if let Err(e) = run_tui().await {
        log::info!("TUI error: {:?}", e);
    }
}

/// Resolve the best available log file path
fn resolve_log_file_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sam_dir = std::path::PathBuf::from(home).join(".sam");

    if let Err(e) = std::fs::create_dir_all(&sam_dir) {
        log::warn!("Failed to create ~/.sam directory: {}", e);
    }

    let preferred_path = sam_dir.join("output.log");

    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&preferred_path)
    {
        Ok(_) => {
            log::debug!("Using preferred log file: {:?}", preferred_path);
            preferred_path
        }
        Err(e) => {
            log::warn!("Cannot write to preferred log file {:?}: {}, trying fallbacks", preferred_path, e);
            find_fallback_log_path()
        }
    }
}

fn find_fallback_log_path() -> std::path::PathBuf {
    if let Ok(temp_dir) = std::env::var("TMPDIR") {
        let tmpdir_path = std::path::PathBuf::from(temp_dir).join("sam_output.log");
        if std::fs::OpenOptions::new().create(true).write(true).append(true).open(&tmpdir_path).is_ok() {
            return tmpdir_path;
        }
    }
    if std::path::Path::new("/tmp").exists() {
        return std::path::PathBuf::from("/tmp/sam_output.log");
    }
    let opt_sam = std::path::Path::new("/opt/sam");
    if opt_sam.exists() && opt_sam.is_dir() {
        return opt_sam.join("output.log");
    }
    log::warn!("No suitable directory found for TUI log file, skipping file logging");
    std::path::PathBuf::new()
}

/// Run the TUI event loop
async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Load user config and command history
    let user_config = crate::services::config::SamUserConfig::load();
    let saved_history = load_command_history();

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
        cpu_history: state::RingBuffer::new(60),
        memory_history: state::RingBuffer::new(60),
    }));

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
        coding_agent_input: String::new(),
        coding_agent_input_mode: true,
        coding_agent_response: String::new(),
        coding_agent_model: user_config.default_model(),
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
        command_history: saved_history,
        history_search_mode: false,
        history_search_query: String::new(),
        notifications: Vec::new(),
        command_palette: state::CommandPalette::default(),
        vim_mode: user_config.vim_keybindings(),
    }));

    // Spawn background status updater
    status_updater::spawn_status_updater(service_status.clone());

    // Set up panic hook
    std::panic::set_hook(Box::new(|info| {
        log::error!("\nSAM TUI PANIC: {info}");
        terminal::restore_terminal_state();
        eprintln!("\rTUI encountered an error and has been reset. {}", info);
    }));

    // Install signal handlers
    // SAFETY: These unsafe blocks are necessary for signal handler registration.
    // The libc::signal function requires unsafe as it deals with C function pointers.
    // The function pointers (terminal::handle_suspend, etc.) are static and safe to use.
    #[cfg(unix)]
    {
        unsafe {
            libc::signal(libc::SIGTSTP, terminal::handle_suspend as libc::sighandler_t);
            libc::signal(libc::SIGCONT, terminal::handle_continue as libc::sighandler_t);
            libc::signal(libc::SIGWINCH, terminal::handle_resize as libc::sighandler_t);
        }
    }

    let backend = CrosstermBackend::new(io::stdout());
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide)?;
    TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);

    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let _guard = TerminalRestoreGuard;

    let mut input = String::new();
    let output_lines = Arc::new(Mutex::new(vec![
        "Welcome to the SAM Command Interface!".to_string(),
        "Type 'help' to see available commands.".to_string(),
        "Press Ctrl+C or type 'exit' to quit.".to_string(),
    ]));

    let human_name = helpers::get_human_name();
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut scroll_offset: u16 = 0;
    let mut output_height: usize = 10;

    let mut show_cursor = true;
    let mut cursor_tick: u8 = 0;
    let mut last_known_tui_state = TuiState::default();

    loop {
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
                (ServiceStatus::default(), last_known_tui_state.clone())
            }
        };

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

                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),  // Navigation bar
                        Constraint::Min(0),     // Main content
                        Constraint::Length(3),  // Status bar with sparklines
                    ])
                    .split(size);

                render::nav_bar::render_nav_bar(f, main_chunks[0], &tui_state_local.mode);

                // Status bar at bottom
                render::status_bar::render_status_bar(f, main_chunks[2], &status);

                match tui_state_local.mode {
                    TuiMode::Command => render::command::render_command_mode(
                        f, main_chunks[1], &status, input_ref, show_cursor,
                        output_lines_guard, scroll_offset, &mut local_output_height,
                    ),
                    TuiMode::Services => render::services::render_services_mode(
                        f, main_chunks[1], &status, tui_state_local.selected_service,
                    ),
                    TuiMode::Logs => render::logs::render_logs_mode(
                        f, main_chunks[1], tui_state_local, show_cursor,
                    ),
                    TuiMode::SystemInfo => render::system_info::render_system_info_mode(
                        f, main_chunks[1], &status,
                    ),
                    TuiMode::Database => render::database::render_database_mode(
                        f, main_chunks[1], &tui_state_local.db_table_list, tui_state_local.selected_table,
                    ),
                    TuiMode::Files => render::files::render_files_mode(
                        f, main_chunks[1], &tui_state_local.file_browser_path,
                    ),
                    TuiMode::Help => render::help::render_help_mode(
                        f, main_chunks[1], tui_state_local.help_scroll,
                    ),
                    TuiMode::CodingAgent => render::coding_agent::render_coding_agent_mode(
                        f, main_chunks[1], tui_state_local, show_cursor, output_lines_guard,
                    ),
                }

                // Overlay: notification toasts
                render::toasts::render_toasts(f, size, &tui_state_local.notifications);

                // Overlay: command palette
                render::palette::render_command_palette(f, size, &tui_state_local.command_palette);
            })?;
            output_height = local_output_height;
            Ok::<(), std::io::Error>(())
        }));

        if let Err(e) = draw_result {
            log::error!("TUI draw error (recovering): {:?}", e);
            if let Err(refresh_err) = terminal::force_terminal_refresh(&mut terminal) {
                log::error!("Failed to refresh corrupted terminal: {:?}", refresh_err);
            }
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
                #[cfg(windows)]
                {
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                }
                // Expire old notifications
                {
                    let mut state = tui_state.lock().await;
                    state.notifications.retain(|n| n.created_at.elapsed() < n.duration);
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        break
                    }
                    // Command palette toggle
                    KeyCode::Char('p') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        let mut state = tui_state.lock().await;
                        state.command_palette.visible = !state.command_palette.visible;
                        if state.command_palette.visible {
                            state.command_palette.query.clear();
                            state.command_palette.selected = 0;
                            state.command_palette.actions = vec![
                                state::PaletteAction { label: "Command".into(), description: "Switch to command mode (F1)".into(), mode: Some(TuiMode::Command) },
                                state::PaletteAction { label: "Services".into(), description: "Service management (F2)".into(), mode: Some(TuiMode::Services) },
                                state::PaletteAction { label: "Logs".into(), description: "View system logs (F3)".into(), mode: Some(TuiMode::Logs) },
                                state::PaletteAction { label: "System".into(), description: "System information (F4)".into(), mode: Some(TuiMode::SystemInfo) },
                                state::PaletteAction { label: "Database".into(), description: "Database management (F5)".into(), mode: Some(TuiMode::Database) },
                                state::PaletteAction { label: "Files".into(), description: "File browser (F6)".into(), mode: Some(TuiMode::Files) },
                                state::PaletteAction { label: "Help".into(), description: "Help screen (F7)".into(), mode: Some(TuiMode::Help) },
                                state::PaletteAction { label: "AI Code".into(), description: "AI coding agent (F8)".into(), mode: Some(TuiMode::CodingAgent) },
                            ];
                        }
                    }
                    // Function keys for mode switching
                    KeyCode::F(1) => { tui_state.lock().await.mode = TuiMode::Command; }
                    KeyCode::F(2) => { tui_state.lock().await.mode = TuiMode::Services; }
                    KeyCode::F(3) => { tui_state.lock().await.mode = TuiMode::Logs; }
                    KeyCode::F(4) => { tui_state.lock().await.mode = TuiMode::SystemInfo; }
                    KeyCode::F(5) => { tui_state.lock().await.mode = TuiMode::Database; }
                    KeyCode::F(6) => { tui_state.lock().await.mode = TuiMode::Files; }
                    KeyCode::F(7) => { tui_state.lock().await.mode = TuiMode::Help; }
                    KeyCode::F(8) => {
                        let mut state = tui_state.lock().await;
                        state.mode = TuiMode::CodingAgent;
                        state.coding_agent_working_dir = current_dir.display().to_string();
                    }

                    _ => {
                        // Handle command palette events first
                        let palette_visible = {
                            let state = tui_state.lock().await;
                            state.command_palette.visible
                        };
                        if palette_visible {
                            match key.code {
                                KeyCode::Esc => { tui_state.lock().await.command_palette.visible = false; }
                                KeyCode::Enter => {
                                    let mut state = tui_state.lock().await;
                                    let query_lower = state.command_palette.query.to_lowercase();
                                    let filtered: Vec<state::PaletteAction> = state.command_palette.actions
                                        .iter()
                                        .filter(|a| query_lower.is_empty() || a.label.to_lowercase().contains(&query_lower) || a.description.to_lowercase().contains(&query_lower))
                                        .cloned()
                                        .collect();
                                    if let Some(action) = filtered.get(state.command_palette.selected) {
                                        if let Some(mode) = &action.mode {
                                            state.mode = mode.clone();
                                        }
                                    }
                                    state.command_palette.visible = false;
                                }
                                KeyCode::Up => {
                                    let mut state = tui_state.lock().await;
                                    state.command_palette.selected = state.command_palette.selected.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    let mut state = tui_state.lock().await;
                                    state.command_palette.selected = state.command_palette.selected.saturating_add(1);
                                }
                                KeyCode::Char(c) => { tui_state.lock().await.command_palette.query.push(c); }
                                KeyCode::Backspace => { tui_state.lock().await.command_palette.query.pop(); }
                                _ => {}
                            }
                            continue;
                        }

                        let current_mode = {
                            let state = tui_state.lock().await;
                            state.mode.clone()
                        };

                        match current_mode {
                            TuiMode::Command => {
                                if let events::EventResult::Break = events::handle_command_mode(
                                    key, &mut input, &output_lines, &mut current_dir,
                                    &human_name, output_height, &mut scroll_offset, &mut terminal,
                                ).await {
                                    break;
                                }
                            }
                            TuiMode::Services => {
                                events::handle_services_mode(key, &tui_state, &output_lines).await;
                            }
                            TuiMode::Logs => {
                                events::handle_logs_mode(key, &tui_state).await;
                            }
                            TuiMode::Help => {
                                events::handle_help_mode(key, &tui_state).await;
                            }
                            TuiMode::CodingAgent => {
                                events::handle_coding_agent_mode(key, &tui_state, &output_lines).await;
                            }
                            _ => {
                                // Other modes - basic navigation placeholder
                            }
                        }
                    }
                }
            }
        }
    }

    // Save command history on exit
    {
        let state = tui_state.lock().await;
        save_command_history(&state.command_history);
    }

    Ok(())
}
