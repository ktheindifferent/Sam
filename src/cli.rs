use std::io::{self, Write};
use std::env;
use colored::*;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Layout, Constraint, Direction},
    text::{Span, Line},
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::thread;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::io::BufReader;
use std::io::BufRead;
use std::process::Stdio;
use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::sync::mpsc::{self, Sender};
use std::io::{Read};
// Add this import for catch_unwind on async blocks
use futures::FutureExt;

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

// Move ServiceStatus struct to module scope and derive Debug, Default, Clone
#[derive(Debug, Default, Clone)]
struct ServiceStatus {
    crawler: String,
    redis: String,
    docker: String, // Add docker status
    update_count: u64, // Add a counter to show updates
}

/// Starts the interactive command prompt
///
/// This function checks for required Postgres environment variables,
/// initializes the TUI logger, and launches the TUI event loop.
pub async fn start_prompt() {
    log::info!("[sam cli] start_prompt() called");
    check_postgres_env();
    // Initialize tui-logger (new crate)
    tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Debug);

    if let Err(e) = run_tui().await {
        log::info!("TUI error: {:?}", e);
    }
}

/// Run the TUI event loop
///
/// Handles user input, command execution, and UI rendering.
async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[sam cli] run_tui() called");

    // Only ONE service_status and updater spawn at the top
    let service_status = Arc::new(Mutex::new(ServiceStatus {
        crawler: "unknown".to_string(),
        redis: "unknown".to_string(),
        docker: "unknown".to_string(),
        update_count: 0,
    }));
   
    log::info!("[sam status updater] about to spawn background updater task (top of run_tui)");
    let service_status_clone = service_status.clone();
    tokio::spawn(async move {
        log::info!("[sam status updater] background updater task STARTED");
        let mut count = 0u64;
        loop {
            log::info!("[sam status updater] loop iteration #{count}");
            // Add logs before/after each status call
            log::info!("[sam status updater] checking crawler status");
            let crawler = std::panic::catch_unwind(|| crate::sam::services::crawler::service_status().to_string())
                .unwrap_or_else(|e| {
                    log::error!("[sam status updater] crawler status panicked: {:?}", e);
                    "error".to_string()
                });
            log::info!("[sam status updater] crawler status: {}", crawler);

            log::info!("[sam status updater] checking redis status");
            let redis = std::panic::catch_unwind(|| crate::sam::services::redis::status().to_string())
                .unwrap_or_else(|e| {
                    log::error!("[sam status updater] redis status panicked: {:?}", e);
                    "error".to_string()
                });
            log::info!("[sam status updater] redis status: {}", redis);

            log::info!("[sam status updater] checking docker status");
            let docker = std::panic::catch_unwind(|| crate::sam::services::docker::status().to_string())
                .unwrap_or_else(|e| {
                    log::error!("[sam status updater] docker status panicked: {:?}", e);
                    "error".to_string()
                });
            log::info!("[sam status updater] docker status: {}", docker);

            log::info!(
                "[sam status updater] update #{count}: crawler={:?}, redis={:?}, docker={:?}",
                crawler, redis, docker
            );

            if let Ok(mut status) = service_status_clone.try_lock() {
                status.crawler = if crawler.is_empty() { format!("unknown{}", count % 5) } else { crawler };
                status.redis = if redis.is_empty() { format!("unknown{}", count % 5) } else { redis };
                status.docker = if docker.is_empty() { format!("unknown{}", count % 5) } else { docker };
                status.update_count = count;
                count += 1;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // Set a panic hook to print panics to stderr
    std::panic::set_hook(Box::new(|info| {
        log::error!("\nSAM TUI PANIC: {info}");
        // Try to restore terminal state
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        // Flush to ensure message is visible
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure terminal is restored even if panic or error
    struct DropGuard;
    impl Drop for DropGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
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
        let stdout_writer = PipeWriter { sender: tx_out };
        // let _ = std::io::set_print(Some(Box::new(stdout_writer)));

        // Redirect stderr
        let stderr_writer = PipeWriter { sender: tx_err };
        // let _ = std::io::set_panic(Some(Box::new(stderr_writer)));
    }
    // Spawn a thread to forward lines from rx to output_lines
    {
        let output_lines = output_lines.clone();
        std::thread::spawn(move || {
            while let Ok(line) = rx.recv() {
                let output_lines = output_lines.clone();
                let line = line.clone();
                // Use tokio runtime if available, otherwise block
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    rt.spawn(async move {
                        append_line(&output_lines, line).await;
                    });
                } else {
                    futures::executor::block_on(append_line(&output_lines, line));
                }
            }
        });
    }
    // --- End Add ---

    let human_name = get_human_name();
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut scroll_offset: u16 = 0;
    let mut output_height: usize = 10;

    // Blinking cursor state
    let mut show_cursor = true;
    let mut cursor_tick: u8 = 0;

    loop {
        // Fetch service status for display (lock for shortest possible time)
        let status = {
            let guard = service_status.lock().await;
            guard.clone()
        };

        // FIX: Acquire output_lines lock asynchronously and clone before draw
        let output_lines_snapshot = {
            let lines = output_lines.lock().await;
            lines.clone()
        };

        let draw_result = catch_unwind(AssertUnwindSafe(|| {
            let mut local_output_height = output_height;
            let input_ref = &input;
            let status = status.clone(); // already cloned, no lock held here
            let output_lines_guard = &output_lines_snapshot;

            terminal.draw(|f| {
                let size = f.size();
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(66),
                        Constraint::Percentage(34),
                    ])
                    .split(size);

                // Add a vertical split for left side: [status][output][input]
                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // status block
                        Constraint::Min(3),    // output
                        Constraint::Length(3), // input
                    ])
                    .split(main_chunks[0]);

                local_output_height = left_chunks[1].height.max(1) as usize;

                let cursor_char = if show_cursor { "_" } else { " " };
                let input_display = format!("{}{}", input_ref, cursor_char);

                // Service status block
                let status_lines = vec![
                    Line::from(vec![
                        Span::styled("Crawler: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
                        Span::styled(
                            &status.crawler,
                            match status.crawler.as_str() {
                                "running" => ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                                "stopped" => ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                                _ => ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                            }
                        ),
                        Span::raw("    "),
                        Span::styled("Redis: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
                        Span::styled(
                            &status.redis,
                            match status.redis.as_str() {
                                "running" => ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                                "stopped" => ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                                "not installed" => ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                                _ => ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                            }
                        ),
                        Span::raw("    "),
                        Span::styled("Docker: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
                        Span::styled(
                            &status.docker,
                            match status.docker.as_str() {
                                "running" => ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                                "stopped" => ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                                "not installed" => ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                                _ => ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                            }
                        ),
                        Span::raw(format!("    [update #{}]", status.update_count)),
                    ])
                ];
                let status_widget = Paragraph::new(status_lines)
                    .block(Block::default().borders(Borders::ALL).title("Service Status"));

                let output: Vec<Line> = output_lines_guard.iter().map(|l| Line::from(Span::raw(l))).collect();

                let output_widget = Paragraph::new(output)
                    .block(Block::default().borders(Borders::ALL).title("Output"))
                    .scroll((scroll_offset, 0))
                    .wrap(ratatui::widgets::Wrap { trim: false });

                let input_widget = Paragraph::new(input_display)
                    .block(Block::default().borders(Borders::ALL).title("Command"));

                // Instead of using a persistent tui_logger_widget, create it here:
                let tui_logger_widget = TuiLoggerWidget::default()
                    .block(Block::default().borders(Borders::ALL).title("Logs"))
                    .output_separator('|')
                    .output_level(Some(TuiLoggerLevelOutput::Long))
                    .output_target(true)
                    .output_timestamp(Some("%H:%M:%S".to_string()));

                // Render new status block
                f.render_widget(status_widget, left_chunks[0]);
                f.render_widget(output_widget, left_chunks[1]);
                f.render_widget(input_widget, left_chunks[2]);
                f.render_widget(tui_logger_widget, main_chunks[1]);
            })?;
            output_height = local_output_height;
            Ok::<(), std::io::Error>(())
        }));

        if let Err(e) = draw_result {
            let mut lines = output_lines.lock().await;
            lines.push(format!("TUI draw panic: {:?}", e));
            log::error!("TUI draw panic: {:?}", e);
            break;
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
            lines.push(format!("TUI poll panic: {:?}", e));
            log::error!("TUI poll panic: {:?}", e);
            break;
        }

        if let Ok(Ok(true)) = poll_result {
            let read_result = catch_unwind(AssertUnwindSafe(|| event::read()));
            if let Err(e) = read_result {
                let mut lines = futures::executor::block_on(output_lines.lock());
                lines.push(format!("TUI read panic: {:?}", e));
                log::error!("TUI read panic: {:?}", e);
                break;
            }
            if let Ok(Ok(Event::Key(key))) = read_result {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => break,
                    KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                    KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                    KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                    KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                    KeyCode::Enter => {
                        let cmd = input.trim().to_string();
                        if cmd == "exit" || cmd == "quit" {
                            break;
                        }
                        if !cmd.is_empty() {
                            append_line(&output_lines, format!("┌─[{}]─> {}", human_name, cmd)).await;
                            handle_command(
                                &cmd,
                                &output_lines,
                                &mut current_dir,
                                &human_name,
                                output_height,
                                &mut scroll_offset,
                            ).await;
                        }
                        input.clear();
                    }
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => { input.pop(); },
                    _ => {}
                }
            }
        }
    }

    // DropGuard will restore terminal state here
    Ok(())
}

/// Get the human user's name from file or fallback
fn get_human_name() -> String {
    std::fs::read_to_string("/opt/sam/whoismyhuman")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "sam".to_string())
}

/// Append a line to the output_lines (thread-safe)
async fn append_line(output_lines: &Arc<Mutex<Vec<String>>>, line: String) {
    let mut lines = output_lines.lock().await;
    lines.push(line);
}

/// Append multiple lines to the output_lines (thread-safe)
async fn append_lines<I: IntoIterator<Item = String>>(output_lines: &Arc<Mutex<Vec<String>>>, lines: I) {
    let mut guard = output_lines.lock().await;
    guard.extend(lines);
}

/// Helper to run a spinner while executing an async closure, then update output_lines with a message.
async fn run_with_spinner<F, Fut>(
    output_lines: &Arc<Mutex<Vec<String>>>,
    message: &str,
    done_message: impl FnOnce(&mut Vec<String>, &str) + Send + 'static,
    fut: F,
) where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = String> + Send + 'static,
{
    let output_lines = output_lines.clone();
    let message = message.to_string();
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_running = Arc::new(Mutex::new(true));
    let spinner_flag = spinner_running.clone();

    let spinner_index = {
        let mut lines = output_lines.lock().await;
        lines.push(format!("⠋ {}", message));
        lines.len() - 1
    };

    // Spinner async task
    let spinner_output_lines = output_lines.clone();
    let message_clone = message.clone();
    let spinner_flag_clone = spinner_flag.clone();
    tokio::spawn(async move {
        let mut i = 0;
        loop {
            {
                let running = spinner_flag_clone.lock().await;
                if !*running {
                    break;
                }
                let mut lines = spinner_output_lines.lock().await;
                if spinner_index < lines.len() {
                    lines[spinner_index] = format!("{} {}", spinner_chars[i % spinner_chars.len()], message_clone);
                }
            }
            i += 1;
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    });

    // Execute future and update when done
    let spinner_flag2 = spinner_running.clone();
    let output_lines2 = output_lines.clone();
    let done_message = Box::new(done_message);
    let spinner_idx = spinner_index;

    tokio::spawn(async move {
        let result = fut().await;
        {
            let mut running = spinner_flag2.lock().await;
            *running = false;
        }
        let mut lines = output_lines2.lock().await;
        if spinner_idx < lines.len() {
            done_message(&mut lines, &result);
        }
    });
}

/// Helper to append output and play TTS in a single async block
async fn append_and_tts(output_lines: Arc<Mutex<Vec<String>>>, text: String) {
    append_line(&output_lines, text.clone()).await;
    match crate::sam::services::tts::get(text.clone().replace("┌─[sam]─>", "")) {
        Ok(wav_bytes) => {
            if let Err(e) = play_wav_from_bytes_send(&wav_bytes) {
                append_line(&output_lines, format!("TTS playback error: {}", e)).await;
            }
        }
        Err(e) => {
            append_line(&output_lines, format!("TTS error: {}", e)).await;
        }
    }
}

/// Handle a command entered by the user
async fn handle_command(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &mut std::path::PathBuf,
    human_name: &str,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    match cmd {
        "help" => append_lines(output_lines, get_help_lines()).await,
        "clear" => output_lines.lock().await.clear(),
        "setup" => { tokio::spawn(crate::sam::setup::install()); }
        "ls" => {
            match std::fs::read_dir(&current_dir) {
                Ok(entries) => {
                    let mut files = vec![];
                    for entry in entries.flatten() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let file_type = entry.file_type().ok();
                        if let Some(ft) = file_type {
                            if ft.is_dir() {
                                files.push(format!("{}/", file_name));
                            } else {
                                files.push(file_name);
                            }
                        } else {
                            files.push(file_name);
                        }
                    }
                    let mut lines = vec![format!("Files in {}:", current_dir.display())];
                    lines.extend(files);
                    append_lines(output_lines, lines).await;
                }
                Err(e) => append_line(output_lines, format!("ls error: {}", e)).await,
            }
        }
        "version" => {
            append_lines(output_lines, vec![
                "███████     █████     ███    ███    ".to_string(),
                "██         ██   ██    ████  ████    ".to_string(),
                "███████    ███████    ██ ████ ██    ".to_string(),
                "     ██    ██   ██    ██  ██  ██    ".to_string(),
                "███████ ██ ██   ██ ██ ██      ██ ██ ".to_string(),
                "Smart Artificial Mind".to_string(),
                format!("VERSION: {:?}", crate::VERSION),
                "Copyright 2021-2026 The Open Sam Foundation (OSF)".to_string(),
                "Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)".to_string(),
                "Licensed under GPLv3....see LICENSE file.".to_string(),
            ]).await;
        }
        "status" => {
            // Use sysinfo for cross-platform process/system info
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            let pid = sysinfo::get_current_pid().ok();
            let process = pid.and_then(|p| sys.process(p));
            let mem_total = sys.total_memory();
            let mem_used = sys.used_memory();
            let cpu_usage = process.map(|proc| proc.cpu_usage()).unwrap_or(0.0);
            let mem_proc = process.map(|proc| proc.memory()).unwrap_or(0);
            let os = sysinfo::System::name().unwrap_or_else(|| "Unknown".to_string());
            let os_ver = sysinfo::System::os_version().unwrap_or_default();
            let kernel = sysinfo::System::kernel_version().unwrap_or_default();
            let arch = std::env::consts::ARCH;
            let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_else(|| "Unknown".to_string());
            let version = format!("{:?}", crate::VERSION);

            let lines = vec![
                format!("Executable: {}", exe),
                format!("User: {}", human_name),
                format!("Current Directory: {}", current_dir.display()),
                format!("PID: {}", pid.map(|p| p.as_u32()).unwrap_or(0)),
                format!("Version: {}", version),
                format!("OS: {} {} ({})", os, os_ver, arch),
                format!("Kernel: {}", kernel),
                format!("CPU Usage: {:.2}%", cpu_usage),
                format!("Process Memory: {} MiB", mem_proc / 1024 / 1024),
                format!("System Memory: {} MiB used / {} MiB total", mem_used / 1024, mem_total / 1024),
                format!("PID: {}", pid.map(|p| p.as_u32()).unwrap_or(0)),
            ];
            append_lines(output_lines, lines).await;
        }
        "crawler start" => {
            crate::sam::services::crawler::start_service_async().await;
            append_line(output_lines, "Crawler service started.".to_string()).await;
        }
        "crawler stop" => {
            run_with_spinner(
                output_lines,
                "Stopping crawler service...",
                |lines, _| lines.push("Crawler service stopped.".to_string()),
                || async {
                    crate::sam::services::crawler::stop_service();
                    "done".to_string()
                },
            ).await;
        }
        "crawler status" => {
            run_with_spinner(
                output_lines,
                "Checking crawler service status...",
                |lines, status| lines.push(format!("Crawler service status: {}", status)),
                || async {
                    crate::sam::services::crawler::service_status().to_string()
                },
            ).await;
        }
        "redis install" => {
            run_with_spinner(
                output_lines,
                "Installing Redis via Docker...",
                |lines, _| lines.push("Redis install complete.".to_string()),
                || async {
                    crate::sam::services::redis::install().await;
                    "done".to_string()
                },
            ).await;
        }
        "redis start" => {
            run_with_spinner(
                output_lines,
                "Starting Redis via Docker...",
                |lines, _| lines.push("Redis start command issued.".to_string()),
                || async {
                    crate::sam::services::redis::start().await;
                    "done".to_string()
                },
            ).await;
        }
        "redis stop" => {
            run_with_spinner(
                output_lines,
                "Stopping Redis via Docker...",
                |lines, _| lines.push("Redis stop command issued.".to_string()),
                || async {
                    crate::sam::services::redis::stop().await;
                    "done".to_string()
                },
            ).await;
        }
        "redis status" => {
            run_with_spinner(
                output_lines,
                "Checking Redis service status...",
                |lines, status| lines.push(format!("Redis service status: {}", status)),
                || async {
                    crate::sam::services::redis::status().to_string()
                },
            ).await;
        }
        "docker start" => {
            run_with_spinner(
                output_lines,
                "Starting Docker daemon...",
                |lines, _| lines.push("Docker start command issued.".to_string()),
                || async {
                    crate::sam::services::docker::start().await;
                    "done".to_string()
                },
            ).await;
        }
        "docker stop" => {
            run_with_spinner(
                output_lines,
                "Stopping Docker daemon...",
                |lines, _| lines.push("Docker stop command issued.".to_string()),
                || async {
                    crate::sam::services::docker::stop().await;
                    "done".to_string()
                },
            ).await;
        }
        "docker status" => {
            let status = crate::sam::services::docker::status();
            append_line(output_lines, format!("Docker daemon status: {}", status)).await;
        }
        _ if cmd.starts_with("cd ") => {
            handle_cd(cmd, output_lines, current_dir).await;
        }
        _ if cmd.starts_with("darknet ") => {
            handle_darknet(cmd, output_lines).await;
        }
        _ if cmd.starts_with("tts ") => {
            let input = cmd.strip_prefix("tts ").unwrap().trim();
            if input.is_empty() {
                append_line(output_lines, "Usage: tts <text>".to_string()).await;
            } else {
                append_line(output_lines, format!("Synthesizing speech for: '{}'", input)).await;
                let output_lines = output_lines.clone();
                let text = input.to_string();
                tokio::spawn(append_and_tts(output_lines, text));
            }
        }
        "llama install" => {
            append_line(output_lines, "Starting llama model installer...".to_string()).await;
            let output_lines = output_lines.clone();
            tokio::spawn(async move {
                
           
                let llama_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/llama.cpp");
                let mut cmake_cmd = Command::new("cmake");
                cmake_cmd.current_dir(&llama_src)
                    .arg("-B")
                    .arg("build");

                let mut build_cmd = Command::new("cmake");
                build_cmd.current_dir(&llama_src)
                    .arg("--build")
                    .arg("build")
                    .arg("--config")
                    .arg("Release");

             
                let output_lines2 = output_lines.clone();
                let _ = run_command_stream_lines(cmake_cmd, move |line| {
                    let value = output_lines2.clone();
                    tokio::spawn(async move {
                        append_line(&value, format!("cmake: {}", line)).await;
                    });
                });

                let output_lines3 = output_lines.clone();
                let _ = run_command_stream_lines(build_cmd, move |line| {
                    let value = output_lines3.clone();
                    tokio::spawn(async move {
                        append_line(&value, format!("build: {}", line)).await;
                    });
                });


                // Copy built binaries to /opt/sam/bin and set executable permissions
                let bin_dir = llama_src.join("build/bin");
                let target_dir = std::path::Path::new("/opt/sam/bin");
                let binaries = ["llama-simple", "llama-bench", "llama-cli"];
                for bin in &binaries {
                    let src = bin_dir.join(bin);
                    let dst = target_dir.join(bin);
                    match fs::copy(&src, &dst) {
                        Ok(_) => {
                            // Set +x permissions
                            let mut perms = fs::metadata(&dst).unwrap().permissions();
                            perms.set_mode(0o755);
                            fs::set_permissions(&dst, perms).unwrap();
                            append_line(&output_lines, format!("Installed {} to {}", bin, dst.display())).await;
                        }
                        Err(e) => {
                            append_line(&output_lines, format!("Failed to install {}: {}", bin, e)).await;
                        }
                    }
                }

                // Show spinner while downloading models (blocking)
                let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner_running = Arc::new(Mutex::new(true));
                let spinner_flag = spinner_running.clone();
                let output_lines_clone = output_lines.clone();

                // Add spinner line and get its index
                let spinner_index = {
                    let mut lines = output_lines.lock().await;
                    lines.push("⠋ Downloading Llama v2 and v3 models...".to_string());
                    lines.len() - 1
                };

                // Spinner thread
                let spinner_output_lines = output_lines.clone();
                tokio::spawn(async move {
                    let mut i = 0;
                    while *spinner_flag.lock().await {
                        {
                            let mut lines = spinner_output_lines.lock().await;
                            if spinner_index < lines.len() {
                                lines[spinner_index] = format!("{} Downloading Llama v2 and v3 models...", spinner_chars[i % spinner_chars.len()]);
                            }
                        }
                        i += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                    }
                });

                // Run blocking downloads in a separate thread
                let spinner_flag2 = spinner_running.clone();
                let output_lines2 = output_lines.clone();
                let spinner_index2 = spinner_index;
                tokio::task::spawn_blocking(move || {
                    let v2_result = crate::sam::services::llama::LlamaService::download_v2_model();
                    let v3_result = crate::sam::services::llama::LlamaService::download_v3_model();

                    *spinner_flag2.blocking_lock() = false;
                    let mut lines = output_lines2.blocking_lock();
                    if spinner_index2 < lines.len() {
                        if v2_result.is_ok() && v3_result.is_ok() {
                            lines[spinner_index2] = "Llama v2 and v3 models downloaded successfully.".to_string();
                        } else {
                            let mut msg = String::new();
                            if let Err(e) = v2_result {
                                msg.push_str(&format!("Failed to download v2 model: {}. ", e));
                            }
                            if let Err(e) = v3_result {
                                msg.push_str(&format!("Failed to download v3 model: {}", e));
                            }
                            lines[spinner_index2] = msg;
                        }
                    }
                });

                append_line(&output_lines, "llama install: done.".to_string()).await;
            });
        }
        _ if cmd.starts_with("llama v2 ") => {
            let prompt = cmd.trim_start_matches("llama v2 ").trim();
            if prompt.is_empty() {
            append_line(output_lines, "Usage: llama v2 <prompt>".to_string()).await;
            } else {
            let prompt = prompt.to_string();
            let output_lines = output_lines.clone();

            // Spinner setup
            let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner_running = Arc::new(Mutex::new(true));
            let spinner_flag = spinner_running.clone();

            // Add spinner line and get its index
            let spinner_index = {
                let mut lines = output_lines.lock().await;
                lines.push("⠋ Querying llama v2...".to_string());
                lines.len() - 1
            };

            // Spinner task
            let spinner_output_lines = output_lines.clone();
            tokio::spawn(async move {
                let mut i = 0;
                while *spinner_flag.lock().await {
                {
                    let mut lines = spinner_output_lines.lock().await;
                    if spinner_index < lines.len() {
                    lines[spinner_index] = format!("{} Querying llama v2...", spinner_chars[i % spinner_chars.len()]);
                    }
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                }
            });

            // Query in blocking thread
            let spinner_flag2 = spinner_running.clone();
            let output_lines2 = output_lines.clone();
            tokio::task::spawn_blocking(move || {
                let result = crate::sam::services::llama::LlamaService::query_v2(&prompt);
                let mut lines = output_lines2.blocking_lock();
                *spinner_flag2.blocking_lock() = false;
                if spinner_index < lines.len() {
                match result {
                    Ok(result) => {
                    let text = result.trim().to_string();
                    lines[spinner_index] = format!("llama v2: {}", text);
                    let output_lines = output_lines2.clone();
                    tokio::spawn(append_and_tts(output_lines, format!("llama v2: {}", text)));
                    },
                    Err(e) => {
                    lines[spinner_index] = format!("llama v2 error: {}", e);
                    }
                }
                }
            });
            }
        }
        _ if cmd.starts_with("llama v2-tiny ") => {
            let prompt = cmd.trim_start_matches("llama v2-tiny ").trim();
            if prompt.is_empty() {
            append_line(output_lines, "Usage: llama v2-tiny <prompt>".to_string()).await;
            } else {
            let prompt = prompt.to_string();
            let output_lines = output_lines.clone();

            // Spinner setup
            let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner_running = Arc::new(Mutex::new(true));
            let spinner_flag = spinner_running.clone();

            // Add spinner line and get its index
            let spinner_index = {
                let mut lines = output_lines.lock().await;
                lines.push("⠋ Querying llama v2-tiny...".to_string());
                lines.len() - 1
            };

            // Spinner task
            let spinner_output_lines = output_lines.clone();
            tokio::spawn(async move {
                let mut i = 0;
                while *spinner_flag.lock().await {
                {
                    let mut lines = spinner_output_lines.lock().await;
                    if spinner_index < lines.len() {
                    lines[spinner_index] = format!("{} Querying llama v2-tiny...", spinner_chars[i % spinner_chars.len()]);
                    }
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                }
            });

            // Query in blocking thread
            let spinner_flag2 = spinner_running.clone();
            let output_lines2 = output_lines.clone();
            tokio::task::spawn_blocking(move || {
                let result = crate::sam::services::llama::LlamaService::query_v2_tiny(&prompt);
                let mut lines = output_lines2.blocking_lock();
                *spinner_flag2.blocking_lock() = false;
                if spinner_index < lines.len() {
                match result {
                    Ok(result) => {
                    let text = result.trim().to_string();
                    lines[spinner_index] = format!("llama v2-tiny: {}", text);
                    let output_lines = output_lines2.clone();
                    tokio::spawn(append_and_tts(output_lines, format!("llama v2-tiny: {}", text)));
                    },
                    Err(e) => {
                    lines[spinner_index] = format!("llama v2-tiny error: {}", e);
                    }
                }
                }
            });
            }
        }
        _ if cmd.starts_with("llama ") => {
            let rest = cmd["llama ".len()..].to_string();
            let mut split = rest.splitn(2, ' ');
            let model_path_str = split.next().unwrap_or("").to_string();
            let prompt_str = split.next().unwrap_or("").to_string();

            if model_path_str.is_empty() || prompt_str.is_empty() {
                append_line(output_lines, "Usage: llama <model_path> <prompt>".to_string()).await;
            } else {
                let model_path = std::path::PathBuf::from(model_path_str);
                let prompt = prompt_str;
                let output_lines = output_lines.clone();
                tokio::task::spawn_blocking(move || {
                    match crate::sam::services::llama::LlamaService::query(&model_path, &prompt) {
                        Ok(result) => {
                            let text = result.trim().to_string();
                            let output_lines = output_lines.clone();
                            tokio::spawn(append_and_tts(output_lines, format!("llama: {}", text)));
                        },
                        Err(e) => {
                            let output_lines = output_lines.clone();
                            tokio::spawn(async move {
                                append_line(&output_lines, format!("llama error: {}", e)).await;
                            });
                        }
                    }
                });
            }
        }
        "lifx start" => {
            crate::sam::services::lifx::start_service();
            append_line(output_lines, "LIFX service started.".to_string()).await;
        }
        "lifx stop" => {
            crate::sam::services::lifx::stop_service();
            append_line(output_lines, "LIFX service stopped.".to_string()).await;
        }
        "lifx status" => {
            let status = crate::sam::services::lifx::status_service();
            append_line(output_lines, format!("LIFX service status: {}", status)).await;
        }
        _ if cmd.starts_with("crawl search ") => {
            let query = cmd.trim_start_matches("crawl search ").trim();
            if query.is_empty() {
                append_line(output_lines, "Usage: crawl search <query>".to_string()).await;
            } else {
                let query = query.to_string();
                let output_lines = output_lines.clone();
                tokio::spawn(async move {
                    use crate::sam::services::crawler::CrawledPage;
                    match CrawledPage::query_by_relevance_async(&query, 10).await {
                        Ok(scored_pages) if !scored_pages.is_empty() => {
                            append_line(&output_lines, format!("Found {} results:", scored_pages.len())).await;
                            for (page, score) in scored_pages {
                                append_line(&output_lines, format!("URL: {}", page.url)).await;
                                append_line(&output_lines, format!("Score: {}", score)).await;
                                if !page.tokens.is_empty() {
                                    let snippet: String = page.tokens.iter().take(20).cloned().collect::<Vec<_>>().join(" ");
                                    append_line(&output_lines, format!("Tokens: {}...", snippet)).await;
                                }
                                append_line(&output_lines, "-----------------------------".to_string()).await;
                            }
                        }
                        Ok(_) => append_line(&output_lines, "No results found.".to_string()).await,
                        Err(e) => append_line(&output_lines, format!("Search error: {}", e)).await,
                    }
                });
            }
        }
        _ => {
            match crate::sam::services::rivescript::query(cmd) {
                Ok(reply) => {
                    let text = reply.text.clone();
                    let output_lines = output_lines.clone();
                    tokio::spawn(append_and_tts(output_lines, format!("┌─[sam]─> {}", text)));
                }
                Err(e) => append_line(output_lines, format!("┌─[sam]─> [error: {}]", e)).await,
            }
        }
    }
    // Scroll to bottom if needed
    let output_window_height = output_height;
    let lines = output_lines.lock().await;
    *scroll_offset = 0;
    if lines.len() > output_window_height {
        *scroll_offset = lines.len() as u16 - output_window_height as u16 + 2;
    }
}

/// Return help lines for the CLI
fn get_help_lines() -> Vec<String> {
    vec![
        "help                  - Show this help message".to_string(),
        "http start|stop       - Control HTTP/web services".to_string(),
        "debug [module] [level]- Set debug level (error, warn, info, debug, trace)".to_string(),
        "status                - Show system status".to_string(),
        "services              - List all available services".to_string(),
        "version               - Show SAM version information".to_string(),
        "errors                - Show/hide error output in CLI".to_string(),
        "clear                 - Clear the terminal screen".to_string(),
        "exit, quit            - Exit the command prompt".to_string(),
        "ls                    - List files in current directory".to_string(),
        "cd <dir>              - Change current directory".to_string(),
        "tts <text>            - Convert text to speech and play it".to_string(),
        "llama install         - Install or update Llama.cpp models".to_string(),
        "llama <model_path> <prompt> - Query a Llama.cpp model".to_string(),
        "llama v2 <prompt>     - Query a Llama v2 model".to_string(),
        "lifx start            - Start the LIFX service".to_string(),
        "lifx stop             - Stop the LIFX service".to_string(),
        "lifx status           - Show LIFX service status".to_string(),
        "crawler start           - Start the background web crawler".to_string(),
        "crawler stop            - Stop the background web crawler".to_string(),
        "crawler status          - Show crawler service status".to_string(),
        "crawl search <query>   - Search crawled pages for a keyword".to_string(),
        "redis install           - Install Redis using Docker".to_string(),
        "redis start             - Start the Redis Docker container".to_string(),
        "redis stop              - Stop the Redis Docker container".to_string(),
        "redis status            - Show Redis Docker container status".to_string(),
        "docker start             - Start the Docker daemon/service".to_string(),
        "docker stop              - Stop the Docker daemon/service".to_string(),
        "docker status            - Show Docker daemon/service status".to_string(),
    ]
}

/// Handle 'cd' command
async fn handle_cd(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &mut std::path::PathBuf) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let new_dir = parts[1].trim();
        let new_path = if new_dir.starts_with('/') {
            std::path::PathBuf::from(new_dir)
        } else {
            current_dir.join(new_dir)
        };
        if new_path.is_dir() {
            *current_dir = new_path.canonicalize().unwrap_or(new_path);
            append_line(output_lines, format!("Changed directory to {}", current_dir.display())).await;
        } else {
            append_line(output_lines, format!("cd: no such directory: {}", new_dir)).await;
        }
    } else {
        append_line(output_lines, "Usage: cd <directory>".to_string()).await;
    }
}

/// Handle 'darknet' command with spinner and async detection
async fn handle_darknet(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let image_path = parts[1].trim().to_string();
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_running = Arc::new(Mutex::new(true));
        let spinner_flag = spinner_running.clone();
        let output_lines_clone = output_lines.clone();

        // Add a spinner line and get its index
        let spinner_index = {
            let mut lines = output_lines.lock().await;
            lines.push(format!("Running darknet_detect on: {}", image_path));
            lines.push("⠋ Detecting...".to_string());
            lines.len() - 1
        };

        // Spinner thread
        let spinner_output_lines = output_lines.clone();
        tokio::spawn(async move {
            let mut i = 0;
            while *spinner_flag.lock().await {
                {
                    let mut lines = spinner_output_lines.lock().await;
                    if spinner_index < lines.len() {
                        lines[spinner_index] = format!("{} Detecting...", spinner_chars[i % spinner_chars.len()]);
                    }
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        });

        // Async darknet task
        let spinner_flag2 = spinner_running.clone();
        tokio::spawn(async move {
            let output = crate::sam::services::darknet::darknet_detect(&image_path).await;
            let mut lines = output_lines_clone.lock().await;
            *spinner_flag2.lock().await = false;
            if spinner_index < lines.len() {
                match output {
                    Ok(result) => {
                        if let Some(best) = result.objects.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)) {
                            lines[spinner_index] = format!("Detected: {}", best.name);
                        } else {
                            lines[spinner_index] = "No objects detected.".to_string();
                        }
                    }
                    Err(e) => {
                        lines[spinner_index] = format!("darknet error: {}", e);
                    }
                }
            }
        });
    } else {
        append_line(output_lines, "Usage: darknet <image_path>".to_string()).await;
    }
}

/// Play a WAV file from bytes using rodio (cross-platform)
fn play_wav_from_bytes(wav_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
 

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(wav_bytes.to_vec());
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

// Add a Send+Sync error version for play_wav_from_bytes
fn play_wav_from_bytes_send(wav_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(wav_bytes.to_vec());
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

/// Check for missing Postgres ENV vars and prompt user if missing
///
/// Prompts interactively for any missing required environment variables.
pub fn check_postgres_env() {
    let vars = ["PG_DBNAME", "PG_USER", "PG_PASS", "PG_ADDRESS"];
    let mut missing = vec![];
    for v in vars.iter() {
        match std::env::var(v) {
            Ok(val) if !val.trim().is_empty() => {},
            _ => missing.push(*v),
        }
    }
    if !missing.is_empty() {
        log::info!("{}", "Postgres credentials missing:".red().bold());
        for v in missing {
            loop {
                print!("{}", format!("Enter value for {}: ", v).cyan().bold());
                io::stdout().flush().unwrap();
                let mut val = String::new();
                if io::stdin().read_line(&mut val).is_ok() {
                    let val = val.trim();
                    if !val.is_empty() {
                        env::set_var(v, val);
                        break;
                    }
                }
                log::info!("{}", format!("{} cannot be empty.", v).red());
            }
        }
    }
}


/// Run a command and stream its stdout/stderr lines to a callback
pub fn run_command_stream_lines<F>(
    mut cmd: Command,
    mut on_line: F,
) -> io::Result<i32>
where
    F: FnMut(String) + Send + 'static,
{
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut reader = BufReader::new(stdout);
    let mut err_reader = BufReader::new(stderr);

    let mut buf = String::new();
    let mut err_buf = String::new();

    loop {
        buf.clear();
        err_buf.clear();
        let stdout_read = reader.read_line(&mut buf)?;
        let stderr_read = err_reader.read_line(&mut err_buf)?;

        if stdout_read == 0 && stderr_read == 0 {
            break;
        }
        if !buf.trim().is_empty() {
            on_line(buf.trim_end().to_string());
        }
        if !err_buf.trim().is_empty() {
            on_line(err_buf.trim_end().to_string());
        }
    }
    let status = child.wait()?;
    Ok(status.code().unwrap_or(-1))
}