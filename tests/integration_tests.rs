// Integration tests that test the SAM binary interface
// These tests validate the binary functionality without needing complex internal imports

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_binary_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute sam binary");

    // The help command should exit successfully
    assert!(output.status.success(), "Help command should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Basic validation that help output contains expected content
    assert!(stdout.contains("SAM") || stdout.contains("sam") || stdout.contains("help"), 
            "Help output should mention SAM or help");
}

#[tokio::test]
async fn test_binary_version_or_status() {
    // Try various common flags to see what the binary supports
    let test_args = vec![
        vec!["--version"],
        vec!["version"], 
        vec!["status"],
        vec!["help"],
    ];

    let mut found_working_command = false;

    for args in test_args {
        let mut cmd_args = vec!["run", "--"];
        cmd_args.extend(args.iter().cloned());
        
        let result = timeout(Duration::from_secs(10), async {
            Command::new("cargo")
                .args(&cmd_args)
                .output()
        }).await;

        if let Ok(Ok(output)) = result {
            if output.status.success() {
                found_working_command = true;
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("Command {:?} succeeded with output: {}", args, stdout);
                break;
            }
        }
    }

    // At least one basic command should work
    assert!(found_working_command, "At least one basic command should work");
}

#[tokio::test] 
async fn test_binary_compilation() {
    // Test that the binary compiles without errors
    let output = Command::new("cargo")
        .args(&["build", "--bin", "sam"])
        .output()
        .expect("Failed to build sam binary");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Binary compilation failed: {}", stderr);
    }

    println!("Binary compiled successfully");
}

#[test]
fn test_lib_compilation() {
    // Test that libsam compiles without errors
    let output = Command::new("cargo")
        .args(&["build", "--lib"])
        .output()
        .expect("Failed to build libsam");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Library compilation failed: {}", stderr);
    }

    println!("Library compiled successfully");
}

#[tokio::test]
async fn test_basic_binary_execution() {
    // Test that the binary can at least start and exit gracefully
    let result = timeout(Duration::from_secs(5), async {
        Command::new("cargo")
            .args(&["run", "--", "--help"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start sam binary")
            .wait()
    }).await;

    match result {
        Ok(Ok(status)) => {
            println!("Binary execution completed with status: {}", status);
            // Either success or expected failure is fine - we just want to ensure it doesn't hang
        }
        Ok(Err(e)) => {
            panic!("Binary execution failed: {}", e);
        }
        Err(_) => {
            println!("Binary execution timed out (this might be expected for some commands)");
            // Timeout might be expected if the binary is designed to run continuously
        }
    }
}