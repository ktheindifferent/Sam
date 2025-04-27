use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_redis(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "redis install" => {
            crate::sam::cli::spinner::run_with_spinner(
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
            crate::sam::cli::spinner::run_with_spinner(
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
            crate::sam::cli::spinner::run_with_spinner(
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
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Checking Redis service status...",
                |lines, status| lines.push(format!("Redis service status: {}", status)),
                || async {
                    crate::sam::services::redis::status().await.to_string()
                },
            ).await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown redis command.".to_string());
        }
    }
}
