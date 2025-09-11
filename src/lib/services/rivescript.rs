// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use crate::http::api::io::IOReply;
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
    SamMemoryError(#[from] crate::memory::Error),
    #[error("Toolkit error: {0}")]
    ToolkitError(#[from] crate::tools::Error),
    #[error("Other error: {0}")]
    Other(String),
}

// use std::io::Write;

#[allow(unexpected_cfgs)]
pub fn query(input: &str) -> anyhow::Result<IOReply> {
    // Try multiple paths for brain.py
    let brain_paths = vec![
        "./scripts/rivescript/brain.py",
        "/opt/sam/scripts/rivescript/brain.py",
        "/Users/calebsmith/Documents/ktheindifferent/Sam/scripts/rivescript/brain.py",
    ];
    
    let mut rivescript_reply = String::new();
    let mut success = false;
    
    for brain_path in brain_paths {
        match crate::tools::safe_cmd("python3", &[brain_path, input]) {
            Ok(reply) => {
                rivescript_reply = reply;
                success = true;
                break;
            }
            Err(_) => continue,
        }
    }
    
    if !success {
        return Err(anyhow::anyhow!("Brain script not found in any location"));
    }

    if rivescript_reply.contains(":::::") {
        // TODO - Parse Command
    }

    let io = IOReply {
        text: rivescript_reply,
        timestamp: 0,
        response_type: "io".to_string(),
        executed_actions: Vec::new(),
        context_updates: None,
    };

    Ok(io)
}
