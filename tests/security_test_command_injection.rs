/// Security test: Command Injection Prevention
/// 
/// This test verifies that the SAM project uses safe command execution
/// methods that prevent command injection attacks.

#[cfg(test)]
mod security_tests {
    use std::path::Path;
    
    #[test]
    fn test_no_vulnerable_cmd_function_calls() {
        // Verify that tools.rs no longer contains the unsafe cmd() function
        let tools_path = Path::new("src/lib/tools.rs");
        assert!(tools_path.exists(), "tools.rs should exist");
        
        let content = std::fs::read_to_string(tools_path)
            .expect("Should read tools.rs");
        
        // The unsafe cmd() function should have been removed
        assert!(!content.contains("pub fn cmd(command: &str) -> Result<String>"),
                "Unsafe cmd() function should be removed from tools.rs");
        
        // The unsafe uinx_cmd() function should have been removed
        assert!(!content.contains("pub fn uinx_cmd(command: &str)"),
                "Unsafe uinx_cmd() function should be removed from tools.rs");
        
        // safe_cmd should exist
        assert!(content.contains("pub fn safe_cmd(program: &str, args: &[&str])"),
                "safe_cmd() function should exist in tools.rs");
        
        // safe_uinx_cmd should exist
        assert!(content.contains("pub fn safe_uinx_cmd(program: &str, args: &[&str])"),
                "safe_uinx_cmd() function should exist in tools.rs");
    }
    
    #[test]
    fn test_sprec_uses_safe_command_execution() {
        let sprec_path = Path::new("src/lib/services/sprec.rs");
        assert!(sprec_path.exists(), "sprec.rs should exist");
        
        let content = std::fs::read_to_string(sprec_path)
            .expect("Should read sprec.rs");
        
        // Should not use unsafe cmd() with shell injection
        assert!(!content.contains("crate::tools::cmd(\"python3"),
                "sprec.rs should not use unsafe cmd() with hardcoded python3 commands");
        
        // Should use safe_cmd instead
        assert!(content.contains("crate::tools::safe_cmd(\"python3\""),
                "sprec.rs should use safe_cmd() for command execution");
    }
    
    #[test]
    fn test_no_shell_injection_patterns_in_codebase() {
        // Check critical service files for shell injection patterns
        let critical_files = vec![
            "src/lib/services/snapcast.rs",
            "src/lib/services/who.rs",
            "src/lib/services/sound.rs",
        ];
        
        for file_path in critical_files {
            let path = Path::new(file_path);
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .expect(&format!("Should read {}", file_path));
                
                // Check for direct shell command patterns that could be dangerous
                // Good pattern: safe_uinx_cmd("program", &["arg1", "arg2"])
                // Bad pattern: uinx_cmd("shell command with user input".to_string())
                
                // Verify the file doesn't have the dangerous commented pattern
                // (We fixed it earlier)
                if file_path == "src/lib/services/sound.rs" {
                    assert!(!content.contains("crate::tools::uinx_cmd(\"aplay"),
                            "sound.rs should not use uinx_cmd with direct shell commands");
                }
            }
        }
    }
    
    #[test]
    fn test_safe_cmd_function_api() {
        // Document the safe API usage pattern
        let expected_usage = "safe_cmd(program: &str, args: &[&str]) -> Result<String>";
        
        let tools_path = Path::new("src/lib/tools.rs");
        let content = std::fs::read_to_string(tools_path)
            .expect("Should read tools.rs");
        
        // The function should separate program from arguments
        assert!(content.contains("Command::new(program)"),
                "safe_cmd should use Command::new with program name only");
        
        assert!(content.contains(".args(args)"),
                "safe_cmd should pass arguments separately using .args()");
        
        // No shell invocation
        assert!(!content.contains("Command::new(\"sh\")"),
                "safe_cmd should not use shell (sh) for execution");
    }
    
    #[test]
    fn test_connection_pool_safety() {
        let pool_path = Path::new("src/lib/db/connection_pool.rs");
        if pool_path.exists() {
            let content = std::fs::read_to_string(pool_path)
                .expect("Should read connection_pool.rs");
            
            // Should not have transmute for unsafe type conversion
            assert!(!content.contains("std::mem::transmute"),
                    "connection_pool.rs should not use unsafe transmute");
            
            // Should use safe type conversions
            assert!(content.contains("ToSql + Sync"),
                    "Should use safe type trait bounds for parameters");
        }
    }
}
