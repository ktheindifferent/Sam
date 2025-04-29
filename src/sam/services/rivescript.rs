// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::io::{Write};
use error_chain::error_chain;
use crate::sam::http::api::io::IOReply;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        Json(serde_json::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        SamMemoryError(crate::sam::memory::Error);
        ToolkitError(crate::sam::tools::Error);
    }
}

pub fn query(input: &str) -> Result<IOReply> {
    let rivescript_reply = crate::sam::tools::cmd(format!("python3 /opt/sam/scripts/rivescript/brain.py \"{input}\"").as_str())?;

    if rivescript_reply.contains(":::::"){
        // TODO - Parse Command
    } 

    let io = IOReply{
        text: rivescript_reply,
        timestamp: 0,
        response_type: "io".to_string()
    };

    Ok(io)
}
