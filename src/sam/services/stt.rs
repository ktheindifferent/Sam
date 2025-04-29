// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::{Request, Response, post_input};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod whisper;
 // Add missing import for tools module

/// Represents the result of an STT prediction.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct STTPrediction {
    pub stt: String,         // Transcribed text
    pub human: String,       // Identified speaker
    pub confidence: f64,     // Confidence score
}

/// Represents the reply from the STT server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct STTReply {
    pub text: String,                // Transcribed text
    pub time: f64,                   // Processing time
    pub response_type: Option<String>, // Type of response (e.g., "stt")
}

// Processes audio using Whisper and SPREC.
pub fn process(file_path: String) -> Result<STTPrediction, crate::sam::services::Error> {
    let whisper = whisper_quick(file_path.clone())?;
    let sprec = crate::sam::services::sprec::predict(&file_path)?;
    Ok(STTPrediction {
        stt: whisper,
        human: sprec.human,
        confidence: sprec.confidence,
    })
}

// Processes audio using Whisper GPU and SPREC.
pub fn gpu_process(file_path: String) -> Result<STTPrediction, crate::sam::services::Error> {
    let whisper = whisper_gpu(file_path.clone())?;
    let sprec = crate::sam::services::sprec::predict(&file_path)?;
    Ok(STTPrediction {
        stt: whisper,
        human: sprec.human,
        confidence: sprec.confidence,
    })
}

// Processes audio using DeepSpeech and SPREC.
pub fn deep_speech_process(file_path: String) -> Result<STTPrediction, crate::sam::services::Error> {
    let reply = upload(file_path.clone())?;
    let sprec = crate::sam::services::sprec::predict(&file_path)?;
    Ok(STTPrediction {
        stt: reply.text,
        human: sprec.human,
        confidence: sprec.confidence,
    })
}

// Runs Whisper on the provided audio file.
pub fn whisper(file_path: String) -> Result<String, crate::sam::services::Error> {
    prepare_audio(&file_path)?;
    crate::sam::tools::cmd(&format!("/opt/sam/bin/whisper -m /opt/sam/models/ggml-large.bin -f {file_path}.16.wav -otxt"))?;
    let data = read_and_cleanup(file_path)?;
    Ok(data)
}

// Runs Whisper in quick mode with a smaller model.
pub fn whisper_quick(file_path: String) -> Result<String, crate::sam::services::Error> {
    prepare_audio(&file_path)?;
    crate::sam::tools::cmd(&format!("/opt/sam/bin/whisper -m /opt/sam/models/ggml-tiny.bin -f {file_path}.16.wav -otxt -t 4"))?;
    let data = read_and_cleanup(file_path)?;
    Ok(data)
}

// Runs Whisper on GPU for faster processing.
pub fn whisper_gpu(file_path: String) -> Result<String, crate::sam::services::Error> {
    prepare_audio(&file_path)?;
    crate::sam::tools::cmd(&format!("/opt/sam/bin/whisper-gpu -m /opt/sam/models/ggml-tiny.bin -f {file_path}.16.wav -otxt -t 8"))?;
    let data = read_and_cleanup(file_path)?;
    Ok(data)
}

// Prepares audio for processing by converting or copying it.
fn prepare_audio(file_path: &String) -> Result<(), crate::sam::services::Error> {
    crate::sam::tools::cmd(&format!("cp {file_path} {file_path}.16.wav"))?;
    Ok(())
}

// Reads the processed file and cleans up temporary files.
fn read_and_cleanup(file_path: String) -> Result<String, crate::sam::services::Error> {
    let data = std::fs::read_to_string(format!("{file_path}.16.wav.txt").as_str())?;
    std::fs::remove_file(format!("{file_path}.16.wav").as_str())?;
    std::fs::remove_file(format!("{file_path}.16.wav.txt").as_str())?;
    Ok(data)
}

// Patches Whisper configuration files for compatibility.
pub fn patch_whisper_wts(file_path: String) -> Result<(), crate::sam::services::Error> {
    let mut data = std::fs::read_to_string(&file_path)?;
    data = data.replace("ffmpeg", "/opt/sam/bin/ffmpeg")
               .replace("/System/Library/Fonts/Supplemental/Courier New Bold.ttf", "/opt/sam/fonts/courier.ttf");
    std::fs::remove_file(&file_path)?;
    std::fs::write(file_path, data)?;
    Ok(())
}

// Handles incoming STT requests and processes audio files.
pub fn handle(_current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/stt" {
        let data = post_input!(request, { audio_data: rouille::input::post::BufferedFile })?;
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(e) => {
                log::error!("Failed to get system time: {}", e);
                return Ok(Response::text("Internal server error").with_status_code(500));
            }
        };
        let tmp_file_path = format!("/opt/sam/tmp/{timestamp}.wav");
        let mut file = File::create(&tmp_file_path)?;
        file.write_all(&data.audio_data.data)?;
        let mut idk = upload(tmp_file_path)?;
        if idk.text.contains("sam") {
            return Ok(Response::redirect_303(format!("/api/io?input={}", idk.text.replace("sam ", ""))));
        }
        idk.response_type = Some("stt".to_string());
        return Ok(Response::json(&idk));
    }
    Ok(Response::empty_404())
}

// Initializes the STT service by starting the Docker container.
pub fn init() {
    if let Err(e) = thread::Builder::new().name("stt".to_string()).spawn(|| {
        crate::sam::tools::uinx_cmd("docker run -p 8002:8000 p0indexter/stt");
    }) {
        log::error!("Failed to initialize STT server: {}", e);
    } else {
        log::info!("STT server started successfully");
    }
}

// Uploads audio to the STT server and retrieves the transcription.
pub fn upload(tmp_file_path: String) -> Result<STTReply, crate::sam::services::Error> {
    let form = reqwest::blocking::multipart::Form::new().text("method", "base").file("speech", &tmp_file_path)?;
    let client = reqwest::blocking::Client::builder().timeout(None).build()?;
    let response = client.post("http://192.168.86.28:8050/api/services/whisper").multipart(form).send()?.json()?;
    Ok(response)
}

// impl From<crate::sam::tools::Error> for crate::sam::services::Error {
//     fn from(err: crate::sam::tools::Error) -> Self {
//         crate::sam::services::Error::from(err.to_string())
//     }
// }