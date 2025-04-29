use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_tts(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let input = cmd.strip_prefix("tts ").unwrap().trim();
    if input.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("Usage: tts <text>".to_string());
    } else {
        let mut out = output_lines.lock().await;
        out.push(format!("Synthesizing speech for: '{input}'"));
        let output_lines = output_lines.clone();
        let text = input.to_string();
        tokio::spawn(crate::sam::cli::helpers::append_and_tts(output_lines, text));
    }
}
