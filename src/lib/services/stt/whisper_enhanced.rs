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
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub model_path: PathBuf,
    pub language: Option<String>,
    pub translate: bool,
    pub n_threads: i32,
    pub use_gpu: bool,
    pub temperature: f32,
    pub beam_size: i32,
    pub best_of: i32,
    pub max_context: i32,
    pub initial_prompt: Option<String>,
    pub suppress_blank: bool,
    pub suppress_non_speech_tokens: bool,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("/opt/sam/models/ggml-base.bin"),
            language: Some("en".to_string()),
            translate: false,
            n_threads: 4,
            use_gpu: false,
            temperature: 0.0,
            beam_size: 5,
            best_of: 5,
            max_context: -1,
            initial_prompt: None,
            suppress_blank: true,
            suppress_non_speech_tokens: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperSegment {
    pub text: String,
    pub start_time: i64,
    pub end_time: i64,
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperResult {
    pub text: String,
    pub segments: Vec<WhisperSegment>,
    pub language: String,
    pub duration_ms: u128,
}

pub struct WhisperEngine {
    context: Arc<Mutex<WhisperContext>>,
    config: WhisperConfig,
}

impl WhisperEngine {
    pub fn new(config: WhisperConfig) -> Result<Self, crate::services::Error> {
        if !config.model_path.exists() {
            return Err(crate::services::Error::from(format!(
                "Model file not found: {}",
                config.model_path.display()
            )));
        }

        let mut params = WhisperContextParameters::default();
        if config.use_gpu {
            params.use_gpu(true);
        }

        let ctx = WhisperContext::new_with_params(
            config.model_path.to_str().ok_or_else(|| {
                crate::services::Error::from("Invalid model path")
            })?,
            params,
        )
        .map_err(|e| crate::services::Error::from(format!("Failed to load model: {}", e)))?;

        Ok(Self {
            context: Arc::new(Mutex::new(ctx)),
            config,
        })
    }

    pub fn transcribe(&self, audio_data: &[f32]) -> Result<WhisperResult, crate::services::Error> {
        let start_time = std::time::Instant::now();
        
        let ctx = self.context.lock().map_err(|e| {
            crate::services::Error::from(format!("Failed to lock context: {}", e))
        })?;
        
        let mut state = ctx.create_state().map_err(|e| {
            crate::services::Error::from(format!("Failed to create state: {}", e))
        })?;

        let params = self.create_params();
        
        state.full(params, audio_data).map_err(|e| {
            crate::services::Error::from(format!("Failed to run model: {}", e))
        })?;

        let result = self.extract_results(&mut state)?;
        let duration_ms = start_time.elapsed().as_millis();

        Ok(WhisperResult {
            text: result.0,
            segments: result.1,
            language: result.2,
            duration_ms,
        })
    }

    pub fn transcribe_file(&self, audio_path: &Path) -> Result<WhisperResult, crate::services::Error> {
        let audio_data = self.load_audio_file(audio_path)?;
        self.transcribe(&audio_data)
    }

    fn create_params(&self) -> FullParams {
        let strategy = if self.config.beam_size > 1 {
            SamplingStrategy::BeamSearch {
                beam_size: self.config.beam_size,
                patience: 1.0,
            }
        } else {
            SamplingStrategy::Greedy {
                best_of: self.config.best_of,
            }
        };

        let mut params = FullParams::new(strategy);
        
        params.set_n_threads(self.config.n_threads);
        params.set_translate(self.config.translate);
        
        if let Some(ref lang) = self.config.language {
            params.set_language(Some(lang));
        }
        
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(self.config.suppress_blank);
        // params.set_suppress_non_speech_tokens(self.config.suppress_non_speech_tokens); // Method not available in current API
        params.set_temperature(self.config.temperature);
        
        // if self.config.max_context > 0 {
        //     params.set_max_context(self.config.max_context); // Method not available in current API
        // }
        
        if let Some(ref prompt) = self.config.initial_prompt {
            params.set_initial_prompt(prompt.as_str());
        }

        params
    }

    fn extract_results(&self, state: &mut whisper_rs::WhisperState) -> Result<(String, Vec<WhisperSegment>, String), crate::services::Error> {
        let num_segments = state.full_n_segments().map_err(|e| {
            crate::services::Error::from(format!("Failed to get segments: {}", e))
        })?;

        let mut full_text = String::new();
        let mut segments = Vec::new();

        for i in 0..num_segments {
            let text = state.full_get_segment_text(i).map_err(|e| {
                crate::services::Error::from(format!("Failed to get segment text: {}", e))
            })?;
            
            let start_time = state.full_get_segment_t0(i).map_err(|e| {
                crate::services::Error::from(format!("Failed to get segment start time: {}", e))
            })?;
            
            let end_time = state.full_get_segment_t1(i).map_err(|e| {
                crate::services::Error::from(format!("Failed to get segment end time: {}", e))
            })?;
            
            let prob = 0.0; // state.full_get_segment_prob(i).unwrap_or(0.0); // Method not available in current API

            full_text.push_str(&text);
            full_text.push(' ');

            segments.push(WhisperSegment {
                text,
                start_time,
                end_time,
                probability: prob,
            });
        }

        // let language = state.full_lang_id().map_err(|e| {
        //     crate::services::Error::from(format!("Failed to get language: {}", e))
        // })?; // Method not available in current API
        
        let lang_str = "unknown".to_string(); // TODO: Implement proper language detection

        Ok((full_text.trim().to_string(), segments, lang_str))
    }

    fn load_audio_file(&self, path: &Path) -> Result<Vec<f32>, crate::services::Error> {
        use hound::WavReader;
        
        let mut reader = WavReader::open(path).map_err(|e| {
            crate::services::Error::from(format!("Failed to open audio file: {}", e))
        })?;
        
        let spec = reader.spec();
        
        if spec.channels != 1 {
            return Err(crate::services::Error::from(
                "Audio must be mono channel. Please convert to mono first."
            ));
        }
        
        if spec.sample_rate != 16000 {
            return Err(crate::services::Error::from(
                "Audio must be 16kHz sample rate. Please resample first."
            ));
        }

        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().collect()
            }
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max_val))
                    .collect()
            }
        };

        samples.map_err(|e| {
            crate::services::Error::from(format!("Failed to read audio samples: {}", e))
        })
    }

    pub fn convert_audio_to_16khz_mono(
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), crate::services::Error> {
        use std::process::Command;
        
        let status = Command::new("ffmpeg")
            .args([
                "-i", input_path.to_str().unwrap(),
                "-ar", "16000",
                "-ac", "1",
                "-c:a", "pcm_s16le",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| {
                crate::services::Error::from(format!("Failed to run ffmpeg: {}", e))
            })?;

        if !status.success() {
            return Err(crate::services::Error::from(
                "FFmpeg conversion failed"
            ));
        }

        Ok(())
    }
}

pub struct WhisperService {
    engines: Arc<Mutex<Vec<WhisperEngine>>>,
    default_config: WhisperConfig,
}

impl WhisperService {
    pub fn new() -> Result<Self, crate::services::Error> {
        let default_config = WhisperConfig::default();
        let engine = WhisperEngine::new(default_config.clone())?;
        
        Ok(Self {
            engines: Arc::new(Mutex::new(vec![engine])),
            default_config,
        })
    }

    pub fn with_config(config: WhisperConfig) -> Result<Self, crate::services::Error> {
        let engine = WhisperEngine::new(config.clone())?;
        
        Ok(Self {
            engines: Arc::new(Mutex::new(vec![engine])),
            default_config: config,
        })
    }

    pub fn transcribe(&self, audio_data: &[f32]) -> Result<WhisperResult, crate::services::Error> {
        let engines = self.engines.lock().map_err(|e| {
            crate::services::Error::from(format!("Failed to lock engines: {}", e))
        })?;
        
        if engines.is_empty() {
            return Err(crate::services::Error::from("No engines available"));
        }
        
        engines[0].transcribe(audio_data)
    }

    pub fn transcribe_file(&self, audio_path: &Path) -> Result<WhisperResult, crate::services::Error> {
        let engines = self.engines.lock().map_err(|e| {
            crate::services::Error::from(format!("Failed to lock engines: {}", e))
        })?;
        
        if engines.is_empty() {
            return Err(crate::services::Error::from("No engines available"));
        }
        
        engines[0].transcribe_file(audio_path)
    }

    pub fn add_model(&mut self, config: WhisperConfig) -> Result<(), crate::services::Error> {
        let engine = WhisperEngine::new(config)?;
        let mut engines = self.engines.lock().map_err(|e| {
            crate::services::Error::from(format!("Failed to lock engines: {}", e))
        })?;
        engines.push(engine);
        Ok(())
    }
}

/// Initialize the Whisper STT service
pub async fn initialize() -> anyhow::Result<()> {
    log::info!("Whisper STT service initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WhisperConfig::default();
        assert_eq!(config.n_threads, 4);
        assert_eq!(config.language, Some("en".to_string()));
        assert!(!config.translate);
    }

    #[test]
    fn test_whisper_segment_serialization() {
        let segment = WhisperSegment {
            text: "Hello world".to_string(),
            start_time: 0,
            end_time: 1500,
            probability: 0.95,
        };
        
        let json = serde_json::to_string(&segment).unwrap();
        let deserialized: WhisperSegment = serde_json::from_str(&json).unwrap();
        
        assert_eq!(segment.text, deserialized.text);
        assert_eq!(segment.start_time, deserialized.start_time);
    }
}