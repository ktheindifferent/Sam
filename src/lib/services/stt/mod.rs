//! Speech-to-Text services module
use rouille::{post_input, Response};
use serde::Serialize;
use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::Mutex;
pub mod external;
pub mod whisper;
pub mod whisper_enhanced;

pub use whisper_enhanced::{WhisperConfig, WhisperResult, WhisperService};

#[derive(Debug, Clone)]
pub struct STTPrediction {
    pub text: String,
    pub confidence: f32,
    pub language: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct SttStatusResponse {
    available: bool,
    default_model_path: String,
    default_model_present: bool,
    supported_endpoints: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SttErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct SttTranscribeResponse {
    text: String,
    confidence: f32,
    language: Option<String>,
    duration_ms: u128,
    segments: Vec<whisper_enhanced::WhisperSegment>,
}

// Stub function for API compatibility
pub fn patch_whisper_wts() -> anyhow::Result<()> {
    // TODO: Implement whisper weights patching
    Ok(())
}

/// Placeholder for deep speech processing
/// TODO: Implement actual deep speech functionality
pub fn deep_speech_process(file_path: String) -> Result<STTPrediction, Box<dyn std::error::Error>> {
    log::warn!(
        "deep_speech_process called but not implemented for file: {}",
        file_path
    );
    Ok(STTPrediction {
        text: String::new(), // Empty string means no speech detected
        confidence: 0.0,
        language: None,
        duration_ms: 0,
    })
}

pub fn handle(_session: Option<String>, request: &rouille::Request) -> rouille::Response {
    match (request.method(), request.url().as_str()) {
        ("GET", "/api/stt/status") | ("GET", "/api/services/stt/status") => {
            let config = WhisperConfig::default();
            Response::json(&SttStatusResponse {
                available: config.model_path.exists(),
                default_model_path: config.model_path.display().to_string(),
                default_model_present: config.model_path.exists(),
                supported_endpoints: vec![
                    "GET /api/stt/status".to_string(),
                    "POST /api/stt/transcribe".to_string(),
                ],
            })
        }
        ("POST", "/api/stt/transcribe") | ("POST", "/api/services/stt/transcribe") => {
            handle_transcribe(request)
        }
        _ => Response::empty_404(),
    }
}

fn handle_transcribe(request: &rouille::Request) -> rouille::Response {
    let data = match post_input!(request, {
        audio_data: rouille::input::post::BufferedFile,
        language: Option<String>,
        translate: Option<String>,
    }) {
        Ok(data) => data,
        Err(e) => {
            return Response::json(&SttErrorResponse {
                error: format!("Invalid transcription request: {}", e),
            })
            .with_status_code(400);
        }
    };

    let mut config = WhisperConfig::default();
    if let Some(language) = data.language {
        config.language = Some(language);
    }
    if let Some(translate) = data.translate {
        config.translate = translate.eq_ignore_ascii_case("true");
    }

    let request_id = nanoid::nanoid!();
    let input_path = std::env::temp_dir().join(format!("sam_stt_{}_input", request_id));
    let wav_path = std::env::temp_dir().join(format!("sam_stt_{}_16k.wav", request_id));

    if let Err(e) = std::fs::write(&input_path, &data.audio_data.data) {
        return Response::json(&SttErrorResponse {
            error: format!("Failed to write uploaded audio: {}", e),
        })
        .with_status_code(500);
    }

    let result = (|| -> Result<whisper_enhanced::WhisperResult, crate::services::Error> {
        whisper_enhanced::WhisperEngine::convert_audio_to_16khz_mono(&input_path, &wav_path)?;
        let service = WhisperService::with_config(config)?;
        service.transcribe_file(&wav_path)
    })();

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&wav_path);

    match result {
        Ok(result) => {
            let confidence = if result.segments.is_empty() {
                0.0
            } else {
                result.segments.iter().map(|s| s.probability).sum::<f32>()
                    / result.segments.len() as f32
            };

            Response::json(&SttTranscribeResponse {
                text: result.text,
                confidence,
                language: Some(result.language),
                duration_ms: result.duration_ms,
                segments: result.segments,
            })
        }
        Err(e) => Response::json(&SttErrorResponse {
            error: format!("Transcription failed: {}", e),
        })
        .with_status_code(500),
    }
}

// Helper: Run a command and stream output lines
async fn run_command_stream_lines(
    mut cmd: Command,
    output_lines: Option<&Arc<Mutex<Vec<String>>>>,
    prefix: &str,
) -> io::Result<()> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
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
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{} failed: {:?}", prefix, lines),
        ));
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        for (file, url) in models {
            let model_path = format!("/opt/sam/models/{file}");
            if !Path::new(&model_path).exists() {
                let mut wget_cmd = Command::new("wget");
                wget_cmd.arg("-O").arg(&model_path).arg(url);
                run_command_stream_lines(wget_cmd, output_lines, "wget").await?;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        for (file, url) in models {
            let model_path = format!("C:\\opt\\sam\\models\\{file}");
            if !Path::new(&model_path).exists() {
                let mut curl_cmd = Command::new("curl");
                curl_cmd.arg("-L").arg("-o").arg(&model_path).arg(url);
                run_command_stream_lines(curl_cmd, output_lines, "curl").await?;
            }
        }
    }

    ensure_whisper_binary_with_output(output_lines).await?;

    #[cfg(not(target_os = "windows"))]
    {
        for bin in ["whisper-server", "whisper-bench", "whisper-cli"] {
            let mut chmod_cmd = Command::new("chmod");
            chmod_cmd.arg("+x").arg(format!("/opt/sam/bin/{}", bin));
            let _ = run_command_stream_lines(chmod_cmd, output_lines, "chmod").await;
        }
    }
    crate::println(output_lines, "Whisper install: done.".to_string()).await;
    Ok(())
}
pub async fn ensure_whisper_binary_with_output(
    output_lines: Option<&Arc<Mutex<Vec<String>>>>,
) -> io::Result<()> {
    let whisper_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/whisper.cpp");
    let whisper_bin = PathBuf::from("/opt/sam/bin/whisper-cli");
    let whisper_bench = PathBuf::from("/opt/sam/bin/whisper-bench");
    let whisper_server = PathBuf::from("/opt/sam/bin/whisper-server");
    let build_dir = whisper_src.join("build");
    fs::create_dir_all(&build_dir).await?;

    if whisper_bin.exists() && whisper_bench.exists() && whisper_server.exists() {
        crate::println(
            output_lines,
            "whisper-cli binary already exists.".to_string(),
        )
        .await;
        return Ok(());
    }

    // Run cmake -B build
    let mut cmake_config = Command::new("cmake");
    cmake_config
        .current_dir(whisper_src.clone())
        .arg("-B")
        .arg("build");
    run_command_stream_lines(cmake_config, output_lines, "cmake-config").await?;

    // Run cmake --build build --config Release
    let mut cmake_build = Command::new("cmake");
    cmake_build
        .current_dir(whisper_src.clone())
        .arg("--build")
        .arg("build")
        .arg("--config")
        .arg("Release");
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
            crate::println(
                output_lines,
                format!("{} binary not found after build", bin),
            )
            .await;
            continue;
        };

        fs::copy(&src_bin, &target_bin).await?;
        let mut chmod_cmd = Command::new("chmod");
        chmod_cmd.arg("+x").arg(&target_bin);
        let _ = run_command_stream_lines(chmod_cmd, output_lines, "chmod").await;
        crate::println(
            output_lines,
            format!("Installed binary: {}", target_bin.display()),
        )
        .await;
    }

    Ok(())
}
