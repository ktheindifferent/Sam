use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_docker(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "docker start" => {
            crate::cli::spinner::run_with_spinner(
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
            crate::cli::spinner::run_with_spinner(
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
            let mut out = output_lines.lock().await;
            out.push(format!("Docker daemon status: {}", status));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown docker command.".to_string());
        }
    }
}
