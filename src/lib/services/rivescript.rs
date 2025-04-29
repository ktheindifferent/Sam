// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs::File;
use std::io::{Write};
use error_chain::error_chain;

use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
pub async fn install() -> std::io::Result<()> {
    let data = include_bytes!("../../../scripts/rivescript/brain.py");

    let mut pos = 0;
    let mut buffer = TokioFile::create("/opt/sam/scripts/rivescript/brain.py").await?;

    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..]).await?;
        pos += bytes_written;
    }
    buffer.flush().await?;

    let data = include_bytes!("../../../scripts/rivescript/eg.zip");

    let mut pos = 0;
    let mut buffer = TokioFile::create("/opt/sam/scripts/rivescript/eg.zip").await?;

    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..]).await?;
        pos += bytes_written;
    }
    buffer.flush().await?;

    let _ = crate::extract_zip_async("/opt/sam/scripts/rivescript/eg.zip", "/opt/sam/scripts/rivescript/").await?;
    let _ = crate::cmd_async("rm -rf /opt/sam/scripts/rivescript/eg.zip").await?;

    Ok(())
}
