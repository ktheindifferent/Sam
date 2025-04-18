// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use error_chain::error_chain;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
        TchError(tch::TchError);
        PostError(rouille::input::post::PostError);
        ParseFloatError(std::num::ParseFloatError);
    
        SamMemoryError(crate::sam::memory::Error);
        // ClError(opencl3::error_codes::ClError);
    }
}

pub mod darknet;
pub mod dropbox;
pub mod jupiter;
pub mod lifx;
pub mod media;
pub mod notifications;
pub mod osf;
pub mod rivescript;
pub mod rtsp;
pub mod socket;
pub mod sound;
pub mod sprec;
pub mod storage;
pub mod stt;
pub mod tts;
// pub mod whisper;
pub mod who;