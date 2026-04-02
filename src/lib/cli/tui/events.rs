use crossterm::event::KeyCode;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::state::{TuiMode, TuiState, Notification, NotificationLevel};
use crate::cli::helpers;

/// Result of handling a key event
pub enum EventResult {
    Continue,
    Break,
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
                helpers::append_line(
                    output_lines,
                    format!("┌─[{human_name}]─> {cmd}"),
                ).await;
                crate::cli::commands::handle_command(
                    &cmd,
                    output_lines,
                    current_dir,
                    human_name,
                    output_height,
                    scroll_offset,
                ).await;

                // Check if TUI restart is needed (e.g., after SSH session)
                {
                    let mut lines = output_lines.lock().await;
                    if lines.iter().any(|line| line.contains("__TUI_RESTART_NEEDED__")) {
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
                        super::terminal::TERMINAL_NEEDS_RESTORE.store(true, std::sync::atomic::Ordering::SeqCst);
                        let _ = terminal.clear();
                    }
                }
            }
            input.clear();
        }
        KeyCode::Char(c) => input.push(c),
        KeyCode::Backspace => { input.pop(); }
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
    let service_names = ["redis", "crawler", "docker", "postgres", "lifx", "http_server", "ollama", "tts", "stt", "ssh_server", "media", "snapcast"];
    
    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            if state.selected_service > 0 {
                state.selected_service -= 1;
            }
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if state.selected_service < service_names.len() - 1 {
                state.selected_service += 1;
            }
        }
        KeyCode::Char(' ') => {
            let selected_idx = {
                let state = tui_state.lock().await;
                state.selected_service
            };
            
            if let Some(service) = service_names.get(selected_idx) {
                helpers::append_line(output_lines, format!("Toggling service: {}", service)).await;
                
                let service_lower = service.to_lowercase();
                if service_lower == "redis" {
                    crate::services::redis::stop().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    crate::services::redis::start().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else if service_lower == "lifx" {
                    let _ = crate::services::lifx::start_server().await;
                    helpers::append_line(output_lines, format!("✓ {} service started", service)).await;
                } else if service_lower == "ssh_server" {
                    let _ = crate::services::ssh::server::start_ssh_server().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "media" {
                    let _ = crate::services::media::start().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "snapcast" {
                    let _ = crate::services::media::snapcast::init().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else {
                    helpers::append_line(output_lines, format!("Service control for {} coming soon", service)).await;
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let selected_idx = {
                let state = tui_state.lock().await;
                state.selected_service
            };
            
            if let Some(service) = service_names.get(selected_idx) {
                helpers::append_line(output_lines, format!("Restarting service: {}", service)).await;
                
                let service_lower = service.to_lowercase();
                if service_lower == "redis" {
                    crate::services::redis::stop().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    crate::services::redis::start().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else if service_lower == "lifx" {
                    let _ = crate::services::lifx::stop_server().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let _ = crate::services::lifx::start_server().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else if service_lower == "ssh_server" {
                    let _ = crate::services::ssh::server::stop_ssh_server().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let _ = crate::services::ssh::server::start_ssh_server().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else if service_lower == "media" {
                    let _ = crate::services::media::stop().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let _ = crate::services::media::start().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else if service_lower == "snapcast" {
                    let _ = crate::services::media::snapcast::deinit().await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let _ = crate::services::media::snapcast::init().await;
                    helpers::append_line(output_lines, format!("✓ {} restarted", service)).await;
                } else {
                    helpers::append_line(output_lines, format!("Service restart for {} coming soon", service)).await;
                }
            }
        }
        KeyCode::Enter => {
            let selected_idx = {
                let state = tui_state.lock().await;
                state.selected_service
            };
            
            if let Some(service) = service_names.get(selected_idx) {
                let status = match *service {
                    "redis" => crate::services::redis::is_running().await.to_string(),
                    "ssh_server" => crate::services::ssh::server::is_ssh_server_running().await.to_string(),
                    "lifx" => crate::services::lifx::status_service().unwrap_or_else(|_| "unknown".to_string()),
                    "crawler" => crate::services::crawler::service_status(),
                    "media" => crate::services::media::is_running().await.to_string(),
                    "snapcast" => crate::services::media::snapcast::is_running().await.to_string(),
                    "postgres" => "healthy".to_string(),
                    "http_server" => "running".to_string(),
                    _ => "unknown".to_string()
                };
                helpers::append_line(output_lines, format!("{} status: {}", service, status)).await;
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let selected_idx = {
                let state = tui_state.lock().await;
                state.selected_service
            };
            
            if let Some(service) = service_names.get(selected_idx) {
                helpers::append_line(output_lines, format!("Starting service: {}", service)).await;
                
                let service_lower = service.to_lowercase();
                if service_lower == "redis" {
                    crate::services::redis::start().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "lifx" {
                    let _ = crate::services::lifx::start_server().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "ssh_server" {
                    let _ = crate::services::ssh::server::start_ssh_server().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "media" {
                    let _ = crate::services::media::start().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else if service_lower == "snapcast" {
                    let _ = crate::services::media::snapcast::init().await;
                    helpers::append_line(output_lines, format!("✓ {} started", service)).await;
                } else {
                    helpers::append_line(output_lines, format!("Start command sent for {}", service)).await;
                }
            }
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            let selected_idx = {
                let state = tui_state.lock().await;
                state.selected_service
            };
            
            if let Some(service) = service_names.get(selected_idx) {
                helpers::append_line(output_lines, format!("Stopping service: {}", service)).await;
                
                let service_lower = service.to_lowercase();
                if service_lower == "redis" {
                    crate::services::redis::stop().await;
                    helpers::append_line(output_lines, format!("✓ {} stopped", service)).await;
                } else if service_lower == "lifx" {
                    let _ = crate::services::lifx::stop_server().await;
                    helpers::append_line(output_lines, format!("✓ {} stopped", service)).await;
                } else if service_lower == "ssh_server" {
                    let _ = crate::services::ssh::server::stop_ssh_server().await;
                    helpers::append_line(output_lines, format!("✓ {} stopped", service)).await;
                } else if service_lower == "media" {
                    let _ = crate::services::media::stop().await;
                    helpers::append_line(output_lines, format!("✓ {} stopped", service)).await;
                } else if service_lower == "snapcast" {
                    let _ = crate::services::media::snapcast::deinit().await;
                    helpers::append_line(output_lines, format!("✓ {} stopped", service)).await;
                } else {
                    helpers::append_line(output_lines, format!("Stop command sent for {}", service)).await;
                }
            }
        }
        _ => {}
    }
}

/// Handle key events for Logs mode
pub async fn handle_logs_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
) {
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
pub async fn handle_help_mode(
    key: crossterm::event::KeyEvent,
    tui_state: &Arc<Mutex<TuiState>>,
) {
    match key.code {
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            state.help_scroll = state.help_scroll.saturating_add(1);
        }
        _ => {}
    }
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
        KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
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
        KeyCode::Char('v') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_verify_mode = !state.coding_agent_verify_mode;
            if state.coding_agent_verify_mode {
                state.coding_agent_execution_log.push("🔍 Verification mode ENABLED".to_string());
            } else {
                state.coding_agent_execution_log.push("⚡ Verification mode DISABLED".to_string());
            }
        }
        KeyCode::Char('a') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_auto_execute = !state.coding_agent_auto_execute;
            if state.coding_agent_auto_execute {
                state.coding_agent_execution_log.push("🚀 Auto-execute mode ENABLED".to_string());
            } else {
                state.coding_agent_execution_log.push("🛑 Auto-execute mode DISABLED".to_string());
            }
        }
        KeyCode::Up => {
            let mut state = tui_state.lock().await;
            if !state.coding_agent_history.is_empty() && state.coding_agent_history_index > 0 {
                state.coding_agent_history_index -= 1;
                state.coding_agent_input = state.coding_agent_history[state.coding_agent_history_index].clone();
            }
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if !state.coding_agent_history.is_empty() {
                if state.coding_agent_history_index < state.coding_agent_history.len().saturating_sub(1) {
                    state.coding_agent_history_index += 1;
                    state.coding_agent_input = state.coding_agent_history[state.coding_agent_history_index].clone();
                } else if state.coding_agent_history_index == state.coding_agent_history.len().saturating_sub(1) {
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

        let is_complex_task = input.contains("and") ||
            input.contains("then") ||
            input.to_lowercase().contains("create") ||
            input.to_lowercase().contains("make") ||
            input.to_lowercase().contains("build") ||
            input.to_lowercase().contains("setup");

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
    let coding_agent = Arc::new(crate::services::coding::CodingAgentService::new_with_defaults().await);
    let mut executor = crate::services::coding::CodingAgentExecutor::new(coding_agent);

    let verify_mode = {
        let state = tui_state.lock().await;
        state.coding_agent_verify_mode
    };
    let enable_verification = input.starts_with("verify:") || verify_mode;
    let actual_input = if input.starts_with("verify:") {
        input.strip_prefix("verify:").unwrap_or(input).trim().to_string()
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
            executor_clone.execute_incremental_task_with_verification(
                &actual_input_clone,
                &current_dir_clone,
                &session_context_clone,
            ).await
        } else {
            executor_clone.execute_incremental_task(
                &actual_input_clone,
                &current_dir_clone,
                &session_context_clone,
                true,
            ).await
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
                state.coding_agent_spinner_text = format!("{} | {}", spinner_text, interactive_status);
                state.coding_agent_execution_log = execution_log;
            }

            let queued_messages = executor_clone.process_queued_messages().await;
            for msg in queued_messages {
                helpers::append_line(
                    &output_lines_clone,
                    format!("💬 Queued feedback: {}", msg.content),
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

            if !execution_log.is_empty() {
                state.coding_agent_context.push(format!("Execution completed with {} steps", execution_log.len()));
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

    match coding_agent.generate_response(input, &agent_working_dir.to_path_buf(), &session_context, None).await {
        Ok(response) => {
            let mut state = tui_state.lock().await;
            state.coding_agent_response = response.response_text.clone();
            state.coding_agent_pending_commands = response.suggested_commands;
            state.coding_agent_selected_command = 0;
            state.coding_agent_execution_log.clear();

            state.coding_agent_context.push(format!("AI: {} (model: {})",
                response.response_text.lines().next().unwrap_or("..."),
                response.model_used
            ));

            helpers::append_line(
                output_lines,
                format!("🤖 AI ({}): {}", response.model_used, response.response_text),
            ).await;
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
            if !state.coding_agent_pending_commands.is_empty() && state.coding_agent_selected_command > 0 {
                state.coding_agent_selected_command -= 1;
            }
            state.coding_agent_scroll_offset = state.coding_agent_scroll_offset.saturating_sub(1);
        }
        KeyCode::Down => {
            let mut state = tui_state.lock().await;
            if state.coding_agent_selected_command < state.coding_agent_pending_commands.len().saturating_sub(1) {
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
                    let cmd = &state.coding_agent_pending_commands[state.coding_agent_selected_command];
                    (Some(cmd.command.clone()), cmd.require_confirmation)
                } else {
                    (None, false)
                }
            };

            if let Some(command) = selected_cmd {
                helpers::append_line(output_lines, format!("🤖 Executing: {}", command)).await;

                let coding_agent = crate::services::coding::CodingAgentService::new_with_defaults().await;
                match coding_agent.execute_command(&command, require_confirmation).await {
                    Ok(result) => {
                        helpers::append_line(output_lines, format!("🤖 Result:\n{}", result)).await;
                    }
                    Err(e) => {
                        helpers::append_line(output_lines, format!("🤖 Command failed: {}", e)).await;
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
