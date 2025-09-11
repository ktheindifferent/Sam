use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
// File operations will be used for actual file I/O when needed

/// Handle the tar command - create and extract archives
pub async fn handle_tar(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: tar [options] [archive] [files...]".to_string());
        out.push("Options:".to_string());
        out.push("  -c, --create     Create new archive".to_string());
        out.push("  -x, --extract    Extract files from archive".to_string());
        out.push("  -t, --list       List contents of archive".to_string());
        out.push("  -f, --file       Specify archive filename".to_string());
        out.push("  -v, --verbose    Verbose output".to_string());
        out.push("  -z, --gzip       Filter through gzip".to_string());
        out.push("Examples:".to_string());
        out.push("  tar -cf archive.tar file1 file2    # Create archive".to_string());
        out.push("  tar -xf archive.tar                # Extract archive".to_string());
        out.push("  tar -tf archive.tar                # List contents".to_string());
        return;
    }
    
    let mut create = false;
    let mut extract = false;
    let mut list = false;
    let mut verbose = false;
    let mut use_gzip = false;
    let mut archive_file = None;
    let mut file_args = Vec::new();
    let mut i = 1;
    
    // Parse arguments
    while i < args.len() {
        let arg = args[i];
        if arg.starts_with('-') && arg.len() > 1 {
            let opts = &arg[1..];
            for opt in opts.chars() {
                match opt {
                    'c' => create = true,
                    'x' => extract = true,
                    't' => list = true,
                    'f' => {
                        if i + 1 < args.len() {
                            archive_file = Some(args[i + 1]);
                            i += 1; // Skip archive filename
                        } else {
                            let mut out = output_lines.lock().await;
                            out.push("tar: option 'f' requires an argument".to_string());
                            return;
                        }
                    }
                    'v' => verbose = true,
                    'z' => use_gzip = true,
                    _ => {
                        let mut out = output_lines.lock().await;
                        out.push(format!("tar: unknown option: -{}", opt));
                        return;
                    }
                }
            }
        } else {
            // If no archive file set yet and this looks like one, use it
            if archive_file.is_none() && (arg.ends_with(".tar") || arg.ends_with(".tar.gz") || arg.ends_with(".tgz")) {
                archive_file = Some(arg);
            } else {
                file_args.push(arg);
            }
        }
        i += 1;
    }
    
    // Validate arguments
    let mode_count = [create, extract, list].iter().filter(|&&x| x).count();
    if mode_count == 0 {
        let mut out = output_lines.lock().await;
        out.push("tar: must specify one of -c, -x, or -t".to_string());
        return;
    }
    if mode_count > 1 {
        let mut out = output_lines.lock().await;
        out.push("tar: cannot specify more than one of -c, -x, or -t".to_string());
        return;
    }
    
    let Some(archive_name) = archive_file else {
        let mut out = output_lines.lock().await;
        out.push("tar: must specify archive filename with -f".to_string());
        return;
    };
    
    let archive_path = resolve_path(archive_name, current_dir);
    
    if create {
        create_tar_archive(&archive_path, &file_args, current_dir, verbose, use_gzip, output_lines).await;
    } else if extract {
        extract_tar_archive(&archive_path, current_dir, verbose, use_gzip, output_lines).await;
    } else if list {
        list_tar_archive(&archive_path, verbose, use_gzip, output_lines).await;
    }
}

/// Create a tar archive (simplified implementation)
async fn create_tar_archive(
    archive_path: &PathBuf,
    files: &[&str],
    current_dir: &PathBuf,
    verbose: bool,
    use_gzip: bool,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let mut out = output_lines.lock().await;
    
    if files.is_empty() {
        out.push("tar: no files specified to archive".to_string());
        return;
    }
    
    // Simple tar implementation - create a list of files with their contents
    match create_simple_archive(archive_path, files, current_dir, use_gzip) {
        Ok(file_count) => {
            if verbose {
                out.push(format!("tar: created archive {} with {} files", archive_path.display(), file_count));
            } else {
                out.push(format!("Created archive: {}", archive_path.display()));
            }
            
            if verbose {
                for file in files {
                    out.push(format!("  added: {}", file));
                }
            }
        }
        Err(e) => {
            out.push(format!("tar: error creating archive: {}", e));
        }
    }
}

/// Extract a tar archive (simplified implementation)
async fn extract_tar_archive(
    archive_path: &PathBuf,
    current_dir: &PathBuf,
    verbose: bool,
    use_gzip: bool,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let mut out = output_lines.lock().await;
    
    if !archive_path.exists() {
        out.push(format!("tar: {}: No such file or directory", archive_path.display()));
        return;
    }
    
    match extract_simple_archive(archive_path, current_dir, use_gzip) {
        Ok(files) => {
            if verbose {
                out.push(format!("tar: extracted {} files from {}", files.len(), archive_path.display()));
                for file in &files {
                    out.push(format!("  extracted: {}", file));
                }
            } else {
                out.push(format!("Extracted {} files from: {}", files.len(), archive_path.display()));
            }
        }
        Err(e) => {
            out.push(format!("tar: error extracting archive: {}", e));
        }
    }
}

/// List contents of tar archive (simplified implementation)
async fn list_tar_archive(
    archive_path: &PathBuf,
    verbose: bool,
    use_gzip: bool,
    output_lines: &Arc<Mutex<Vec<String>>>,
) {
    let mut out = output_lines.lock().await;
    
    if !archive_path.exists() {
        out.push(format!("tar: {}: No such file or directory", archive_path.display()));
        return;
    }
    
    match list_simple_archive(archive_path, use_gzip) {
        Ok(files) => {
            if verbose {
                out.push(format!("Archive: {}", archive_path.display()));
            }
            for file in files {
                out.push(file);
            }
        }
        Err(e) => {
            out.push(format!("tar: error listing archive: {}", e));
        }
    }
}

/// Simple archive format: each file as "FILENAME:LENGTH\nCONTENT"
fn create_simple_archive(
    archive_path: &PathBuf,
    files: &[&str],
    current_dir: &PathBuf,
    use_gzip: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut archive_content = String::new();
    let mut file_count = 0;
    
    for file_name in files {
        let file_path = resolve_path(file_name, current_dir);
        
        if !file_path.exists() {
            continue; // Skip non-existent files
        }
        
        if file_path.is_file() {
            let content = std::fs::read_to_string(&file_path)?;
            archive_content.push_str(&format!("{}:{}\n{}\n", file_name, content.len(), content));
            file_count += 1;
        } else if file_path.is_dir() {
            // For directories, recursively add files
            if let Ok(entries) = std::fs::read_dir(&file_path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Ok(content) = std::fs::read_to_string(&entry_path) {
                            let relative_name = format!("{}/{}", file_name, entry.file_name().to_string_lossy());
                            archive_content.push_str(&format!("{}:{}\n{}\n", relative_name, content.len(), content));
                            file_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    let final_content = if use_gzip {
        compress_content(&archive_content)?
    } else {
        archive_content.into_bytes()
    };
    
    std::fs::write(archive_path, final_content)?;
    Ok(file_count)
}

fn extract_simple_archive(
    archive_path: &PathBuf,
    current_dir: &PathBuf,
    use_gzip: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let archive_bytes = std::fs::read(archive_path)?;
    
    let content = if use_gzip {
        decompress_content(&archive_bytes)?
    } else {
        String::from_utf8(archive_bytes)?
    };
    
    let mut extracted_files = Vec::new();
    let mut lines = content.lines();
    
    while let Some(header) = lines.next() {
        if let Some(colon_pos) = header.find(':') {
            let filename = &header[..colon_pos];
            let length: usize = header[colon_pos + 1..].parse().unwrap_or(0);
            
            // Read the file content (next 'length' characters)
            let mut file_content = String::new();
            let mut chars_read = 0;
            
            while chars_read < length {
                if let Some(line) = lines.next() {
                    if chars_read > 0 {
                        file_content.push('\n');
                        chars_read += 1;
                    }
                    file_content.push_str(line);
                    chars_read += line.len();
                } else {
                    break;
                }
            }
            
            // Create file
            let file_path = resolve_path(filename, current_dir);
            
            // Create directories if needed
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            std::fs::write(&file_path, file_content)?;
            extracted_files.push(filename.to_string());
        }
    }
    
    Ok(extracted_files)
}

fn list_simple_archive(
    archive_path: &PathBuf,
    use_gzip: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let archive_bytes = std::fs::read(archive_path)?;
    
    let content = if use_gzip {
        decompress_content(&archive_bytes)?
    } else {
        String::from_utf8(archive_bytes)?
    };
    
    let mut files = Vec::new();
    
    for line in content.lines() {
        if let Some(colon_pos) = line.find(':') {
            let filename = &line[..colon_pos];
            if !filename.is_empty() {
                files.push(filename.to_string());
            }
        }
    }
    
    Ok(files)
}

/// Handle the gzip command - compress files
pub async fn handle_gzip(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: gzip [-d] [-c] [-f] [-v] <file>...".to_string());
        out.push("Options:".to_string());
        out.push("  -d, --decompress  Decompress files".to_string());
        out.push("  -c, --stdout      Write to stdout".to_string());
        out.push("  -f, --force       Force overwrite".to_string());
        out.push("  -v, --verbose     Verbose output".to_string());
        return;
    }
    
    let mut decompress = false;
    let mut to_stdout = false;
    let mut force = false;
    let mut verbose = false;
    let mut file_args = Vec::new();
    let mut i = 1;
    
    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-d" | "--decompress" => decompress = true,
            "-c" | "--stdout" => to_stdout = true,
            "-f" | "--force" => force = true,
            "-v" | "--verbose" => verbose = true,
            _ if args[i].starts_with('-') && args[i].len() > 1 => {
                let opts = &args[i][1..];
                for opt in opts.chars() {
                    match opt {
                        'd' => decompress = true,
                        'c' => to_stdout = true,
                        'f' => force = true,
                        'v' => verbose = true,
                        _ => {
                            let mut out = output_lines.lock().await;
                            out.push(format!("gzip: unknown option: -{}", opt));
                            return;
                        }
                    }
                }
            }
            _ => file_args.push(args[i]),
        }
        i += 1;
    }
    
    if file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("gzip: no files specified".to_string());
        return;
    }
    
    let mut out = output_lines.lock().await;
    
    for file_name in file_args {
        let file_path = resolve_path(file_name, current_dir);
        
        if !file_path.exists() {
            out.push(format!("gzip: {}: No such file or directory", file_name));
            continue;
        }
        
        if decompress {
            // Decompress
            let output_path = if file_name.ends_with(".gz") {
                file_path.with_extension("")
            } else {
                file_path.with_extension("decompressed")
            };
            
            if output_path.exists() && !force {
                out.push(format!("gzip: {}: already exists (use -f to force)", output_path.display()));
                continue;
            }
            
            match decompress_file(&file_path, &output_path, to_stdout) {
                Ok(()) => {
                    if verbose {
                        out.push(format!("gzip: decompressed {} -> {}", file_name, output_path.display()));
                    }
                    if !to_stdout {
                        // Remove original compressed file
                        let _ = std::fs::remove_file(&file_path);
                    }
                }
                Err(e) => {
                    out.push(format!("gzip: {}: {}", file_name, e));
                }
            }
        } else {
            // Compress
            let output_path = file_path.with_extension("gz");
            
            if output_path.exists() && !force {
                out.push(format!("gzip: {}: already exists (use -f to force)", output_path.display()));
                continue;
            }
            
            match compress_file(&file_path, &output_path, to_stdout) {
                Ok(()) => {
                    if verbose {
                        out.push(format!("gzip: compressed {} -> {}", file_name, output_path.display()));
                    }
                    if !to_stdout {
                        // Remove original file
                        let _ = std::fs::remove_file(&file_path);
                    }
                }
                Err(e) => {
                    out.push(format!("gzip: {}: {}", file_name, e));
                }
            }
        }
    }
}

/// Handle the gunzip command - decompress gzip files
pub async fn handle_gunzip(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>, current_dir: &PathBuf) {
    // gunzip is equivalent to gzip -d
    let modified_cmd = cmd.replacen("gunzip", "gzip -d", 1);
    handle_gzip(&modified_cmd, output_lines, current_dir).await;
}

/// Compress a file using a simple compression algorithm
fn compress_file(
    input_path: &PathBuf,
    output_path: &PathBuf,
    to_stdout: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(input_path)?;
    let compressed = compress_content(&content)?;
    
    if to_stdout {
        // In a real implementation, this would write to stdout
        // For CLI purposes, we'll indicate this
        return Ok(());
    }
    
    std::fs::write(output_path, compressed)?;
    Ok(())
}

/// Decompress a file using a simple decompression algorithm
fn decompress_file(
    input_path: &PathBuf,
    output_path: &PathBuf,
    to_stdout: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let compressed_content = std::fs::read(input_path)?;
    let decompressed = decompress_content(&compressed_content)?;
    
    if to_stdout {
        // In a real implementation, this would write to stdout
        // For CLI purposes, we'll indicate this
        return Ok(());
    }
    
    std::fs::write(output_path, decompressed)?;
    Ok(())
}

/// Simple compression algorithm (RLE-like for demonstration)
/// In a real implementation, you'd use flate2 crate for actual gzip compression
fn compress_content(content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Very simple run-length encoding for demonstration
    let bytes = content.as_bytes();
    let mut compressed = Vec::new();
    
    // Add magic header
    compressed.extend_from_slice(b"SGZIP");
    
    let mut i = 0;
    while i < bytes.len() {
        let current_byte = bytes[i];
        let mut count = 1;
        
        // Count consecutive identical bytes
        while i + count < bytes.len() && bytes[i + count] == current_byte && count < 255 {
            count += 1;
        }
        
        if count > 3 || current_byte == 0 {
            // Use RLE for runs of 4+ or null bytes
            compressed.push(0); // Escape byte
            compressed.push(count as u8);
            compressed.push(current_byte);
        } else {
            // Direct copy for short runs
            for _ in 0..count {
                if current_byte == 0 {
                    compressed.push(0);
                    compressed.push(1);
                    compressed.push(0);
                } else {
                    compressed.push(current_byte);
                }
            }
        }
        
        i += count;
    }
    
    Ok(compressed)
}

/// Simple decompression algorithm
fn decompress_content(compressed: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    // Check magic header
    if compressed.len() < 5 || &compressed[0..5] != b"SGZIP" {
        return Err("Invalid compressed format".into());
    }
    
    let mut decompressed = Vec::new();
    let mut i = 5; // Skip magic header
    
    while i < compressed.len() {
        if compressed[i] == 0 && i + 2 < compressed.len() {
            // RLE sequence
            let count = compressed[i + 1] as usize;
            let byte_val = compressed[i + 2];
            
            for _ in 0..count {
                decompressed.push(byte_val);
            }
            i += 3;
        } else {
            // Direct byte
            decompressed.push(compressed[i]);
            i += 1;
        }
    }
    
    String::from_utf8(decompressed).map_err(|e| e.into())
}

/// Helper function to resolve relative/absolute paths
fn resolve_path(path: &str, current_dir: &PathBuf) -> PathBuf {
    if path.starts_with('/') || (cfg!(windows) && path.len() > 1 && path.chars().nth(1) == Some(':')) {
        PathBuf::from(path)
    } else {
        current_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_decompress() {
        let original = "Hello world! This is a test string with repeated characters: aaaaaabbbbbb";
        let compressed = compress_content(original).unwrap();
        let decompressed = decompress_content(&compressed).unwrap();
        
        assert_eq!(original, decompressed);
    }
    
    #[test]
    fn test_compress_empty() {
        let original = "";
        let compressed = compress_content(original).unwrap();
        let decompressed = decompress_content(&compressed).unwrap();
        
        assert_eq!(original, decompressed);
    }
}
