use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct GrepOptions {
    pub ignore_case: bool,
    pub show_line_numbers: bool,
    pub invert_match: bool,
    pub recursive: bool,
    pub count_only: bool,
    pub files_with_matches: bool,
    pub with_filename: bool,
    pub no_filename: bool,
}

pub async fn handle_grep(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: grep [options] <pattern> [files...]".to_string());
        out.push("Options:".to_string());
        out.push("  -i, --ignore-case    Ignore case distinctions".to_string());
        out.push("  -n, --line-number    Show line numbers".to_string());
        out.push("  -v, --invert-match   Invert match (show non-matching lines)".to_string());
        out.push("  -r, --recursive      Search directories recursively".to_string());
        out.push("  -c, --count          Count matching lines only".to_string());
        out.push("  -l, --files-with-matches  Show only filenames with matches".to_string());
        out.push("  -H, --with-filename  Always print filename".to_string());
        out.push("  -h, --no-filename    Never print filename".to_string());
        return;
    }
    
    // Parse options and arguments
    let mut options = GrepOptions::default();
    let mut pattern = "";
    let mut files = Vec::new();
    let mut i = 1;
    
    while i < args.len() {
        match args[i] {
            "-i" | "--ignore-case" => options.ignore_case = true,
            "-n" | "--line-number" => options.show_line_numbers = true,
            "-v" | "--invert-match" => options.invert_match = true,
            "-r" | "--recursive" => options.recursive = true,
            "-c" | "--count" => options.count_only = true,
            "-l" | "--files-with-matches" => options.files_with_matches = true,
            "-H" | "--with-filename" => options.with_filename = true,
            "-h" | "--no-filename" => options.no_filename = true,
            arg if arg.starts_with('-') => {
                let mut out = output_lines.lock().await;
                out.push(format!("grep: unknown option: {}", arg));
                return;
            }
            arg => {
                if pattern.is_empty() {
                    pattern = arg;
                } else {
                    files.push(arg);
                }
            }
        }
        i += 1;
    }
    
    if pattern.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("grep: missing pattern".to_string());
        return;
    }
    
    // If no files specified, read from stdin (for pipe support)
    if files.is_empty() {
        files.push("-"); // Stdin placeholder
    }
    
    // Perform grep operation
    match grep_files(pattern, &files, &options, current_dir).await {
        Ok(results) => {
            let mut out = output_lines.lock().await;
            out.extend(results);
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("grep: {}", e));
        }
    }
}

async fn grep_files(
    pattern: &str,
    files: &[&str],
    options: &GrepOptions,
    current_dir: &PathBuf,
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    let regex_pattern = if options.ignore_case {
        format!("(?i){}", regex::escape(pattern))
    } else {
        regex::escape(pattern)
    };
    
    let regex = match regex::Regex::new(&regex_pattern) {
        Ok(r) => r,
        Err(e) => return Err(format!("Invalid pattern: {}", e)),
    };
    
    for file_pattern in files {
        if *file_pattern == "-" {
            // Handle stdin input (for pipe support)
            continue; // For now, skip stdin - will implement with pipe support
        }
        
        let file_path = if file_pattern.starts_with('/') {
            PathBuf::from(file_pattern)
        } else {
            current_dir.join(file_pattern)
        };
        
        if options.recursive && file_path.is_dir() {
            // Recursive directory search
            match grep_directory(&regex, &file_path, options, files.len() > 1).await {
                Ok(mut dir_results) => results.append(&mut dir_results),
                Err(e) => results.push(format!("grep: {}: {}", file_pattern, e)),
            }
        } else if file_path.exists() && file_path.is_file() {
            // Single file search
            match grep_file(&regex, &file_path, options, files.len() > 1).await {
                Ok(mut file_results) => results.append(&mut file_results),
                Err(e) => results.push(format!("grep: {}: {}", file_pattern, e)),
            }
        } else {
            // Handle glob patterns
            match expand_glob_pattern(file_pattern, current_dir) {
                Ok(expanded_files) => {
                    let show_filename = files.len() > 1 || expanded_files.len() > 1;
                    for expanded_file in &expanded_files {
                        if expanded_file.is_file() {
                            match grep_file(&regex, &expanded_file, options, show_filename).await {
                                Ok(mut file_results) => results.append(&mut file_results),
                                Err(e) => results.push(format!("grep: {}: {}", expanded_file.display(), e)),
                            }
                        }
                    }
                }
                Err(_) => {
                    results.push(format!("grep: {}: No such file or directory", file_pattern));
                }
            }
        }
    }
    
    Ok(results)
}

pub async fn grep_file(
    regex: &regex::Regex,
    file_path: &PathBuf,
    options: &GrepOptions,
    show_filename: bool,
) -> Result<Vec<String>, String> {
    let content = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return Err(format!("Cannot read file: {}", e)),
    };
    
    let mut results = Vec::new();
    let mut match_count = 0;
    let should_show_filename = (show_filename && !options.no_filename) || options.with_filename;
    let filename_cow = file_path.to_string_lossy();
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename_cow);
    
    for (line_number, line) in content.lines().enumerate() {
        let is_match = regex.is_match(line);
        let should_include = if options.invert_match { !is_match } else { is_match };
        
        if should_include {
            match_count += 1;
            
            if options.count_only {
                // Just count, don't collect lines
                continue;
            }
            
            if options.files_with_matches {
                // Just return filename and stop
                results.push(if should_show_filename {
                    filename.to_string()
                } else {
                    filename.to_string()
                });
                break;
            }
            
            // Format the output line
            let mut output_line = String::new();
            
            if should_show_filename {
                output_line.push_str(&format!("{}:", filename));
            }
            
            if options.show_line_numbers {
                output_line.push_str(&format!("{}:", line_number + 1));
            }
            
            output_line.push_str(line);
            results.push(output_line);
        }
    }
    
    // Handle count-only output
    if options.count_only {
        let mut output_line = String::new();
        if should_show_filename {
            output_line.push_str(&format!("{}:", filename));
        }
        output_line.push_str(&match_count.to_string());
        results.push(output_line);
    }
    
    Ok(results)
}

async fn grep_directory(
    regex: &regex::Regex,
    dir_path: &PathBuf,
    options: &GrepOptions,
    show_filename: bool,
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    
    fn visit_dir(
        dir: &PathBuf,
        regex: &regex::Regex,
        options: &GrepOptions,
        show_filename: bool,
        results: &mut Vec<String>,
    ) -> Result<(), String> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("Cannot read directory: {}", e)),
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            
            let path = entry.path();
            
            if path.is_file() {
                // Process file
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(grep_file(regex, &path, options, show_filename))
                }) {
                    Ok(mut file_results) => results.append(&mut file_results),
                    Err(e) => results.push(format!("grep: {}: {}", path.display(), e)),
                }
            } else if path.is_dir() && options.recursive {
                // Recurse into subdirectory
                if let Err(e) = visit_dir(&path, regex, options, show_filename, results) {
                    results.push(format!("grep: {}: {}", path.display(), e));
                }
            }
        }
        
        Ok(())
    }
    
    visit_dir(dir_path, regex, options, show_filename, &mut results)?;
    Ok(results)
}

fn expand_glob_pattern(pattern: &str, current_dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let full_pattern = if pattern.starts_with('/') {
        pattern.to_string()
    } else {
        current_dir.join(pattern).to_string_lossy().to_string()
    };
    
    match glob::glob(&full_pattern) {
        Ok(paths) => {
            let mut results = Vec::new();
            for path_result in paths {
                match path_result {
                    Ok(path) => results.push(path),
                    Err(_) => continue,
                }
            }
            Ok(results)
        }
        Err(e) => Err(format!("Pattern error: {}", e)),
    }
}

// Handle grep with input from stdin (for pipe support)
pub async fn handle_grep_with_input(
    cmd: &str,
    input_lines: &[String],
    output_lines: &Arc<Mutex<Vec<String>>>,
    _current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: grep [options] <pattern>".to_string());
        return;
    }
    
    // Parse options and arguments
    let mut options = GrepOptions::default();
    let mut pattern = "";
    let mut i = 1;
    
    while i < args.len() {
        match args[i] {
            "-i" | "--ignore-case" => options.ignore_case = true,
            "-n" | "--line-number" => options.show_line_numbers = true,
            "-v" | "--invert-match" => options.invert_match = true,
            "-c" | "--count" => options.count_only = true,
            arg if arg.starts_with('-') => {
                let mut out = output_lines.lock().await;
                out.push(format!("grep: unknown option: {}", arg));
                return;
            }
            arg => {
                if pattern.is_empty() {
                    pattern = arg;
                    break; // In pipe context, pattern is the only non-option argument
                }
            }
        }
        i += 1;
    }
    
    if pattern.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("grep: missing pattern".to_string());
        return;
    }
    
    // Create regex pattern
    let regex_pattern = if options.ignore_case {
        format!("(?i){}", regex::escape(pattern))
    } else {
        regex::escape(pattern)
    };
    
    let regex = match regex::Regex::new(&regex_pattern) {
        Ok(r) => r,
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("grep: Invalid pattern: {}", e));
            return;
        }
    };
    
    // Process input lines
    let mut results = Vec::new();
    let mut match_count = 0;
    
    for (line_number, line) in input_lines.iter().enumerate() {
        let is_match = regex.is_match(line);
        let should_include = if options.invert_match { !is_match } else { is_match };
        
        if should_include {
            match_count += 1;
            
            if !options.count_only {
                let mut output_line = String::new();
                
                if options.show_line_numbers {
                    output_line.push_str(&format!("{}:", line_number + 1));
                }
                
                output_line.push_str(line);
                results.push(output_line);
            }
        }
    }
    
    // Handle count-only output
    if options.count_only {
        results.push(match_count.to_string());
    }
    
    let mut out = output_lines.lock().await;
    out.extend(results);
}
