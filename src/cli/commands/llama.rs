use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_llama(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "llama install" => {
            let output_lines = output_lines.clone();
            let _ = output_lines.lock().await.push("Starting llama model installer...".to_string());
            tokio::spawn(async move {
                // ...existing code for llama install logic (cmake, build, copy, spinner, download, etc)...
                // See original file for full implementation.
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
                // ...existing code for spinner and blocking query for llama v2...
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
                // ...existing code for spinner and blocking query for llama v2-tiny...
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
                            tokio::spawn(crate::cli::helpers::append_and_tts(output_lines, format!("llama: {}", text)));
                        },
                        Err(e) => {
                            let output_lines = output_lines.clone();
                            tokio::spawn(async move {
                                let mut out = output_lines.lock().await;
                                out.push(format!("llama error: {}", e));
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
