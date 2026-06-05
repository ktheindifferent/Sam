// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::{Request, Response, post_input};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::services::voice::{VoiceService, VoiceConfig};
use crate::services::stt::whisper_enhanced::{WhisperConfig, WhisperService};
use crate::services::tts::{TtsConfig, TtsService, TtsRequest, AudioFormat};

#[derive(Serialize, Deserialize)]
struct TranscribeRequest {
    audio_data: Vec<u8>,
    language: Option<String>,
    translate: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct TranscribeResponse {
    text: String,
    segments: Vec<SegmentInfo>,
    language: String,
    confidence: f32,
    duration_ms: u128,
}

#[derive(Serialize, Deserialize)]
struct SegmentInfo {
    text: String,
    start_ms: i64,
    end_ms: i64,
    confidence: f32,
}

#[derive(Serialize, Deserialize)]
struct SynthesizeRequest {
    text: String,
    voice: Option<String>,
    language: Option<String>,
    speed: Option<f32>,
    pitch: Option<f32>,
    volume: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct SynthesizeResponse {
    audio_url: String,
    format: String,
    duration_ms: u128,
    cached: bool,
}

#[derive(Serialize, Deserialize)]
struct VoiceCommandRequest {
    command: String,
    context: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct VoiceCommandResponse {
    response_text: String,
    audio_url: String,
    action: Option<String>,
    metadata: Option<serde_json::Value>,
}

lazy_static::lazy_static! {
    static ref VOICE_SERVICE: VoiceService = VoiceService::new();
}

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    match request.url().as_str() {
        "/api/voice/transcribe" => handle_transcribe(request),
        "/api/voice/synthesize" => handle_synthesize(request),
        "/api/voice/command" => handle_voice_command(request),
        "/api/voice/config" => handle_config(request),
        "/api/voice/status" => handle_status(request),
        _ => Ok(Response::empty_404()),
    }
}

fn handle_transcribe(request: &Request) -> Result<Response, crate::http::Error> {
    let data = post_input!(request, { 
        audio_data: rouille::input::post::BufferedFile,
        language: Option<String>,
        translate: Option<String>
    })?;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| crate::http::Error::from(format!("Time error: {}", e)))?
        .as_secs();
    
    let tmp_file_path = PathBuf::from(format!("/tmp/sam_audio_{}.wav", timestamp));
    let mut file = File::create(&tmp_file_path)?;
    file.write_all(&data.audio_data.data)?;
    drop(file);
    
    let mut config = WhisperConfig::default();
    if let Some(lang) = data.language {
        config.language = Some(lang);
    }
    if let Some(translate) = data.translate {
        config.translate = translate == "true";
    }
    
    let service = WhisperService::with_config(config)
        .map_err(|e| crate::http::Error::from(format!("Whisper init error: {}", e)))?;
    
    let result = service.transcribe_file(&tmp_file_path)
        .map_err(|e| crate::http::Error::from(format!("Transcription error: {}", e)))?;
    
    std::fs::remove_file(&tmp_file_path).ok();
    
    let response = TranscribeResponse {
        text: result.text,
        segments: result.segments.into_iter().map(|s| SegmentInfo {
            text: s.text,
            start_ms: s.start_time,
            end_ms: s.end_time,
            confidence: s.probability,
        }).collect(),
        language: result.language,
        confidence: result.segments.iter().map(|s| s.probability).sum::<f32>() 
            / result.segments.len().max(1) as f32,
        duration_ms: result.duration_ms,
    };
    
    Ok(Response::json(&response))
}

fn handle_synthesize(request: &Request) -> Result<Response, crate::http::Error> {
    let input: SynthesizeRequest = rouille::input::json_input(request)?;
    
    let tts_request = TtsRequest {
        text: input.text,
        voice: input.voice,
        language: input.language,
        speed: input.speed,
        pitch: input.pitch,
        volume: input.volume,
        format: AudioFormat::Wav,
    };
    
    let service = TtsService::new(TtsConfig::default())
        .map_err(|e| crate::http::Error::from(format!("TTS init error: {}", e)))?;
    
    let result = service.synthesize(tts_request)
        .map_err(|e| crate::http::Error::from(format!("Synthesis error: {}", e)))?;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| crate::http::Error::from(format!("Time error: {}", e)))?
        .as_secs();
    
    let audio_filename = format!("tts_{}.wav", timestamp);
    let audio_path = PathBuf::from(format!("/opt/sam/www/audio/{}", audio_filename));
    
    let audio_dir = audio_path
        .parent()
        .ok_or_else(|| crate::http::Error::from("Unable to resolve audio output directory"))?;
    std::fs::create_dir_all(audio_dir)?;
    
    let mut file = File::create(&audio_path)?;
    file.write_all(&result.audio_data)?;
    drop(file);
    
    let response = SynthesizeResponse {
        audio_url: format!("/audio/{}", audio_filename),
        format: "wav".to_string(),
        duration_ms: result.duration_ms,
        cached: result.cached,
    };
    
    Ok(Response::json(&response))
}

fn handle_voice_command(request: &Request) -> Result<Response, crate::http::Error> {
    let input: VoiceCommandRequest = rouille::input::json_input(request)?;
    
    let response = VoiceCommandResponse {
        response_text: format!("Processing command: {}", input.command),
        audio_url: "/audio/response.wav".to_string(),
        action: Some("process".to_string()),
        metadata: input.context,
    };
    
    Ok(Response::json(&response))
}

fn handle_config(request: &Request) -> Result<Response, crate::http::Error> {
    if request.method() == "GET" {
        let config = VoiceConfig::default();
        Ok(Response::json(&config))
    } else if request.method() == "POST" {
        let config: VoiceConfig = rouille::input::json_input(request)?;
        
        VOICE_SERVICE.initialize(config.clone())
            .map_err(|e| crate::http::Error::from(format!("Config error: {}", e)))?;
        
        Ok(Response::json(&config))
    } else {
        Ok(Response::empty_405())
    }
}

fn handle_status(request: &Request) -> Result<Response, crate::http::Error> {
    #[derive(Serialize)]
    struct StatusResponse {
        whisper_available: bool,
        tts_available: bool,
        voice_assistant_initialized: bool,
        supported_languages: Vec<String>,
        supported_voices: Vec<String>,
    }
    
    let status = StatusResponse {
        whisper_available: true,
        tts_available: true,
        voice_assistant_initialized: false,
        supported_languages: vec![
            "en".to_string(),
            "es".to_string(),
            "fr".to_string(),
            "de".to_string(),
            "it".to_string(),
            "pt".to_string(),
            "ru".to_string(),
            "zh".to_string(),
            "ja".to_string(),
            "ko".to_string(),
        ],
        supported_voices: vec![
            "default".to_string(),
            "male".to_string(),
            "female".to_string(),
        ],
    };
    
    Ok(Response::json(&status))
}
