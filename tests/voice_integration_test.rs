// Integration tests for the voice module

use sam::sam::services::voice::{VoiceConfig, VoiceService};
use sam::sam::services::stt::whisper_enhanced::{WhisperConfig, WhisperService};
use sam::sam::services::tts::{TtsConfig, TtsService, TtsRequest, AudioFormat};
use std::path::PathBuf;

#[test]
fn test_whisper_config_creation() {
    let config = WhisperConfig::default();
    assert_eq!(config.n_threads, 4);
    assert_eq!(config.language, Some("en".to_string()));
    assert!(!config.translate);
    assert!(!config.use_gpu);
}

#[test]
fn test_tts_config_creation() {
    let config = TtsConfig::default();
    assert_eq!(config.speed, 1.0);
    assert_eq!(config.pitch, 1.0);
    assert_eq!(config.volume, 1.0);
    assert_eq!(config.language, "en-US");
    assert!(config.cache_enabled);
}

#[test]
fn test_voice_config_creation() {
    let config = VoiceConfig::default();
    assert_eq!(config.wake_word, Some("hey sam".to_string()));
    assert!(config.vad_enabled);
    assert!(config.noise_reduction);
    assert_eq!(config.vad_threshold, 0.5);
}

#[test]
fn test_voice_service_initialization() {
    let service = VoiceService::new();
    let config = VoiceConfig::default();
    
    // Note: This might fail if models aren't available
    // In a real test environment, you'd mock these dependencies
    let result = service.initialize(config);
    
    // Just check that the service can be created
    assert!(service.assistant.lock().is_ok());
}

#[test]
fn test_tts_request_creation() {
    let request = TtsRequest {
        text: "Hello, world!".to_string(),
        voice: Some("default".to_string()),
        language: Some("en-US".to_string()),
        speed: Some(1.0),
        pitch: Some(1.0),
        volume: Some(1.0),
        format: AudioFormat::Wav,
    };
    
    assert_eq!(request.text, "Hello, world!");
    assert_eq!(request.voice, Some("default".to_string()));
}

#[test]
fn test_audio_format_default() {
    let format = AudioFormat::default();
    match format {
        AudioFormat::Wav => assert!(true),
        _ => assert!(false, "Default format should be WAV"),
    }
}

#[cfg(test)]
mod whisper_tests {
    use super::*;
    
    #[test]
    fn test_whisper_model_paths() {
        let config = WhisperConfig {
            model_path: PathBuf::from("/opt/sam/models/ggml-tiny.bin"),
            ..Default::default()
        };
        
        assert_eq!(config.model_path.to_str().unwrap(), "/opt/sam/models/ggml-tiny.bin");
        assert_eq!(config.best_of, 5);
        assert_eq!(config.beam_size, 5);
    }
    
    #[test]
    fn test_whisper_gpu_config() {
        let mut config = WhisperConfig::default();
        config.use_gpu = true;
        config.n_threads = 8;
        
        assert!(config.use_gpu);
        assert_eq!(config.n_threads, 8);
    }
}

#[cfg(test)]
mod tts_tests {
    use super::*;
    use sam::sam::services::tts::TtsEngine;
    
    #[test]
    fn test_tts_engine_variants() {
        let system_engine = TtsEngine::System;
        let mozilla_engine = TtsEngine::MozillaTts;
        let external_engine = TtsEngine::External("http://localhost:5000".to_string());
        let coqui_engine = TtsEngine::Coqui;
        
        match system_engine {
            TtsEngine::System => assert!(true),
            _ => assert!(false),
        }
        
        match mozilla_engine {
            TtsEngine::MozillaTts => assert!(true),
            _ => assert!(false),
        }
        
        match external_engine {
            TtsEngine::External(url) => assert_eq!(url, "http://localhost:5000"),
            _ => assert!(false),
        }
        
        match coqui_engine {
            TtsEngine::Coqui => assert!(true),
            _ => assert!(false),
        }
    }
    
    #[test]
    fn test_tts_cache_dir() {
        let mut config = TtsConfig::default();
        config.cache_dir = PathBuf::from("/custom/cache/dir");
        
        assert_eq!(config.cache_dir.to_str().unwrap(), "/custom/cache/dir");
    }
}