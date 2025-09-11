use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::misc::handle_pwd;

    #[tokio::test]
    async fn test_pwd_command() {
        // Create output buffer
        let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        
        // Test with current directory
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        
        // Call our pwd function
        handle_pwd(&output_lines, &current_dir).await;
        
        // Get the result
        let result = output_lines.lock().await;
        
        // Verify they match
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], current_dir.display().to_string());
        
        println!("✅ pwd command works correctly!");
        println!("Expected: {}", current_dir.display());
        println!("Got: {}", result[0]);
    }
}
