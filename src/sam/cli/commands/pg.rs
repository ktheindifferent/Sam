use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_pg(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "pg install" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Installing PostgreSQL...",
                |lines, _| lines.push("PostgreSQL install complete.".to_string()),
                || async {
                    libsam::services::pg::install().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "pg start" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Starting PostgreSQL...",
                |lines, _| lines.push("PostgreSQL start command issued.".to_string()),
                || async {
                    libsam::services::pg::start().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "pg stop" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Stopping PostgreSQL...",
                |lines, _| lines.push("PostgreSQL stop command issued.".to_string()),
                || async {
                    libsam::services::pg::stop().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "pg status" => {
            let status = libsam::services::pg::status();
            let mut out = output_lines.lock().await;
            out.push(format!("PostgreSQL status: {status}"));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown pg command.".to_string());
        }
    }
}
