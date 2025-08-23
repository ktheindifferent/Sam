// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// TODO - Ability to use multiple stt servers
// Cloud -> Internal Cloud -> Localhost
// TODO - Don't start docker unless localhost has been called

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};
use crate::sam::services::Error;

pub struct WhisperWorker {
    pub pid: u32,
    pub is_working: bool,
    pub whisper_state: whisper_rs::WhisperState,
}
// Note: Default implementation removed. Use WhisperWorker::new() directly
// which returns a Result for proper error handling

impl WhisperWorker {
    pub fn new() -> Result<WhisperWorker, Error> {
        let params = whisper_rs::WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params("/opt/sam/models/ggml-base.bin", params)
            .map_err(|e| Error::from(format!("Failed to load Whisper model: {}", e)))?;
        let state = ctx.create_state()
            .map_err(|e| Error::from(format!("Failed to create Whisper state: {}", e)))?;
        Ok(WhisperWorker {
            pid: 0,
            is_working: false,
            whisper_state: state,
        })
    }

    pub fn transcribe(
        mut self,
        audio_data: Vec<f32>,
    ) -> Result<Vec<String>, crate::sam::services::Error> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        // params.set_translate(true);
        // params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // now we can run the model
        // note the key we use here is the one we created above
        self.whisper_state
            .full(params, &audio_data[..])
            .map_err(|e| Error::from(format!("Failed to run Whisper model: {}", e)))?;

        // fetch the results
        let num_segments = self
            .whisper_state
            .full_n_segments()
            .map_err(|e| Error::from(format!("Failed to get number of segments: {}", e)))?;
        let mut segments: Vec<String> = Vec::new();
        for i in 0..num_segments {
            let segment = self
                .whisper_state
                .full_get_segment_text(i)
                .map_err(|e| Error::from(format!("Failed to get segment {}: {}", i, e)))?;
            segments.push(segment);
        }
        Ok(segments)
    }
}

pub struct WhisperService;

impl WhisperService {}
