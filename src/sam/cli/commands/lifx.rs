use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_lifx(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "lifx start" => {
            crate::sam::services::lifx::start_service();
            let mut out = output_lines.lock().await;
            out.push("LIFX service started.".to_string());
        }
        "lifx stop" => {
            crate::sam::services::lifx::stop_service();
            let mut out = output_lines.lock().await;
            out.push("LIFX service stopped.".to_string());
        }
        "lifx status" => {
            let status = crate::sam::services::lifx::status_service();
            let mut out = output_lines.lock().await;
            out.push(format!("LIFX service status: {}", status));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown lifx command.".to_string());
        }
    }
}
