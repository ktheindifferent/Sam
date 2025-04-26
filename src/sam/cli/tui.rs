use super::{commands, helpers, spinner};
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
    sms: String, // Add sms status
    update_count: u64, // Add a counter to show updates
}

/// Starts the interactive command prompt
///
/// This function checks for required Postgres environment variables,
/// initializes the TUI logger, and launches the TUI event loop.
pub async fn start_prompt() {
    log::info!("[sam cli] start_prompt() called");
    helpers::check_postgres_env();
    // Initialize tui-logger (new crate)
    tui_logger::init_logger(log::LevelFilter::Info).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);
 
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

/// Run the TUI event loop
///
/// Handles user input, command execution, and UI rendering.
async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {

    // Only ONE service_status and updater spawn at the top
    let service_status = Arc::new(Mutex::new(ServiceStatus {
        crawler: "unknown".to_string(),
        redis: "unknown".to_string(),
        docker: "unknown".to_string(),
        sms: "unknown".to_string(), // Add sms status
        update_count: 0,
    }));
   
    let service_status_clone = service_status.clone();
    tokio::spawn(async move {
        let mut count = 0u64;
        loop {
            let crawler = std::panic::catch_unwind(|| crate::sam::services::crawler::service_status().to_string())
                .unwrap_or_else(|_| {
                    "error".to_string()
                });

            let redis = std::panic::catch_unwind(|| crate::sam::services::redis::status().to_string())
                .unwrap_or_else(|_| {
                    "error".to_string()
                });

            let docker = std::panic::catch_unwind(|| crate::sam::services::docker::status().to_string())
                .unwrap_or_else(|_| {
                    "error".to_string()
                });

            let sms = std::panic::catch_unwind(|| crate::sam::services::sms::status().to_string())
                .unwrap_or_else(|_| {
                    "error".to_string()
                });

            if let Ok(mut status) = service_status_clone.try_lock() {
                status.crawler = if crawler.is_empty() { format!("unknown{}", count % 5) } else { crawler };
                status.redis = if redis.is_empty() { format!("unknown{}", count % 5) } else { redis };
                status.docker = if docker.is_empty() { format!("unknown{}", count % 5) } else { docker };
                status.sms = if sms.is_empty() { format!("unknown{}", count % 5) } else { sms };
                status.update_count = count;
                count += 1;
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
        let stdout_writer = PipeWriter { sender: tx_out };
        // let _ = std::io::set_print(Some(Box::new(stdout_writer)));

        // Redirect stderr
        let stderr_writer = PipeWriter { sender: tx_err };
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
                        Span::raw("    "),
                        Span::styled("SMS: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
                        Span::styled(
                            &status.sms,
                            match status.sms.as_str() {
                                "running" => ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                                "stopped" => ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                                _ => ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                            }
                        ),
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
                            helpers::append_line(&output_lines, format!("┌─[{}]─> {}", human_name, cmd)).await;
                            commands::handle_command(
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