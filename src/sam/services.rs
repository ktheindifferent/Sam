// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.
use thiserror::Error;
pub type Result<T> = anyhow::Result<T>;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),
    #[error("Post input error: {0}")]
    PostError(#[from] rouille::input::post::PostError),
    #[error("Parse float error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("Sam memory error: {0}")]
    SamMemoryError(#[from] crate::sam::memory::Error),
    #[error("Tools error: {0}")]
    ToolsError(#[from] crate::sam::tools::Error),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Error = ServiceError;

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

// Core Services
pub mod darknet;
pub mod docker;
pub mod dropbox;
pub mod jupiter;
pub mod lifx;
pub mod llama;
pub mod matter;
pub mod media;
pub mod notifications;
pub mod osf;
pub mod password_manager;
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
pub mod voice;
pub mod vulnerability_scanner;
// pub mod whisper;
pub mod crawler;
pub mod mdns;
pub mod p2p;
pub mod sms;
pub mod who;

// Additional Services (Enhanced Features)
pub mod backup;
pub mod clamav;
pub mod copilot;
pub mod file_storage;
pub mod git;
pub mod github;
pub mod openai;
pub mod ssh;

// Service Orchestration
pub mod orchestrator;
