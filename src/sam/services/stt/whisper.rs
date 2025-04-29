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
use std::io::{self};
use std::process::{Command, Stdio};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};
use tokio::fs;
use std::env;
use std::path::PathBuf;



pub struct WhisperWorker {
    pub pid: u32,
    pub is_working: bool,
    pub whisper_state: whisper_rs::WhisperState,
}
impl WhisperWorker {
    pub fn new() -> WhisperWorker {
        let params = whisper_rs::WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params("/opt/sam/models/ggml-base.bin", params)
            .expect("failed to load model");
        let state = ctx.create_state().expect("failed to create state");
        WhisperWorker {
            pid: 0,
            is_working: false,
            whisper_state: state,
        }
    }

    pub fn transcribe(mut self, audio_data: Vec<f32>) -> Result<Vec<String>, crate::sam::services::Error>{
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
        self.whisper_state.full(params, &audio_data[..]).expect("failed to run model");

        // fetch the results
        let num_segments = self.whisper_state.full_n_segments().expect("failed to get number of segments");
        let mut segments: Vec<String> = Vec::new();
        for i in 0..num_segments {
            let segment = self.whisper_state
                .full_get_segment_text(i)
                .expect("failed to get segment");
            segments.push(segment);
        }
        Ok(segments)
    }
}

pub struct WhisperService;

impl WhisperService {
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

    pub async fn install() -> io::Result<String> {
        let mut log = String::new();
        log.push_str(&Self::ensure_whisper_binary_with_output().await?);
        log.push_str("Whisper binary installed.\n");

        
        Ok(log)
    }
}




