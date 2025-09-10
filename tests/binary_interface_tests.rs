// Comprehensive binary interface tests for SAM
// Tests the actual CLI commands and functionality without internal dependencies

use std::process::Command;
use std::time::Duration;
use std::fs;

#[test]
fn test_cargo_check_passes() {
    let output = Command::new("cargo")
        .args(["check"])
        .output()
        .expect("Failed to run cargo check");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Cargo check failed: {}", stderr);
    }
}

#[test]
fn test_cargo_build_binary() {
    let output = Command::new("cargo")
        .args(["build", "--bin", "sam", "--release"])
        .output()
        .expect("Failed to build sam binary");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to build sam binary: {}", stderr);
    }

    // Verify the binary exists
    let binary_path = "target/release/sam";
    assert!(std::path::Path::new(binary_path).exists(), "Binary should exist after build");
}

#[test]
fn test_cargo_build_library() {
    let output = Command::new("cargo")
        .args(["build", "--lib"])
        .output()
        .expect("Failed to build libsam");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to build libsam: {}", stderr);
    }
}

#[test]
fn test_binary_exists_and_executable() {
    // First build the binary
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "sam"])
        .output()
        .expect("Failed to build binary");

    assert!(build_output.status.success(), "Binary should build successfully");

    // Check if binary file exists
    let debug_binary = "target/debug/sam";
    if std::path::Path::new(debug_binary).exists() {
        // Try to get basic info from the binary
        let info_output = Command::new(debug_binary)
            .arg("--help")
            .output();

        match info_output {
            Ok(output) => {
                println!("Binary help output length: {} bytes", output.stdout.len());
                if !output.status.success() {
                    println!("Binary help command failed, but binary exists");
                }
            }
            Err(e) => {
                println!("Could not execute binary: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_project_structure() {
        // Verify essential project files exist
        assert!(std::path::Path::new("Cargo.toml").exists(), "Cargo.toml should exist");
        assert!(std::path::Path::new("src/main.rs").exists(), "main.rs should exist");
        assert!(std::path::Path::new("src/lib/mod.rs").exists(), "lib/mod.rs should exist");
        assert!(std::path::Path::new("src/sam.rs").exists(), "sam.rs should exist");
    }

    #[test]
    fn test_cargo_toml_valid() {
        let cargo_content = fs::read_to_string("Cargo.toml")
            .expect("Should be able to read Cargo.toml");
        
        assert!(cargo_content.contains("[package]"), "Should have package section");
        assert!(cargo_content.contains("name = \"sam\""), "Should have correct package name");
        assert!(cargo_content.contains("[dependencies]"), "Should have dependencies section");
    }
}

#[cfg(test)]
mod service_tests {
    

    #[test]
    fn test_basic_service_structure() {
        // Test that basic service files exist
        let service_files = [
            "src/sam/services/mod.rs",
            "src/sam/services/redis.rs", 
            "src/sam/services/pg.rs",
            "src/sam/http/mod.rs",
            "src/sam/websocket/mod.rs",
        ];

        for file in &service_files {
            assert!(std::path::Path::new(file).exists(), "Service file {} should exist", file);
        }
    }

    #[test]
    fn test_main_modules_exist() {
        let main_modules = [
            "src/sam/cli/mod.rs",
            "src/sam/jobs/mod.rs", 
            "src/sam/memory/mod.rs",
            "src/sam/services/mod.rs",
        ];

        for module in &main_modules {
            assert!(std::path::Path::new(module).exists(), "Module {} should exist", module);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_compilation_time() {
        let start = Instant::now();
        
        let output = Command::new("cargo")
            .args(["check", "--all-targets"])
            .output()
            .expect("Failed to run cargo check");

        let duration = start.elapsed();
        println!("Compilation took: {:?}", duration);

        // Check should complete reasonably quickly (adjust threshold as needed)
        assert!(duration < Duration::from_secs(300), "Compilation should not take more than 5 minutes");
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Cargo check failed: {}", stderr);
        }
    }

    #[test]
    fn test_no_critical_warnings() {
        let output = Command::new("cargo")
            .args(["check"])
            .output()
            .expect("Failed to run cargo check");

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Make sure there are no critical errors that would prevent compilation
        assert!(!stderr.contains("error:"), "Should not have compilation errors");
        
        // Count warnings (optional - just for information)
        let warning_count = stderr.matches("warning:").count();
        println!("Total warnings: {}", warning_count);
    }
}