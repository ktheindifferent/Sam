// Advanced Coding Agent Demo - Showcasing intelligent code analysis features
// This example demonstrates the enhanced coding agent capabilities:
// - Code analysis and metrics
// - Test generation
// - Refactoring suggestions
// - Debugging assistance
// - Code review

use sam::lib::services::coding::agent::{
    CodingAgentService, CodingAgentConfig
};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== SAM Advanced Coding Agent Demo ===\n");

    // Create the coding agent with default configuration
    let config = CodingAgentConfig::default();
    let agent = CodingAgentService::new(config);

    // Get current directory
    let current_dir = std::env::current_dir()?;

    // Demo 1: Code Analysis
    println!("Demo 1: Code Analysis");
    println!("---------------------");

    // Create a sample Rust file for analysis
    let sample_code = r#"
use std::collections::HashMap;

fn calculate_fibonacci(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }

    // TODO: Optimize this with memoization
    calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2)
}

fn main() {
    let result = calculate_fibonacci(10).unwrap(); // This will panic!
    println!("Fibonacci of 10: {}", result);

    let mut data = HashMap::new();
    data.insert("key", "value");

    // Very long line that exceeds the recommended 100 character limit and should be broken into multiple lines for better readability
    println!("Data: {:?}", data);
}
"#;

    // Save to temp file
    let temp_file = current_dir.join("temp_analysis.rs");
    tokio::fs::write(&temp_file, sample_code).await?;

    // Analyze the code
    match agent.analyze_code_file(&temp_file, &current_dir).await {
        Ok(report) => {
            println!("Language: {}", report.language);
            println!("Metrics:");
            println!("  - Total lines: {}", report.metrics.total_lines);
            println!("  - Cyclomatic complexity: {}", report.metrics.cyclomatic_complexity);
            println!("  - Max nesting depth: {}", report.metrics.max_nesting_depth);

            println!("\nCode Structure:");
            println!("  - Functions: {:?}", report.structure.functions);
            println!("  - Code lines: {}", report.structure.code_lines);
            println!("  - Comment lines: {}", report.structure.comment_lines);

            println!("\nIssues Found:");
            for issue in &report.issues {
                println!("  - Line {}: {} - {}", issue.line,
                    format!("{:?}", issue.severity), issue.message);
            }

            println!("\nAI Suggestions:");
            for (i, suggestion) in report.suggestions.iter().enumerate() {
                println!("  {}. {}", i + 1, suggestion);
            }
        }
        Err(e) => println!("Analysis failed: {}", e),
    }

    // Demo 2: Test Generation
    println!("\n\nDemo 2: Test Generation");
    println!("-----------------------");

    let function_code = r#"
fn validate_email(email: &str) -> bool {
    if email.is_empty() {
        return false;
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}
"#;

    match agent.generate_tests(function_code, "rust", &current_dir).await {
        Ok(tests) => {
            println!("Generated Tests:");
            println!("{}", tests);
        }
        Err(e) => println!("Test generation failed: {}", e),
    }

    // Demo 3: Refactoring Suggestions
    println!("\n\nDemo 3: Refactoring Suggestions");
    println!("--------------------------------");

    let code_to_refactor = r#"
fn process_data(data: Vec<i32>) -> i32 {
    let mut sum = 0;
    for i in 0..data.len() {
        if data[i] > 0 {
            if data[i] < 100 {
                sum = sum + data[i];
            } else {
                sum = sum + 100;
            }
        }
    }
    return sum;
}
"#;

    match agent.suggest_refactoring(code_to_refactor, "rust", "simplification").await {
        Ok(suggestion) => {
            println!("Original Code:");
            println!("{}", suggestion.original_code);
            println!("\nRefactored Code:");
            println!("{}", suggestion.refactored_code);
            println!("\nExplanation:");
            println!("{}", suggestion.explanation);
            println!("\nBenefits:");
            for benefit in &suggestion.benefits {
                println!("  - {}", benefit);
            }
        }
        Err(e) => println!("Refactoring suggestion failed: {}", e),
    }

    // Demo 4: Debugging Assistance
    println!("\n\nDemo 4: Debugging Assistance");
    println!("-----------------------------");

    let error_message = "thread 'main' panicked at 'called `Option::unwrap()` on a `None` value'";
    let error_context = r#"
let config = load_config("config.toml");
let port = config.get("server").unwrap().get("port").unwrap();
println!("Server port: {}", port);
"#;

    match agent.debug_assistance(error_message, error_context, "rust").await {
        Ok(help) => {
            println!("Error: {}", help.error);
            println!("\nRoot Cause:");
            println!("{}", help.root_cause);

            println!("\nDebugging Steps:");
            for (i, step) in help.debugging_steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }

            println!("\nPotential Fixes:");
            for fix in &help.potential_fixes {
                println!("  - {}", fix);
            }

            println!("\nPrevention:");
            println!("{}", help.prevention);
        }
        Err(e) => println!("Debugging assistance failed: {}", e),
    }

    // Demo 5: Code Review
    println!("\n\nDemo 5: Interactive Code Review");
    println!("--------------------------------");

    let code_to_review = r#"
use std::fs::File;
use std::io::Read;

fn read_password_file() -> String {
    let mut file = File::open("/etc/passwd").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    return contents;
}

fn authenticate(username: &str, password: &str) -> bool {
    if username == "admin" && password == "password123" {
        true
    } else {
        false
    }
}
"#;

    match agent.review_code(code_to_review, "rust", vec!["security", "error handling"]).await {
        Ok(review) => {
            println!("Overall Score: {}/10", review.overall_score);
            println!("\nSummary:");
            println!("{}", review.summary);

            println!("\nIssues:");
            for issue in &review.issues {
                println!("  - [{}] {}", issue.severity, issue.description);
            }

            println!("\nSecurity Concerns:");
            for concern in &review.security_concerns {
                println!("  ⚠️  {}", concern);
            }

            println!("\nPerformance Notes:");
            for note in &review.performance_notes {
                println!("  ⚡ {}", note);
            }

            println!("\nSuggestions:");
            for suggestion in &review.suggestions {
                println!("  💡 {}", suggestion);
            }
        }
        Err(e) => println!("Code review failed: {}", e),
    }

    // Clean up temp file
    let _ = tokio::fs::remove_file(&temp_file).await;

    println!("\n=== Demo Complete ===");
    println!("\nThe coding agent now provides:");
    println!("✅ Comprehensive code analysis with metrics");
    println!("✅ Automatic test generation");
    println!("✅ Intelligent refactoring suggestions");
    println!("✅ Debugging assistance with root cause analysis");
    println!("✅ Interactive code reviews with security focus");
    println!("✅ Language-aware code understanding");
    println!("✅ Real-time streaming responses");
    println!("✅ Conversation memory for context");
    println!("✅ Error recovery with fallback models");

    Ok(())
}