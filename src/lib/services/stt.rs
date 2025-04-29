use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;


// Helper: Run a command and stream output lines
async fn run_command_stream_lines(mut cmd: Command, output_lines: Option<&Arc<Mutex<Vec<String>>>>, prefix: &str) -> io::Result<()> {
    let mut child = cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;
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
        return Err(io::Error::new(io::ErrorKind::Other, format!("{} failed: {:?}", prefix, lines)));
    }
    Ok(())
}

pub async fn install(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> io::Result<()> {
    crate::println(output_lines, "Starting Whisper install...".to_string()).await;
    ensure_whisper_binary_with_output(output_lines).await?;
    crate::println(output_lines, "Whisper binary installed.".to_string()).await;
    let models = vec![
        (
            "ggml-base.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bi?download=true",
        ),
        (
            "ggml-tiny.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin?download=true",
        ),
        (
            "ggml-base.en.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin?download=true",
        ),
        (
            "ggml-medium.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin?download=true",
        ),
        (
            "ggml-large.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large.bin?download=true",
        ),
    ];

    for (file, url) in models {
        let model_path = format!("/opt/sam/models/{file}");
        if !Path::new(&model_path).exists() {
            let mut wget_cmd = Command::new("wget");
            wget_cmd.arg("-O").arg(&model_path).arg(url);
            run_command_stream_lines(wget_cmd, output_lines, "wget").await?;
        }
    }

    let _build = ensure_whisper_binary_with_output(output_lines).await?;

    for bin in ["whisper-server", "whisper-bench", "whisper-cli"] {
        let mut chmod_cmd = Command::new("chmod");
        chmod_cmd.arg("+x").arg(format!("/opt/sam/bin/{}", bin));
        let _ = run_command_stream_lines(chmod_cmd, output_lines, "chmod").await;
    }
    crate::println(output_lines, "Whisper install: done.".to_string()).await;
    Ok(())
}
pub async fn ensure_whisper_binary_with_output(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> io::Result<()> {
    let whisper_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/whisper.cpp");
    let whisper_bin = PathBuf::from("/opt/sam/bin/whisper-cli");
    let whisper_bench = PathBuf::from("/opt/sam/bin/whisper-bench");
    let whisper_server = PathBuf::from("/opt/sam/bin/whisper-server");
    let build_dir = whisper_src.join("build");
    fs::create_dir_all(&build_dir).await?;

    if whisper_bin.exists() && whisper_bench.exists() && whisper_server.exists() {
        crate::println(output_lines, "whisper-cli binary already exists.".to_string()).await;
        return Ok(());
    }

    // Run cmake -B build
    let mut cmake_config = Command::new("cmake");
    cmake_config.current_dir(whisper_src.clone()).arg("-B").arg("build");
    run_command_stream_lines(cmake_config, output_lines, "cmake-config").await?;

    // Run cmake --build build --config Release
    let mut cmake_build = Command::new("cmake");
    cmake_build.current_dir(whisper_src.clone()).arg("--build").arg("build").arg("--config").arg("Release");
    run_command_stream_lines(cmake_build, output_lines, "cmake-build").await?;

    // Copy all the bins: whisper-server, whisper-bench, whisper-cli
    let bin_names = ["whisper-server", "whisper-bench", "whisper-cli"];
    fs::create_dir_all("/opt/sam/bin").await?;
    for bin in &bin_names {
        let built_bin = build_dir.join(format!("bin/{}", bin));
        let built_bin_alt = build_dir.join(bin);
        let target_bin = PathBuf::from("/opt/sam/bin").join(bin);

        let src_bin = if built_bin.exists() {
            built_bin
        } else if built_bin_alt.exists() {
            built_bin_alt
        } else {
            crate::println(output_lines, format!("{} binary not found after build", bin)).await;
            continue;
        };

        fs::copy(&src_bin, &target_bin).await?;
        let mut chmod_cmd = Command::new("chmod");
        chmod_cmd.arg("+x").arg(&target_bin);
        let _ = run_command_stream_lines(chmod_cmd, output_lines, "chmod").await;
        crate::println(output_lines, format!("Installed binary: {}", target_bin.display())).await;
    }

    Ok(())
}
