use std::io::{self, Write};
use std::env;
use colored::*;
use tui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Constraint, Direction},
    text::{Span, Spans},
};
use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput, TuiWidgetState};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::sync::Mutex;
use std::sync::Arc;
use std::thread;
/// Global flag for error display
static mut SHOW_ERRORS: bool = false;

/// Represents a CLI command with name and arguments
#[derive(Debug)]
struct Command {
    name: String,
    args: Vec<String>,
}

/// Starts the interactive command prompt
///
/// This function checks for required Postgres environment variables,
/// initializes the TUI logger, and launches the TUI event loop.
pub async fn start_prompt() {
    check_postgres_env();
    tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Debug);

    if let Err(e) = run_tui().await {
        println!("TUI error: {:?}", e);
    }
}

/// Run the TUI event loop
///
/// Handles user input, command execution, and UI rendering.
async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let output_lines = Arc::new(Mutex::new(vec![
        "Welcome to the SAM Command Interface!".to_string(),
        "Type 'help' to see available commands.".to_string(),
        "Press Ctrl+C or type 'exit' to quit.".to_string(),
    ]));

    let mut tui_state = TuiWidgetState::new();
    let human_name = get_human_name();
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut scroll_offset: u16 = 0;
    let mut output_height: usize = 10;

    // Blinking cursor state
    let mut show_cursor = true;
    let mut cursor_tick: u8 = 0;

    loop {
        {
            let output_lines_guard = output_lines.lock().unwrap();
            terminal.draw(|f| {
                let size = f.size();
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(66),
                        Constraint::Percentage(34),
                    ].as_ref())
                    .split(size);

                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(3),
                        Constraint::Length(3),
                    ].as_ref())
                    .split(main_chunks[0]);

                let output: Vec<Spans> = output_lines_guard.iter().map(|l| Spans::from(Span::raw(l))).collect();
                output_height = left_chunks[0].height.max(1) as usize;
                let total_lines = output_lines_guard.len();

                let cursor_char = if show_cursor { "_" } else { " " };
                let input_display = format!("{}{}", input, cursor_char);

                let output_widget = Paragraph::new(output)
                    .block(Block::default().borders(Borders::ALL).title("Output"))
                    .scroll((scroll_offset, 0))
                    .wrap(tui::widgets::Wrap { trim: false });

                let input_widget = Paragraph::new(input_display)
                    .block(Block::default().borders(Borders::ALL).title("Command"));

                let system_widget = TuiLoggerWidget::default()
                    .block(Block::default().borders(Borders::ALL).title("System"))
                    .output_separator('│')
                    .output_timestamp(Some("%H:%M:%S".to_string()))
                    .output_level(Some(TuiLoggerLevelOutput::Long))
                    .output_target(false)
                    .state(&mut tui_state);

                f.render_widget(output_widget, left_chunks[0]);
                f.render_widget(input_widget, left_chunks[1]);
                f.render_widget(system_widget, main_chunks[1]);
            })?;
        }

        // Blinking cursor: toggle every 5 ticks (~500ms if poll is 100ms)
        cursor_tick = cursor_tick.wrapping_add(1);
        if cursor_tick >= 5 {
            show_cursor = !show_cursor;
            cursor_tick = 0;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
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
                            append_line(&output_lines, format!("┌─[{}]─> {}", human_name, cmd));
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
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Get the human user's name from file or fallback
fn get_human_name() -> String {
    std::fs::read_to_string("/opt/sam/whoismyhuman")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "sam".to_string())
}

/// Append a line to the output_lines (thread-safe)
fn append_line(output_lines: &Arc<Mutex<Vec<String>>>, line: String) {
    let mut lines = output_lines.lock().unwrap();
    lines.push(line);
}

/// Append multiple lines to the output_lines (thread-safe)
fn append_lines<I: IntoIterator<Item = String>>(output_lines: &Arc<Mutex<Vec<String>>>, lines: I) {
    let mut guard = output_lines.lock().unwrap();
    guard.extend(lines);
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
        "help" => {
            append_lines(output_lines, get_help_lines());
        }
        "clear" => {
            let mut lines = output_lines.lock().unwrap();
            lines.clear();
        }
        "setup" => {
            let _ = tokio::spawn(crate::sam::setup::install());
        }
        "ls" => {
            match std::fs::read_dir(&current_dir) {
                Ok(entries) => {
                    let mut files = vec![];
                    for entry in entries {
                        if let Ok(entry) = entry {
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
                    }
                    let mut lines = vec![format!("Files in {}:", current_dir.display())];
                    lines.extend(files);
                    append_lines(output_lines, lines);
                }
                Err(e) => append_line(output_lines, format!("ls error: {}", e)),
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
                "Developed by Caleb Mitchell Smith (PixelCoda)".to_string(),
                "Licensed under GPLv3....see LICENSE file.".to_string(),
            ]);
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
            let os_ver = sysinfo::System::os_version().unwrap_or_else(|| "".to_string());
            let kernel = sysinfo::System::kernel_version().unwrap_or_else(|| "".to_string());
            let arch = std::env::consts::ARCH;
            let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_else(|| "Unknown".to_string());
            let version = format!("{:?}", crate::VERSION);

            let mut lines = vec![
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
            append_lines(output_lines, lines);
        }
        _ if cmd.starts_with("cd ") => {
            handle_cd(cmd, output_lines, current_dir);
        }
        _ if cmd.starts_with("darknet ") => {
            handle_darknet(cmd, output_lines).await;
        }
        _ if cmd.starts_with("tts ") => {
            let input = cmd.strip_prefix("tts ").unwrap().trim();
            if input.is_empty() {
                append_line(output_lines, "Usage: tts <text>".to_string());
            } else {
                append_line(output_lines, format!("Synthesizing speech for: '{}'", input));
                let output_lines = output_lines.clone();
                let text = input.to_string();
                tokio::spawn(async move {
                    match crate::sam::services::tts::get(text.clone()) {
                        Ok(wav_bytes) => {
                            // Play the WAV using rodio
                            if let Err(e) = play_wav_from_bytes(&wav_bytes) {
                                append_line(&output_lines, format!("TTS playback error: {}", e));
                            } else {
                                append_line(&output_lines, "TTS playback complete.".to_string());
                            }
                        }
                        Err(e) => {
                            append_line(&output_lines, format!("TTS error: {}", e));
                        }
                    }
                });
            }
        }
        _ if cmd.starts_with("llama ") => {
            // Parse arguments as owned Strings to avoid lifetime issues
            let rest = cmd["llama ".len()..].to_string();
            let mut split = rest.splitn(2, ' ');
            let model_path_str = split.next().unwrap_or("").to_string();
            let prompt_str = split.next().unwrap_or("").to_string();

            if model_path_str.is_empty() || prompt_str.is_empty() {
                append_line(output_lines, "Usage: llama <model_path> <prompt>".to_string());
            } else {
                let model_path = std::path::PathBuf::from(model_path_str);
                let prompt = prompt_str;
                let output_lines = output_lines.clone();
                // Run blocking code in a separate thread to avoid blocking the async runtime
                tokio::task::spawn_blocking(move || {
                    match crate::sam::services::llama::LlamaService::query(&model_path, &prompt) {
                        Ok(result) => {
                            let text = result.trim().to_string();
                            append_line(&output_lines, format!("llama: {}", text));
                            // TTS for llama reply
                            match crate::sam::services::tts::get(text.clone()) {
                                Ok(wav_bytes) => {
                                    if let Err(e) = play_wav_from_bytes(&wav_bytes) {
                                        append_line(&output_lines, format!("TTS playback error: {}", e));
                                    }
                                }
                                Err(e) => {
                                    append_line(&output_lines, format!("TTS error: {}", e));
                                }
                            }
                        },
                        Err(e) => append_line(&output_lines, format!("llama error: {}", e)),
                    }
                });
            }
        }
        _ => {
            match crate::sam::services::rivescript::query(cmd) {
                Ok(reply) => {
                    let text = reply.text.clone();
                    let output_lines = output_lines.clone();
                    // Spawn TTS and output in one async block
                    tokio::spawn(async move {
                        append_line(&output_lines, format!("┌─[sam]─> {}", text));
                        match crate::sam::services::tts::get(text.clone()) {
                            Ok(wav_bytes) => {
                                if let Err(e) = play_wav_from_bytes(&wav_bytes) {
                                    append_line(&output_lines, format!("TTS playback error: {}", e));
                                }
                            }
                            Err(e) => {
                                append_line(&output_lines, format!("TTS error: {}", e));
                            }
                        }
                    });
                }
                Err(e) => append_line(output_lines, format!("┌─[sam]─> [error: {}]", e)),
            }
        }
    }
    // Scroll to bottom if needed
    let output_window_height = output_height;
    let mut lines = output_lines.lock().unwrap();
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
        "llama <model_path> <prompt> - Query a Llama.cpp model".to_string(),
    ]
}

/// Handle 'cd' command
fn handle_cd(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &mut std::path::PathBuf) {
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
            append_line(output_lines, format!("Changed directory to {}", current_dir.display()));
        } else {
            append_line(output_lines, format!("cd: no such directory: {}", new_dir));
        }
    } else {
        append_line(output_lines, "Usage: cd <directory>".to_string());
    }
}

/// Handle 'darknet' command with spinner and async detection
async fn handle_darknet(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let image_path = parts[1].trim().to_string();
        let spinner_chars = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_running = Arc::new(Mutex::new(true));
        let spinner_flag = spinner_running.clone();
        let output_lines_clone = output_lines.clone();

        // Add a spinner line and get its index
        let spinner_index = {
            let mut lines = output_lines.lock().unwrap();
            lines.push(format!("Running darknet_detect on: {}", image_path));
            lines.push("⠋ Detecting...".to_string());
            lines.len() - 1
        };

        // Spinner thread
        let spinner_output_lines = output_lines.clone();
        thread::spawn(move || {
            let mut i = 0;
            while *spinner_flag.lock().unwrap() {
                {
                    let mut lines = spinner_output_lines.lock().unwrap();
                    if spinner_index < lines.len() {
                        lines[spinner_index] = format!("{} Detecting...", spinner_chars[i % spinner_chars.len()]);
                    }
                }
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        });

        // Async darknet task
        let spinner_flag2 = spinner_running.clone();
        tokio::spawn(async move {
            let output = crate::sam::services::darknet::darknet_detect(&image_path).await;
            let mut lines = output_lines_clone.lock().unwrap();
            *spinner_flag2.lock().unwrap() = false;
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
        append_line(output_lines, "Usage: darknet <image_path>".to_string());
    }
}

/// Play a WAV file from bytes using rodio (cross-platform)
fn play_wav_from_bytes(wav_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use rodio::{Decoder, OutputStream, Sink};

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
        println!("{}", "Postgres credentials missing:".red().bold());
        for v in missing {
            loop {
                print!("{}", format!("Enter value for {}: ", v).cyan().bold());
                io::stdout().flush().unwrap();
                let mut val = String::new();
                if io::stdin().read_line(&mut val).is_ok() {
                    let val = val.trim();
                    if (!val.is_empty()) {
                        env::set_var(v, val);
                        break;
                    }
                }
                println!("{}", format!("{} cannot be empty.", v).red());
            }
        }
    }
}
