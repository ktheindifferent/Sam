use sam::cli::commands::help::handle_help;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_help_contains_filesystem_commands() {
    let output_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    
    // Call the help function
    handle_help(&output_lines).await;
    
    // Get the output
    let result = output_lines.lock().await;
    let help_text = result.join("\n");
    
    // Verify that all new filesystem commands are documented
    assert!(help_text.contains("pwd"), "Help should contain pwd command");
    assert!(help_text.contains("mkdir"), "Help should contain mkdir command"); 
    assert!(help_text.contains("rmdir"), "Help should contain rmdir command");
    assert!(help_text.contains("cp <src> <dest>"), "Help should contain cp command");
    assert!(help_text.contains("mv <src> <dest>"), "Help should contain mv command");
    assert!(help_text.contains("rm <file>"), "Help should contain rm command");
    
    // Verify command descriptions
    assert!(help_text.contains("Print current working directory"), "pwd should have proper description");
    assert!(help_text.contains("Create directories"), "mkdir should have proper description");
    assert!(help_text.contains("Remove empty directories"), "rmdir should have proper description");
    assert!(help_text.contains("Copy files or directories"), "cp should have proper description");
    assert!(help_text.contains("Move/rename files"), "mv should have proper description");
    assert!(help_text.contains("Remove files or directories"), "rm should have proper description");
    
    println!("✅ All filesystem commands are properly documented in help!");
}
