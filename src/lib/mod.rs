// use futures::stream::StreamExt;
use std::fs;
use std::io::{self, Result};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::PermissionsExt;

use std::path::Path;
use std::process::Command;
use tokio::fs as async_fs;
use tokio::io::{self as async_io, AsyncWriteExt};
use tokio::process::Command as TokioCommand;
use zip::read::ZipArchive;

pub mod services;

// pub use self::cmd_async;

pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn print_banner(user: String) {
    println!("███████     █████     ███    ███    ");
    println!("██         ██   ██    ████  ████    ");
    println!("███████    ███████    ██ ████ ██    ");
    println!("     ██    ██   ██    ██  ██  ██    ");
    println!("███████ ██ ██   ██ ██ ██      ██ ██ ");
    println!("Smart Artificial Mind");
    println!("VERSION: {VERSION:?}");
    println!("Copyright 2021-2026 The Open Sam Foundation (OSF)");
    println!("Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)");
    println!("Licensed under GPLv3....see LICENSE file.");
    println!("================================================");
    println!("Hello, {user}");
    println!("================================================");
}

pub async fn cmd_async(command: &str) -> Result<String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())

    }
    #[cfg(target_os = "windows")]
    {
        let output = TokioCommand::new("cmd")
            .arg("/C")
            .arg(command)
            .output()
            .await?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

}
/// Asynchronously determines the current human user for SAM.
/// Tries `/opt/sam/whoismyhuman`, then `SAM_USER` env var, then falls back to "unknown_human".
pub async fn get_human() -> String {
    // Try reading from file
    if let Ok(contents) = async_fs::read_to_string("/opt/sam/whoismyhuman").await {
        let user = contents.trim();
        if !user.is_empty() {
            return user.to_string();
        }
    }
    // Try environment variable
    if let Ok(env_user) = std::env::var("SAM_USER") {
        let user = env_user.trim();
        if !user.is_empty() {
            return user.to_string();
        }
    }
    // Fallback
    "unknown_human".to_string()
}

pub async fn run_and_log_async(cmd: &str, args: &[&str]) -> Result<()> {
    let output = TokioCommand::new(cmd)
        .args(args)
        .output()
        .await;

    match output {
        Ok(output) => {
            if !output.status.success() {
                // Special handling for winget exit code 0x8a15002b (or -1980082133)
                let code = output.status.code().unwrap_or(0);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                if (code == 0x8a15002b_u32 as i32 || code == -1980082133)
                    && (stdout.contains("already installed")
                        || stdout.contains("No available upgrade found")
                        || stdout.contains("No newer package versions are available"))
                {
                    log::info!(
                        "Command `{}` exit code {} but stdout indicates already installed or up to date: {}",
                        cmd,
                        code,
                        stdout
                    );
                    return Ok(());
                }
                log::error!(
                    "Command `{}` failed with status {}: {}",
                    cmd,
                    output.status,
                    stderr
                );
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Command `{}` failed with status {}: {}",
                        cmd,
                        output.status,
                        stderr
                    ),
                ));
            }
            log::info!(
                "Command `{}` output: {}",
                cmd,
                String::from_utf8_lossy(&output.stdout)
            );
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to execute command `{}`: {}", cmd, e);
            Err(e)
        }
    }
}

pub fn run_and_log(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd)
        .args(args)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                log::error!(
                    "Command `{}` failed with status {}: {}",
                    cmd,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Command `{}` failed with status {}: {}",
                        cmd,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            log::info!(
                "Command `{}` output: {}",
                cmd,
                String::from_utf8_lossy(&output.stdout)
            );
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to execute command `{}`: {}", cmd, e);
            Err(e)
        }
    }
}

pub async fn println(output_lines: Option<&std::sync::Arc<tokio::sync::Mutex<Vec<String>>>>, line: String) {
    if let Some(lines) = output_lines {
        let mut linesx = lines.lock().await;
        linesx.push(line);
    } else {
        log::info!("{}", line);
    }
}

pub fn cmd(command: &str) -> Result<String> {
    let output = Command::new("sh").arg("-c").arg(command).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn extract_zip_async(zip_path: &str, extract_path: &str) -> Result<()> {
    let file = async_fs::File::open(zip_path).await?;
    let mut buffer = Vec::new();
    let mut reader = async_io::BufReader::new(file);
    async_io::AsyncReadExt::read_to_end(&mut reader, &mut buffer).await?;
    let cursor = std::io::Cursor::new(buffer);
    let mut archive = ZipArchive::new(cursor)?;

    // Collect file metadata first to avoid borrow issues
    let mut file_infos = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(io::Error::other)?;
        let name = file.name().to_string();
        let is_dir = file.name().ends_with('/');
        let enclosed_name = file.enclosed_name().map(|p| p.to_owned());
        let unix_mode = file.unix_mode();
        let mut contents = Vec::new();
        if !is_dir {
            std::io::copy(&mut file, &mut contents)?;
        }
        file_infos.push((name, is_dir, enclosed_name, unix_mode, contents));
    }

    // Now process each file asynchronously
    let futs = file_infos
        .into_iter()
        .map(|(_name, is_dir, enclosed_name, unix_mode, contents)| {
            let extract_path = extract_path.to_owned();
            async move {
                let outpath = match enclosed_name {
                    Some(p) => Path::new(&extract_path).join(p),
                    None => return Err(io::Error::other("Invalid path")),
                };

                if is_dir {
                    async_fs::create_dir_all(&outpath).await?;
                } else {
                    if let Some(parent) = outpath.parent() {
                        async_fs::create_dir_all(parent).await?;
                    }
                    let mut outfile = async_fs::File::create(&outpath).await?;
                    outfile.write_all(&contents).await?;
                    #[cfg(unix)]
                    if let Some(mode) = unix_mode {
                        async_fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                            .await?;
                    }
                }
                Ok(())
            }
        });

    futures::future::try_join_all(futs).await?;
    Ok(())
}

/// Extracts a ZIP file to the specified directory.
pub fn extract_zip(zip_path: &str, extract_path: &str) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(extract_path).join(
            file.enclosed_name()
                .ok_or_else(|| io::Error::other("Invalid path"))?,
        );

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
