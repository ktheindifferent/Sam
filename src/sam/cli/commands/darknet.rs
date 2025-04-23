use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_darknet(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    // ...move darknet command logic here from original...
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let image_path = parts[1].trim().to_string();
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner_running = Arc::new(Mutex::new(true));
        let spinner_flag = spinner_running.clone();
        let output_lines_clone = output_lines.clone();

        let spinner_index = {
            let mut lines = output_lines.lock().await;
            lines.push(format!("Running darknet_detect on: {}", image_path));
            lines.push("⠋ Detecting...".to_string());
            lines.len() - 1
        };

        let spinner_output_lines = output_lines.clone();
        tokio::spawn(async move {
            let mut i = 0;
            while *spinner_flag.lock().await {
                {
                    let mut lines = spinner_output_lines.lock().await;
                    if spinner_index < lines.len() {
                        lines[spinner_index] = format!("{} Detecting...", spinner_chars[i % spinner_chars.len()]);
                    }
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        });

        let spinner_flag2 = spinner_running.clone();
        tokio::spawn(async move {
            let output = crate::sam::services::darknet::darknet_detect(&image_path).await;
            let mut lines = output_lines_clone.lock().await;
            *spinner_flag2.lock().await = false;
            if spinner_index < lines.len() {
                match output {
                    Ok(result) => {
                        if let Some(best) = result.objects.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)) {
                            lines[spinner_index] = format!("Detected: {}", best.name);
                        } else {
                            lines[spinner_index] = "No objects detected.".to_string();
                        }
                    }
                    Err(e) => {
                        lines[spinner_index] = format!("darknet error: {}", e);
                    }
                }
            }
        });
    } else {
        let mut out = output_lines.lock().await;
        out.push("Usage: darknet <image_path>".to_string());
    }
}
