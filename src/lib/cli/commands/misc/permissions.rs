use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Handle the chmod command - change file permissions
pub async fn handle_chmod(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 3 {
        let mut out = output_lines.lock().await;
        out.push("Usage: chmod [-R] <mode> <file>...".to_string());
        out.push("  mode: octal (e.g., 755) or symbolic (e.g., u+x,g-w,o=r)".to_string());
        return;
    }

    let mut recursive = false;
    let mut mode_arg = "";
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-R" | "--recursive" => recursive = true,
            _ => {
                if mode_arg.is_empty() {
                    mode_arg = args[i];
                } else {
                    file_args.push(args[i]);
                }
            }
        }
        i += 1;
    }

    if mode_arg.is_empty() || file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("chmod: missing mode or file operand".to_string());
        return;
    }

    let mode = match parse_chmod_mode(mode_arg) {
        Ok(m) => m,
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("chmod: invalid mode '{}': {}", mode_arg, e));
            return;
        }
    };

    let mut out = output_lines.lock().await;

    for file_name in file_args {
        let file_path = resolve_path(file_name, current_dir);

        if let Err(e) = chmod_file(&file_path, mode, recursive) {
            out.push(format!("chmod: {}: {}", file_name, e));
        }
    }
}

/// Handle the chown command - change file ownership
pub async fn handle_chown(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();

    if args.len() < 3 {
        let mut out = output_lines.lock().await;
        out.push("Usage: chown [-R] <owner>[:<group>] <file>...".to_string());
        return;
    }

    let mut recursive = false;
    let mut owner_arg = "";
    let mut file_args = Vec::new();
    let mut i = 1;

    // Parse arguments
    while i < args.len() {
        match args[i] {
            "-R" | "--recursive" => recursive = true,
            _ => {
                if owner_arg.is_empty() {
                    owner_arg = args[i];
                } else {
                    file_args.push(args[i]);
                }
            }
        }
        i += 1;
    }

    if owner_arg.is_empty() || file_args.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("chown: missing owner or file operand".to_string());
        return;
    }

    // Parse owner:group format
    let (owner, group) = if owner_arg.contains(':') {
        let parts: Vec<&str> = owner_arg.splitn(2, ':').collect();
        (
            Some(parts[0]),
            if parts[1].is_empty() {
                None
            } else {
                Some(parts[1])
            },
        )
    } else {
        (Some(owner_arg), None)
    };

    let mut out = output_lines.lock().await;

    for file_name in file_args {
        let file_path = resolve_path(file_name, current_dir);

        if let Err(e) = chown_file(&file_path, owner, group, recursive) {
            out.push(format!("chown: {}: {}", file_name, e));
        }
    }
}

/// Parse chmod mode string (octal or symbolic)
fn parse_chmod_mode(mode_str: &str) -> Result<u32, String> {
    // Try octal first
    if mode_str.chars().all(|c| c.is_ascii_digit()) {
        match u32::from_str_radix(mode_str, 8) {
            Ok(mode) if mode <= 0o7777 => return Ok(mode),
            _ => return Err("invalid octal mode".to_string()),
        }
    }

    // Parse symbolic mode (simplified implementation)
    let mut mode = 0o644; // Default mode

    // Split by comma for multiple operations
    for operation in mode_str.split(',') {
        mode = parse_symbolic_operation(mode, operation)?;
    }

    Ok(mode)
}

/// Parse a single symbolic chmod operation (like u+x or go-w)
fn parse_symbolic_operation(current_mode: u32, operation: &str) -> Result<u32, String> {
    let chars: Vec<char> = operation.chars().collect();
    if chars.len() < 3 {
        return Err("invalid symbolic operation".to_string());
    }

    // Parse who (user, group, other, all)
    let mut i = 0;
    let mut user = false;
    let mut group = false;
    let mut other = false;

    while i < chars.len() && "ugoa".contains(chars[i]) {
        match chars[i] {
            'u' => user = true,
            'g' => group = true,
            'o' => other = true,
            'a' => {
                user = true;
                group = true;
                other = true;
            }
            _ => {}
        }
        i += 1;
    }

    // If no who specified, default to all
    if !user && !group && !other {
        user = true;
        group = true;
        other = true;
    }

    // Parse operation (+, -, =)
    if i >= chars.len() {
        return Err("missing operation".to_string());
    }

    let op = chars[i];
    i += 1;

    if !"+-=".contains(op) {
        return Err("invalid operation".to_string());
    }

    // Parse permissions
    let mut perm_bits = 0u32;
    while i < chars.len() {
        match chars[i] {
            'r' => perm_bits |= 0o4,
            'w' => perm_bits |= 0o2,
            'x' => perm_bits |= 0o1,
            's' => {} // setuid/setgid - simplified
            't' => {} // sticky bit - simplified
            _ => return Err(format!("invalid permission '{}'", chars[i])),
        }
        i += 1;
    }

    let mut new_mode = current_mode;

    // Apply operation to each target (user, group, other)
    if user {
        let shift = 6;
        match op {
            '+' => new_mode |= perm_bits << shift,
            '-' => new_mode &= !(perm_bits << shift),
            '=' => {
                new_mode &= !(0o7 << shift);
                new_mode |= perm_bits << shift;
            }
            _ => {}
        }
    }

    if group {
        let shift = 3;
        match op {
            '+' => new_mode |= perm_bits << shift,
            '-' => new_mode &= !(perm_bits << shift),
            '=' => {
                new_mode &= !(0o7 << shift);
                new_mode |= perm_bits << shift;
            }
            _ => {}
        }
    }

    if other {
        let shift = 0;
        match op {
            '+' => new_mode |= perm_bits << shift,
            '-' => new_mode &= !(perm_bits << shift),
            '=' => {
                new_mode &= !(0o7 << shift);
                new_mode |= perm_bits << shift;
            }
            _ => {}
        }
    }

    Ok(new_mode)
}

/// Cross-platform chmod implementation
#[cfg(unix)]
fn chmod_file(
    path: &PathBuf,
    mode: u32,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    // Set permissions on the file/directory itself
    let permissions = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, permissions)?;

    // If recursive and it's a directory, recurse into subdirectories
    if recursive && path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            chmod_file(&entry.path(), mode, true)?;
        }
    }

    Ok(())
}

/// Windows chmod implementation (simplified)
#[cfg(windows)]
fn chmod_file(
    path: &PathBuf,
    mode: u32,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Windows doesn't have the same permission model as Unix
    // We'll implement a simplified version that handles read-only flag

    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // If write permission is not set (mode & 0o200 == 0), make it read-only
    if (mode & 0o200) == 0 {
        permissions.set_readonly(true);
    } else {
        permissions.set_readonly(false);
    }

    std::fs::set_permissions(path, permissions)?;

    // If recursive and it's a directory, recurse into subdirectories
    if recursive && path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            chmod_file(&entry.path(), mode, true)?;
        }
    }

    Ok(())
}

/// Fallback chmod for other platforms
#[cfg(not(any(unix, windows)))]
fn chmod_file(
    _path: &PathBuf,
    _mode: u32,
    _recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Not supported on this platform
    Err("chmod not supported on this platform".into())
}

/// Cross-platform chown implementation
#[cfg(unix)]
fn chown_file(
    path: &PathBuf,
    owner: Option<&str>,
    group: Option<&str>,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // This is a simplified implementation
    // In a real implementation, we'd use libc::chown() and resolve user/group names to IDs

    // For now, we'll just return an error indicating limited support
    if owner.is_some() || group.is_some() {
        return Err(
            "chown functionality requires elevated privileges and is not fully implemented".into(),
        );
    }

    // If recursive and it's a directory, we'd recurse here
    if recursive && path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            chown_file(&entry.path(), owner, group, true)?;
        }
    }

    Ok(())
}

/// Windows chown implementation (not supported)
#[cfg(windows)]
fn chown_file(
    _path: &PathBuf,
    _owner: Option<&str>,
    _group: Option<&str>,
    _recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Windows doesn't have Unix-style ownership model
    Err("chown not supported on Windows".into())
}

/// Fallback chown for other platforms
#[cfg(not(any(unix, windows)))]
fn chown_file(
    _path: &PathBuf,
    _owner: Option<&str>,
    _group: Option<&str>,
    _recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("chown not supported on this platform".into())
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
    fn test_parse_chmod_mode_octal() {
        assert_eq!(parse_chmod_mode("755").unwrap(), 0o755);
        assert_eq!(parse_chmod_mode("644").unwrap(), 0o644);
        assert_eq!(parse_chmod_mode("777").unwrap(), 0o777);
    }

    #[test]
    fn test_parse_symbolic_operation() {
        // Test basic operations
        assert_eq!(parse_symbolic_operation(0o644, "u+x").unwrap(), 0o744);
        assert_eq!(parse_symbolic_operation(0o755, "g-w").unwrap(), 0o735);
        assert_eq!(parse_symbolic_operation(0o644, "o=rwx").unwrap(), 0o647);
    }

    #[test]
    fn test_parse_chmod_mode_invalid() {
        assert!(parse_chmod_mode("999").is_err());
        assert!(parse_chmod_mode("abc").is_ok()); // Will be parsed as symbolic
    }
}
