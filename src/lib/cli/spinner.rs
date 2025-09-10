use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run_with_spinner<F, Fut>(
    output_lines: &Arc<Mutex<Vec<String>>>,
    message: &str,
    done_message: impl FnOnce(&mut Vec<String>, &str) + Send + 'static,
    fut: F,
) where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = String> + Send + 'static,
{
    let output_lines = output_lines.clone();
    let message = message.to_string();
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_running = Arc::new(Mutex::new(true));
    let spinner_flag = spinner_running.clone();

    let spinner_index = {
        let mut lines = output_lines.lock().await;
        lines.push(format!("⠋ {message}"));
        lines.len() - 1
    };

    // Spinner async task
    let spinner_output_lines = output_lines.clone();
    let message_clone = message.clone();
    let spinner_flag_clone = spinner_flag.clone();
    tokio::spawn(async move {
        let mut i = 0;
        loop {
            {
                let running = spinner_flag_clone.lock().await;
                if !*running {
                    break;
                }
                let mut lines = spinner_output_lines.lock().await;
                if spinner_index < lines.len() {
                    lines[spinner_index] = format!(
                        "{} {}",
                        spinner_chars[i % spinner_chars.len()],
                        message_clone
                    );
                }
            }
            i += 1;
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    });

    // Execute future and update when done
    let spinner_flag2 = spinner_running.clone();
    let output_lines2 = output_lines.clone();
    let done_message = Box::new(done_message);
    let spinner_idx = spinner_index;

    tokio::spawn(async move {
        let result = fut().await;
        {
            let mut running = spinner_flag2.lock().await;
            *running = false;
        }
        let mut lines = output_lines2.lock().await;
        if spinner_idx < lines.len() {
            done_message(&mut lines, &result);
        }
    });
}
