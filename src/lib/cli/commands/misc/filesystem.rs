use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_clear(output_lines: &Arc<Mutex<Vec<String>>>) {
    output_lines.lock().await.clear();
}

pub async fn handle_pwd(output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let mut out = output_lines.lock().await;
    out.push(current_dir.display().to_string());
}

pub async fn handle_mkdir(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: mkdir [-p] <directory>...".to_string());
        return;
    }

    let mut create_parents = false;
    let mut dir_args = Vec::new();

    // Parse arguments
    for arg in &args[1..] {
        match *arg {
            "-p" | "--parents" => create_parents = true,
            _ => dir_args.push(*arg),
        }
    }

    if dir_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("mkdir: missing operand".to_string());
        return;
    }

    let mut out = output_lines.lock().await;

    for dir_name in dir_args {
        let dir_path = if dir_name.starts_with('/') {
            // Absolute path
            PathBuf::from(dir_name)
        } else {
            // Relative path
            current_dir.join(dir_name)
        };

        let result = if create_parents {
            std::fs::create_dir_all(&dir_path)
        } else {
            std::fs::create_dir(&dir_path)
        };

        match result {
            Ok(()) => {
                out.push(format!("Created directory: {}", dir_path.display()));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => {
                    out.push(format!("mkdir: {}: File exists", dir_name));
                }
                std::io::ErrorKind::NotFound => {
                    out.push(format!("mkdir: {}: No such file or directory", dir_name));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("mkdir: {}: Permission denied", dir_name));
                }
                _ => {
                    out.push(format!("mkdir: {}: {}", dir_name, e));
                }
            },
        }
    }
}

pub async fn handle_rmdir(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: rmdir <directory>...".to_string());
        return;
    }

    let mut out = output_lines.lock().await;

    for dir_name in &args[1..] {
        let dir_path = if dir_name.starts_with('/') {
            // Absolute path
            PathBuf::from(dir_name)
        } else {
            // Relative path
            current_dir.join(dir_name)
        };

        match std::fs::remove_dir(&dir_path) {
            Ok(()) => {
                out.push(format!("Removed directory: {}", dir_path.display()));
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        out.push(format!("rmdir: {}: No such file or directory", dir_name));
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        out.push(format!("rmdir: {}: Permission denied", dir_name));
                    }
                    std::io::ErrorKind::Other => {
                        // Directory not empty is usually reported as "Other"
                        out.push(format!("rmdir: {}: Directory not empty", dir_name));
                    }
                    _ => {
                        out.push(format!("rmdir: {}: {}", dir_name, e));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_handle_pwd() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/Users/test/directory");

        handle_pwd(&output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "/Users/test/directory");
    }

    #[tokio::test]
    async fn test_handle_mkdir_usage() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/tmp");

        handle_mkdir("mkdir", &output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Usage: mkdir"));
    }

    #[tokio::test]
    async fn test_handle_rmdir_usage() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/tmp");

        handle_rmdir("rmdir", &output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Usage: rmdir"));
    }

    #[tokio::test]
    async fn test_handle_cp_usage() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/tmp");

        handle_cp("cp", &output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Usage: cp"));
    }

    #[tokio::test]
    async fn test_handle_mv_usage() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/tmp");

        handle_mv("mv", &output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Usage: mv"));
    }

    #[tokio::test]
    async fn test_handle_rm_usage() {
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let test_dir = PathBuf::from("/tmp");

        handle_rm("rm", &output_lines, &test_dir).await;

        let result = output_lines.lock().await;
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Usage: rm"));
    }
}

pub async fn handle_cp(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 3 {
        let mut out = output_lines.lock().await;
        out.push("Usage: cp [-r] <source> <destination>".to_string());
        return;
    }

    let mut recursive = false;
    let mut file_args = Vec::new();

    // Parse arguments
    for arg in &args[1..] {
        match *arg {
            "-r" | "--recursive" => recursive = true,
            _ => file_args.push(*arg),
        }
    }

    if file_args.len() != 2 {
        let mut out = output_lines.lock().await;
        out.push("cp: exactly two file arguments required".to_string());
        return;
    }

    let source = file_args[0];
    let destination = file_args[1];

    let source_path = if source.starts_with('/') {
        PathBuf::from(source)
    } else {
        current_dir.join(source)
    };

    let dest_path = if destination.starts_with('/') {
        PathBuf::from(destination)
    } else {
        current_dir.join(destination)
    };

    let mut out = output_lines.lock().await;

    // Check if source exists
    if !source_path.exists() {
        out.push(format!("cp: {}: No such file or directory", source));
        return;
    }

    if source_path.is_dir() {
        if !recursive {
            out.push(format!(
                "cp: {}: Is a directory (use -r for recursive copy)",
                source
            ));
            return;
        }

        // Recursive directory copy
        match copy_dir_recursive(&source_path, &dest_path) {
            Ok(()) => {
                out.push(format!(
                    "Copied directory {} to {}",
                    source_path.display(),
                    dest_path.display()
                ));
            }
            Err(e) => {
                out.push(format!("cp: {}", e));
            }
        }
    } else {
        // File copy
        match std::fs::copy(&source_path, &dest_path) {
            Ok(_) => {
                out.push(format!(
                    "Copied {} to {}",
                    source_path.display(),
                    dest_path.display()
                ));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("cp: {}: Permission denied", source));
                }
                std::io::ErrorKind::AlreadyExists => {
                    out.push(format!("cp: {}: File exists", destination));
                }
                _ => {
                    out.push(format!("cp: {}: {}", source, e));
                }
            },
        }
    }
}

fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !src.is_dir() {
        return Err(format!("{} is not a directory", src.display()).into());
    }

    // Create destination directory
    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

pub async fn handle_mv(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() != 3 {
        let mut out = output_lines.lock().await;
        out.push("Usage: mv <source> <destination>".to_string());
        return;
    }

    let source = args[1];
    let destination = args[2];

    let source_path = if source.starts_with('/') {
        PathBuf::from(source)
    } else {
        current_dir.join(source)
    };

    let dest_path = if destination.starts_with('/') {
        PathBuf::from(destination)
    } else {
        current_dir.join(destination)
    };

    let mut out = output_lines.lock().await;

    // Check if source exists
    if !source_path.exists() {
        out.push(format!("mv: {}: No such file or directory", source));
        return;
    }

    match std::fs::rename(&source_path, &dest_path) {
        Ok(()) => {
            out.push(format!(
                "Moved {} to {}",
                source_path.display(),
                dest_path.display()
            ));
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                out.push(format!("mv: {}: No such file or directory", source));
            }
            std::io::ErrorKind::PermissionDenied => {
                out.push(format!("mv: {}: Permission denied", source));
            }
            std::io::ErrorKind::AlreadyExists => {
                out.push(format!("mv: {}: File exists", destination));
            }
            _ => {
                out.push(format!("mv: {}: {}", source, e));
            }
        },
    }
}

pub async fn handle_rm(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: rm [-r] [-f] <file>...".to_string());
        return;
    }

    let mut recursive = false;
    let mut force = false;
    let mut file_args = Vec::new();

    // Parse arguments
    for arg in &args[1..] {
        match *arg {
            "-r" | "--recursive" => recursive = true,
            "-f" | "--force" => force = true,
            "-rf" | "-fr" => {
                recursive = true;
                force = true;
            }
            _ => file_args.push(*arg),
        }
    }

    if file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("rm: missing operand".to_string());
        return;
    }

    let mut out = output_lines.lock().await;

    for file_name in file_args {
        let file_path = if file_name.starts_with('/') {
            PathBuf::from(file_name)
        } else {
            current_dir.join(file_name)
        };

        if !file_path.exists() {
            if !force {
                out.push(format!("rm: {}: No such file or directory", file_name));
            }
            continue;
        }

        let result = if file_path.is_dir() {
            if !recursive {
                out.push(format!(
                    "rm: {}: Is a directory (use -r to remove directories)",
                    file_name
                ));
                continue;
            }
            std::fs::remove_dir_all(&file_path)
        } else {
            std::fs::remove_file(&file_path)
        };

        match result {
            Ok(()) => {
                let file_type = if file_path.is_dir() {
                    "directory"
                } else {
                    "file"
                };
                out.push(format!("Removed {}: {}", file_type, file_path.display()));
            }
            Err(e) => {
                if !force {
                    match e.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            out.push(format!("rm: {}: Permission denied", file_name));
                        }
                        std::io::ErrorKind::NotFound => {
                            out.push(format!("rm: {}: No such file or directory", file_name));
                        }
                        _ => {
                            out.push(format!("rm: {}: {}", file_name, e));
                        }
                    }
                }
            }
        }
    }
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
            for arg in &args[1..args.len() - 1] {
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
                    formatted_line = formatted_line.replace('\t', "^I").replace('\r', "^M");

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
