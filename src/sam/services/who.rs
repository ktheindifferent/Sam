// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs::File;
use std::io::{Result, Write};

/// Writes the provided data to the specified file path.
fn write_file(data: &[u8], path: &str) -> Result<()> {
    let mut pos = 0;
    let mut buffer = File::create(path)?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }
    Ok(())
}

/// Installs necessary scripts and datasets for the "who" service.
pub fn install() -> Result<()> {
    // Write the Python script
    write_file(
        include_bytes!("../../../scripts/who.io/who2.py"),
        "/opt/sam/scripts/who.io/who2.py",
    )?;

    // Write the trained KNN model
    write_file(
        include_bytes!("../../../scripts/who.io/trained_knn_model.clf"),
        "/opt/sam/scripts/who.io/trained_knn_model.clf",
    )?;

    // Write and extract Barack Obama dataset
    let obama_zip = "/opt/sam/scripts/who.io/dataset/barack_obama.zip";
    write_file(
        include_bytes!("../../../scripts/who.io/dataset/barack_obama.zip"),
        obama_zip,
    )?;
    let _ = crate::sam::tools::extract_zip(obama_zip, "/opt/sam/scripts/who.io/dataset/");
    crate::sam::tools::uinx_cmd(&format!("rm -rf {obama_zip}"));

    // Write and extract Donald Trump dataset
    let trump_zip = "/opt/sam/scripts/who.io/dataset/donald_trump.zip";
    write_file(
        include_bytes!("../../../scripts/who.io/dataset/donald_trump.zip"),
        trump_zip,
    )?;
    let _ = crate::sam::tools::extract_zip(trump_zip, "/opt/sam/scripts/who.io/dataset/");
    crate::sam::tools::uinx_cmd(&format!("rm -rf {trump_zip}"));

    Ok(())
}
