use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_mdns(cmd: &str, output_lines: Arc<Mutex<Vec<String>>>) {
    match cmd {
        "mdns discover" => {
            crate::sam::services::mdns::start_discovery(output_lines.clone()).await;
            output_lines.lock().await.push("mDNS started.".to_string());
        }
        "mdns stop" => {
            crate::sam::services::mdns::stop_discovery().await;
            crate::sam::services::mdns::stop_broadcast_and_task().await;
            output_lines.lock().await.push("mDNS stopped.".to_string());
        }
        "mdns broadcast" => {
            let responder = libmdns::Responder::new().unwrap();
            let _svc = responder.register(
                "_tcp".to_owned(),
                "_opensam".to_owned(),
                5353,
                &["path=/"],
            );
            output_lines.lock().await.push("mDNS broadcast started.".to_string());
        }
        "mdns broadcast stop" => {
            crate::sam::services::mdns::stop_broadcast_and_task().await;
            output_lines.lock().await.push("mDNS broadcast stopped and responder dropped.".to_string());
        }
        "mdns status" => {
            let (discover_running, broadcast_running) = crate::sam::services::mdns::mdns_status().await;
            output_lines.lock().await.push(format!(
                "mDNS status: discover running: {}, broadcast running: {}",
                discover_running, broadcast_running
            ));
        }
        _ => {
            output_lines.lock().await.push("Unknown mDNS command.".to_string());
        }
    }
}
