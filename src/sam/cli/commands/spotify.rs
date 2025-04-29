use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_spotify(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "spotify start" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Starting Spotify playback service...",
                |lines, _| lines.push("Spotify playback started.".to_string()),
                || async {
                    crate::sam::services::spotify::start().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "spotify stop" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Stopping Spotify playback service...",
                |lines, _| lines.push("Spotify playback stopped.".to_string()),
                || async {
                    crate::sam::services::spotify::stop().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "spotify status" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Checking Spotify playback status...",
                |lines, status| lines.push(format!("Spotify status: {status}")),
                || async { crate::sam::services::spotify::status() },
            )
            .await;
        }
        "spotify play" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Resuming Spotify playback...",
                |lines, _| lines.push("Spotify playback resumed.".to_string()),
                || async {
                    crate::sam::services::spotify::play().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "spotify pause" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Pausing Spotify playback...",
                |lines, _| lines.push("Spotify playback paused.".to_string()),
                || async {
                    crate::sam::services::spotify::pause().await;
                    "done".to_string()
                },
            )
            .await;
        }
        "spotify shuffle" => {
            crate::sam::cli::spinner::run_with_spinner(
                output_lines,
                "Toggling Spotify shuffle...",
                |lines, _| lines.push("Spotify shuffle toggled.".to_string()),
                || async {
                    crate::sam::services::spotify::shuffle().await;
                    "done".to_string()
                },
            )
            .await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown spotify command.".to_string());
        }
    }
}
