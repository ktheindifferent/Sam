// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::sam::services::stt::whisper_enhanced::{WhisperConfig, WhisperService, WhisperResult};
use crate::sam::services::tts::enhanced::{TtsConfig, TtsService, TtsRequest, AudioFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub stt_config: WhisperConfig,
    pub tts_config: TtsConfig,
    pub wake_word: Option<String>,
    pub audio_input_device: Option<String>,
    pub audio_output_device: Option<String>,
    pub vad_enabled: bool,
    pub vad_threshold: f32,
    pub noise_reduction: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_config: WhisperConfig::default(),
            tts_config: TtsConfig::default(),
            wake_word: Some("hey sam".to_string()),
            audio_input_device: None,
            audio_output_device: None,
            vad_enabled: true,
            vad_threshold: 0.5,
            noise_reduction: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub text: String,
    pub confidence: f32,
    pub speaker: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceResponse {
    pub text: String,
    pub audio_data: Vec<u8>,
    pub format: AudioFormat,
    pub duration_ms: u128,
}

pub struct VoiceAssistant {
    stt_service: Arc<WhisperService>,
    tts_service: Arc<TtsService>,
    config: VoiceConfig,
    is_listening: Arc<Mutex<bool>>,
    conversation_history: Arc<Mutex<Vec<ConversationEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationEntry {
    pub role: ConversationRole,
    pub text: String,
    pub timestamp: u64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ConversationRole {
    User,
    Assistant,
    System,
}

impl VoiceAssistant {
    pub fn new(config: VoiceConfig) -> Result<Self, crate::sam::services::Error> {
        let stt_service = WhisperService::with_config(config.stt_config.clone())?;
        let tts_service = TtsService::new(config.tts_config.clone())?;
        
        Ok(Self {
            stt_service: Arc::new(stt_service),
            tts_service: Arc::new(tts_service),
            config,
            is_listening: Arc::new(Mutex::new(false)),
            conversation_history: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn start_listening(&self) -> Result<(), crate::sam::services::Error> {
        let mut listening = self.is_listening.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock listening state: {}", e))
        })?;
        
        if *listening {
            return Ok(());
        }
        
        *listening = true;
        
        self.spawn_audio_capture_thread()?;
        
        Ok(())
    }

    pub fn stop_listening(&self) -> Result<(), crate::sam::services::Error> {
        let mut listening = self.is_listening.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock listening state: {}", e))
        })?;
        
        *listening = false;
        Ok(())
    }

    pub fn process_audio(&self, audio_path: &Path) -> Result<VoiceCommand, crate::sam::services::Error> {
        let temp_wav = PathBuf::from("/tmp/sam_audio_temp.wav");
        crate::sam::services::stt::whisper_enhanced::WhisperEngine::convert_audio_to_16khz_mono(audio_path, &temp_wav)?;
        
        let result = self.stt_service.transcribe_file(&temp_wav)?;
        
        std::fs::remove_file(&temp_wav).ok();
        
        let command = VoiceCommand {
            text: result.text.clone(),
            confidence: self.calculate_confidence(&result),
            speaker: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| crate::sam::services::Error::from(format!("Time error: {}", e)))?
                .as_secs(),
        };
        
        self.add_to_history(ConversationRole::User, &command.text)?;
        
        Ok(command)
    }

    pub fn generate_response(&self, text: &str) -> Result<VoiceResponse, crate::sam::services::Error> {
        let tts_request = TtsRequest {
            text: text.to_string(),
            voice: None,
            language: Some(self.config.tts_config.language.clone()),
            speed: Some(self.config.tts_config.speed),
            pitch: Some(self.config.tts_config.pitch),
            volume: Some(self.config.tts_config.volume),
            format: AudioFormat::Wav,
        };
        
        let tts_result = self.tts_service.synthesize(tts_request)?;
        
        self.add_to_history(ConversationRole::Assistant, text)?;
        
        Ok(VoiceResponse {
            text: text.to_string(),
            audio_data: tts_result.audio_data,
            format: tts_result.format,
            duration_ms: tts_result.duration_ms,
        })
    }

    pub fn speak(&self, text: &str) -> Result<(), crate::sam::services::Error> {
        let response = self.generate_response(text)?;
        self.play_audio(&response.audio_data)?;
        Ok(())
    }

    fn spawn_audio_capture_thread(&self) -> Result<(), crate::sam::services::Error> {
        use std::thread;
        
        let is_listening = Arc::clone(&self.is_listening);
        let wake_word = self.config.wake_word.clone();
        let stt_service = Arc::clone(&self.stt_service);
        
        thread::Builder::new()
            .name("voice_capture".to_string())
            .spawn(move || {
                log::info!("Voice capture thread started");
                
                loop {
                    let should_continue = {
                        match is_listening.lock() {
                            Ok(guard) => *guard,
                            Err(_) => false,
                        }
                    };
                    if !should_continue {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                
                log::info!("Voice capture thread stopped");
            })
            .map_err(|e| crate::sam::services::Error::from(format!("Failed to spawn thread: {}", e)))?;
        
        Ok(())
    }

    fn calculate_confidence(&self, result: &WhisperResult) -> f32 {
        if result.segments.is_empty() {
            return 0.0;
        }
        
        let total_prob: f32 = result.segments.iter().map(|s| s.probability).sum();
        total_prob / result.segments.len() as f32
    }

    fn add_to_history(&self, role: ConversationRole, text: &str) -> Result<(), crate::sam::services::Error> {
        let mut history = self.conversation_history.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock history: {}", e))
        })?;
        
        history.push(ConversationEntry {
            role,
            text: text.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| crate::sam::services::Error::from(format!("Time error: {}", e)))?
                .as_secs(),
            metadata: None,
        });
        
        if history.len() > 100 {
            history.drain(0..50);
        }
        
        Ok(())
    }

    fn play_audio(&self, audio_data: &[u8]) -> Result<(), crate::sam::services::Error> {
        use std::process::Command;
        use std::io::Write;
        use rand::{distributions::Alphanumeric, Rng};
        
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let temp_file = format!("/tmp/sam_audio_{}.wav", rand_name);
        
        let mut file = std::fs::File::create(&temp_file)?;
        file.write_all(audio_data)?;
        drop(file);
        
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("aplay")
                .arg(&temp_file)
                .status();
            
            if Command::new("paplay").arg(&temp_file).status().is_err() {
                let _ = Command::new("play").arg(&temp_file).status();
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("afplay")
                .arg(&temp_file)
                .status();
        }
        
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("powershell")
                .args(&[
                    "-Command",
                    &format!("(New-Object Media.SoundPlayer '{}').PlaySync()", temp_file),
                ])
                .status();
        }
        
        std::fs::remove_file(&temp_file).ok();
        
        Ok(())
    }

    pub fn get_conversation_history(&self) -> Result<Vec<ConversationEntry>, crate::sam::services::Error> {
        let history = self.conversation_history.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock history: {}", e))
        })?;
        Ok(history.clone())
    }

    pub fn clear_conversation_history(&self) -> Result<(), crate::sam::services::Error> {
        let mut history = self.conversation_history.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock history: {}", e))
        })?;
        history.clear();
        Ok(())
    }
}

pub struct VoiceService {
    assistant: Arc<Mutex<Option<VoiceAssistant>>>,
}

impl Default for VoiceService {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceService {
    pub fn new() -> Self {
        Self {
            assistant: Arc::new(Mutex::new(None)),
        }
    }

    pub fn initialize(&self, config: VoiceConfig) -> Result<(), crate::sam::services::Error> {
        let assistant = VoiceAssistant::new(config)?;
        let mut guard = self.assistant.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock assistant: {}", e))
        })?;
        *guard = Some(assistant);
        Ok(())
    }

    pub fn process_command(&self, audio_path: &Path) -> Result<VoiceCommand, crate::sam::services::Error> {
        let guard = self.assistant.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock assistant: {}", e))
        })?;
        
        match &*guard {
            Some(assistant) => assistant.process_audio(audio_path),
            None => Err(crate::sam::services::Error::from("Voice service not initialized")),
        }
    }

    pub fn speak(&self, text: &str) -> Result<(), crate::sam::services::Error> {
        let guard = self.assistant.lock().map_err(|e| {
            crate::sam::services::Error::from(format!("Failed to lock assistant: {}", e))
        })?;
        
        match &*guard {
            Some(assistant) => assistant.speak(text),
            None => Err(crate::sam::services::Error::from("Voice service not initialized")),
        }
    }
}

/// Initialize the voice service
pub async fn initialize() -> anyhow::Result<()> {
    log::info!("Voice service initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_config_default() {
        let config = VoiceConfig::default();
        assert_eq!(config.wake_word, Some("hey sam".to_string()));
        assert!(config.vad_enabled);
        assert!(config.noise_reduction);
    }

    #[test]
    fn test_voice_service_creation() {
        let service = VoiceService::new();
        let guard = service.assistant.lock().unwrap();
        assert!(guard.is_none());
    }
}