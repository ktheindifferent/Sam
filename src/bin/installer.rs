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

    configure_opencl_and_clang_paths()?;

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
    log::info!("Starting Windows pre-installation steps...");

    // 1. Ensure Chocolatey is installed and available
    ensure_chocolatey_installed().await?;
    // 2. Install required system packages via Chocolatey
    // install_choco_packages();
    let choco_packages = ["ffmpeg", "git-lfs", "opencv", "python3", "make", "unzip", "curl"];
    libsam::services::chocolatey::install_packages(&choco_packages).await?;
    // 3. Ensure vcpkg is installed and bootstrapped & install deps
    let vcpkg_deps = ["libflac", "libogg", "libvorbis", "opus", "soxr", "boost", "curl"];
    libsam::services::vcpkg::install_packages(&vcpkg_deps, "x64-windows").await?;
    // 4. Refresh environment variables
    refresh_env_vars();
    // 5. Ensure Python is installed and available in PATH
    ensure_python();
    // 6. Install required Python packages
    install_python_packages();
    // 7. Ensure git is installed and available in PATH
    ensure_git_installed().await?;
    // 8. Create all required /opt/sam directories
    create_opt_sam_directories().await;

    Ok(())
}

/// Installs Chocolatey and verifies its presence.
async fn ensure_chocolatey_installed() -> Result<()> {
    let _ = libsam::services::chocolatey::install().await?;
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
    log::info!("Verifying Chocolatey installation...");
    if !std::path::Path::new(choco_path).exists() {
        log::error!("Chocolatey is still not available after attempted install. Please ensure C:\\ProgramData\\chocolatey\\bin is in your PATH and choco.exe exists.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Chocolatey not found after install").into());
    } else {
        log::info!("Chocolatey found at {}", choco_path);
    }
    Ok(())
}

/// Refreshes environment variables so newly installed tools are available.
fn refresh_env_vars() {
    log::info!("Refreshing environment variables with refreshenv...");
    // let result = libsam::run_and_log("refreshenv", &[]);
    let refreshenv_path = "C:\\ProgramData\\chocolatey\\bin\\refreshenv.cmd";
    let result = libsam::run_and_log(refreshenv_path, &[]);
    match result {
        Ok(_) => log::info!("Environment variables refreshed."),
        Err(e) => log::warn!("Failed to refresh environment variables: {}", e),
    }
}

fn ensure_python() {
    // Check if Python is installed and available in PATH
    let python_path = "C:\\ProgramData\\chocolatey\\bin\\python3.13.exe";
    if !std::path::Path::new(python_path).exists() {
        log::error!("Python not found at {}. Please install Python 3.13 or later.", python_path);
        return;
    } else {
        log::info!("Python found at {}", python_path);
    }
}

/// Installs required Python packages using pip.
fn install_python_packages() {
    // let python_path = "C:\\ProgramData\\chocolatey\\bin\\python3.13.exe";
    let result = libsam::run_and_log("python", &["-m", "ensurepip", "--upgrade"]);
    match result {
        Ok(_) => log::info!("pip upgraded successfully."),
        Err(e) => log::error!("Failed to upgrade pip: {}", e),
    }
    refresh_env_vars();
    // let pip_path = "C:\\ProgramData\\chocolatey\\bin\\pip3.13.exe";
    let pip_args = ["install", "rivescript", "pexpect"];
    log::info!("Running: {} {}", "pip3", pip_args.join(" "));
    let result = libsam::run_and_log("pip3", &pip_args);
    match result {
        Ok(_) => log::info!("Python package installation succeeded."),
        Err(e) => log::error!("Python package installation failed: {}", e),
    }
}

/// Ensures git is installed and available in PATH, using Chocolatey or MSYS2 as fallback.
async fn ensure_git_installed() -> Result<()> {
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
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
            log::error!("git.exe not found after Chocolatey install. Please install Git for Windows manually and add it to your PATH.");
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "git not found after Chocolatey install").into());
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
    Ok(())
}

/// Creates all required /opt/sam directories.
async fn create_opt_sam_directories() {
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
// async fn check_gpu_devices() -> Result<()> {
//     let devices = get_all_devices(CL_DEVICE_TYPE_GPU);
//     if devices.is_err() {
//         log::info!("No GPU devices found!");
//     } else {
//         let _ = libsam::cmd_async("touch /opt/sam/gpu").await?;
//     }
//     Ok(())
// }

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
//         ("darknet", crate::sam::services::darknet::install as fn() -> std::result::Result<(), std::io::Error>), // REMOVE THIS LINE
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


#[cfg(target_os = "windows")]
pub fn configure_opencl_and_clang_paths() -> Result<()> {
    use std::env;
    use std::io::{self, Write};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn prompt_for_path(lib_name: &str) -> Option<PathBuf> {
        let mut input = String::new();
        loop {
            print!("Could not find {lib_name}. Please enter the full path to {lib_name} (or leave blank to skip): ");
            io::stdout().flush().ok();
            input.clear();
            if io::stdin().read_line(&mut input).is_err() {
                return None;
            }
            let trimmed = input.trim();
            if trimmed.is_empty() {
                return None;
            }
            let path = PathBuf::from(trimmed);
            if path.exists() && path.file_name().map_or(false, |f| f.eq_ignore_ascii_case(lib_name)) {
                return Some(path);
            } else {
                println!("Invalid path or file name. Please try again.");
            }
        }
    }

    fn get_arch() -> &'static str {
        if cfg!(target_pointer_width = "64") { "x64" } else { "x86" }
    }

    // Search for opencl.lib
    let search_paths = [
        r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8\lib\x64",
        r"C:\Program Files\LLVM\bin",
    ];
    let arch = get_arch();
    let mut opencl_lib: Option<PathBuf> = None;
    for base in &search_paths {
        let path = Path::new(base);
        if path.exists() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Some(name) = p.file_name() {
                        if name.eq_ignore_ascii_case("opencl.lib") && p.exists() {
                            opencl_lib = Some(p);
                            break;
                        }
                    }
                }
            }
        }
        if opencl_lib.is_some() { break; }
    }
    if opencl_lib.is_none() {
        opencl_lib = prompt_for_path("opencl.lib");
    }
    if let Some(lib_path) = &opencl_lib {
        if let Some(parent) = lib_path.parent() {
            env::set_var("LIB", parent);
            println!("LIB environment variable set to {}", parent.display());
        }
    } else {
        println!("opencl.lib not found and not provided. LIB will not be set.");
    }

    // Search for libclang.dll
    let mut clang_dll: Option<PathBuf> = None;
    for base in &search_paths {
        let path = Path::new(base);
        if path.exists() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Some(name) = p.file_name() {
                        if name.eq_ignore_ascii_case("libclang.dll") && p.exists() {
                            clang_dll = Some(p);
                            break;
                        }
                    }
                }
            }
        }
        if clang_dll.is_some() { break; }
    }
    if clang_dll.is_none() {
        clang_dll = prompt_for_path("libclang.dll");
    }
    if let Some(dll_path) = &clang_dll {
        if let Some(parent) = dll_path.parent() {
            env::set_var("LIBCLANG_PATH", parent);
            println!("LIBCLANG_PATH environment variable set to {}", parent.display());
        }
    } else {
        println!("libclang.dll not found and not provided. LIBCLANG_PATH will not be set.");
    }

    // Use opencl3 to verify OpenCL is available
    #[link(name = "opencl")]
    #[link(name = "clang")]
    use opencl3::platform::get_platforms;
    match get_platforms() {
        Ok(platforms) if !platforms.is_empty() => {
            println!("OpenCL platforms found: {}", platforms.len());
            for (i, p) in platforms.iter().enumerate() {
                println!("Platform {}: {}", i, p.name().unwrap_or_default());
            }
        }
        Ok(_) => {
            println!("No OpenCL platforms found. Check your LIB path and OpenCL installation.");
        }
        Err(e) => {
            println!("Error querying OpenCL platforms: {e}");
        }
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






