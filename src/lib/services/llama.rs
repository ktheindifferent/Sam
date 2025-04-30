use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::{fs, process::Command};
use std::sync::{Arc};
use tokio::sync::Mutex;
// use futures::StreamExt;
use tokio::io::AsyncBufReadExt;
use std::path::PathBuf;

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
        run_command_stream_lines(git_cmd, output_lines.clone(), "git").await?;
    }

    // Build with CMake
    let mut cmake_cmd = Command::new("cmake");
    cmake_cmd.arg("-DLLAMA_CURL=OFF").arg("-DGGML_CCACHE=OFF").arg(".").current_dir(&scripts_dir);
    run_command_stream_lines(cmake_cmd, output_lines.clone(), "cmake").await?;

    let mut build_cmd = Command::new("cmake");
    build_cmd.arg("--build").arg(".").current_dir(&scripts_dir);
    run_command_stream_lines(build_cmd, output_lines.clone(), "cmake-build").await?;

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
