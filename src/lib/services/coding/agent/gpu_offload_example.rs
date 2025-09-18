// Example usage of GPU offloading with Ollama for desktop GPU acceleration
use anyhow::Result;
use crate::services::coding::agent::gpu_offload::{
    GpuOffloadManager, GpuOffloadConfig, GpuProvider,
};

/// Example configuration for using your GPU desktop rig with Ollama
pub async fn setup_ollama_gpu_offload() -> Result<GpuOffloadManager> {
    // Configure for your GPU desktop rig
    let config = GpuOffloadConfig {
        enabled: true,
        provider: GpuProvider::Ollama,
        auto_scale: false, // Not needed for local GPU
        max_instances: 1,
        budget_limit: None, // No cost for local GPU
        budget_alert_threshold: 0.0,
        idle_timeout_minutes: 60,
        preferred_gpu_types: vec!["RTX 4090".to_string()], // Your GPU
        preferred_regions: vec!["local".to_string()],
        min_vram_gb: 24,

        // Point to your GPU desktop rig running Ollama
        ollama_endpoint: Some("http://192.168.1.100:11434".to_string()), // Replace with your desktop IP
        ollama_api_key: None, // Usually not needed for local
        ollama_models: vec![
            "deepseek-coder:33b".to_string(),
            "codellama:34b".to_string(),
            "mixtral:8x22b".to_string(),
            "llama3:70b".to_string(),
        ],
        ollama_gpu_layers: Some(99), // Use all GPU layers
    };

    Ok(GpuOffloadManager::new(config))
}

/// Example of using GPU-accelerated code generation
pub async fn generate_code_with_gpu(manager: &GpuOffloadManager, session_id: &str) -> Result<()> {
    // Start GPU instance (connects to Ollama)
    let instance = manager.start_gpu_instance(session_id).await?;
    println!("Connected to GPU instance: {:?}", instance);

    // List available models
    let models = manager.list_available_models(session_id).await?;
    println!("Available models: {:?}", models);

    // Generate some code
    let prompt = r#"
Write a highly optimized Rust function to find all prime numbers up to n using the Sieve of Eratosthenes.
Include SIMD optimizations if possible.
"#;

    let generated_code = manager.generate_code(session_id, prompt, Some("deepseek-coder:33b".to_string())).await?;
    println!("Generated code:\n{}", generated_code);

    // Chat for code review
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You are an expert Rust programmer focused on performance optimization."
        }),
        serde_json::json!({
            "role": "user",
            "content": "Review this code for performance improvements:\n\n```rust\nfn fibonacci(n: u32) -> u64 {\n    if n <= 1 { return n as u64; }\n    fibonacci(n - 1) + fibonacci(n - 2)\n}\n```"
        }),
    ];

    let review = manager.chat_code(session_id, messages, None).await?;
    println!("Code review:\n{}", review);

    // Get GPU stats
    let gpu_stats = manager.get_gpu_stats(session_id).await?;
    println!("GPU stats: {}", serde_json::to_string_pretty(&gpu_stats)?);

    // Stop instance when done (for Ollama, this just cleans up tracking)
    manager.stop_gpu_instance(session_id).await?;

    Ok(())
}

/// Environment variable configuration for easy setup
pub fn setup_env_for_ollama() {
    // Set these environment variables to configure Ollama GPU offloading:
    // OLLAMA_HOST=http://your-gpu-desktop:11434
    // OLLAMA_API_KEY=your-key-if-needed
    // OLLAMA_NUM_GPU=99  # Use all GPU layers

    println!("To use your GPU desktop rig:");
    println!("1. Install Ollama on your GPU desktop: curl -fsSL https://ollama.ai/install.sh | sh");
    println!("2. Start Ollama: OLLAMA_HOST=0.0.0.0:11434 ollama serve");
    println!("3. Pull models: ollama pull deepseek-coder:33b");
    println!("4. Set OLLAMA_HOST environment variable: export OLLAMA_HOST=http://192.168.1.100:11434");
    println!("5. Run SAM with GPU offloading enabled");
}