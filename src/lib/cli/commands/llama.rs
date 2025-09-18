use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::{fs, process::Command};
use tokio::io::AsyncBufReadExt;
use std::path::PathBuf;

// use crate::cli::helpers::{run_command_stream_lines, append_line, append_and_tts};

pub async fn handle_llama(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "llama install" => {
            let output_lines_cloned = Arc::clone(output_lines);
            crate::cli::spinner::run_with_spinner(
                output_lines,
                "Installing llama models and binaries...",
                |lines, _| lines.push("llama install: done.".to_string()),
                move || {
                    let output_lines = Arc::clone(&output_lines_cloned);
                    async move {
                        match install(Some(&output_lines)).await {
                            Ok(_) => {}
                            Err(e) => {
                                let mut out = output_lines.lock().await;
                                out.push(format!("llama install error: {}", e));
                            }
                        }
                        "done".to_string()
                    }
                },
            )
            .await;
        }
        _ if cmd.starts_with("llama2 ") => {
            let prompt = cmd.trim_start_matches("llama2 ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama2 <prompt>".to_string());
            } else {
                crate::cli::spinner::run_with_spinner(
                    output_lines,
                    "Querying llama2...",
                    |lines, result| lines.push(format!("llama2: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::services::llms::llama::LlamaService::query_v2(&prompt)
                                .unwrap_or_else(|e| format!("llama2 error: {}", e))
                        }
                    },
                )
                .await;
            }
        }
        _ if cmd.starts_with("llama2-tiny ") => {
            let prompt = cmd.trim_start_matches("llama2-tiny ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama2-tiny <prompt>".to_string());
            } else {
                crate::cli::spinner::run_with_spinner(
                    output_lines,
                    "Querying llama2-tiny...",
                    |lines, result| lines.push(format!("llama2-tiny: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::services::llms::llama::LlamaService::query_v2_tiny(&prompt)
                                .unwrap_or_else(|e| format!("llama2-tiny error: {}", e))
                        }
                    },
                )
                .await;
            }
        }
        _ if cmd.starts_with("llama ") => {
            let rest = cmd["llama ".len()..].trim().to_string();
            
            if rest.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama <model_path> <prompt> or llama <prompt> (with default model)".to_string());
                return;
            }
            
            // Check if the input might be just a prompt (no model path specified)
            let default_model_path = PathBuf::from("/opt/sam/models/tinyllama-1.1b-chat-v1.0.Q4_0.gguf");
            
            // Check if the first part looks like a file path (contains /, ends with .gguf, etc.)
            // If not, assume the entire input is a prompt and use the default model
            let (model_path_str, prompt_str) = if rest.contains('/') || rest.ends_with(".gguf") || rest.ends_with(".bin") {
                // Looks like a model path was provided, split on the first space
                let mut split = rest.splitn(2, ' ');
                let first_part = split.next().unwrap_or("").to_string();
                let second_part = split.next().unwrap_or("").to_string();
                
                if second_part.is_empty() {
                    let mut out = output_lines.lock().await;
                    out.push("Usage: llama <model_path> <prompt>".to_string());
                    return;
                }
                (first_part, second_part)
            } else if default_model_path.exists() {
                // Treat entire input as prompt and use default model
                (default_model_path.to_string_lossy().to_string(), rest)
            } else {
                // No default model exists, show usage
                let mut out = output_lines.lock().await;
                out.push("Default model not found at /opt/sam/models/tinyllama-1.1b-chat-v1.0.Q4_0.gguf".to_string());
                out.push("Usage: llama <model_path> <prompt> or llama <prompt> (with default model)".to_string());
                return;
            };

            crate::cli::spinner::run_with_spinner(
                output_lines,
                &format!("Querying llama model {}...", model_path_str),
                |lines, result| lines.push(format!("llama: {}", result)),
                move || {
                    let model_path = std::path::PathBuf::from(model_path_str.clone());
                    let prompt = prompt_str.clone();
                    async move {
                        crate::services::llms::llama::LlamaService::query(&model_path, &prompt)
                            .unwrap_or_else(|e| format!("llama error: {}", e))
                    }
                },
            )
            .await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown llama command.".to_string());
        }
    }
}

// Helper: Run a command and stream output lines
async fn run_command_stream_lines(mut cmd: Command, output_lines: Option<&Arc<Mutex<Vec<String>>>>, prefix: &str) -> Result<()> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut lines = vec![];
    if let Some(stdout) = stdout {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await? {
            crate::println(output_lines, line.clone()).await;
            if output_lines.is_none() {
                let msg = format!("{}: {}", prefix, line);
                println!("{}", msg);
            }
            lines.push(line);
        }
    }
    if let Some(stderr) = stderr {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await? {
            crate::println(output_lines, line.clone()).await;
            if output_lines.is_none() {
                let msg = format!("{}: {}", prefix, line);
                println!("{}", msg);
            }
            lines.push(line);
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("{} failed: {:?}", prefix, lines);
    }
    Ok(())
}

pub async fn install(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<()> {
    let repositories_dir = Path::new("/opt/sam/repositories");
    let llama_repo_dir = repositories_dir.join("llama.cpp");
    let llama_cli = PathBuf::from("/opt/sam/bin/llama-cli");

    if llama_cli.exists() {
        crate::println(output_lines, "llama-cli binary already exists.".to_string()).await;
        return Ok(());
    }

    let repo_url = "https://github.com/ggml-org/llama.cpp.git";
    let bin_dir = Path::new("/opt/sam/bin");

    // Ensure /opt/sam/repositories exists
    if !repositories_dir.exists() {
        fs::create_dir_all(repositories_dir)
            .await
            .context("Failed to create /opt/sam/repositories directory")?;
        crate::println(output_lines, "Created /opt/sam/repositories".to_string()).await;
    }

    // Clone if not already present
    if !llama_repo_dir.exists() {
        let mut git_cmd = Command::new("git");
        git_cmd.arg("clone").arg(repo_url).arg(&llama_repo_dir);
        run_command_stream_lines(git_cmd, output_lines, "git").await?;
    }

    // Build with CMake
    let mut cmake_cmd = Command::new("cmake");
    cmake_cmd.arg("-DLLAMA_CURL=OFF").arg("-DGGML_CCACHE=OFF").arg(".").current_dir(&llama_repo_dir);
    run_command_stream_lines(cmake_cmd, output_lines, "cmake").await?;

    let mut build_cmd = Command::new("cmake");
    build_cmd.arg("--build").arg(".").current_dir(&llama_repo_dir);
    run_command_stream_lines(build_cmd, output_lines, "cmake-build").await?;

    // Ensure /opt/sam/bin exists
    if !bin_dir.exists() {
        fs::create_dir_all(bin_dir)
            .await
            .context("Failed to create /opt/sam/bin directory")?;
        crate::println(output_lines, "Created /opt/sam/bin".to_string()).await;
    }

    // Copy binaries (llama, main, etc.)
    let mut entries = fs::read_dir(&llama_repo_dir)
        .await
        .context("Failed to read llama.cpp repository directory")?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(fname) = path.file_name() {
                let fname = fname.to_string_lossy();
                if fname.starts_with("llama") || fname == "main" {
                    let dest = bin_dir.join(fname.as_ref());
                    fs::copy(&path, &dest)
                        .await
                        .with_context(|| format!("Failed to copy {:?} to {:?}", path, dest))?;
                    crate::println(output_lines, format!("Installed {} to {}", fname, dest.display())).await;
                }
            }
        }
    }

    crate::println(output_lines, "llama install: done.".to_string()).await;
    Ok(())
}