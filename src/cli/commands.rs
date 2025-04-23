use super::{helpers, spinner};
use std::path::PathBuf;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
pub async fn handle_command(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &mut PathBuf,
    human_name: &str,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    match cmd {
        "help" => helpers::append_lines(output_lines, get_help_lines()).await,
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
                    helpers::append_lines(output_lines, lines).await;
                }
                Err(e) => helpers::append_line(output_lines, format!("ls error: {}", e)).await,
            }
        }
        "version" => {
            helpers::append_lines(output_lines, vec![
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
            helpers::append_lines(output_lines, lines).await;
        }
        "crawler start" => {
            crate::sam::services::crawler::start_service_async().await;
            helpers::append_line(output_lines, "Crawler service started.".to_string()).await;
        }
        "crawler stop" => {
            spinner::run_with_spinner(
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
            spinner::run_with_spinner(
                output_lines,
                "Checking crawler service status...",
                |lines, status| lines.push(format!("Crawler service status: {}", status)),
                || async {
                    crate::sam::services::crawler::service_status().to_string()
                },
            ).await;
        }
        "redis install" => {
            spinner::run_with_spinner(
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
            spinner::run_with_spinner(
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
            spinner::run_with_spinner(
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
            spinner::run_with_spinner(
                output_lines,
                "Checking Redis service status...",
                |lines, status| lines.push(format!("Redis service status: {}", status)),
                || async {
                    crate::sam::services::redis::status().to_string()
                },
            ).await;
        }
        // --- Postgres service commands ---
        "pg install" => {
            spinner::run_with_spinner(
                output_lines,
                "Installing PostgreSQL...",
                |lines, _| lines.push("PostgreSQL install complete.".to_string()),
                || async {
                    crate::sam::services::pg::install().await;
                    "done".to_string()
                },
            ).await;
        }
        "pg start" => {
            spinner::run_with_spinner(
                output_lines,
                "Starting PostgreSQL...",
                |lines, _| lines.push("PostgreSQL start command issued.".to_string()),
                || async {
                    crate::sam::services::pg::start().await;
                    "done".to_string()
                },
            ).await;
        }
        "pg stop" => {
            spinner::run_with_spinner(
                output_lines,
                "Stopping PostgreSQL...",
                |lines, _| lines.push("PostgreSQL stop command issued.".to_string()),
                || async {
                    crate::sam::services::pg::stop().await;
                    "done".to_string()
                },
            ).await;
        }
        "pg status" => {
            let status = crate::sam::services::pg::status();
            helpers::append_line(output_lines, format!("PostgreSQL status: {}", status)).await;
        }
        "docker start" => {
            spinner::run_with_spinner(
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
            spinner::run_with_spinner(
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
            helpers::append_line(output_lines, format!("Docker daemon status: {}", status)).await;
        }
        "spotify start" => {
            spinner::run_with_spinner(
                output_lines,
                "Starting Spotify playback service...",
                |lines, _| lines.push("Spotify playback started.".to_string()),
                || async {
                    crate::sam::services::spotify::start().await;
                    "done".to_string()
                },
            ).await;
        }
        "spotify stop" => {
            spinner::run_with_spinner(
                output_lines,
                "Stopping Spotify playback service...",
                |lines, _| lines.push("Spotify playback stopped.".to_string()),
                || async {
                    crate::sam::services::spotify::stop().await;
                    "done".to_string()
                },
            ).await;
        }
        "spotify status" => {
            spinner::run_with_spinner(
                output_lines,
                "Checking Spotify playback status...",
                |lines, status| lines.push(format!("Spotify status: {}", status)),
                || async {
                    crate::sam::services::spotify::status()
                },
            ).await;
        }
        "spotify play" => {
            spinner::run_with_spinner(
                output_lines,
                "Resuming Spotify playback...",
                |lines, _| lines.push("Spotify playback resumed.".to_string()),
                || async {
                    crate::sam::services::spotify::play().await;
                    "done".to_string()
                },
            ).await;
        }
        "spotify pause" => {
            spinner::run_with_spinner(
                output_lines,
                "Pausing Spotify playback...",
                |lines, _| lines.push("Spotify playback paused.".to_string()),
                || async {
                    crate::sam::services::spotify::pause().await;
                    "done".to_string()
                },
            ).await;
        }
        "spotify shuffle" => {
            spinner::run_with_spinner(
                output_lines,
                "Toggling Spotify shuffle...",
                |lines, _| lines.push("Spotify shuffle toggled.".to_string()),
                || async {
                    crate::sam::services::spotify::shuffle().await;
                    "done".to_string()
                },
            ).await;
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
                helpers::append_line(output_lines, "Usage: tts <text>".to_string()).await;
            } else {
                helpers::append_line(output_lines, format!("Synthesizing speech for: '{}'", input)).await;
                let output_lines = output_lines.clone();
                let text = input.to_string();
                tokio::spawn(helpers::append_and_tts(output_lines, text));
            }
        }
        "llama install" => {
            helpers::append_line(output_lines, "Starting llama model installer...".to_string()).await;
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
                let _ = helpers::run_command_stream_lines(cmake_cmd, move |line| {
                    let value = output_lines2.clone();
                    tokio::spawn(async move {
                        helpers::append_line(&value, format!("cmake: {}", line)).await;
                    });
                });

                let output_lines3 = output_lines.clone();
                let _ = helpers::run_command_stream_lines(build_cmd, move |line| {
                    let value = output_lines3.clone();
                    tokio::spawn(async move {
                        helpers::append_line(&value, format!("build: {}", line)).await;
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
                            helpers::append_line(&output_lines, format!("Installed {} to {}", bin, dst.display())).await;
                        }
                        Err(e) => {
                            helpers::append_line(&output_lines, format!("Failed to install {}: {}", bin, e)).await;
                        }
                    }
                }

                // Show spinner while downloading models (blocking)
                let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner_running = Arc::new(Mutex::new(true));
                let spinner_flag = spinner_running.clone();

                // Clone output_lines for each async block to avoid move errors
                let output_lines_spinner = output_lines.clone();
                let output_lines_download = output_lines.clone();

                // Add spinner line and get its index
                let spinner_index = {
                    let mut lines = output_lines_spinner.lock().await;
                    lines.push("⠋ Downloading Llama v2 and v3 models...".to_string());
                    lines.len() - 1
                };

                // Spinner thread
                tokio::spawn(async move {
                    let mut i = 0;
                    while *spinner_flag.lock().await {
                        {
                            let mut lines = output_lines_spinner.lock().await;
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
                let spinner_index2 = spinner_index;
                tokio::task::spawn_blocking(move || {
                    let v2_result = crate::sam::services::llama::LlamaService::download_v2_model();
                    let v3_result = crate::sam::services::llama::LlamaService::download_v3_model();

                    *spinner_flag2.blocking_lock() = false;
                    let mut lines = output_lines_download.blocking_lock();
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

                helpers::append_line(&output_lines, "llama install: done.".to_string()).await;
            });
        }
        _ if cmd.starts_with("llama v2 ") => {
            let prompt = cmd.trim_start_matches("llama v2 ").trim();
            if prompt.is_empty() {
            helpers::append_line(output_lines, "Usage: llama v2 <prompt>".to_string()).await;
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
                    tokio::spawn(helpers::append_and_tts(output_lines, format!("llama v2: {}", text)));
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
            helpers::append_line(output_lines, "Usage: llama v2-tiny <prompt>".to_string()).await;
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
                    tokio::spawn(helpers::append_and_tts(output_lines, format!("llama v2-tiny: {}", text)));
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
                helpers::append_line(output_lines, "Usage: llama <model_path> <prompt>".to_string()).await;
            } else {
                let model_path = std::path::PathBuf::from(model_path_str);
                let prompt = prompt_str;
                let output_lines = output_lines.clone();
                tokio::task::spawn_blocking(move || {
                    match crate::sam::services::llama::LlamaService::query(&model_path, &prompt) {
                        Ok(result) => {
                            let text = result.trim().to_string();
                            let output_lines = output_lines.clone();
                            tokio::spawn(helpers::append_and_tts(output_lines, format!("llama: {}", text)));
                        },
                        Err(e) => {
                            let output_lines = output_lines.clone();
                            tokio::spawn(async move {
                                helpers::append_line(&output_lines, format!("llama error: {}", e)).await;
                            });
                        }
                    }
                });
            }
        }
        "lifx start" => {
            crate::sam::services::lifx::start_service();
            helpers::append_line(output_lines, "LIFX service started.".to_string()).await;
        }
        "lifx stop" => {
            crate::sam::services::lifx::stop_service();
            helpers::append_line(output_lines, "LIFX service stopped.".to_string()).await;
        }
        "lifx status" => {
            let status = crate::sam::services::lifx::status_service();
            helpers::append_line(output_lines, format!("LIFX service status: {}", status)).await;
        }
        _ if cmd.starts_with("crawl search ") => {
            let query = cmd.trim_start_matches("crawl search ").trim();
            if query.is_empty() {
                helpers::append_line(output_lines, "Usage: crawl search <query>".to_string()).await;
            } else {
                let query = query.to_string();
                let output_lines = output_lines.clone();
                tokio::spawn(async move {
                    use crate::sam::services::crawler::CrawledPage;
                    match CrawledPage::query_by_relevance_async(&query, 10).await {
                        Ok(scored_pages) if !scored_pages.is_empty() => {
                            helpers::append_line(&output_lines, format!("Found {} results:", scored_pages.len())).await;
                            for (page, score) in scored_pages {
                                helpers::append_line(&output_lines, format!("URL: {}", page.url)).await;
                                helpers::append_line(&output_lines, format!("Score: {}", score)).await;
                                if !page.tokens.is_empty() {
                                    let snippet: String = page.tokens.iter().take(20).cloned().collect::<Vec<_>>().join(" ");
                                    helpers::append_line(&output_lines, format!("Tokens: {}...", snippet)).await;
                                }
                                helpers::append_line(&output_lines, "-----------------------------".to_string()).await;
                            }
                        }
                        Ok(_) => helpers::append_line(&output_lines, "No results found.".to_string()).await,
                        Err(e) => helpers::append_line(&output_lines, format!("Search error: {}", e)).await,
                    }
                });
            }
        }
        _ => {
            match crate::sam::services::rivescript::query(cmd) {
                Ok(reply) => {
                    let text = reply.text.clone();
                    let output_lines = output_lines.clone();
                    tokio::spawn(helpers::append_and_tts(output_lines, format!("┌─[sam]─> {}", text)));
                }
                Err(e) => helpers::append_line(output_lines, format!("┌─[sam]─> [error: {}]", e)).await,
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

pub async fn handle_cd(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &mut PathBuf) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let new_dir = parts[1].trim();
        let new_path = if new_dir.starts_with('/') {
            PathBuf::from(new_dir)
        } else {
            current_dir.join(new_dir)
        };
        if new_path.is_dir() {
            *current_dir = new_path.canonicalize().unwrap_or(new_path);
            helpers::append_line(output_lines, format!("Changed directory to {}", current_dir.display())).await;
        } else {
            helpers::append_line(output_lines, format!("cd: no such directory: {}", new_dir)).await;
        }
    } else {
        helpers::append_line(output_lines, "Usage: cd <directory>".to_string()).await;
    }
}

pub async fn handle_darknet(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
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
        helpers::append_line(output_lines, "Usage: darknet <image_path>".to_string()).await;
    }
}

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
        "spotify start             - Start Spotify playback service".to_string(),
        "spotify stop              - Stop Spotify playback service".to_string(),
        "spotify status            - Show Spotify playback status".to_string(),
        "spotify play              - Resume Spotify playback".to_string(),
        "spotify pause             - Pause Spotify playback".to_string(),
        "spotify shuffle           - Toggle Spotify shuffle mode".to_string(),
    ]
}