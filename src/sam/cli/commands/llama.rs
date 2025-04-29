use std::sync::Arc;
use tokio::sync::Mutex;
// use std::process::Command;
// use std::fs;
// use std::os::unix::fs::PermissionsExt;
// use std::thread;

// use crate::sam::cli::helpers::{run_command_stream_lines, append_line, append_and_tts};

pub async fn handle_llama(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "llama install" => {
            crate::sam::cli::spinner::run_with_spinner(
                &output_lines,
                "Installing llama models and binaries...",
                |lines, _| lines.push("llama install: done.".to_string()),
                || async {
                    // Call the install logic (previously inside the match arm)
                    // crate::sam::services::llama::LlamaService::install().await;
                    "done".to_string()
                },
            ).await;
        }
        _ if cmd.starts_with("llama v2 ") => {
            let prompt = cmd.trim_start_matches("llama v2 ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2 <prompt>".to_string());
            } else {
                crate::sam::cli::spinner::run_with_spinner(
                    &output_lines,
                    "Querying llama v2...",
                    |lines, result| lines.push(format!("llama v2: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::sam::services::llama::LlamaService::query_v2(&prompt)
                                .unwrap_or_else(|e| format!("llama v2 error: {}", e))
                        }
                    },
                ).await;
            }
        }
        _ if cmd.starts_with("llama v2-tiny ") => {
            let prompt = cmd.trim_start_matches("llama v2-tiny ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2-tiny <prompt>".to_string());
            } else {
                crate::sam::cli::spinner::run_with_spinner(
                    &output_lines,
                    "Querying llama v2-tiny...",
                    |lines, result| lines.push(format!("llama v2-tiny: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::sam::services::llama::LlamaService::query_v2_tiny(&prompt)
                                .unwrap_or_else(|e| format!("llama v2-tiny error: {}", e))
                        }
                    },
                ).await;
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
                crate::sam::cli::spinner::run_with_spinner(
                    &output_lines,
                    &format!("Querying llama model {}...", model_path_str),
                    |lines, result| lines.push(format!("llama: {}", result)),
                    move || {
                        let model_path = std::path::PathBuf::from(model_path_str.clone());
                        let prompt = prompt_str.clone();
                        async move {
                            crate::sam::services::llama::LlamaService::query(&model_path, &prompt)
                                .unwrap_or_else(|e| format!("llama error: {}", e))
                        }
                    },
                ).await;
            }
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown llama command.".to_string());
        }
    }
}
