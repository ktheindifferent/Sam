use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_clear(output_lines: &Arc<Mutex<Vec<String>>>) {
    output_lines.lock().await.clear();
}

pub async fn handle_setup() {
    tokio::spawn(crate::sam::setup::install());
}

pub async fn handle_ls(output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
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
            let mut out = output_lines.lock().await;
            out.extend(lines);
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("ls error: {}", e));
        }
    }
}

pub async fn handle_version(output_lines: &Arc<Mutex<Vec<String>>>) {
    let lines = vec![
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
    ];
    let mut out = output_lines.lock().await;
    out.extend(lines);
}

pub async fn handle_default(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match crate::sam::services::rivescript::query(cmd) {
        Ok(reply) => {
            let text = reply.text.clone();
            let output_lines = output_lines.clone();
            tokio::spawn(crate::cli::helpers::append_and_tts(output_lines, format!("┌─[sam]─> {}", text)));
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("┌─[sam]─> [error: {}]", e));
        }
    }
}
