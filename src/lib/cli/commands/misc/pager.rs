use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_less(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: less <filename>".to_string());
        out.push(
            "Commands: j/k (scroll), q (quit), g/G (go to start/end), /<search> (search)"
                .to_string(),
        );
        return;
    }

    let filename = args[1];
    let file_path = if filename.starts_with('/') {
        // Absolute path
        PathBuf::from(filename)
    } else {
        // Relative path
        current_dir.join(filename)
    };

    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let mut out = output_lines.lock().await;

            // Parse options
            let mut show_line_numbers = false;
            let mut case_insensitive_search = false;

            for arg in &args[1..args.len() - 1] {
                match *arg {
                    "-n" | "--line-numbers" => show_line_numbers = true,
                    "-i" | "--ignore-case" => case_insensitive_search = true,
                    _ => {}
                }
            }

            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();

            // Calculate visible area (reserve space for status line)
            let visible_lines = if output_height > 3 {
                output_height - 3
            } else {
                1
            };
            let start_line = (*scroll_offset as usize).min(total_lines.saturating_sub(1));
            let end_line = (start_line + visible_lines).min(total_lines);

            // Clear current output
            out.clear();

            // Add header
            out.push(format!(
                "=== {} (lines {}-{} of {}) ===",
                filename,
                start_line + 1,
                end_line,
                total_lines
            ));

            // Display visible portion of file
            for (i, line_content) in lines
                .iter()
                .enumerate()
                .skip(start_line)
                .take(visible_lines)
            {
                let formatted_line = if show_line_numbers {
                    format!("{:6}: {}", i + 1, line_content)
                } else {
                    line_content.to_string()
                };
                out.push(formatted_line);
            }

            // Add status/help line
            let progress_percent = if total_lines > 0 {
                ((end_line as f32 / total_lines as f32) * 100.0) as u16
            } else {
                100
            };

            if total_lines <= visible_lines {
                out.push("(END) - Use 'q' to quit, 'less <filename>' to view again".to_string());
            } else {
                let progress_str = if end_line >= total_lines {
                    "100".to_string()
                } else {
                    progress_percent.to_string()
                };
                out.push(format!(
                    ":{}-{}% - j/k:scroll, g/G:start/end, q:quit, /<pattern>:search",
                    progress_percent.min(100),
                    progress_str
                ));
            }

            // Reset scroll offset if we've reached the end
            if end_line >= total_lines && *scroll_offset > 0 {
                *scroll_offset = (total_lines.saturating_sub(visible_lines)) as u16;
            }
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("less: {}: No such file or directory", filename));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("less: {}: Permission denied", filename));
                }
                std::io::ErrorKind::InvalidData => {
                    out.push(format!("less: {}: Invalid UTF-8 data", filename));
                }
                _ => {
                    out.push(format!("less: {}: {}", filename, e));
                }
            }
        }
    }
}

// Helper function for less navigation commands
pub async fn handle_less_nav(
    cmd: &str,
    filename: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    match cmd {
        "j" | "down" => {
            // Scroll down one line
            let file_path = if filename.starts_with('/') {
                PathBuf::from(filename)
            } else {
                current_dir.join(filename)
            };

            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let total_lines = content.lines().count();
                let visible_lines = if output_height > 3 {
                    output_height - 3
                } else {
                    1
                };

                if (*scroll_offset as usize) + visible_lines < total_lines {
                    *scroll_offset += 1;
                }
            }

            // Refresh display
            handle_less(
                &format!("less {}", filename),
                output_lines,
                current_dir,
                output_height,
                scroll_offset,
            )
            .await;
        }
        "k" | "up" => {
            // Scroll up one line
            if *scroll_offset > 0 {
                *scroll_offset -= 1;
            }

            // Refresh display
            handle_less(
                &format!("less {}", filename),
                output_lines,
                current_dir,
                output_height,
                scroll_offset,
            )
            .await;
        }
        "g" | "home" => {
            // Go to beginning
            *scroll_offset = 0;
            handle_less(
                &format!("less {}", filename),
                output_lines,
                current_dir,
                output_height,
                scroll_offset,
            )
            .await;
        }
        "G" | "end" => {
            // Go to end
            let file_path = if filename.starts_with('/') {
                PathBuf::from(filename)
            } else {
                current_dir.join(filename)
            };

            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let total_lines = content.lines().count();
                let visible_lines = if output_height > 3 {
                    output_height - 3
                } else {
                    1
                };
                *scroll_offset = total_lines.saturating_sub(visible_lines) as u16;
            }

            handle_less(
                &format!("less {}", filename),
                output_lines,
                current_dir,
                output_height,
                scroll_offset,
            )
            .await;
        }
        "q" | "quit" => {
            // Quit less view
            let mut out = output_lines.lock().await;
            out.clear();
            out.push("Exited less viewer.".to_string());
        }
        _ if cmd.starts_with('/') => {
            // Search functionality
            let search_term = &cmd[1..];
            if !search_term.is_empty() {
                handle_less_search(
                    filename,
                    search_term,
                    output_lines,
                    current_dir,
                    output_height,
                    scroll_offset,
                )
                .await;
            }
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push(format!(
                "Unknown less command: {}. Use j/k to scroll, g/G for start/end, q to quit.",
                cmd
            ));
        }
    }
}

// Helper function for search in less
async fn handle_less_search(
    filename: &str,
    search_term: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    let file_path = if filename.starts_with('/') {
        PathBuf::from(filename)
    } else {
        current_dir.join(filename)
    };

    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let current_line = *scroll_offset as usize;

            // Search from current position forward
            for (i, line) in lines.iter().enumerate().skip(current_line) {
                if line.to_lowercase().contains(&search_term.to_lowercase()) {
                    *scroll_offset = i as u16;
                    handle_less(
                        &format!("less {}", filename),
                        output_lines,
                        current_dir,
                        output_height,
                        scroll_offset,
                    )
                    .await;

                    let mut out = output_lines.lock().await;
                    let current_len = out.len();
                    if current_len > 0 {
                        // Replace status line with search result
                        out[current_len - 1] = format!(
                            "Found '{}' at line {} - j/k:scroll, n:next, q:quit",
                            search_term,
                            i + 1
                        );
                    }
                    return;
                }
            }

            // If not found from current position, search from beginning
            for (i, line) in lines.iter().enumerate() {
                if i >= current_line {
                    break;
                }
                if line.to_lowercase().contains(&search_term.to_lowercase()) {
                    *scroll_offset = i as u16;
                    handle_less(
                        &format!("less {}", filename),
                        output_lines,
                        current_dir,
                        output_height,
                        scroll_offset,
                    )
                    .await;

                    let mut out = output_lines.lock().await;
                    let current_len = out.len();
                    if current_len > 0 {
                        out[current_len - 1] = format!(
                            "Found '{}' at line {} (wrapped) - j/k:scroll, n:next, q:quit",
                            search_term,
                            i + 1
                        );
                    }
                    return;
                }
            }

            // Not found
            let mut out = output_lines.lock().await;
            out.push(format!("Pattern '{}' not found", search_term));
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("Search error: {}", e));
        }
    }
}
