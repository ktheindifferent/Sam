use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

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
            let mut out = output_lines.lock().await;
            out.push(format!("Changed directory to {}", current_dir.display()));
        } else {
            let mut out = output_lines.lock().await;
            out.push(format!("cd: no such directory: {new_dir}"));
        }
    } else {
        let mut out = output_lines.lock().await;
        out.push("Usage: cd <directory>".to_string());
    }
}
