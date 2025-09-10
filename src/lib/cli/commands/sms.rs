use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_sms(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "sms start" => {
            crate::cli::spinner::run_with_spinner(
                output_lines,
                "Starting SMS service...",
                |lines, _| lines.push("SMS service started.".to_string()),
                || async {
                    crate::services::sms::start().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "sms stop" => {
            crate::cli::spinner::run_with_spinner(
                output_lines,
                "Stopping SMS service...",
                |lines, _| lines.push("SMS service stopped.".to_string()),
                || async {
                    crate::services::sms::stop().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "sms status" => {
            let status = crate::services::sms::status();
            let mut out = output_lines.lock().await;
            out.push(format!("SMS service status: {status}"));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown sms command.".to_string());
        }
    }
}
