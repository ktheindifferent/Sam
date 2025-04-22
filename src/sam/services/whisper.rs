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

use rouille::Request;
use rouille::Response;
use rouille::post_input;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};



pub struct WhisperWorker {
    pub pid: u32,
    pub is_working: bool,
    pub whisper_state: whisper_rs::WhisperState<'a>
}
impl WhisperWorker<'_> {
    pub fn new() -> WhisperWorker<'static> {
        let ctx = WhisperContext::new("/opt/sam/models/ggml-base.bin").expect("failed to load model");
        let mut state = ctx.create_state().expect("failed to create state");
        WhisperWorker{
            pid: 0,
            is_working: false,
            work_path: String::new(),
            whisper_state: state
        }
    }

    pub fn transcribe(sef: Self, audio_data: Vec<f32>) -> Result<Vec<String>, crate::sam::services::Error>{
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
        sef.whisper_state.full(params, &audio_data[..]).expect("failed to run model");

        // fetch the results
        let num_segments = sef.whisper_state.full_n_segments().expect("failed to get number of segments");
        
        let mut segments: Vec<String> = Vec::new();
        for i in 0..num_segments {
            let segment = sef.whisper_state
                .full_get_segment_text(i)
                .expect("failed to get segment");
            segments.push(segment);
        }

        return Ok(segments);

    }

}




