use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};
use tokio::fs;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::io;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
pub async fn install() -> io::Result<String> {
    let mut log = String::new();
    log.push_str(&ensure_whisper_binary_with_output().await?);
    log.push_str("Whisper binary installed.\n");
    let models = vec![
        ("ggml-base.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"),
        ("ggml-tiny.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"),
        ("ggml-base.en.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"),
        ("ggml-medium.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"),
        ("ggml-large.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large.bin"),
    ];

    for (file, url) in models {
        if !Path::new(&format!("/opt/sam/models/{}", file)).exists() {
            let _ = crate::cmd_async(&format!("wget -O /opt/sam/models/{} {}", file, url)).await?;
        }
    }
    let _ = crate::cmd_async("chmod +x /opt/sam/bin/ffmpeg").await?;
    let _ = crate::cmd_async("chmod +x /opt/sam/bin/whisper").await?;
    let _ = crate::cmd_async("chmod +x /opt/sam/bin/whisper-cli").await?;
    Ok(log)
}



pub async fn ensure_whisper_binary_with_output() -> io::Result<String> {

    let whisper_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/whisper.cpp");
    let whisper_bin = PathBuf::from("/opt/sam/bin/whisper-cli");
    let build_dir = whisper_src.join("build");
    fs::create_dir_all(&build_dir).await?;

    if whisper_bin.exists() {
        return Ok("whisper-cli binary already exists.".to_string());
    }

    let mut output_log = String::new();

    // Run cmake -B build
    let cmake_config = Command::new("cmake")
        .current_dir(whisper_src.clone())
        .arg("-B")
        .arg("build")
        .output()?;
    output_log.push_str("--- cmake configure ---\n");
    output_log.push_str(&String::from_utf8_lossy(&cmake_config.stdout));
    output_log.push_str(&String::from_utf8_lossy(&cmake_config.stderr));
    if !cmake_config.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to configure whisper.cpp with cmake\n{}", output_log),
        ));
    }

    // Run cmake --build build --config Release
    let cmake_build = Command::new("cmake")
        .current_dir(whisper_src.clone())
        .arg("--build")
        .arg("build")
        .arg("--config")
        .arg("Release")
        .output()?;
    output_log.push_str("--- cmake build ---\n");
    output_log.push_str(&String::from_utf8_lossy(&cmake_build.stdout));
    output_log.push_str(&String::from_utf8_lossy(&cmake_build.stderr));
    if !cmake_build.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to build whisper.cpp with cmake\n{}", output_log),
        ));
    }

    // Find the built binary
    let built_bin = build_dir.join("bin/whisper-cli");
    let built_bin_alt = build_dir.join("whisper-cli");
    let target_bin = PathBuf::from("/opt/sam/bin").join("whisper-cli");

    let src_bin = if built_bin.exists() {
        built_bin
    } else if built_bin_alt.exists() {
        built_bin_alt
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("whisper-cli binary not found after build\n{}", output_log),
        ));
    };

    fs::create_dir_all("/opt/sam/bin").await?;
    fs::copy(&src_bin, &target_bin).await?;
    let _ = Command::new("chmod").arg("+x").arg(&target_bin).output();
    output_log.push_str(&format!("Installed binary: {}\n", target_bin.display()));

    Ok(output_log)
}

