use libsam::services::coding::CodingAgentService;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Testing SAM Coding Agent with Remote Ollama Server");
    println!("================================================");

    // Initialize the coding agent
    let agent = CodingAgentService::new_with_defaults().await;

    println!("\n✓ Coding agent initialized");
    println!("  - Will use server from ~/.sam/coding_agent/ollama_config.json");
    println!("  - Expected: http://172.16.0.125:11434 with gpt-oss:20b");

    // Test a simple generation
    let current_dir = std::env::current_dir().unwrap();
    let session_context = vec![];

    println!("\n📝 Testing code generation...");
    println!("Prompt: Write a Python function that returns the fibonacci sequence");

    match agent
        .generate_response(
            "Write a Python function that returns the fibonacci sequence",
            &current_dir,
            &session_context,
            None,
        )
        .await
    {
        Ok(response) => {
            println!("\n✅ Success! Response received:");
            println!("Model used: {}", response.model_used);
            println!("\nGenerated code:");
            println!("{}", response.response_text);

            if response.model_used == "gpt-oss:20b" {
                println!("\n🎉 Confirmed: Using remote GPU server with gpt-oss:20b!");
            }
        }
        Err(e) => {
            println!("\n❌ Error: {}", e);
            println!("\nTroubleshooting:");
            println!(
                "1. Check if the remote server is running: curl http://172.16.0.125:11434/api/tags"
            );
            println!(
                "2. Verify the configuration file: cat ~/.sam/coding_agent/ollama_config.json"
            );
            println!("3. Ensure gpt-oss:20b model is available on the server");
        }
    }
}
