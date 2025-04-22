// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::os::unix::fs::PermissionsExt; // Added for `from_mode`
use error_chain::error_chain;
use crate::sam::tools; // Add missing import for tools module

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
    }
}

impl From<zip::result::ZipError> for tools::Error {
    fn from(err: zip::result::ZipError) -> Self {
        tools::Error::from(err.to_string())
    }
}

/// Executes a Python 3 command and returns its output as a `String`.
pub fn python3(command: &str) -> Result<String> {
    let output = Command::new("python3")
        .arg(command)
        .output()
        .map_err(Error::from)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Executes a shell command and returns its output as a `String`.
pub fn cmd(command: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(Error::from)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Executes a Linux shell command and logs the result.
pub fn uinx_cmd(command: &str) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output();

    match output {
        Ok(cmd) if cmd.status.success() => {
            log::info!("{}:{}", command, String::from_utf8_lossy(&cmd.stdout));
        }
        Ok(cmd) => {
            log::error!("{}:{}", command, String::from_utf8_lossy(&cmd.stderr));
        }
        Err(e) => {
            log::error!("Failed to execute command '{}': {}", command, e);
        }
    }
}

/// Checks if a WAV file contains sounds above a certain threshold.
pub fn does_wav_have_sounds(audio_filename: &str) -> Result<bool> {
    let threshold = 14000_i16;
    let mut has_sounds = false;

    let mut audio_file = hound::WavReader::open(audio_filename)?; // Add `mut` to fix borrow issue
    let raw_samples = audio_file.samples::<i16>().filter_map(|result| result.ok()); // Fixed closure type

    for (i, sample) in raw_samples.enumerate() {
        if i % 100 == 0 && (sample > threshold || sample < -threshold) {
            has_sounds = true;
            break;
        }
    }

    Ok(has_sounds)
}

/// Extracts a ZIP file to the specified directory.
pub fn extract_zip(zip_path: &str, extract_path: &str) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(extract_path).join(file.enclosed_name().ok_or("Invalid path")?);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}