use std::sync::Arc;
use tokio::sync::Mutex;
use std::process::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::thread;

use crate::sam::cli::helpers::{run_command_stream_lines, append_line, append_and_tts};

pub async fn handle_llama(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "llama install" => {
            let output_lines = output_lines.clone();
            output_lines
                .lock()
                .await
                .push("Starting llama model installer...".to_string());
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
                    append_line(&output_lines2, format!("cmake: {}", line));
                });

                let output_lines3 = output_lines.clone();
                let _ = run_command_stream_lines(build_cmd, move |line| {
                    append_line(&output_lines3, format!("build: {}", line));
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
                            append_line(&output_lines, format!("Installed {} to {}", bin, dst.display()));
                        }
                        Err(e) => {
                            append_line(&output_lines, format!("Failed to install {}: {}", bin, e));
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
                    let mut lines = output_lines.blocking_lock();
                    lines.push("⠋ Downloading Llama v2 and v3 models...".to_string());
                    lines.len() - 1
                };

                // Spinner thread
                let spinner_output_lines = output_lines.clone();
                thread::spawn(move || {
                    let mut i = 0;
                    while *spinner_flag.blocking_lock() {
                        {
                            let mut lines = spinner_output_lines.blocking_lock();
                            if spinner_index < lines.len() {
                                lines[spinner_index] = format!("{} Downloading Llama v2 and v3 models...", spinner_chars[i % spinner_chars.len()]);
                            }
                        }
                        i += 1;
                        std::thread::sleep(std::time::Duration::from_millis(80));
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


                append_line(&output_lines, "llama install: done.".to_string());
            });
        }
        _ if cmd.starts_with("llama v2 ") => {
            let prompt = cmd.trim_start_matches("llama v2 ").trim();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2 <prompt>".to_string());
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
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2-tiny <prompt>".to_string());
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
                let mut out = output_lines.lock().await;
                out.push("Usage: llama <model_path> <prompt>".to_string());
            } else {
                let model_path = std::path::PathBuf::from(model_path_str);
                let prompt = prompt_str;
                let output_lines = output_lines.clone();
                tokio::task::spawn_blocking(move || {
                    match crate::sam::services::llama::LlamaService::query(&model_path, &prompt) {
                        Ok(result) => {
                            let text = result.trim().to_string();
                            let output_lines = output_lines.clone();
                            tokio::spawn(crate::sam::cli::helpers::append_and_tts(
                                output_lines,
                                format!("llama: {text}"),
                            ));
                        }
                        Err(e) => {
                            let output_lines = output_lines.clone();
                            tokio::spawn(async move {
                                let mut out = output_lines.lock().await;
                                out.push(format!("llama error: {e}"));
                            });
                        }
                    }
                });
            }
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown llama command.".to_string());
        }
    }
}
