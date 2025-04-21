// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use std::fs::File;
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
    let rivescript_reply = crate::sam::tools::cmd(format!("python3 /opt/sam/scripts/rivescript/brain.py \"{}\"", input).as_str())?;

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

pub fn install() -> std::io::Result<()> {
    let data = include_bytes!("../../../scripts/rivescript/brain.py");

    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/scripts/rivescript/brain.py")?;

    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../scripts/rivescript/eg.zip");

    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/scripts/rivescript/eg.zip")?;

    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    crate::sam::tools::extract_zip("/opt/sam/scripts/rivescript/eg.zip", "/opt/sam/scripts/rivescript/");
    crate::sam::tools::uinx_cmd("rm -rf /opt/sam/scripts/rivescript/eg.zip");

    Ok(())
}
