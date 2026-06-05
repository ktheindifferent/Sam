use crossterm::event::KeyCode;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::state::{
    Notification, NotificationLevel, ServiceCatalogEntry, TuiMode, TuiState, SERVICE_CATALOG,
};
use crate::cli::helpers;

/// Result of handling a key event
pub enum EventResult {
    Continue,
    Break,
}

#[derive(Debug, Clone, Copy)]
enum ServiceCommand {
    Start,
    Stop,
    Restart,
    Toggle,
}

/// Handle key events for Command mode
pub async fn handle_command_mode(
    key: crossterm::event::KeyEvent,
    input: &mut String,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &mut std::path::PathBuf,
    human_name: &str,
    output_height: usize,
    scroll_offset: &mut u16,
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> EventResult {
    match key.code {
        KeyCode::PageUp => *scroll_offset = scroll_offset.saturating_sub(5),
        KeyCode::PageDown => *scroll_offset = scroll_offset.saturating_add(5),
        KeyCode::Up => *scroll_offset = scroll_offset.saturating_sub(1),
        KeyCode::Down => *scroll_offset = scroll_offset.saturating_add(1),
        KeyCode::Enter => {
            let cmd = input.trim().to_string();
            if cmd == "exit" || cmd == "quit" {
                return EventResult::Break;
            }
            if !cmd.is_empty() {
                helpers::append_line(output_lines, format!("┌─[{human_name}]─> {cmd}")).await;
                crate::cli::commands::handle_command(
                    &cmd,
                    output_lines,
                    current_dir,
                    human_name,
                    output_height,
                    scroll_offset,
                )
                .await;

                // Check if TUI restart is needed (e.g., after SSH session)
                {
                    let mut lines = output_lines.lock().await;
                    if lines
                        .iter()
                        .any(|line| line.contains("__TUI_RESTART_NEEDED__"))
                    {
                        lines.retain(|line| !line.contains("__TUI_RESTART_NEEDED__"));
                        drop(lines);

                        super::terminal::restore_terminal_state();
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        let _ = crossterm::terminal::enable_raw_mode();
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::EnterAlternateScreen,
                            crossterm::cursor::Hide
                        );
                        super::terminal::TERMINAL_NEEDS_RESTORE
                            .store(true, std::sync::atomic::Ordering::SeqCst);
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
    }
    EventResult::Continue
}

/// Handle key events for Services mode
pub async fn handle_services_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            if state.selected_service > 0 {
                state.selected_service -= 1;
            }
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if state.selected_service + 1 < SERVICE_CATALOG.len() {
                state.selected_service += 1;
            }
        }
        KeyCode::Char(' ') => {
            run_selected_service_command(
                tui_state,
                output_lines,
                ServiceCommand::Toggle,
                "Toggling",
            )
            .await;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            run_selected_service_command(
                tui_state,
                output_lines,
                ServiceCommand::Restart,
                "Restarting",
            )
            .await;
        }
        KeyCode::Enter => {
            if let Some(service) = selected_service(tui_state).await {
                let status = current_service_status(service.key).await;
                helpers::append_line(
                    output_lines,
                    format!("{} status: {}", service.label, status),
                )
                .await;
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            run_selected_service_command(
                tui_state,
                output_lines,
                ServiceCommand::Start,
                "Starting",
            )
            .await;
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            run_selected_service_command(tui_state, output_lines, ServiceCommand::Stop, "Stopping")
                .await;
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            if let Some(service) = selected_service(tui_state).await {
                let mut state = tui_state.lock().await;
                state.mode = TuiMode::Logs;
                state.log_filter_text = service.key.to_string();
                state.log_scroll_offset = 0;
                state.notifications.push(Notification {
                    message: format!("Filtering logs for {}", service.label),
                    level: NotificationLevel::Info,
                    created_at: std::time::Instant::now(),
                    duration: std::time::Duration::from_secs(3),
                });
            }
        }
        _ => {}
    }
}

async fn selected_service(tui_state: &Arc<Mutex<TuiState>>) -> Option<ServiceCatalogEntry> {
    let selected_idx = {
        let state = tui_state.lock().await;
        state.selected_service
    };
    SERVICE_CATALOG.get(selected_idx).copied()
}

async fn run_selected_service_command(
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
    command: ServiceCommand,
    verb: &str,
) {
    let Some(service) = selected_service(tui_state).await else {
        return;
    };

    helpers::append_line(output_lines, format!("{verb} service: {}", service.label)).await;

    let result = run_service_command(service.key, command).await;
    match result {
        Ok(message) => {
            helpers::append_line(output_lines, format!("✓ {} {}", service.label, message)).await;
        }
        Err(message) => {
            helpers::append_line(output_lines, format!("{}: {}", service.label, message)).await;
        }
    }
}

async fn run_service_command(key: &str, command: ServiceCommand) -> Result<String, String> {
    match command {
        ServiceCommand::Start => start_service(key).await,
        ServiceCommand::Stop => stop_service(key).await,
        ServiceCommand::Restart => {
            stop_service(key).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            start_service(key).await?;
            Ok("restarted".to_string())
        }
        ServiceCommand::Toggle => {
            let status = current_service_status(key).await;
            if super::state::is_healthy_status(&status) {
                stop_service(key).await
            } else {
                start_service(key).await
            }
        }
    }
}

async fn start_service(key: &str) -> Result<String, String> {
    match key {
        "crawler" => {
            crate::services::crawler::start_service();
            Ok("started".to_string())
        }
        "redis" => {
            crate::services::redis::start().await;
            Ok("started".to_string())
        }
        "docker" => {
            crate::services::docker::start().await;
            Ok("start requested".to_string())
        }
        "sms" => {
            crate::services::sms::start().await;
            Ok("started".to_string())
        }
        "lifx" => crate::services::lifx::start_server()
            .await
            .map(|_| "started".to_string())
            .map_err(|e| e.to_string()),
        "ollama" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            service.start_service().await.map_err(|e| e.to_string())
        }
        "ssh_server" => crate::services::ssh::server::start_ssh_server()
            .await
            .map(|_| "started".to_string())
            .map_err(|e| e.to_string()),
        "media" => crate::services::media::start()
            .await
            .map(|_| "started".to_string())
            .map_err(|e| e.to_string()),
        "snapcast" => crate::services::media::snapcast::init()
            .await
            .map(|_| "started".to_string())
            .map_err(|e| e.to_string()),
        "http_server" => Err("HTTP server is managed by this process".to_string()),
        "postgres" => Err("PostgreSQL control is not wired to the TUI yet".to_string()),
        "tts" | "stt" => Err("service control is not wired to the TUI yet".to_string()),
        _ => Err("unknown service".to_string()),
    }
}

async fn stop_service(key: &str) -> Result<String, String> {
    match key {
        "crawler" => {
            crate::services::crawler::stop_service();
            Ok("stopped".to_string())
        }
        "redis" => {
            crate::services::redis::stop().await;
            Ok("stopped".to_string())
        }
        "docker" => {
            crate::services::docker::stop().await;
            Ok("stop requested".to_string())
        }
        "sms" => {
            crate::services::sms::stop().await;
            Ok("stopped".to_string())
        }
        "lifx" => crate::services::lifx::stop_server()
            .await
            .map(|_| "stopped".to_string())
            .map_err(|e| e.to_string()),
        "ollama" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            service.stop_service().await.map_err(|e| e.to_string())
        }
        "ssh_server" => {
            crate::services::ssh::server::stop_ssh_server().await;
            Ok("stopped".to_string())
        }
        "media" => crate::services::media::stop()
            .await
            .map(|_| "stopped".to_string())
            .map_err(|e| e.to_string()),
        "snapcast" => crate::services::media::snapcast::deinit()
            .await
            .map(|_| "stopped".to_string())
            .map_err(|e| e.to_string()),
        "http_server" => Err("HTTP server is managed by this process".to_string()),
        "postgres" => Err("PostgreSQL control is not wired to the TUI yet".to_string()),
        "tts" | "stt" => Err("service control is not wired to the TUI yet".to_string()),
        _ => Err("unknown service".to_string()),
    }
}

async fn current_service_status(key: &str) -> String {
    match key {
        "crawler" => crate::services::crawler::service_status().to_string(),
        "redis" => crate::services::redis::status().await.to_string(),
        "docker" => crate::services::docker::status().to_string(),
        "sms" => crate::services::sms::status().to_string(),
        "postgres" => match tokio::time::timeout(
            std::time::Duration::from_millis(500),
            crate::services::pg::health_check(),
        )
        .await
        {
            Ok(Ok(_)) => "connected".to_string(),
            Ok(Err(_)) => "disconnected".to_string(),
            Err(_) => "timeout".to_string(),
        },
        "lifx" => crate::services::lifx::status_service().unwrap_or_else(|_| "unknown".to_string()),
        "http_server" => "running".to_string(),
        "ollama" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            if service.is_installed().await {
                if service.is_running().await {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                }
            } else {
                "not installed".to_string()
            }
        }
        "tts" | "stt" => "unknown".to_string(),
        "ssh_server" => {
            if crate::services::ssh::server::is_ssh_server_running().await {
                "running".to_string()
            } else {
                "stopped".to_string()
            }
        }
        "media" => {
            if crate::services::media::is_running().await {
                "running".to_string()
            } else {
                "stopped".to_string()
            }
        }
        "snapcast" => {
            if crate::services::media::snapcast::is_running().await {
                "running".to_string()
            } else {
                "stopped".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

/// Handle key events for Logs mode
pub async fn handle_logs_mode(key: crossterm::event::KeyEvent, tui_state: &Arc<Mutex<TuiState>>) {
    let current_input_mode = {
        let state = tui_state.lock().await;
        state.log_input_mode
    };

    if current_input_mode {
        match key.code {
            KeyCode::Esc => {
                let mut state = tui_state.lock().await;
                state.log_input_mode = false;
            }
            KeyCode::Enter => {
                let mut state = tui_state.lock().await;
                state.log_input_mode = false;
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
        match key.code {
            KeyCode::Up => {
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
                let mut state = tui_state.lock().await;
                state.log_filter_text.clear();
                state.log_scroll_offset = 0;
            }
            _ => {}
        }
    }
}

/// Handle key events for Help mode
pub async fn handle_help_mode(key: crossterm::event::KeyEvent, tui_state: &Arc<Mutex<TuiState>>) {
    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_add(1);
        }
        KeyCode::PageUp => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_sub(8);
        }
        KeyCode::PageDown => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_add(8);
        }
        _ => {}
    }
}

/// Handle key events for Database mode
pub async fn handle_database_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
) {
    let table_count = {
        let state = tui_state.lock().await;
        if state.db_table_list.is_empty() {
            2
        } else {
            state.db_table_list.len()
        }
    };

    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            state.selected_table = state.selected_table.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if state.selected_table + 1 < table_count {
                state.selected_table += 1;
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let mut state = tui_state.lock().await;
            state.db_table_list = vec!["settings".to_string(), "services".to_string()];
            state.selected_table = state
                .selected_table
                .min(state.db_table_list.len().saturating_sub(1));
        }
        _ => {}
    }
}

/// Handle key events for Files mode
pub async fn handle_files_mode(key: crossterm::event::KeyEvent, tui_state: &Arc<Mutex<TuiState>>) {
    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            state.selected_file = state.selected_file.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            let entry_count = count_dir_entries(&state.file_browser_path);
            if entry_count > 0 && state.selected_file + 1 < entry_count {
                state.selected_file += 1;
            }
        }
        KeyCode::PageUp => {
            let mut state = tui_state.lock().await;
            state.selected_file = state.selected_file.saturating_sub(10);
        }
        KeyCode::PageDown => {
            let mut state = tui_state.lock().await;
            let entry_count = count_dir_entries(&state.file_browser_path);
            if entry_count > 0 {
                state.selected_file = (state.selected_file + 10).min(entry_count - 1);
            }
        }
        KeyCode::Home => {
            let mut state = tui_state.lock().await;
            state.selected_file = 0;
        }
        KeyCode::End => {
            let mut state = tui_state.lock().await;
            let entry_count = count_dir_entries(&state.file_browser_path);
            state.selected_file = entry_count.saturating_sub(1);
        }
        KeyCode::Enter => {
            let selected_path = {
                let state = tui_state.lock().await;
                selected_dir_entry(&state.file_browser_path, state.selected_file)
            };
            if let Some(path) = selected_path {
                let mut state = tui_state.lock().await;
                state.file_browser_path = path;
                state.selected_file = 0;
            }
        }
        KeyCode::Backspace => {
            let mut state = tui_state.lock().await;
            if let Some(parent) = state.file_browser_path.parent() {
                state.file_browser_path = parent.to_path_buf();
                state.selected_file = 0;
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let mut state = tui_state.lock().await;
            state.selected_file = state
                .selected_file
                .min(count_dir_entries(&state.file_browser_path).saturating_sub(1));
        }
        _ => {}
    }
}

fn count_dir_entries(path: &std::path::Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.metadata().is_ok())
                .count()
        })
        .unwrap_or_default()
}

fn selected_dir_entry(path: &std::path::Path, selected: usize) -> Option<std::path::PathBuf> {
    let mut entries = std::fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let kind_rank = if metadata.is_dir() { 0 } else { 1 };
            Some((
                kind_rank,
                entry.file_name().to_string_lossy().to_lowercase(),
                entry.path(),
                metadata.is_dir(),
            ))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    entries
        .get(selected)
        .and_then(|(_, _, path, is_dir)| if *is_dir { Some(path.clone()) } else { None })
}

/// Handle key events for CodingAgent mode
pub async fn handle_coding_agent_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let current_input_mode = {
        let state = tui_state.lock().await;
        state.coding_agent_input_mode
    };

    if current_input_mode {
        handle_coding_agent_input_mode(key, tui_state, output_lines).await;
    } else {
        handle_coding_agent_nav_mode(key, tui_state, output_lines).await;
    }
}

async fn handle_coding_agent_input_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    match key.code {
        KeyCode::Esc => {
            let mut state = tui_state.lock().await;
            state.coding_agent_input_mode = false;
        }
        KeyCode::Char('c')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
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
            handle_coding_agent_enter(tui_state, output_lines).await;
        }
        KeyCode::Char('v')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            let mut state = tui_state.lock().await;
            state.coding_agent_verify_mode = !state.coding_agent_verify_mode;
            if state.coding_agent_verify_mode {
                state
                    .coding_agent_execution_log
                    .push("🔍 Verification mode ENABLED".to_string());
            } else {
                state
                    .coding_agent_execution_log
                    .push("⚡ Verification mode DISABLED".to_string());
            }
        }
        KeyCode::Char('a')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            let mut state = tui_state.lock().await;
            state.coding_agent_auto_execute = !state.coding_agent_auto_execute;
            if state.coding_agent_auto_execute {
                state
                    .coding_agent_execution_log
                    .push("🚀 Auto-execute mode ENABLED".to_string());
            } else {
                state
                    .coding_agent_execution_log
                    .push("🛑 Auto-execute mode DISABLED".to_string());
            }
        }
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            if !state.coding_agent_history.is_empty() && state.coding_agent_history_index > 0 {
                state.coding_agent_history_index -= 1;
                state.coding_agent_input =
                    state.coding_agent_history[state.coding_agent_history_index].clone();
            }
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if !state.coding_agent_history.is_empty() {
                if state.coding_agent_history_index
                    < state.coding_agent_history.len().saturating_sub(1)
                {
                    state.coding_agent_history_index += 1;
                    state.coding_agent_input =
                        state.coding_agent_history[state.coding_agent_history_index].clone();
                } else if state.coding_agent_history_index
                    == state.coding_agent_history.len().saturating_sub(1)
                {
                    state.coding_agent_history_index = state.coding_agent_history.len();
                    state.coding_agent_input.clear();
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
}

async fn handle_coding_agent_enter(
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    // Check if executor is already running
    let is_executing = {
        let state = tui_state.lock().await;
        if let Some(ref executor) = state.coding_agent_executor {
            executor.is_executing().await
        } else {
            false
        }
    };

    // If already executing, queue the message
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
                helpers::append_line(output_lines, format!("💬 Message queued: {}", message)).await;
            }
        }
        return;
    }

    // Check if it's a complex multi-step task
    let (input, agent_working_dir, should_auto_execute) = {
        let mut state = tui_state.lock().await;
        let input = state.coding_agent_input.clone();
        state.coding_agent_input.clear();
        state.coding_agent_input_mode = false;

        let is_complex_task = input.contains("and")
            || input.contains("then")
            || input.to_lowercase().contains("create")
            || input.to_lowercase().contains("make")
            || input.to_lowercase().contains("build")
            || input.to_lowercase().contains("setup");

        let working_dir_path = std::path::PathBuf::from(&state.coding_agent_working_dir);
        (input, working_dir_path, is_complex_task)
    };

    if !input.trim().is_empty() {
        // Add to context and history
        {
            let mut state = tui_state.lock().await;
            state.coding_agent_history.push(input.clone());
            state.coding_agent_history_index = state.coding_agent_history.len();
            state.coding_agent_context.push(format!("User: {}", input));
        }

        helpers::append_line(output_lines, format!("🤖 User: {}", input)).await;

        if should_auto_execute {
            execute_complex_task(tui_state, output_lines, &input, &agent_working_dir).await;
        } else {
            execute_simple_query(tui_state, output_lines, &input, &agent_working_dir).await;
        }
    }
}

async fn execute_complex_task(
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
    input: &str,
    agent_working_dir: &std::path::Path,
) {
    let coding_agent =
        Arc::new(crate::services::coding::CodingAgentService::new_with_defaults().await);
    let mut executor = crate::services::coding::CodingAgentExecutor::new(coding_agent);

    let verify_mode = {
        let state = tui_state.lock().await;
        state.coding_agent_verify_mode
    };
    let enable_verification = input.starts_with("verify:") || verify_mode;
    let actual_input = if input.starts_with("verify:") {
        input
            .strip_prefix("verify:")
            .unwrap_or(input)
            .trim()
            .to_string()
    } else {
        input.to_string()
    };

    if enable_verification {
        executor.enable_verification().await;
    }

    {
        let mut state = tui_state.lock().await;
        state.coding_agent_executor = Some(executor.clone());
        state.coding_agent_execution_log.clear();
        state.coding_agent_spinner_text = "🤖 Planning task...".to_string();
    }

    let session_context = {
        let mut context_lines = Vec::new();
        let state = tui_state.lock().await;
        for ctx in &state.coding_agent_context {
            context_lines.push(ctx.clone());
        }
        let lines = output_lines.lock().await;
        let recent_lines: Vec<String> = lines.iter().rev().take(20).rev().cloned().collect();
        context_lines.extend(recent_lines);
        context_lines
    };

    let executor_clone = executor.clone();
    let actual_input_clone = actual_input.clone();
    let current_dir_clone = agent_working_dir.to_path_buf();
    let session_context_clone = session_context.clone();
    let tui_state_clone = tui_state.clone();
    let output_lines_clone = output_lines.clone();

    let _message_sender = executor.setup_message_channel();

    tokio::spawn(async move {
        let result = if enable_verification {
            executor_clone
                .execute_incremental_task_with_verification(
                    &actual_input_clone,
                    &current_dir_clone,
                    &session_context_clone,
                )
                .await
        } else {
            executor_clone
                .execute_incremental_task(
                    &actual_input_clone,
                    &current_dir_clone,
                    &session_context_clone,
                    true,
                )
                .await
        };

        if let Err(e) = result {
            let mut state = tui_state_clone.lock().await;
            state.coding_agent_response = format!("Execution failed: {}", e);
            state.coding_agent_spinner_text.clear();
            state.coding_agent_executor = None;
            state.coding_agent_input_mode = true;
            state.coding_agent_context.push(format!("Error: {}", e));
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
                state.coding_agent_spinner_text =
                    format!("{} | {}", spinner_text, interactive_status);
                state.coding_agent_execution_log = execution_log;
            }

            let queued_messages = executor_clone.process_queued_messages().await;
            for msg in queued_messages {
                helpers::append_line(
                    &output_lines_clone,
                    format!("💬 Queued feedback: {}", msg.content),
                )
                .await;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        // Final update when done
        let execution_log = executor_clone.get_execution_log().await;
        {
            let mut state = tui_state_clone.lock().await;
            state.coding_agent_spinner_text.clear();
            state.coding_agent_execution_log = execution_log.clone();

            if !execution_log.is_empty() {
                state.coding_agent_context.push(format!(
                    "Execution completed with {} steps",
                    execution_log.len()
                ));
                for log in execution_log.iter().take(3) {
                    if log.contains("✅") || log.contains("❌") {
                        state.coding_agent_context.push(log.clone());
                    }
                }
            }

            state.coding_agent_executor = None;
            state.coding_agent_input_mode = true;
        }

        for log_line in execution_log {
            if log_line.starts_with("✅") || log_line.starts_with("❌") {
                helpers::append_line(&output_lines_clone, format!("🤖 {}", log_line)).await;
            }
        }
    });
}

async fn execute_simple_query(
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
    input: &str,
    agent_working_dir: &std::path::Path,
) {
    let coding_agent = crate::services::coding::CodingAgentService::new_with_defaults().await;

    let session_context = {
        let mut context_lines = Vec::new();
        let state = tui_state.lock().await;
        for ctx in &state.coding_agent_context {
            context_lines.push(ctx.clone());
        }
        let lines = output_lines.lock().await;
        let recent_lines: Vec<String> = lines.iter().rev().take(20).rev().cloned().collect();
        context_lines.extend(recent_lines);
        context_lines
    };

    match coding_agent
        .generate_response(
            input,
            &agent_working_dir.to_path_buf(),
            &session_context,
            None,
        )
        .await
    {
        Ok(response) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_response = response.response_text.clone();
            state.coding_agent_pending_commands = response.suggested_commands;
            state.coding_agent_selected_command = 0;
            state.coding_agent_execution_log.clear();

            state.coding_agent_context.push(format!(
                "AI: {} (model: {})",
                response.response_text.lines().next().unwrap_or("..."),
                response.model_used
            ));

            helpers::append_line(
                output_lines,
                format!(
                    "🤖 AI ({}): {}",
                    response.model_used, response.response_text
                ),
            )
            .await;
        }
        Err(e) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_response = format!("Error: {}", e);
            state.coding_agent_pending_commands.clear();
            state.coding_agent_execution_log.clear();

            helpers::append_line(output_lines, format!("🤖 Error: {}", e)).await;
        }
    }
}

async fn handle_coding_agent_nav_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    match key.code {
        KeyCode::Enter => {
            let mut state = tui_state.lock().await;
            state.coding_agent_input_mode = true;
        }
        KeyCode::Tab => {
            let mut state = tui_state.lock().await;
            state.coding_agent_panel_focus = (state.coding_agent_panel_focus + 1) % 4;
        }
        KeyCode::F(1) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_show_help = !state.coding_agent_show_help;
        }
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            if !state.coding_agent_pending_commands.is_empty()
                && state.coding_agent_selected_command > 0
            {
                state.coding_agent_selected_command -= 1;
            }
            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if state.coding_agent_selected_command
                < state.coding_agent_pending_commands.len().saturating_sub(1)
            {
                state.coding_agent_selected_command += 1;
            }
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
            let (selected_cmd, require_confirmation) = {
                let state = tui_state.lock().await;
                if state.coding_agent_selected_command < state.coding_agent_pending_commands.len() {
                    let cmd =
                        &state.coding_agent_pending_commands[state.coding_agent_selected_command];
                    (Some(cmd.command.clone()), cmd.require_confirmation)
                } else {
                    (None, false)
                }
            };

            if let Some(command) = selected_cmd {
                helpers::append_line(output_lines, format!("🤖 Executing: {}", command)).await;

                let coding_agent =
                    crate::services::coding::CodingAgentService::new_with_defaults().await;
                match coding_agent
                    .execute_command(&command, require_confirmation)
                    .await
                {
                    Ok(result) => {
                        helpers::append_line(output_lines, format!("🤖 Result:\n{}", result)).await;
                    }
                    Err(e) => {
                        helpers::append_line(output_lines, format!("🤖 Command failed: {}", e))
                            .await;
                    }
                }
            }
        }
        KeyCode::Char('c') => {
            let mut state = tui_state.lock().await;
            state.coding_agent_pending_commands.clear();
            state.coding_agent_selected_command = 0;
        }
        _ => {}
    }
}
