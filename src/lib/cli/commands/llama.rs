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
        _ if cmd.starts_with("llama v2 ") => {
            let prompt = cmd.trim_start_matches("llama v2 ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2 <prompt>".to_string());
            } else {
                crate::cli::spinner::run_with_spinner(
                    output_lines,
                    "Querying llama v2...",
                    |lines, result| lines.push(format!("llama v2: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::services::llama::LlamaService::query_v2(&prompt)
                                .unwrap_or_else(|e| format!("llama v2 error: {}", e))
                        }
                    },
                )
                .await;
            }
        }
        _ if cmd.starts_with("llama v2-tiny ") => {
            let prompt = cmd.trim_start_matches("llama v2-tiny ").trim().to_string();
            if prompt.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama v2-tiny <prompt>".to_string());
            } else {
                crate::cli::spinner::run_with_spinner(
                    output_lines,
                    "Querying llama v2-tiny...",
                    |lines, result| lines.push(format!("llama v2-tiny: {}", result)),
                    move || {
                        let prompt = prompt.clone();
                        async move {
                            crate::services::llama::LlamaService::query_v2_tiny(&prompt)
                                .unwrap_or_else(|e| format!("llama v2-tiny error: {}", e))
                        }
                    },
                )
                .await;
            }
        }
        _ if cmd.starts_with("llama ") => {
            let rest = cmd["llama ".len()..].to_string();
            let mut split = rest.splitn(2, ' ');
            let model_path_str = split.next().unwrap_or("").to_string();
            let prompt_str = split.next().unwrap_or("").to_string();

            if model_path_str.is_empty() || prompt_str.is_empty() {
                let mut out = output_lines.lock().await;
                out.push("Usage: llama <model_path> <prompt>".to_string());
            } else {
                crate::cli::spinner::run_with_spinner(
                    output_lines,
                    &format!("Querying llama model {}...", model_path_str),
                    |lines, result| lines.push(format!("llama: {}", result)),
                    move || {
                        let model_path = std::path::PathBuf::from(model_path_str.clone());
                        let prompt = prompt_str.clone();
                        async move {
                            crate::services::llama::LlamaService::query(&model_path, &prompt)
                                .unwrap_or_else(|e| format!("llama error: {}", e))
                        }
                    },
                )
                .await;
            }
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
            let msg = format!("{}: {}", prefix, line);
            crate::println(output_lines, msg.clone()).await;
            if output_lines.is_none() {
                println!("{}", msg);
            }
            lines.push(msg);
        }
    }
    if let Some(stderr) = stderr {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await? {
            let msg = format!("{}: {}", prefix, line);
            crate::println(output_lines, msg.clone()).await;
            if output_lines.is_none() {
                println!("{}", msg);
            }
            lines.push(msg);
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("{} failed: {:?}", prefix, lines);
    }
    Ok(())
}

pub async fn install(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<()> {
    let scripts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/llama.cpp");
    let llama_cli = PathBuf::from("/opt/sam/bin/llama-cli");

    if llama_cli.exists() {
        crate::println(output_lines, "llama-cli binary already exists.".to_string()).await;
        return Ok(());
    }

    let repo_url = "https://github.com/ggml-org/llama.cpp.git";
    let bin_dir = Path::new("/opt/sam/bin");

    // Clone if not already present
    if !scripts_dir.exists() {
        let mut git_cmd = Command::new("git");
        git_cmd.arg("clone").arg(repo_url).arg(&scripts_dir);
        run_command_stream_lines(git_cmd, output_lines, "git").await?;
    }

    // Build with CMake
    let mut cmake_cmd = Command::new("cmake");
    cmake_cmd.arg("-DLLAMA_CURL=OFF").arg("-DGGML_CCACHE=OFF").arg(".").current_dir(&scripts_dir);
    run_command_stream_lines(cmake_cmd, output_lines, "cmake").await?;

    let mut build_cmd = Command::new("cmake");
    build_cmd.arg("--build").arg(".").current_dir(&scripts_dir);
    run_command_stream_lines(build_cmd, output_lines, "cmake-build").await?;

    // Ensure /opt/sam/bin exists
    if !bin_dir.exists() {
        fs::create_dir_all(bin_dir)
            .await
            .context("Failed to create /opt/sam/bin directory")?;
        crate::println(output_lines, "Created /opt/sam/bin".to_string()).await;
    }

    // Copy binaries (llama, main, etc.)
    let mut entries = fs::read_dir(&scripts_dir)
        .await
        .context("Failed to read scripts/llama.cpp directory")?;
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