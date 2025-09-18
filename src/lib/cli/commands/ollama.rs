use std::sync::Arc;
use tokio::sync::Mutex;
use crate::services::llms::ollama::OllamaService;

/// Handle Ollama CLI commands in the TUI
pub async fn handle_ollama(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let service = OllamaService::new_with_defaults();
    
    match cmd.trim() {
        "ollama" | "ollama help" => {
            show_ollama_help(output_lines).await;
        }
        "ollama status" => {
            check_ollama_status(&service, output_lines).await;
        }
        "ollama install" => {
            install_ollama(&service, output_lines).await;
        }
        "ollama start" => {
            start_ollama(&service, output_lines).await;
        }
        "ollama stop" => {
            stop_ollama(&service, output_lines).await;
        }
        "ollama list" | "ollama models" => {
            list_models(&service, output_lines).await;
        }
        "ollama search" => {
            search_models(&service, "", output_lines).await;
        }
        "ollama install-recommended" => {
            install_recommended_models(&service, output_lines).await;
        }
        _ if cmd.starts_with("ollama pull ") => {
            let model = cmd.trim_start_matches("ollama pull ").trim();
            if model.is_empty() {
                let mut lines = output_lines.lock().await;
                lines.push("Usage: ollama pull <model_name>".to_string());
                lines.push("Example: ollama pull llama3.2".to_string());
            } else {
                pull_model(&service, model, output_lines).await;
            }
        }
        _ if cmd.starts_with("ollama remove ") => {
            let model = cmd.trim_start_matches("ollama remove ").trim();
            if model.is_empty() {
                let mut lines = output_lines.lock().await;
                lines.push("Usage: ollama remove <model_name>".to_string());
                lines.push("Example: ollama remove llama3.2".to_string());
            } else {
                remove_model(&service, model, output_lines).await;
            }
        }
        _ if cmd.starts_with("ollama run ") => {
            let rest = cmd.trim_start_matches("ollama run ").trim();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() < 2 || parts[1].is_empty() {
                let mut lines = output_lines.lock().await;
                lines.push("Usage: ollama run <model_name> <prompt>".to_string());
                lines.push("Example: ollama run llama3.2 \"Hello, how are you?\"".to_string());
            } else {
                let model = parts[0];
                let prompt = parts[1];
                generate_text(&service, model, prompt, output_lines).await;
            }
        }
        _ if cmd.starts_with("ollama search ") => {
            let query = cmd.trim_start_matches("ollama search ").trim();
            search_models(&service, query, output_lines).await;
        }
        _ if cmd.starts_with("ollama info ") => {
            let model = cmd.trim_start_matches("ollama info ").trim();
            if model.is_empty() {
                let mut lines = output_lines.lock().await;
                lines.push("Usage: ollama info <model_name>".to_string());
            } else {
                show_model_info(&service, model, output_lines).await;
            }
        }
        _ => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("Unknown ollama command: {}", cmd));
            lines.push("Type 'ollama help' to see available commands".to_string());
        }
    }
}

async fn show_ollama_help(output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push("Ollama CLI Commands:".to_string());
    lines.push("".to_string());
    lines.push("  ollama help                    - Show this help".to_string());
    lines.push("  ollama status                  - Check Ollama service status".to_string());
    lines.push("  ollama install                 - Install Ollama if not present".to_string());
    lines.push("  ollama start                   - Start Ollama service".to_string());
    lines.push("  ollama stop                    - Stop Ollama service".to_string());
    lines.push("".to_string());
    lines.push("Model Management:".to_string());
    lines.push("  ollama list                    - List installed models".to_string());
    lines.push("  ollama pull <model>            - Download a model (e.g., llama3.2)".to_string());
    lines.push("  ollama remove <model>          - Remove a model".to_string());
    lines.push("  ollama search [query]          - Search available models".to_string());
    lines.push("  ollama info <model>            - Show model information".to_string());
    lines.push("  ollama install-recommended     - Install recommended models".to_string());
    lines.push("".to_string());
    lines.push("AI Generation:".to_string());
    lines.push("  ollama run <model> <prompt>    - Generate text with a model".to_string());
    lines.push("".to_string());
    lines.push("Examples:".to_string());
    lines.push("  ollama pull llama3.2".to_string());
    lines.push("  ollama run llama3.2 \"Explain quantum computing\"".to_string());
    lines.push("  ollama search code".to_string());
}

async fn check_ollama_status(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    
    let installed = service.is_installed().await;
    let running = if installed { service.is_running().await } else { false };
    
    lines.push("Ollama Status:".to_string());
    lines.push(format!("  Installed: {}", if installed { "✓ Yes" } else { "✗ No" }));
    lines.push(format!("  Running:   {}", if running { "✓ Yes" } else { "✗ No" }));
    
    if running {
        match service.get_version().await {
            Ok(version) => {
                // Parse version JSON and extract just the version string
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&version) {
                    if let Some(version_str) = v.get("version").and_then(|v| v.as_str()) {
                        lines.push(format!("  Version:   {}", version_str));
                    }
                } else {
                    lines.push(format!("  Version:   {}", version));
                }
            }
            Err(e) => lines.push(format!("  Version:   Error - {}", e)),
        }
        
        match service.get_installed_model_names().await {
            Ok(models) => {
                lines.push(format!("  Models:    {} installed", models.len()));
            }
            Err(e) => lines.push(format!("  Models:    Error - {}", e)),
        }
    }
    
    if !installed {
        lines.push("".to_string());
        lines.push("Run 'ollama install' to install Ollama.".to_string());
    } else if !running {
        lines.push("".to_string());
        lines.push("Run 'ollama start' to start the service.".to_string());
    }
}

async fn install_ollama(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    let service_clone = service.clone();
    
    crate::cli::spinner::run_with_spinner(
        output_lines,
        "Installing Ollama...",
        |lines, result| {
            if result.starts_with("ERROR:") {
                lines.push(result.trim_start_matches("ERROR: ").to_string());
            } else {
                lines.push(format!("✓ {}", result));
            }
        },
        move || {
            async move {
                match service_clone.install().await {
                    Ok(message) => message,
                    Err(e) => format!("ERROR: ✗ Installation failed: {}", e),
                }
            }
        }
    ).await;
}

async fn start_ollama(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push("Starting Ollama service...".to_string());
    drop(lines);
    
    match service.start_service().await {
        Ok(message) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✓ {}", message));
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Start failed: {}", e));
        }
    }
}

async fn stop_ollama(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push("Stopping Ollama service...".to_string());
    drop(lines);
    
    match service.stop_service().await {
        Ok(message) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✓ {}", message));
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Stop failed: {}", e));
        }
    }
}

async fn list_models(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    match service.list_all_models().await {
        Ok((installed_models, available_models)) => {
            let mut lines = output_lines.lock().await;

            // Show installed models first
            if installed_models.is_empty() {
                lines.push("Installed Models (0):".to_string());
                lines.push("".to_string());
                lines.push("  No models installed.".to_string());
            } else {
                lines.push(format!("Installed Models ({}):", installed_models.len()));
                lines.push("".to_string());
                for model in &installed_models {
                    let size_gb = model.size as f64 / (1024.0 * 1024.0 * 1024.0);
                    lines.push(format!("  {} ({:.1} GB)", model.name, size_gb));
                }
            }

            lines.push("".to_string());
            lines.push("Available Models for Installation:".to_string());
            lines.push("".to_string());

            // Group available models by category for better organization
            let mut llama_models = Vec::new();
            let mut code_models = Vec::new();
            let mut mistral_models = Vec::new();
            let mut gemma_models = Vec::new();
            let mut deepseek_models = Vec::new();
            let mut other_models = Vec::new();

            for model in &available_models {
                if model.starts_with("llama") {
                    llama_models.push(model);
                } else if model.starts_with("code") || model.contains("coder") {
                    code_models.push(model);
                } else if model.starts_with("mistral") || model.contains("mixtral") {
                    mistral_models.push(model);
                } else if model.starts_with("gemma") {
                    gemma_models.push(model);
                } else if model.starts_with("deepseek") {
                    deepseek_models.push(model);
                } else {
                    other_models.push(model);
                }
            }

            // Display models by category
            if !llama_models.is_empty() {
                lines.push("  Llama Models:".to_string());
                for model in llama_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            if !code_models.is_empty() {
                lines.push("  Code Models:".to_string());
                for model in code_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            if !mistral_models.is_empty() {
                lines.push("  Mistral Models:".to_string());
                for model in mistral_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            if !gemma_models.is_empty() {
                lines.push("  Gemma Models:".to_string());
                for model in gemma_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            if !deepseek_models.is_empty() {
                lines.push("  DeepSeek Models:".to_string());
                for model in deepseek_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            if !other_models.is_empty() {
                lines.push("  Other Models:".to_string());
                for model in other_models {
                    lines.push(format!("    {}", model));
                }
                lines.push("".to_string());
            }

            lines.push("Use 'ollama pull <model>' to install a model.".to_string());
            lines.push("Example: ollama pull llama3.2:latest".to_string());
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Failed to list models: {}", e));
            if e.to_string().contains("connection") {
                lines.push("Make sure Ollama service is running: 'ollama start'".to_string());
            }
        }
    }
}

async fn pull_model(service: &OllamaService, model: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push("This may take several minutes depending on model size.".to_string());
    drop(lines);
    
    let service_clone = service.clone();
    let model_clone = model.to_string();
    
    crate::cli::spinner::run_with_spinner(
        output_lines,
        &format!("Pulling model: {}...", model),
        |lines, result| {
            if result.starts_with("ERROR:") {
                let error_lines: Vec<&str> = result.trim_start_matches("ERROR: ").split('\n').collect();
                for line in error_lines {
                    if !line.is_empty() {
                        lines.push(line.to_string());
                    }
                }
            } else {
                lines.push(format!("✓ {}", result));
            }
        },
        move || {
            async move {
                match service_clone.pull_model(&model_clone).await {
                    Ok(message) => message,
                    Err(e) => {
                        let mut error_msg = format!("ERROR: ✗ Failed to pull model '{}': {}", model_clone, e);
                        if e.to_string().contains("connection") {
                            error_msg.push_str("\nMake sure Ollama service is running: 'ollama start'");
                        }
                        error_msg
                    }
                }
            }
        }
    ).await;
}

async fn remove_model(service: &OllamaService, model: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match service.remove_model(model).await {
        Ok(message) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✓ {}", message));
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Failed to remove model '{}': {}", model, e));
        }
    }
}

async fn search_models(service: &OllamaService, query: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match service.search_models(query).await {
        Ok(models) => {
            let mut lines = output_lines.lock().await;
            if models.is_empty() {
                if query.is_empty() {
                    lines.push("Popular models available:".to_string());
                } else {
                    lines.push(format!("No models found matching '{}'", query));
                }
            } else {
                if query.is_empty() {
                    lines.push("Popular models available:".to_string());
                } else {
                    lines.push(format!("Models matching '{}' ({}):", query, models.len()));
                }
                lines.push("".to_string());
                for model in models {
                    lines.push(format!("  {}", model));
                }
                lines.push("".to_string());
                lines.push("Use 'ollama pull <model>' to install a model.".to_string());
            }
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Failed to search models: {}", e));
        }
    }
}

async fn show_model_info(service: &OllamaService, model: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match service.show_model(model).await {
        Ok(info) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("Model Information: {}", model));
            lines.push("".to_string());
            
            // Pretty print the JSON information
            if let Ok(formatted) = serde_json::to_string_pretty(&info) {
                // Split into lines and add each one
                for line in formatted.lines() {
                    lines.push(format!("  {}", line));
                }
            } else {
                lines.push(format!("  {}", info));
            }
        }
        Err(e) => {
            let mut lines = output_lines.lock().await;
            lines.push(format!("✗ Failed to get model info for '{}': {}", model, e));
            if e.to_string().contains("connection") {
                lines.push("Make sure Ollama service is running: 'ollama start'".to_string());
            }
        }
    }
}

async fn generate_text(service: &OllamaService, model: &str, prompt: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push(format!("Prompt: {}", prompt));
    lines.push("".to_string());
    drop(lines);
    
    let service_clone = service.clone();
    let model_clone = model.to_string();
    let prompt_clone = prompt.to_string();
    
    crate::cli::spinner::run_with_spinner(
        output_lines,
        &format!("Generating with model '{}'...", model),
        |lines, result| {
            if result.starts_with("ERROR:") {
                lines.push(result.to_string());
            } else {
                lines.push("Response:".to_string());
                lines.push("".to_string());
                
                // Parse the result to extract response and timing
                if let Some((response_text, timing)) = result.split_once("||TIMING||") {
                    for line in response_text.lines() {
                        lines.push(format!("  {}", line));
                    }
                    lines.push("".to_string());
                    lines.push(format!("Generated in {}s", timing));
                } else {
                    for line in result.lines() {
                        lines.push(format!("  {}", line));
                    }
                }
            }
        },
        move || {
            async move {
                match service_clone.generate(&model_clone, &prompt_clone, None).await {
                    Ok(response) => {
                        let duration = if let Some(total_duration) = response.total_duration {
                            format!("{:.2}", total_duration as f64 / 1_000_000_000.0)
                        } else {
                            "unknown".to_string()
                        };
                        format!("{}||TIMING||{}", response.response, duration)
                    }
                    Err(e) => {
                        let mut error_msg = format!("ERROR: ✗ Failed to generate text: {}", e);
                        if e.to_string().contains("connection") {
                            error_msg.push_str("\nMake sure Ollama service is running: 'ollama start'");
                        } else if e.to_string().contains("not found") {
                            error_msg.push_str(&format!("\nModel '{}' not found. Use 'ollama pull {}' to install it.", model_clone, model_clone));
                        }
                        error_msg
                    }
                }
            }
        }
    ).await;
}

async fn install_recommended_models(service: &OllamaService, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut lines = output_lines.lock().await;
    lines.push("This will install: llama3.2, codellama, mistral, gemma2:2b, and phi3:mini".to_string());
    lines.push("This may take several minutes.".to_string());
    drop(lines);

    let service_clone = service.clone();

    crate::cli::spinner::run_with_spinner(
        output_lines,
        "Installing recommended models...",
        |lines, result| {
            if result.starts_with("ERROR:") {
                lines.push(result.trim_start_matches("ERROR: ").to_string());
            } else {
                lines.push("".to_string());
                lines.push("Installation Results:".to_string());
                for line in result.lines() {
                    lines.push(format!("  {}", line));
                }
            }
        },
        move || {
            async move {
                match service_clone.install_recommended_models().await {
                    Ok(message) => message,
                    Err(e) => format!("ERROR: ✗ Failed to install recommended models: {}", e),
                }
            }
        }
    ).await;
}