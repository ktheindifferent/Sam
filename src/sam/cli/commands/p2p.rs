use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_p2p(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "p2p install" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Installing P2P service...",
                |lines, _| lines.push("P2P install complete.".to_string()),
                || async {
                    crate::sam::services::p2p::install().await;
                    "done".to_string()
                },
            ).await;
        }
        "p2p start" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Starting P2P service...",
                |lines, _| lines.push("P2P service started.".to_string()),
                || async {
                    crate::sam::services::p2p::start().await;
                    "done".to_string()
                },
            ).await;
        }
        "p2p stop" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Stopping P2P service...",
                |lines, _| lines.push("P2P service stopped.".to_string()),
                || async {
                    crate::sam::services::p2p::stop().await;
                    "done".to_string()
                },
            ).await;
        }
        "p2p status" => {
            let status = crate::sam::services::p2p::status();
            let mut out = output_lines.lock().await;
            out.push(format!("P2P status: {status}"));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown p2p command.".to_string());
        }
    }
}
