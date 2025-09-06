//! Speech-to-Text services module

pub mod external;
pub mod whisper;
pub mod whisper_enhanced;

pub use whisper_enhanced::{WhisperConfig, WhisperService, WhisperResult};

#[derive(Debug, Clone)]
pub struct STTPrediction {
    pub text: String,
    pub confidence: f32,
    pub language: Option<String>,
    pub duration_ms: u64,
}

// Stub function for API compatibility
pub fn patch_whisper_wts() -> anyhow::Result<()> {
    // TODO: Implement whisper weights patching
    Ok(())
}

/// Placeholder for deep speech processing
/// TODO: Implement actual deep speech functionality
pub fn deep_speech_process(file_path: String) -> Result<STTPrediction, Box<dyn std::error::Error>> {
    log::warn!("deep_speech_process called but not implemented for file: {}", file_path);
    Ok(STTPrediction {
        text: String::new(), // Empty string means no speech detected
        confidence: 0.0,
        language: None,
        duration_ms: 0,
    })
}

pub fn handle(session: Option<String>, request: &rouille::Request) -> rouille::Response {
    // TODO: Implement proper STT HTTP API handling
    rouille::Response::text("STT service not fully implemented")
}