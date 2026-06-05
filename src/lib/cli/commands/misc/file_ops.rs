use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

/// Handle the touch command - create empty files or update timestamps
pub async fn handle_touch(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: touch [-c] [-t timestamp] <file>...".to_string());
        return;
    }

    let mut no_create = false;
    let mut set_time = None;
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-c" | "--no-create" => no_create = true,
            "-t" | "--time" => {
                if i + 1 < args.len() {
                    // Simple timestamp parsing - in real implementation would parse YYYYMMDDHHMM format
                    set_time = Some(SystemTime::now());
                    i += 1; // Skip timestamp argument
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("touch: option '-t' requires an argument".to_string());
                    return;
                }
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }

    if file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("touch: missing file operand".to_string());
        return;
    }

    let mut out = output_lines.lock().await;

    for file_name in file_args {
        let file_path = resolve_path(file_name, current_dir);

        if file_path.exists() {
            // Update timestamps
            match update_file_times(&file_path, set_time) {
                Ok(()) => {
                    // Success - no output for touch unless verbose
                }
                Err(e) => {
                    out.push(format!("touch: {}: {}", file_name, e));
                }
            }
        } else if !no_create {
            // Create new file
            match std::fs::File::create(&file_path) {
                Ok(_) => {
                    // Success - no output for touch unless verbose
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        out.push(format!("touch: {}: Permission denied", file_name));
                    }
                    std::io::ErrorKind::NotFound => {
                        out.push(format!("touch: {}: No such file or directory", file_name));
                    }
                    _ => {
                        out.push(format!("touch: {}: {}", file_name, e));
                    }
                },
            }
        }
    }
}

/// Handle the head command - show first N lines of files
pub async fn handle_head(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: head [-n lines] <file>...".to_string());
        return;
    }

    let mut line_count = 10; // default
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-n" | "--lines" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(n) => line_count = n,
                        Err(_) => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("head: invalid line count: {}", args[i + 1]));
                            return;
                        }
                    }
                    i += 1; // Skip the number argument
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("head: option '-n' requires an argument".to_string());
                    return;
                }
            }
            _ if args[i].starts_with('-') && args[i].len() > 1 => {
                // Handle -N format (e.g., -5)
                match args[i][1..].parse::<usize>() {
                    Ok(n) => line_count = n,
                    Err(_) => {
                        let mut out = output_lines.lock().await;
                        out.push(format!("head: invalid option: {}", args[i]));
                        return;
                    }
                }
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }

    if file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("head: missing file operand".to_string());
        return;
    }

    let mut out = output_lines.lock().await;
    let show_headers = file_args.len() > 1;

    for (file_idx, file_name) in file_args.iter().enumerate() {
        let file_path = resolve_path(file_name, current_dir);

        if show_headers {
            if file_idx > 0 {
                out.push(String::new()); // blank line between files
            }
            out.push(format!("==> {} <==", file_name));
        }

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let end_line = std::cmp::min(line_count, lines.len());
                out.extend(lines[..end_line].iter().map(|s| s.to_string()));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("head: {}: No such file or directory", file_name));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("head: {}: Permission denied", file_name));
                }
                std::io::ErrorKind::IsADirectory => {
                    out.push(format!("head: {}: Is a directory", file_name));
                }
                _ => {
                    out.push(format!("head: {}: {}", file_name, e));
                }
            },
        }
    }
}

/// Handle the tail command - show last N lines of files
pub async fn handle_tail(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: tail [-n lines] [-f] <file>...".to_string());
        return;
    }

    let mut line_count = 10; // default
    let mut follow = false;
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-n" | "--lines" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(n) => line_count = n,
                        Err(_) => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("tail: invalid line count: {}", args[i + 1]));
                            return;
                        }
                    }
                    i += 1; // Skip the number argument
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("tail: option '-n' requires an argument".to_string());
                    return;
                }
            }
            "-f" | "--follow" => follow = true,
            _ if args[i].starts_with('-') && args[i].len() > 1 && args[i] != "-f" => {
                // Handle -N format (e.g., -5)
                match args[i][1..].parse::<usize>() {
                    Ok(n) => line_count = n,
                    Err(_) => {
                        let mut out = output_lines.lock().await;
                        out.push(format!("tail: invalid option: {}", args[i]));
                        return;
                    }
                }
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }

    if file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("tail: missing file operand".to_string());
        return;
    }

    let mut out = output_lines.lock().await;
    let show_headers = file_args.len() > 1;

    for (file_idx, file_name) in file_args.iter().enumerate() {
        let file_path = resolve_path(file_name, current_dir);

        if show_headers {
            if file_idx > 0 {
                out.push(String::new()); // blank line between files
            }
            out.push(format!("==> {} <==", file_name));
        }

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start_line = lines.len().saturating_sub(line_count);
                out.extend(lines[start_line..].iter().map(|s| s.to_string()));

                if follow {
                    out.push(format!(
                        "tail: following {} (Ctrl+C to stop - not implemented in CLI)",
                        file_name
                    ));
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("tail: {}: No such file or directory", file_name));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("tail: {}: Permission denied", file_name));
                }
                std::io::ErrorKind::IsADirectory => {
                    out.push(format!("tail: {}: Is a directory", file_name));
                }
                _ => {
                    out.push(format!("tail: {}: {}", file_name, e));
                }
            },
        }
    }
}

/// Handle the find command - search for files and directories
pub async fn handle_find(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: find <path> [-name pattern] [-type f|d] [-maxdepth N]".to_string());
        return;
    }

    let start_path = resolve_path(args[1], current_dir);
    let mut name_pattern = None;
    let mut file_type = None; // None = both, Some('f') = files, Some('d') = directories
    let mut max_depth = None;
    let mut i = 2;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-name" => {
                if i + 1 < args.len() {
                    name_pattern = Some(args[i + 1]);
                    i += 1;
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("find: option '-name' requires an argument".to_string());
                    return;
                }
            }
            "-type" => {
                if i + 1 < args.len() {
                    match args[i + 1] {
                        "f" => file_type = Some('f'),
                        "d" => file_type = Some('d'),
                        _ => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("find: invalid type '{}'", args[i + 1]));
                            return;
                        }
                    }
                    i += 1;
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("find: option '-type' requires an argument".to_string());
                    return;
                }
            }
            "-maxdepth" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(depth) => max_depth = Some(depth),
                        Err(_) => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("find: invalid maxdepth '{}'", args[i + 1]));
                            return;
                        }
                    }
                    i += 1;
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("find: option '-maxdepth' requires an argument".to_string());
                    return;
                }
            }
            _ => {
                let mut out = output_lines.lock().await;
                out.push(format!("find: unknown option '{}'", args[i]));
                return;
            }
        }
        i += 1;
    }

    if !start_path.exists() {
        let mut out = output_lines.lock().await;
        out.push(format!("find: {}: No such file or directory", args[1]));
        return;
    }

    let mut out = output_lines.lock().await;
    let mut results = Vec::new();

    // Perform the search
    if let Err(e) = find_recursive(
        &start_path,
        &start_path,
        name_pattern,
        file_type,
        max_depth,
        0,
        &mut results,
    ) {
        out.push(format!("find: {}", e));
        return;
    }

    results.sort();
    out.extend(results);
}

/// Recursive helper for find command
fn find_recursive(
    root: &PathBuf,
    current: &PathBuf,
    name_pattern: Option<&str>,
    file_type: Option<char>,
    max_depth: Option<usize>,
    current_depth: usize,
    results: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check depth limit
    if let Some(max) = max_depth {
        if current_depth > max {
            return Ok(());
        }
    }

    // Check if current path matches criteria
    let metadata = std::fs::metadata(current)?;
    let is_dir = metadata.is_dir();
    let is_file = metadata.is_file();

    let type_matches = match file_type {
        Some('f') => is_file,
        Some('d') => is_dir,
        _ => true, // Both files and directories
    };

    let name_matches = if let Some(pattern) = name_pattern {
        if let Some(name) = current.file_name() {
            let name_str = name.to_string_lossy();
            simple_glob_match(pattern, &name_str)
        } else {
            false
        }
    } else {
        true
    };

    if type_matches && name_matches {
        let relative_path = current.strip_prefix(root).unwrap_or(current);
        results.push(relative_path.display().to_string());
    }

    // Recurse into directories
    if is_dir {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Err(_) = find_recursive(
                    root,
                    &path,
                    name_pattern,
                    file_type,
                    max_depth,
                    current_depth + 1,
                    results,
                ) {
                    // Ignore permission errors and continue
                    continue;
                }
            }
        }
    }

    Ok(())
}

/// Simple glob pattern matching (supports * and ?)
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    fn match_recursive(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
        if pi == pattern.len() {
            return ti == text.len();
        }

        if ti == text.len() {
            // Check if remaining pattern is all *
            return pattern[pi..].iter().all(|&c| c == '*');
        }

        match pattern[pi] {
            '*' => {
                // Try matching zero or more characters
                for skip in 0..=(text.len() - ti) {
                    if match_recursive(pattern, text, pi + 1, ti + skip) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // Match exactly one character
                match_recursive(pattern, text, pi + 1, ti + 1)
            }
            c => {
                // Match exact character
                if text[ti] == c {
                    match_recursive(pattern, text, pi + 1, ti + 1)
                } else {
                    false
                }
            }
        }
    }

    match_recursive(&pattern_chars, &text_chars, 0, 0)
}

/// Cross-platform file time update
#[cfg(unix)]
fn update_file_times(
    path: &PathBuf,
    _set_time: Option<SystemTime>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::MetadataExt;

    let metadata = std::fs::metadata(path)?;
    let atime = metadata.atime();
    let mtime = metadata.mtime();

    // On Unix systems, we'd use utime or utimensat
    // For this implementation, we'll just update using file operations
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)?;

    // This is a simplified approach - in a real implementation,
    // we'd use proper system calls to set timestamps
    std::io::Write::flush(&mut file)?;

    Ok(())
}

/// Cross-platform file time update for Windows
#[cfg(windows)]
fn update_file_times(
    path: &PathBuf,
    _set_time: Option<SystemTime>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::OpenOptions;

    // On Windows, opening and closing the file will update the access time
    let _file = OpenOptions::new().write(true).append(true).open(path)?;

    Ok(())
}

/// Cross-platform file time update fallback
#[cfg(not(any(unix, windows)))]
fn update_file_times(
    _path: &PathBuf,
    _set_time: Option<SystemTime>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fallback for other platforms - just return success
    Ok(())
}

/// Helper function to resolve relative/absolute paths
fn resolve_path(path: &str, current_dir: &PathBuf) -> PathBuf {
    if path.starts_with('/')
        || (cfg!(windows) && path.len() > 1 && path.chars().nth(1) == Some(':'))
    {
        PathBuf::from(path)
    } else {
        current_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_glob_match() {
        assert!(simple_glob_match("*.txt", "file.txt"));
        assert!(simple_glob_match("test.*", "test.rs"));
        assert!(simple_glob_match("*.rs", "main.rs"));
        assert!(simple_glob_match("test?", "test1"));
        assert!(simple_glob_match("*", "anything"));

        assert!(!simple_glob_match("*.txt", "file.rs"));
        assert!(!simple_glob_match("test?", "test12"));
        assert!(!simple_glob_match("exact", "different"));
    }
}
