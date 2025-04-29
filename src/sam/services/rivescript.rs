// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use crate::sam::http::api::io::IOReply;
use thiserror::Error;
pub type Result<T> = anyhow::Result<T>;

#[derive(Error, Debug)]
pub enum RiveScriptError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("Sam memory error: {0}")]
    SamMemoryError(#[from] crate::sam::memory::Error),
    #[error("Toolkit error: {0}")]
    ToolkitError(#[from] crate::sam::tools::Error),
    #[error("Other error: {0}")]
    Other(String),
}

// use std::io::Write;

#[allow(unexpected_cfgs)]
pub fn query(input: &str) -> anyhow::Result<IOReply> {
    let rivescript_reply = crate::sam::tools::cmd(
        format!("python3 /opt/sam/scripts/rivescript/brain.py \"{input}\"").as_str(),
    )?;

    if rivescript_reply.contains(":::::") {
        // TODO - Parse Command
    }

    let io = IOReply {
        text: rivescript_reply,
        timestamp: 0,
        response_type: "io".to_string(),
    };

    Ok(io)
}
