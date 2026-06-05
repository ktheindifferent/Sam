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
                                Err(e) => results.push(format!(
                                    "grep: {}: {}",
                                    expanded_file.display(),
                                    e
                                )),
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
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename_cow);

    for (line_number, line) in content.lines().enumerate() {
        let is_match = regex.is_match(line);
        let should_include = if options.invert_match {
            !is_match
        } else {
            is_match
        };

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
                    tokio::runtime::Handle::current().block_on(grep_file(
                        regex,
                        &path,
                        options,
                        show_filename,
                    ))
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
        let should_include = if options.invert_match {
            !is_match
        } else {
            is_match
        };

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

/// Handle the echo command - display text
pub async fn handle_echo(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 2 {
        // echo with no arguments just prints a blank line
        let mut out = output_lines.lock().await;
        out.push(String::new());
        return;
    }

    let mut no_newline = false;
    let mut enable_escape = false;
    let mut text_args = Vec::new();
    let mut i = 1;

    // Parse options
    while i < args.len() {
        match args[i] {
            "-n" => no_newline = true,
            "-e" => enable_escape = true,
            "-E" => enable_escape = false,
            _ if args[i].starts_with('-') => {
                // Handle combined options like -ne
                let opts = &args[i][1..];
                for opt in opts.chars() {
                    match opt {
                        'n' => no_newline = true,
                        'e' => enable_escape = true,
                        'E' => enable_escape = false,
                        _ => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("echo: invalid option: -{}", opt));
                            return;
                        }
                    }
                }
            }
            _ => text_args.push(args[i]),
        }
        i += 1;
    }

    let text = text_args.join(" ");
    let processed_text = if enable_escape {
        process_escape_sequences(&text)
    } else {
        text
    };

    let mut out = output_lines.lock().await;
    if no_newline {
        // For CLI purposes, we can't really avoid newlines, but we can indicate it
        out.push(format!("{} (no newline)", processed_text));
    } else {
        out.push(processed_text);
    }
}

/// Process escape sequences in echo text
fn process_escape_sequences(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' => {
                        result.push('\n');
                        chars.next(); // consume the 'n'
                    }
                    't' => {
                        result.push('\t');
                        chars.next(); // consume the 't'
                    }
                    'r' => {
                        result.push('\r');
                        chars.next(); // consume the 'r'
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next(); // consume the second '\'
                    }
                    'a' => {
                        result.push('\x07'); // bell/alert
                        chars.next();
                    }
                    'b' => {
                        result.push('\x08'); // backspace
                        chars.next();
                    }
                    'f' => {
                        result.push('\x0C'); // form feed
                        chars.next();
                    }
                    'v' => {
                        result.push('\x0B'); // vertical tab
                        chars.next();
                    }
                    _ => {
                        result.push(ch); // keep the backslash if not a recognized escape
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Handle the sort command - sort lines of text files or stdin
pub async fn handle_sort(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    let mut reverse = false;
    let mut unique = false;
    let mut numeric = false;
    let mut ignore_case = false;
    let mut field_separator = None;
    let mut key_field = None;
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-r" | "--reverse" => reverse = true,
            "-u" | "--unique" => unique = true,
            "-n" | "--numeric-sort" => numeric = true,
            "-f" | "--ignore-case" => ignore_case = true,
            "-t" => {
                if i + 1 < args.len() {
                    field_separator = Some(args[i + 1].chars().next().unwrap_or('\t'));
                    i += 1;
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("sort: option '-t' requires an argument".to_string());
                    return;
                }
            }
            "-k" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(k) => key_field = Some(k),
                        Err(_) => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("sort: invalid key: {}", args[i + 1]));
                            return;
                        }
                    }
                    i += 1;
                } else {
                    let mut out = output_lines.lock().await;
                    out.push("sort: option '-k' requires an argument".to_string());
                    return;
                }
            }
            _ if args[i].starts_with('-') => {
                let mut out = output_lines.lock().await;
                out.push(format!("sort: unknown option: {}", args[i]));
                return;
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }

    // Collect all lines to sort
    let mut lines_to_sort = Vec::new();

    if file_args.is_empty() {
        // No files specified - in a real shell this would read from stdin
        let mut out = output_lines.lock().await;
        out.push("sort: no input files specified".to_string());
        return;
    }

    // Read from files
    let mut out = output_lines.lock().await;
    for file_name in file_args {
        let file_path = resolve_path(file_name, current_dir);

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                lines_to_sort.extend(content.lines().map(String::from));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("sort: {}: No such file or directory", file_name));
                    return;
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("sort: {}: Permission denied", file_name));
                    return;
                }
                std::io::ErrorKind::IsADirectory => {
                    out.push(format!("sort: {}: Is a directory", file_name));
                    return;
                }
                _ => {
                    out.push(format!("sort: {}: {}", file_name, e));
                    return;
                }
            },
        }
    }

    // Sort the lines
    if numeric {
        lines_to_sort.sort_by(|a, b| {
            let a_num = extract_sort_key(a, field_separator, key_field)
                .parse::<f64>()
                .unwrap_or(0.0);
            let b_num = extract_sort_key(b, field_separator, key_field)
                .parse::<f64>()
                .unwrap_or(0.0);
            if reverse {
                b_num
                    .partial_cmp(&a_num)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a_num
                    .partial_cmp(&b_num)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });
    } else if ignore_case {
        lines_to_sort.sort_by(|a, b| {
            let a_key = extract_sort_key(a, field_separator, key_field).to_lowercase();
            let b_key = extract_sort_key(b, field_separator, key_field).to_lowercase();
            if reverse {
                b_key.cmp(&a_key)
            } else {
                a_key.cmp(&b_key)
            }
        });
    } else {
        lines_to_sort.sort_by(|a, b| {
            let a_key = extract_sort_key(a, field_separator, key_field);
            let b_key = extract_sort_key(b, field_separator, key_field);
            if reverse {
                b_key.cmp(&a_key)
            } else {
                a_key.cmp(&b_key)
            }
        });
    }

    // Remove duplicates if unique option is set
    if unique {
        lines_to_sort.dedup();
    }

    drop(out); // Release the lock before extending
    let mut out = output_lines.lock().await;
    out.extend(lines_to_sort);
}

/// Handle sort command with input from pipes
pub async fn handle_sort_with_input(
    cmd: &str,
    input_lines: &[String],
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    let mut reverse = false;
    let mut unique = false;
    let mut numeric = false;
    let mut ignore_case = false;
    let mut field_separator = None;
    let mut key_field = None;
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-r" | "--reverse" => reverse = true,
            "-u" | "--unique" => unique = true,
            "-n" | "--numeric-sort" => numeric = true,
            "-f" | "--ignore-case" => ignore_case = true,
            "-t" => {
                if i + 1 < args.len() {
                    field_separator = Some(args[i + 1].chars().next().unwrap_or('\t'));
                    i += 1;
                }
            }
            "-k" => {
                if i + 1 < args.len() {
                    if let Ok(k) = args[i + 1].parse::<usize>() {
                        key_field = Some(k);
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let mut lines_to_sort: Vec<String> = input_lines.to_vec();

    // Sort the lines
    if numeric {
        lines_to_sort.sort_by(|a, b| {
            let a_num = extract_sort_key(a, field_separator, key_field)
                .parse::<f64>()
                .unwrap_or(0.0);
            let b_num = extract_sort_key(b, field_separator, key_field)
                .parse::<f64>()
                .unwrap_or(0.0);
            if reverse {
                b_num
                    .partial_cmp(&a_num)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a_num
                    .partial_cmp(&b_num)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });
    } else if ignore_case {
        lines_to_sort.sort_by(|a, b| {
            let a_key = extract_sort_key(a, field_separator, key_field).to_lowercase();
            let b_key = extract_sort_key(b, field_separator, key_field).to_lowercase();
            if reverse {
                b_key.cmp(&a_key)
            } else {
                a_key.cmp(&b_key)
            }
        });
    } else {
        lines_to_sort.sort_by(|a, b| {
            let a_key = extract_sort_key(a, field_separator, key_field);
            let b_key = extract_sort_key(b, field_separator, key_field);
            if reverse {
                b_key.cmp(&a_key)
            } else {
                a_key.cmp(&b_key)
            }
        });
    }

    // Remove duplicates if unique option is set
    if unique {
        lines_to_sort.dedup();
    }

    let mut out = output_lines.lock().await;
    out.extend(lines_to_sort);
}

/// Extract sort key from line based on field separator and key field
fn extract_sort_key(line: &str, field_separator: Option<char>, key_field: Option<usize>) -> String {
    if let (Some(sep), Some(field)) = (field_separator, key_field) {
        let fields: Vec<&str> = line.split(sep).collect();
        if field > 0 && field <= fields.len() {
            fields[field - 1].to_string()
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    }
}

/// Handle the wc command - count lines, words, and characters
pub async fn handle_wc(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    let mut count_lines = false;
    let mut count_words = false;
    let mut count_chars = false;
    let mut count_bytes = false;
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-l" | "--lines" => count_lines = true,
            "-w" | "--words" => count_words = true,
            "-c" | "--chars" | "--bytes" => count_chars = true,
            "-m" | "--max-line-length" => {
                let mut out = output_lines.lock().await;
                out.push("wc: -m option not implemented".to_string());
                return;
            }
            _ if args[i].starts_with('-') && args[i].len() > 1 => {
                // Handle combined options like -lwc
                let opts = &args[i][1..];
                for opt in opts.chars() {
                    match opt {
                        'l' => count_lines = true,
                        'w' => count_words = true,
                        'c' => count_chars = true,
                        'm' => count_bytes = true,
                        _ => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("wc: invalid option: -{}", opt));
                            return;
                        }
                    }
                }
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }

    // Default: if no options specified, count all
    if !count_lines && !count_words && !count_chars && !count_bytes {
        count_lines = true;
        count_words = true;
        count_chars = true;
    }

    let mut out = output_lines.lock().await;

    if file_args.is_empty() {
        out.push("wc: no input files specified".to_string());
        return;
    }

    let mut total_lines = 0;
    let mut total_words = 0;
    let mut total_chars = 0;
    let show_totals = file_args.len() > 1;

    for file_name in &file_args {
        let file_path = resolve_path(file_name, current_dir);

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let (lines, words, chars) = count_content(&content);

                total_lines += lines;
                total_words += words;
                total_chars += chars;

                let mut output = String::new();

                if count_lines {
                    output.push_str(&format!("{:8}", lines));
                }
                if count_words {
                    output.push_str(&format!("{:8}", words));
                }
                if count_chars || count_bytes {
                    output.push_str(&format!("{:8}", chars));
                }

                output.push_str(&format!(" {}", file_name));
                out.push(output);
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    out.push(format!("wc: {}: No such file or directory", file_name));
                }
                std::io::ErrorKind::PermissionDenied => {
                    out.push(format!("wc: {}: Permission denied", file_name));
                }
                std::io::ErrorKind::IsADirectory => {
                    out.push(format!("wc: {}: Is a directory", file_name));
                }
                _ => {
                    out.push(format!("wc: {}: {}", file_name, e));
                }
            },
        }
    }

    // Show totals if multiple files
    if show_totals {
        let mut total_output = String::new();

        if count_lines {
            total_output.push_str(&format!("{:8}", total_lines));
        }
        if count_words {
            total_output.push_str(&format!("{:8}", total_words));
        }
        if count_chars || count_bytes {
            total_output.push_str(&format!("{:8}", total_chars));
        }

        total_output.push_str(" total");
        out.push(total_output);
    }
}

/// Handle wc command with input from pipes
pub async fn handle_wc_with_input(
    cmd: &str,
    input_lines: &[String],
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    let mut count_lines = false;
    let mut count_words = false;
    let mut count_chars = false;
    let mut count_bytes = false;
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-l" | "--lines" => count_lines = true,
            "-w" | "--words" => count_words = true,
            "-c" | "--chars" | "--bytes" => count_chars = true,
            "-m" | "--max-line-length" => count_bytes = true,
            _ if args[i].starts_with('-') && args[i].len() > 1 => {
                let opts = &args[i][1..];
                for opt in opts.chars() {
                    match opt {
                        'l' => count_lines = true,
                        'w' => count_words = true,
                        'c' => count_chars = true,
                        'm' => count_bytes = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Default: if no options specified, count all
    if !count_lines && !count_words && !count_chars && !count_bytes {
        count_lines = true;
        count_words = true;
        count_chars = true;
    }

    let content = input_lines.join("\n");
    let (lines, words, chars) = count_content(&content);

    let mut output = String::new();

    if count_lines {
        output.push_str(&format!("{:8}", lines));
    }
    if count_words {
        output.push_str(&format!("{:8}", words));
    }
    if count_chars || count_bytes {
        output.push_str(&format!("{:8}", chars));
    }

    let mut out = output_lines.lock().await;
    out.push(output.trim().to_string());
}

/// Count lines, words, and characters in content
fn count_content(content: &str) -> (usize, usize, usize) {
    let lines = content.lines().count();
    let words = content.split_whitespace().count();
    let chars = content.chars().count();

    (lines, words, chars)
}

/// Helper function to resolve relative/absolute paths (shared with file_ops.rs)
fn resolve_path(path: &str, current_dir: &PathBuf) -> PathBuf {
    if path.starts_with('/')
        || (cfg!(windows) && path.len() > 1 && path.chars().nth(1) == Some(':'))
    {
        PathBuf::from(path)
    } else {
        current_dir.join(path)
    }
}
