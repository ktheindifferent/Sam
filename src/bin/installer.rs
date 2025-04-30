// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// Required dependencies
// use opencl3::device::{get_all_devices, CL_DEVICE_TYPE_GPU};
use serde::{Deserialize, Serialize};
use tokio::fs as async_fs;
use thiserror::Error;

use dialoguer::Confirm;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::env;
use std::fs;
use std::io::{self};
use std::path::Path;
use std::process::Command;

// TODO: Wrap in a feature just in case we dont want it
use opencl3::device::get_all_devices;
use opencl3::device::CL_DEVICE_TYPE_GPU;

pub type Result<T> = anyhow::Result<T>;

#[derive(Error, Debug)]
pub enum InstallerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Other error: {0}")]
    Other(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger with color, warning level, and timestamps
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Info)
        .with_timestamps(true)
        .init()
        .unwrap();

    // Store the current username in the SAM_USER environment variable
    let whoami = whoami::username();
    // let opt_sam_path = Path::new("/opt/sam/");
    if whoami != "root" {
        env::set_var("SAM_USER", &whoami);
    }

    // Optionally set environment variables for libraries (uncomment if needed)
    // env::set_var("LIBTORCH", "/app/libtorch/libtorch");
    // env::set_var("LD_LIBRARY_PATH", "${LIBTORCH}/lib:$LD_LIBRARY_PATH");

    // Ensure required environment variables are available for sudo context
    // dependent on OS
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        sudo::with_env(&[
            "LIBTORCH",
            "LD_LIBRARY_PATH",
            "PG_DBNAME",
            "PG_USER",
            "PG_PASS",
            "PG_ADDRESS",
            "SAM_USER",
        ])
        .unwrap();
    }
    // Check if the script is run with sudo
    log::info!("Starting preinstallation...");
    let _ = pre_install().await;

    // Store the current username in the SAM_USER environment variable
    // Cross platform way to get the username
    // let user = whoami::username();
    let user = env::var("SAM_USER").unwrap_or_else(|_| whoami.clone());
    libsam::print_banner(user.clone());
    let opt_sam_path = Path::new("/opt/sam/");
    if user != "root" {
        let file_path = opt_sam_path.join("whoismyhuman");
        if !file_path.exists() {
            if let Err(e) = fs::write(&file_path, &user) {
                log::error!("Failed to create whoismyhuman: {}", e);
            }
        }
        if opt_sam_path.exists() && opt_sam_path.is_dir() {
            let file_path = opt_sam_path.join("whoismyhuman");
            if let Err(e) = fs::write(&file_path, &user) {
                log::error!("Failed to write whoismyhuman: {}", e);
            }
        }
    }

    // log::info!("Checking for GPU devices...");
    // let _ = check_gpu_devices().await?;
    log::info!("Compiling snapcast...");
    let _ = libsam::services::snapcast::install().await?;
    log::info!("Installing darknet...");
    let _ = libsam::services::darknet::install(None).await?;
    log::info!("Installing SPREC...");
    let _ = libsam::services::sprec::install().await?;
    log::info!("Installing LLAMA...");
    let _ = libsam::services::llama::install(None).await?;
    log::info!("Installing STT...");
    let _ = libsam::services::stt::install(None).await?;
    log::info!("Installing Rivescript...");
    let _ = libsam::services::rivescript::install().await?;
    log::info!("Installing Who.io...");
    let _ = libsam::services::who::install().await?;
    log::info!("Installing HTTP server...");
    let _ = libsam::services::http::install().await?;

    Ok(())
}
//     // install_services();
//     log::info!("Setting up default settings...");
//     // crate::sam::http::api::settings::set_defaults();
//     log::info!("Configuring Snapcast...");
//     // crate::sam::services::media::snapcast::configure();
//     log::info!("Installation complete!");

// }

#[cfg(target_os = "windows")]
async fn pre_install() -> Result<()> {
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
    log::info!("Starting Windows pre-installation steps...");

    let _ = libsam::services::chocolatey::install().await?;

    let _ = libsam::services::chocolatey::verify().await?;

    // Install required packages using Chocolatey (including make)
    log::info!("Installing required packages using Chocolatey...");
    let choco_packages = ["ffmpeg", "git-lfs", "opencv", "python3", "make"];
    let _ = libsam::services::chocolatey::install_packages(choco_packages.to_vec()).await?;
    
    // Refresh environment variables so newly installed tools are available
    log::info!("Refreshing environment variables with refreshenv...");
    let result = libsam::run_and_log("refreshenv", &[]);
    match result {
        Ok(_) => log::info!("Environment variables refreshed."),
        Err(e) => log::warn!("Failed to refresh environment variables: {}", e),
    }

    // Install Python packages
    let pip_path = "C:\\Python313\\Scripts\\pip3.exe";
    let pip_args = ["install", "rivescript", "pexpect"];
    log::info!("Running: {} {}", pip_path, pip_args.join(" "));
    let result = libsam::run_and_log(pip_path, &pip_args);
    match result {
        Ok(_) => log::info!("Python package installation succeeded."),
        Err(e) => log::error!("Python package installation failed: {}", e),
    }

    // Try to build git from source first
    let git_url = "https://github.com/git/git/archive/refs/tags/v2.49.0.zip";
    let git_zip_path = "C:\\git.zip";
    let git_dir = "C:\\git\\git-2.49.0";
    let mut build_failed = false;

    if std::path::Path::new(git_zip_path).exists() {
        log::info!("Git zip already exists at {}", git_zip_path);
        if std::path::Path::new(git_dir).exists() {
            log::info!("Git directory already exists at {}", git_dir);
        } else {
            log::info!("Unzipping git...");
            let result = libsam::run_and_log("unzip", &["-o", git_zip_path, "-d", "C:\\git"]);
            match result {
                Ok(_) => log::info!("Git unzipped successfully."),
                Err(e) => {
                    log::error!("Failed to unzip git: {}", e);
                    build_failed = true;
                }
            }
        }
    } else {
        log::info!("Git zip not found, downloading...");
        log::info!("Downloading git from {}", git_url);
        let result = libsam::run_and_log("curl", &["-L", git_url, "-o", git_zip_path]);
        match result {
            Ok(_) => log::info!("Git downloaded successfully."),
            Err(e) => {
                log::error!("Failed to download git: {}", e);
                build_failed = true;
            }
        }
    }

    // Attempt to build git if previous steps succeeded
    if !build_failed && std::path::Path::new(git_dir).exists() {
        log::info!("Attempting to build git from source...");
        // This is a placeholder for the actual build process
        // You would need to implement the build logic here, e.g., using MSYS2/make
        let build_result = libsam::run_and_log("make", &["-C", git_dir]);
        match build_result {
            Ok(_) => log::info!("Git built from source successfully."),
            Err(e) => {
                log::error!("Failed to build git from source: {}", e);
                build_failed = true;
            }
        }
    } else if !std::path::Path::new(git_dir).exists() {
        build_failed = true;
    }

    // If building from source failed, fallback to Chocolatey
    if build_failed {
        log::info!("Building git from source failed or was not possible. Falling back to Chocolatey...");
        let choco_git_args = ["install", "git", "-y"];
        log::info!("Running: {} {}", choco_path, choco_git_args.join(" "));
        let result = libsam::run_and_log(choco_path, &choco_git_args);
        match result {
            Ok(_) => log::info!("Chocolatey git installation succeeded."),
            Err(e) => log::error!("Chocolatey git installation failed: {}", e),
        }
    }

    // Check for git.exe in all subdirectories of Program Files and Program Files (x86)
    let mut found_git = false;
    let search_dirs = [
        "C:\\Program Files",
        "C:\\Program Files (x86)",
    ];
    'outer: for base in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let git_path = path.join("cmd").join("git.exe");
                    if git_path.exists() {
                        let git_dir = git_path.parent().unwrap();
                        let mut paths = std::env::var_os("PATH").unwrap_or_default();
                        let mut new_path = std::env::split_paths(&paths).collect::<Vec<_>>();
                        new_path.push(git_dir.to_path_buf());
                        let joined = std::env::join_paths(new_path).unwrap();
                        std::env::set_var("PATH", &joined);
                        log::info!("Added {} to PATH for git", git_dir.display());
                        found_git = true;
                        break 'outer;
                    }
                }
            }
        }
    }
    if !found_git {
        log::warn!("git.exe not found. Installing Git for Windows using Chocolatey...");
        let result = libsam::run_and_log(choco_path, &["install", "git", "-y"]);
        match result {
            Ok(_) => log::info!("Chocolatey git installation succeeded."),
            Err(e) => {
                log::error!("Chocolatey git installation failed: {}", e);
                return Err(e.into());
            }
        }
        // Search again after install
        let mut found_git = false;
        'outer2: for base in &search_dirs {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let git_path = path.join("cmd").join("git.exe");
                        if git_path.exists() {
                            let git_dir = git_path.parent().unwrap();
                            let mut paths = std::env::var_os("PATH").unwrap_or_default();
                            let mut new_path = std::env::split_paths(&paths).collect::<Vec<_>>();
                            new_path.push(git_dir.to_path_buf());
                            let joined = std::env::join_paths(new_path).unwrap();
                            std::env::set_var("PATH", &joined);
                            log::info!("Added {} to PATH for git", git_dir.display());
                            found_git = true;
                            break 'outer2;
                        }
                    }
                }
            }
        }
        if !found_git {
            // Try to build git from source using MSYS2 if available, otherwise install MSYS2
            let msys2_bash = r"C:\\msys64\\usr\\bin\\bash.exe";
            // Detect system architecture and use the correct MSYS2 installer
            let is_64bit = cfg!(target_pointer_width = "64");
            let msys2_installer_url = if is_64bit {
                "https://github.com/msys2/msys2-installer/releases/latest/download/msys2-x86_64-latest.exe"
            } else {
                "https://github.com/msys2/msys2-installer/releases/latest/download/msys2-i686-latest.exe"
            };
            let msys2_installer_path = r"C:\\msys2-installer.exe";
            if !std::path::Path::new(msys2_bash).exists() {
                log::warn!("MSYS2 not found. Downloading and installing MSYS2...");
                let result = libsam::run_and_log("curl", &["-L", msys2_installer_url, "-o", msys2_installer_path]);
                match result {
                    Ok(_) => log::info!("MSYS2 installer downloaded successfully."),
                    Err(e) => {
                        log::error!("Failed to download MSYS2 installer: {}", e);
                        return Err(e.into());
                    }
                }
                let result = std::process::Command::new(msys2_installer_path)
                    .arg("/S") // Silent install
                    .status();
                match result {
                    Ok(status) if status.success() => log::info!("MSYS2 installed successfully."),
                    Ok(status) => {
                        log::error!("MSYS2 installer failed with exit code: {:?}", status.code());
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "MSYS2 installer failed").into());
                    }
                    Err(e) => {
                        log::error!("Failed to run MSYS2 installer: {}", e);
                        return Err(e.into());
                    }
                }
            }
            if std::path::Path::new(msys2_bash).exists() {
                log::info!("MSYS2 detected. Attempting to build git from source using MSYS2...");
                let git_src = "/c/git/git-2.49.0"; // MSYS2 uses Unix-style paths
                if !std::path::Path::new(msys2_bash).exists() {
                    log::warn!("MSYS2 not found. Downloading and installing MSYS2...");
                    let result = libsam::run_and_log("curl", &["-L", msys2_installer_url, "-o", msys2_installer_path]);
                    match result {
                        Ok(_) => log::info!("MSYS2 installer downloaded successfully."),
                        Err(e) => {
                            log::error!("Failed to download MSYS2 installer: {}", e);
                            return Err(e.into());
                        }
                    }
                    let result = std::process::Command::new(msys2_installer_path)
                        .arg("/S") // Silent install
                        .status();
                    match result {
                        Ok(status) if status.success() => log::info!("MSYS2 installed successfully."),
                        Ok(status) => {
                            log::error!("MSYS2 installer failed with exit code: {:?}", status.code());
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, "MSYS2 installer failed").into());
                        }
                        Err(e) => {
                            log::error!("Failed to run MSYS2 installer: {}", e);
                            return Err(e.into());
                        }
                    }
                }
                if std::path::Path::new(msys2_bash).exists() {
                    log::info!("MSYS2 detected. Attempting to build git from source using MSYS2...");
                    // Use MSYS2 bash with MinGW-w64 environment for native Windows build
                    let msys2_bash = r"C:\\msys64\\usr\\bin\\bash.exe";
                    let git_src = "/c/git/git-2.49.0"; // MSYS2 uses Unix-style paths
                    if std::path::Path::new(msys2_bash).exists() {
                        log::info!("MSYS2 bash detected. Attempting to build git from source using MinGW-w64 environment...");
                        let build_script = format!(
                            "export MSYSTEM=MINGW64; export CHERE_INVOKING=1; \
                            pacman -Sy --noconfirm mingw-w64-x86_64-toolchain mingw-w64-x86_64-cmake mingw-w64-x86_64-perl mingw-w64-x86_64-python3 mingw-w64-x86_64-curl mingw-w64-x86_64-openssl mingw-w64-x86_64-zlib mingw-w64-x86_64-gettext autoconf automake libtool base-devel && \
                            cd {} && make configure && ./configure --prefix=/mingw64 && make all && make install > /c/git/build.log 2>&1",
                            git_src
                        );
                        let status = std::process::Command::new(msys2_bash)
                            .arg("-l")
                            .arg("-c")
                            .arg(&build_script)
                            .status();
                        match status {
                            Ok(status) if status.success() => {
                                log::info!("Git built and installed successfully using MinGW-w64 environment.");
                                // After build, try to find git.exe in mingw64/bin
                                let built_git_path = r"C:\\msys64\\mingw64\\bin\\git.exe";
                                if std::path::Path::new(built_git_path).exists() {
                                    let git_dir = std::path::Path::new(built_git_path).parent().unwrap();
                                    let mut paths = std::env::var_os("PATH").unwrap_or_default();
                                    let mut new_path = std::env::split_paths(&paths).collect::<Vec<_>>();
                                    new_path.push(git_dir.to_path_buf());
                                    let joined = std::env::join_paths(new_path).unwrap();
                                    std::env::set_var("PATH", &joined);
                                    log::info!("Added {} to PATH for built git", git_dir.display());
                                } else {
                                    log::error!("git.exe not found after MinGW-w64 build. Please check the build output in C:/msys64/mingw64/bin and C:/git/build.log.");
                                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "git not found after MinGW-w64 build").into());
                                }
                            }
                            Ok(status) => {
                                log::error!("MinGW-w64 build process failed with exit code: {:?}. See C:/git/build.log for details.", status.code());
                                return Err(std::io::Error::new(std::io::ErrorKind::Other, "MinGW-w64 build failed").into());
                            }
                            Err(e) => {
                                log::error!("Failed to run MSYS2 bash: {}", e);
                                return Err(e.into());
                            }
                        }
                    } else {
                        log::error!("MSYS2 bash not found. Please ensure MSYS2 is installed and bash.exe is available.");
                        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "MSYS2 bash not found").into());
                    }
                } else {
                    log::error!("git.exe still not found after Chocolatey install and MSYS2 install failed. Please install Git for Windows manually and add it to your PATH, or install MSYS2 to build from source.");
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "git not found after Chocolatey install and MSYS2 not available").into());
                }
            }
        }
    }
    // Verify git is working
    log::info!("Verifying git installation...");
    let result = libsam::cmd_async("git --version").await;
    match result {
        Ok(_) => log::info!("git is installed and working."),
        Err(e) => {
            log::error!("git is not working: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "git not working after install").into());
        }
    }


    // Create necessary directories
    let directories = vec![
        "/opt/sam",
        "/opt/sam/bin",
        "/opt/sam/dat",
        "/opt/sam/streams",
        "/opt/sam/models",
        "/opt/sam/models/nst",
        "/opt/sam/files",
        "/opt/sam/fonts",
        "/opt/sam/games",
        "/opt/sam/scripts",
        "/opt/sam/scripts/rivescript",
        "/opt/sam/scripts/who.io",
        "/opt/sam/scripts/who.io/dataset",
        "/opt/sam/scripts/sprec",
        "/opt/sam/scripts/sprec/audio",
        "/opt/sam/scripts/sprec/noise",
        "/opt/sam/scripts/sprec/noise/_background_noise_",
        "/opt/sam/scripts/sprec/noise/other",
        "/opt/sam/tmp",
        "/opt/sam/tmp/youtube",
        "/opt/sam/tmp/youtube/downloads",
        "/opt/sam/tmp/sound",
        "/opt/sam/tmp/observations",
        "/opt/sam/tmp/observations/vwav",
    ];
    for dir in directories {
        if let Err(e) = async_fs::create_dir_all(dir).await {
            log::warn!("Failed to create directory {}: {}", dir, e);
        }
    }



    Ok(())
}


#[cfg(target_os = "linux")]
async fn pre_install() -> Result<()> {
    log::debug!("Installing system dependencies for Linux...");
    let _ = libsam::cmd_async("apt install libx264-dev libssl-dev unzip libavcodec-extra58 python3 pip git git-lfs wget libboost-dev libopencv-dev python3-opencv ffmpeg iputils-ping libasound2-dev libpulse-dev libvorbisidec-dev libvorbis-dev libopus-dev libflac-dev libsoxr-dev alsa-utils libavahi-client-dev avahi-daemon libexpat1-dev libfdk-aac-dev -y").await?;

    log::debug!("Installing Python packages for Linux...");
    let _ = libsam::cmd_async("pip3 install rivescript pexpect").await?;


    // Create necessary directories
    let directories = vec![
        "/opt/sam",
        "/opt/sam/bin",
        "/opt/sam/dat",
        "/opt/sam/streams",
        "/opt/sam/models",
        "/opt/sam/models/nst",
        "/opt/sam/files",
        "/opt/sam/fonts",
        "/opt/sam/games",
        "/opt/sam/scripts",
        "/opt/sam/scripts/rivescript",
        "/opt/sam/scripts/who.io",
        "/opt/sam/scripts/who.io/dataset",
        "/opt/sam/scripts/sprec",
        "/opt/sam/scripts/sprec/audio",
        "/opt/sam/scripts/sprec/noise",
        "/opt/sam/scripts/sprec/noise/_background_noise_",
        "/opt/sam/scripts/sprec/noise/other",
        "/opt/sam/tmp",
        "/opt/sam/tmp/youtube",
        "/opt/sam/tmp/youtube/downloads",
        "/opt/sam/tmp/sound",
        "/opt/sam/tmp/observations",
        "/opt/sam/tmp/observations/vwav",
    ];
    for dir in directories {
        if let Err(e) = async_fs::create_dir_all(dir).await {
            log::warn!("Failed to create directory {}: {}", dir, e);
        }
    }
    let _ = libsam::cmd_async("chmod -R 777 /opt/sam").await;
    let _ = libsam::cmd_async("chown 1000 -R /opt/sam").await;

    Ok(())
}


#[cfg(target_os = "macos")]
async fn pre_install() -> Result<()> {

    log::debug!("Installing system dependencies for MacOS...");
    let user = async_fs::read_to_string("/opt/sam/whoismyhuman")
        .await
        .unwrap_or_else(|_| "sam".to_string())
        .trim()
        .to_string();
    let _ = libsam::cmd_async(&format!(
        "sudo -u {user} brew install x264 openssl unzip ffmpeg python3 git git-lfs wget boost opencv ffmpeg libsndfile pulseaudio opus flac alsa-lib avahi expat fdk-aa cmake"
    )).await?;

    log::debug!("Installing Python packages for MacOS...");
    let _ = libsam::cmd_async("pip3 install rivescript pexpect --break-system-packages")
        .await?;

    // Create necessary directories
    let directories = vec![
        "/opt/sam",
        "/opt/sam/bin",
        "/opt/sam/dat",
        "/opt/sam/streams",
        "/opt/sam/models",
        "/opt/sam/models/nst",
        "/opt/sam/files",
        "/opt/sam/fonts",
        "/opt/sam/games",
        "/opt/sam/scripts",
        "/opt/sam/scripts/rivescript",
        "/opt/sam/scripts/who.io",
        "/opt/sam/scripts/who.io/dataset",
        "/opt/sam/scripts/sprec",
        "/opt/sam/scripts/sprec/audio",
        "/opt/sam/scripts/sprec/noise",
        "/opt/sam/scripts/sprec/noise/_background_noise_",
        "/opt/sam/scripts/sprec/noise/other",
        "/opt/sam/tmp",
        "/opt/sam/tmp/youtube",
        "/opt/sam/tmp/youtube/downloads",
        "/opt/sam/tmp/sound",
        "/opt/sam/tmp/observations",
        "/opt/sam/tmp/observations/vwav",
    ];
    for dir in directories {
        if let Err(e) = async_fs::create_dir_all(dir).await {
            log::warn!("Failed to create directory {}: {}", dir, e);
        }
    }


    let _ = libsam::cmd_async("chmod -R 777 /opt/sam").await;
    let _ = libsam::cmd_async("chown 1000 -R /opt/sam").await;

    Ok(())
}

// Check for GPU devices and create a marker file if found
async fn check_gpu_devices() -> Result<()> {
    let devices = get_all_devices(CL_DEVICE_TYPE_GPU);
    if devices.is_err() {
        log::info!("No GPU devices found!");
    } else {
        let _ = libsam::cmd_async("touch /opt/sam/gpu").await?;
    }
    Ok(())
}

// // Install various services and log their status
// fn install_services() {
//     // Call async darknet install separately
//     let rt = tokio::runtime::Runtime::new().unwrap();
//     rt.block_on(async {
//         match crate::sam::services::darknet::install().await {
//             Ok(_) => log::info!("darknet installed successfully"),
//             Err(e) => log::error!("Failed to install darknet: {}", e),
//         }

//         match crate::sam::services::stt::whisper::WhisperService::install().await {
//             Ok(_) => log::info!("whisper installed successfully"),
//             Err(e) => log::error!("Failed to install whisper: {}", e),
//         }
//     });

//     let services = vec![
//         // ("darknet", crate::sam::services::darknet::install as fn() -> std::result::Result<(), std::io::Error>), // REMOVE THIS LINE
//         ("sprec", crate::sam::services::sprec::install as fn() -> std::result::Result<(), std::io::Error>),
//         ("rivescript", crate::sam::services::rivescript::install as fn() -> std::result::Result<(), std::io::Error>),
//         ("who.io", crate::sam::services::who::install as fn() -> std::result::Result<(), std::io::Error>),
//         ("STT server", crate::sam::services::stt::install as fn() -> std::result::Result<(), std::io::Error>),
//         ("Media service", crate::sam::services::media::install as fn() -> std::result::Result<(), std::io::Error>),
//     ];

//     for (name, install_fn) in services {
//         match install_fn() {
//             Ok(_) => log::info!("{} installed successfully", name),
//             Err(e) => log::error!("Failed to install {}: {}", name, e),
//         }
//     }

//     // Install HTTP server in release mode
//     #[cfg(not(debug_assertions))]
//     match crate::sam::http::install() {
//         Ok(_) => log::info!("HTTP server installed successfully"),
//         Err(e) => log::error!("Failed to install HTTP server: {}", e),
//     }
// }

// Check for updates from the Sam GitHub repository using git2
pub async fn update() -> Result<()> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Open the repository at the Cargo crate root (where Cargo.toml is located)
    let repo = Repository::open(crate_root)?;
    let head = repo.head()?;
    let local_oid = head
        .target()
        .ok_or_else(|| std::io::Error::other("No HEAD found"))?;
    let local_commit = repo.find_commit(local_oid)?;
    let local_short = local_commit.id().to_string();

    // Set up callbacks for authentication (for public repo, this is usually fine)
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::credential_helper(&repo.config()?, _url, username_from_url)
    });
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // Fetch from origin
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["main"], Some(&mut fetch_options), None)?;

    // Get the latest commit on origin/main
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let remote_commit = repo.find_commit(fetch_commit.id())?;
    let remote_short = remote_commit.id().to_string();

    if local_commit.id() != remote_commit.id() {
        log::warn!(
            "A new revision is available for Sam!\nCurrent: {}\nLatest: {}",
            local_short,
            remote_short
        );
        if Confirm::new()
            .with_prompt("Would you like to update Sam using git?")
            .interact()
            .unwrap_or(false)
        {
            // Fast-forward merge
            let mut ref_heads = repo.find_reference("refs/heads/main")?;
            ref_heads.set_target(remote_commit.id(), "Fast-forward to latest origin/main")?;
            repo.set_head("refs/heads/main")?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            log::info!("Sam updated successfully. Please restart the application.");
        } else {
            log::info!("Update skipped by user.");
        }
    } else {
        log::info!("Sam is up to date. Revision: {}", local_short);
    }
    Ok(())
}

// Data structures for package information
pub type Packages = Vec<Package>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub versions: Vec<String>,
    #[serde(rename = "latest_version")]
    pub latest_version: String,
    #[serde(rename = "latest_oid")]
    pub latest_oid: String,
}

// Placeholder for uninstall functionality
pub fn uninstall() {
    // TODO: Implement uninstall logic
}






