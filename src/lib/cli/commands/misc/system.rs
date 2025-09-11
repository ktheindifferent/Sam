use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_setup() {
    // tokio::spawn(crate::setup::install());
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
    match crate::services::rivescript::query(cmd) {
        Ok(reply) => {
            let text = reply.text.clone();
            let output_lines = output_lines.clone();
            tokio::spawn(crate::cli::helpers::append_and_tts(
                output_lines,
                format!("┌─[sam]─> {text}"),
            ));
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("┌─[sam]─> [error: {e}]"));
        }
    }
}
