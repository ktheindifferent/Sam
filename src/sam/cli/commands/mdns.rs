use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

static MDNS_INSTANCE: Lazy<Arc<Mutex<Option<crate::sam::services::mdns::MDns>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static DISCOVER_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static BROADCAST_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn handle_mdns(cmd: &str, output_lines: Arc<Mutex<Vec<String>>>) {
    match cmd {
        "mdns discover" => {
            // Create mDNS instance if not exists
            let mut mdns_guard = MDNS_INSTANCE.lock().await;
            if mdns_guard.is_none() {
                *mdns_guard = Some(crate::sam::services::mdns::MDns::new());
            }
            let mdns = mdns_guard.as_mut().unwrap();

            // Start discover task
            let discover_handle = tokio::spawn({
                let output_lines = output_lines.clone();
                let mut mdns = mdns.clone();
                async move {
                    let _ = mdns.discover_with_output(output_lines).await;
                }
            });
            *DISCOVER_HANDLE.lock().await = Some(discover_handle);

            // Start broadcast task (implement a loop in your broadcast method)
            let broadcast_handle = tokio::spawn({
                let mut mdns = mdns.clone();
                async move {
                    let _ = mdns.broadcast_loop().await;
                }
            });
            *BROADCAST_HANDLE.lock().await = Some(broadcast_handle);

            output_lines.lock().await.push("mDNS started.".to_string());
        }
        "mdns stop" => {
            // Abort discover and broadcast tasks
            if let Some(handle) = DISCOVER_HANDLE.lock().await.take() {
                handle.abort();
            }
            if let Some(handle) = BROADCAST_HANDLE.lock().await.take() {
                handle.abort();
            }
            output_lines.lock().await.push("mDNS stopped.".to_string());
        }
        "mdns broadcast" => {
            // Start broadcast task (implement a loop in your broadcast method)
            let mdns = MDNS_INSTANCE.lock().await.clone().unwrap();
            let broadcast_handle = tokio::spawn({
                let mdns = mdns.clone();
                async move {
                    let _ = mdns.broadcast_loop().await;
                }
            });
            *BROADCAST_HANDLE.lock().await = Some(broadcast_handle);
            output_lines.lock().await.push("mDNS broadcast started.".to_string());
        }
        "mdns broadcast stop" => {
            crate::sam::services::mdns::stop_broadcast();
            if let Some(handle) = BROADCAST_HANDLE.lock().await.take() {
                handle.abort();
            }
            output_lines.lock().await.push("mDNS broadcast stopped and responder dropped.".to_string());
        }
        "mdns status" => {
            let discover_running = DISCOVER_HANDLE.lock().await.as_ref().map(|h| !h.is_finished()).unwrap_or(false);
            let broadcast_running = BROADCAST_HANDLE.lock().await.as_ref().map(|h| !h.is_finished()).unwrap_or(false);
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
