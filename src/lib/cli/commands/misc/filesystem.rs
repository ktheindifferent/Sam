use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_clear(output_lines: &Arc<Mutex<Vec<String>>>) {
    output_lines.lock().await.clear();
}

pub async fn handle_ls(output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    match std::fs::read_dir(current_dir) {
        Ok(entries) => {
            let mut files = vec![];
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_type = entry.file_type().ok();
                if let Some(ft) = file_type {
                    if ft.is_dir() {
                        files.push(format!("{file_name}/"));
                    } else {
                        files.push(file_name);
                    }
                } else {
                    files.push(file_name);
                }
            }
            let mut lines = vec![format!("Files in {}:", current_dir.display())];
            lines.extend(files);
            let mut out = output_lines.lock().await;
            out.extend(lines);
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("ls error: {e}"));
        }
    }
}

pub async fn handle_cat(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: cat <filename>".to_string());
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
            
            // Handle multiple options
            let mut show_line_numbers = false;
            let mut show_non_printing = false;
            let mut squeeze_blank = false;
            
            // Parse options
            for arg in &args[1..args.len()-1] {
                match *arg {
                    "-n" | "--number" => show_line_numbers = true,
                    "-v" | "--show-nonprinting" => show_non_printing = true,
                    "-s" | "--squeeze-blank" => squeeze_blank = true,
                    _ => {}
                }
            }
            
            let lines: Vec<&str> = content.lines().collect();
            let mut output_content = Vec::new();
            let mut blank_line_count = 0;
            
            for (i, line) in lines.iter().enumerate() {
                // Handle squeeze blank lines
                if squeeze_blank {
                    if line.trim().is_empty() {
                        blank_line_count += 1;
                        if blank_line_count > 1 {
                            continue;
                        }
                    } else {
                        blank_line_count = 0;
                    }
                }
                
                let mut formatted_line = line.to_string();
                
                // Show non-printing characters
                if show_non_printing {
                    formatted_line = formatted_line
                        .replace('\t', "^I")
                        .replace('\r', "^M");
                    
                    // Replace other non-printing characters
                    formatted_line = formatted_line
                        .chars()
                        .map(|c| {
                            if c.is_control() && c != '\n' {
                                format!("^{}", (c as u8 + 64) as char)
                            } else {
                                c.to_string()
                            }
                        })
                        .collect();
                }
                
                // Add line numbers
                if show_line_numbers {
                    formatted_line = format!("{:6}\t{}", i + 1, formatted_line);
                }
                
                output_content.push(formatted_line);
            }
            
            // If file is empty or only had blank lines that were squeezed
            if output_content.is_empty() && !content.is_empty() {
                output_content.push(String::new());
            }
            
            out.extend(output_content);
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("cat: {}: No such file or directory", filename));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("cat: {}: Permission denied", filename));
                }
                std::io::ErrorKind::InvalidData => {
                    out.push(format!("cat: {}: Invalid UTF-8 data", filename));
                }
                _ => {
                    out.push(format!("cat: {}: {}", filename, e));
                }
            }
        }
    }
}
