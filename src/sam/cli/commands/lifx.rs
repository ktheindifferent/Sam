use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_lifx(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "lifx start" => {
            let config = crate::sam::services::lifx::Config::default();
            crate::sam::services::lifx::start_service(config);
            let mut out = output_lines.lock().await;
            out.push("LIFX service started.".to_string());
        }
        "lifx stop" => {
            crate::sam::services::lifx::stop_service();
            let mut out = output_lines.lock().await;
            out.push("LIFX service stopped.".to_string());
        }
        "lifx status" => {
            let status = match crate::sam::services::lifx::status_service() {
                Ok(status) => status,
                Err(e) => format!("Error: {}", e),
            };
            let mut out = output_lines.lock().await;
            out.push(format!("LIFX service status: {status}"));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown lifx command.".to_string());
        }
    }
}
