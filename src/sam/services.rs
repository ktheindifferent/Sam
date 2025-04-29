// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.
use crate::sam::tools;
use error_chain::error_chain;
#[allow(unexpected_cfgs)]
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
        // TchError(tch::TchError);
        PostError(rouille::input::post::PostError);
        ParseFloatError(std::num::ParseFloatError);
        SamMemoryError(crate::sam::memory::Error);
        // ClError(opencl3::error_codes::ClError);
    }

    errors {
        Other(msg: String) {
            description("Other error")
            display("{}", msg)
        }
    }
}
// impl From<reqwest::Error> for crate::sam::services::Error {
//     fn from(err: reqwest::Error) -> Self {
//         crate::sam::services::Error::from(err)
//     }
// }
impl From<tools::Error> for crate::sam::services::Error {
    fn from(err: tools::Error) -> Self {
        crate::sam::services::Error::from_kind(crate::sam::services::ErrorKind::Other(
            err.to_string(),
        ))
    }
}

pub mod darknet;
pub mod docker;
pub mod dropbox;
pub mod jupiter;
pub mod lifx;
pub mod llama;
pub mod media;
pub mod notifications;
pub mod osf;
pub mod pg;
pub mod redis;
pub mod rivescript;
pub mod rtsp;
pub mod socket;
pub mod sound;
pub mod spotify;
pub mod sprec;
pub mod storage;
pub mod stt;
pub mod tts;
// pub mod whisper;
pub mod crawler;
pub mod p2p;
pub mod sms;
pub mod who;
