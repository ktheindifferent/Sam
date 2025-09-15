use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import our new pwd function
use sam::cli::commands::misc::handle_pwd;

#[tokio::main]
async fn main() {
    // Create output buffer
    let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    
    // Test with current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    
    println!("Testing pwd command...");
    println!("Expected: {}", current_dir.display());
    
    // Call our pwd function
    handle_pwd(&output_lines, &current_dir).await;
    
    // Get the result
    let result = output_lines.lock().await;
    println!("Got: {}", result[0]);
    
    // Verify they match
    assert_eq!(result[0], current_dir.display().to_string());
    println!("✅ pwd command works correctly!");
}
