use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(unix)]
async fn handle_ssh_unix(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    // Parse SSH command (remove "ssh " prefix if present)
    let ssh_args = if cmd.starts_with("ssh ") {
        &cmd[4..]
    } else {
        cmd
    };

    if ssh_args.trim().is_empty() {
        let mut lines = output_lines.lock().await;
        lines.push("[ssh] Usage: ssh user@host".to_string());
        return;
    }

    // Build SSH command string
    let ssh_command = format!("ssh -o StrictHostKeyChecking=ask -tt {}", ssh_args);
    
    {
        let mut lines = output_lines.lock().await;
        lines.push(format!("[ssh] Starting: {}", ssh_command));
    }

    // Use the TUI takeover to handle the SSH session
    let needs_restart = crate::cli::tui::tui_takeover_ssh_session(&ssh_command);
    
    let mut lines = output_lines.lock().await;
    lines.push("[ssh] SSH session completed".to_string());
    
    // Add special marker if TUI needs restart
    if needs_restart {
        lines.push("__TUI_RESTART_NEEDED__".to_string());
    }
}

#[cfg(windows)]
pub async fn handle_ssh_windows(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    use std::process::{Command, Stdio};
    use tokio::io::{AsyncBufReadExt, BufReader};

    let ssh_args = cmd.trim_start_matches("ssh ").trim();
    
    if ssh_args.is_empty() {
        let mut lines = output_lines.lock().await;
        lines.push("[ssh] Error: No SSH arguments provided".to_string());
        return;
    }

    let mut lines = output_lines.lock().await;
    lines.push("[ssh] Starting SSH session on Windows...".to_string());
    lines.push("[ssh] Note: Interactive SSH sessions have limited support on Windows.".to_string());
    drop(lines);

    let args: Vec<&str> = ssh_args.split_whitespace().collect();
    let mut child = match Command::new("ssh")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("[ssh] Error starting SSH: {}", e));
            lines.push("[ssh] Make sure SSH is installed and in your PATH".to_string());
            return;
        }
    };

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines_stream = reader.lines();
        
        while let Ok(Some(line)) = lines_stream.next_line().await {
            let mut lines = output_lines.lock().await;
            lines.push(format!("[ssh] {}", line));
        }
    }

    let status = child.wait();
    let mut lines = output_lines.lock().await;
    match status {
        Ok(exit_status) => {
            lines.push(format!("[ssh] Session ended with status: {}", exit_status));
        }
        Err(e) => {
            lines.push(format!("[ssh] Error waiting for process: {}", e));
        }
    }
}

pub async fn handle_ssh_command(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    #[cfg(unix)]
    {
        handle_ssh_unix(cmd, output_lines).await;
    }
    #[cfg(windows)]
    {
        handle_ssh_windows(cmd, output_lines).await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let mut lines = output_lines.lock().await;
        lines.push("[ssh] SSH is not supported on this platform.".to_string());
    }
}
