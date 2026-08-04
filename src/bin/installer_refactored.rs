// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs as async_fs;

use dialoguer::Confirm;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(feature = "opencl")]
use opencl3::device::get_all_devices;
#[cfg(feature = "opencl")]
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

/// Main entry point for the installer
#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging();
    setup_environment();

    log::info!("Starting preinstallation...");
    pre_install().await?;

    post_install_setup().await?;
    install_services().await?;
    build_and_deploy_binary().await?;

    log::info!("Installation complete!");
    Ok(())
}

/// Initialize the logger with appropriate settings
fn initialize_logging() {
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Info)
        .with_timestamps(true)
        .init()
        .unwrap();
}

/// Setup environment variables based on the current user and OS
fn setup_environment() {
    let whoami = whoami::username();
    if whoami != "root" {
        env::set_var("SAM_USER", &whoami);
    }

    // Ensure required environment variables are available for sudo context
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
}

/// Platform-specific pre-installation steps
#[cfg(target_os = "windows")]
async fn pre_install() -> Result<()> {
    log::info!("Starting Windows pre-installation steps...");

    // 1. Ensure Chocolatey is installed and available
    ensure_chocolatey_installed().await?;

    // 2. Install required system packages via Chocolatey
    install_chocolatey_packages().await?;

    // 3. Ensure vcpkg is installed and bootstrapped & install deps
    install_vcpkg_dependencies().await?;

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

#[cfg(target_os = "linux")]
async fn pre_install() -> Result<()> {
    log::debug!("Installing system dependencies for Linux...");

    // Install system packages
    install_linux_packages().await?;

    // Install Python packages
    install_python_packages_linux().await?;

    // Create directories and set permissions
    create_opt_sam_directories().await;
    set_linux_permissions().await?;

    Ok(())
}

#[cfg(target_os = "macos")]
async fn pre_install() -> Result<()> {
    // Install package managers
    install_package_managers().await?;

    log::debug!("Installing system dependencies for MacOS...");

    // Install system packages
    install_macos_packages().await?;

    // Install Python packages
    install_python_packages_macos().await?;

    // Create directories and set permissions
    create_opt_sam_directories().await;
    set_macos_permissions().await?;

    Ok(())
}

/// Post-installation setup tasks
async fn post_install_setup() -> Result<()> {
    let user = env::var("SAM_USER").unwrap_or_else(|_| whoami::username());
    libsam::print_banner(user.clone());

    // Create whoismyhuman file if needed
    create_whoismyhuman_file(&user).await?;

    // Check for GPU devices
    check_gpu_devices().await?;

    Ok(())
}

/// Create the whoismyhuman file to identify the user
async fn create_whoismyhuman_file(user: &str) -> Result<()> {
    if user != "root" {
        let opt_sam_path = Path::new("/opt/sam/");
        let file_path = opt_sam_path.join("whoismyhuman");

        if opt_sam_path.exists() && opt_sam_path.is_dir() {
            if let Err(e) = fs::write(&file_path, user) {
                log::error!("Failed to write whoismyhuman: {}", e);
            }
        }
    }
    Ok(())
}

/// Install all services required by Sam
async fn install_services() -> Result<()> {
    log::info!("Installing services...");

    // Install core services
    libsam::services::snapcast::install().await?;
    libsam::services::darknet::install(None).await?;
    libsam::services::stt::install(None).await?;
    libsam::services::http::install().await?;
    libsam::services::emulators::install().await?;

    // Install Who.io with error handling
    match libsam::services::who::install() {
        Ok(_) => log::info!("Who.io installed successfully"),
        Err(e) => log::error!("Failed to install Who.io: {}", e),
    }

    Ok(())
}

/// Build the Sam binary and deploy it to the appropriate location
async fn build_and_deploy_binary() -> Result<()> {
    log::info!("Building Sam in release mode...");

    let build_status = Command::new("cargo")
        .args(["build", "--bin", "sam", "--release"])
        .status();

    match build_status {
        Ok(status) if status.success() => {
            deploy_binary().await?;
            update_path_if_needed();
        }
        Ok(status) => {
            log::error!("Sam build failed with status: {}", status);
        }
        Err(e) => {
            log::error!("Failed to run cargo build: {}", e);
        }
    }

    Ok(())
}

/// Deploy the built binary to the target directory
async fn deploy_binary() -> Result<()> {
    let target_dir = Path::new("target/release");
    let binary_name = if cfg!(windows) { "sam.exe" } else { "sam" };
    let src_bin = target_dir.join(binary_name);
    let dest_bin = Path::new("/opt/sam/bin").join(binary_name);

    if let Err(e) = fs::create_dir_all("/opt/sam/bin") {
        log::error!("Failed to create /opt/sam/bin: {}", e);
    }

    match fs::copy(&src_bin, &dest_bin) {
        Ok(_) => log::info!("Moved binary to {}", dest_bin.display()),
        Err(e) => log::error!("Failed to move binary: {}", e),
    }

    Ok(())
}

/// Update PATH environment variable if /opt/sam/bin is not already included
fn update_path_if_needed() {
    let path_var = env::var("PATH").unwrap_or_default();
    if !path_var.split(':').any(|p| p == "/opt/sam/bin") {
        let new_path = format!("/opt/sam/bin:{}", path_var);
        env::set_var("PATH", &new_path);
        log::info!("/opt/sam/bin added to PATH for this session.");
        println!("To make this change permanent, add 'export PATH=/opt/sam/bin:$PATH' to your shell profile.");
    }
}

// ==================== WINDOWS SPECIFIC FUNCTIONS ====================

#[cfg(target_os = "windows")]
async fn ensure_chocolatey_installed() -> Result<()> {
    let _ = libsam::services::package_managers::windows::chocolatey::install().await?;
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";

    log::info!("Verifying Chocolatey installation...");
    if !std::path::Path::new(choco_path).exists() {
        log::error!("Chocolatey is still not available after attempted install. Please ensure C:\\ProgramData\\chocolatey\\bin is in your PATH and choco.exe exists.");
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Chocolatey not found after install",
        )
        .into());
    } else {
        log::info!("Chocolatey found at {}", choco_path);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn install_chocolatey_packages() -> Result<()> {
    let choco_packages = vec![
        "ffmpeg", "git-lfs", "opencv", "python3", "make", "unzip", "curl",
    ];
    libsam::services::package_managers::windows::chocolatey::install_packages(choco_packages)
        .await?;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn install_vcpkg_dependencies() -> Result<()> {
    let vcpkg_deps = [
        "libflac",
        "libogg",
        "libvorbis",
        "opus",
        "soxr",
        "boost",
        "curl",
    ];
    libsam::services::vcpkg::install_packages(&vcpkg_deps, "x64-windows").await?;
    Ok(())
}

/// Refreshes environment variables so newly installed tools are available.
#[cfg(target_os = "windows")]
fn refresh_env_vars() {
    log::info!("Refreshing environment variables with refreshenv...");
    let refreshenv_path = "C:\\ProgramData\\chocolatey\\bin\\refreshenv.cmd";
    let result = libsam::run_and_log(refreshenv_path, &[]);
    match result {
        Ok(_) => log::info!("Environment variables refreshed."),
        Err(e) => log::warn!("Failed to refresh environment variables: {}", e),
    }
}

#[cfg(target_os = "windows")]
fn ensure_python() {
    // Check if Python is installed and available in PATH
    let python_path = "C:\\ProgramData\\chocolatey\\bin\\python3.13.exe";
    if !std::path::Path::new(python_path).exists() {
        log::error!(
            "Python not found at {}. Please install Python 3.13 or later.",
            python_path
        );
        return;
    } else {
        log::info!("Python found at {}", python_path);
    }
}

/// Installs required Python packages using pip.
#[cfg(target_os = "windows")]
fn install_python_packages() {
    // Upgrade pip
    let result = libsam::run_and_log("python", &["-m", "ensurepip", "--upgrade"]);
    match result {
        Ok(_) => log::info!("pip upgraded successfully."),
        Err(e) => log::error!("Failed to upgrade pip: {}", e),
    }

    refresh_env_vars();

    // Install required packages
    let pip_args = ["install", "rivescript", "pexpect"];
    log::info!("Running: {} {}", "pip3", pip_args.join(" "));
    let result = libsam::run_and_log("pip3", &pip_args);
    match result {
        Ok(_) => log::info!("Python package installation succeeded."),
        Err(e) => log::error!("Python package installation failed: {}", e),
    }
}

/// Ensures git is installed and available in PATH, using Chocolatey or MSYS2 as fallback.
#[cfg(target_os = "windows")]
async fn ensure_git_installed() -> Result<()> {
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";

    // Try to find existing git installation
    if try_find_existing_git().await {
        return verify_git_installation().await;
    }

    // Install git via Chocolatey if not found
    log::warn!("git.exe not found. Installing Git for Windows using Chocolatey...");
    let result = libsam::run_and_log(choco_path, &["install", "git", "-y"]);
    match result {
        Ok(_) => log::info!("Chocolatey git installation succeeded."),
        Err(e) => {
            log::error!("Chocolatey git installation failed: {}", e);
            return Err(e.into());
        }
    }

    // Try to find git again after installation
    if !try_find_existing_git().await {
        log::error!("git.exe not found after Chocolatey install. Please install Git for Windows manually and add it to your PATH.");
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "git not found after Chocolatey install",
        )
        .into());
    }

    verify_git_installation().await
}

#[cfg(target_os = "windows")]
async fn try_find_existing_git() -> bool {
    let search_dirs = ["C:\\Program Files", "C:\\Program Files (x86)"];

    for base in &search_dirs {
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
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(target_os = "windows")]
async fn verify_git_installation() -> Result<()> {
    log::info!("Verifying git installation...");
    let result = libsam::cmd_async("git --version").await;
    match result {
        Ok(_) => log::info!("git is installed and working."),
        Err(e) => {
            log::error!("git is not working: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "git not working after install",
            )
            .into());
        }
    }
    Ok(())
}

// ==================== LINUX SPECIFIC FUNCTIONS ====================

#[cfg(target_os = "linux")]
async fn install_linux_packages() -> Result<()> {
    let packages = vec![
        "libx264-dev",
        "libssl-dev",
        "unzip",
        "libavcodec-extra58",
        "python3",
        "pip",
        "git",
        "git-lfs",
        "wget",
        "libboost-dev",
        "libopencv-dev",
        "python3-opencv",
        "ffmpeg",
        "iputils-ping",
        "libasound2-dev",
        "libpulse-dev",
        "libvorbisidec-dev",
        "libvorbis-dev",
        "libopus-dev",
        "libflac-dev",
        "libsoxr-dev",
        "alsa-utils",
        "libavahi-client-dev",
        "avahi-daemon",
        "libexpat1-dev",
        "libfdk-aac-dev",
    ];
    libsam::services::package_managers::linux::install_packages(packages).await?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn install_python_packages_linux() -> Result<()> {
    let _ = libsam::cmd_async("pip3 install rivescript pexpect").await?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn set_linux_permissions() -> Result<()> {
    let _ = libsam::cmd_async("chmod -R 777 /opt/sam").await?;
    let _ = libsam::cmd_async("chown 1000 -R /opt/sam").await?;
    Ok(())
}

// ==================== MACOS SPECIFIC FUNCTIONS ====================

#[cfg(target_os = "macos")]
async fn install_package_managers() -> Result<()> {
    libsam::services::package_managers::osx::brew::install().await?;
    libsam::services::package_managers::osx::macports::install().await?;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn install_macos_packages() -> Result<()> {
    let packages = vec![
        "x264",
        "openssl",
        "unzip",
        "ffmpeg",
        "python3",
        "git",
        "git-lfs",
        "wget",
        "boost",
        "opencv",
        "libsndfile",
        "pulseaudio",
        "opus",
        "flac",
        "alsa-lib",
        "avahi",
        "expat",
        "fdk-aa",
        "cmake",
    ];
    libsam::services::package_managers::osx::install_packages(packages).await?;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn install_python_packages_macos() -> Result<()> {
    let _ = libsam::cmd_async("pip3 install rivescript pexpect --break-system-packages").await?;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn set_macos_permissions() -> Result<()> {
    let _ = libsam::cmd_async("chmod -R 777 /opt/sam").await?;
    let _ = libsam::cmd_async("chown 1000 -R /opt/sam").await?;
    Ok(())
}

// ==================== SHARED FUNCTIONS ====================

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

// Check for GPU devices and create a marker file if found
async fn check_gpu_devices() -> Result<()> {
    #[cfg(not(feature = "opencl"))]
    {
        log::info!("OpenCL support is not enabled; skipping GPU detection.");
        return Ok(());
    }

    #[cfg(feature = "opencl")]
    {
        let devices = get_all_devices(CL_DEVICE_TYPE_GPU);
        if devices.is_err() {
            log::info!("No GPU devices found!");
        } else {
            let _ = libsam::cmd_async("touch /opt/sam/gpu").await?;
        }
        Ok(())
    }
}

// Check for updates from the Sam GitHub repository using git2
pub async fn update() -> Result<()> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Open the repository at the Cargo crate root (where Cargo.toml is located)
    let repo = Repository::open(crate_root)?;

    // Get local commit info first
    let local_short = {
        let head = repo.head()?;
        let local_oid = head
            .target()
            .ok_or_else(|| std::io::Error::other("No HEAD found"))?;
        let local_commit = repo.find_commit(local_oid)?;
        local_commit.id().to_string()
    };

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

    if fetch_commit.id() != repo.head()?.target().unwrap_or_else(|| git2::Oid::zero()) {
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
            // Re-open repo for mutable operations
            let mut repo = Repository::open(crate_root)?;
            perform_update(&mut repo, remote_commit).await?;
        } else {
            log::info!("Update skipped by user.");
        }
    } else {
        log::info!("Sam is up to date. Revision: {}", local_short);
    }
    Ok(())
}

async fn perform_update(repo: &mut Repository, remote_commit: git2::Commit<'_>) -> Result<()> {
    // Fast-forward merge
    let mut ref_heads = repo.find_reference("refs/heads/main")?;
    ref_heads.set_target(remote_commit.id(), "Fast-forward to latest origin/main")?;
    repo.set_head("refs/heads/main")?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    log::info!("Sam updated successfully. Please restart the application.");
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn configure_opencl_and_clang_paths() -> Result<()> {
    use std::env;
    use std::fs;
    use std::io::{self, Write};
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
            if path.exists()
                && path
                    .file_name()
                    .map_or(false, |f| f.eq_ignore_ascii_case(lib_name))
            {
                return Some(path);
            } else {
                println!("Invalid path or file name. Please try again.");
            }
        }
    }

    fn get_arch() -> &'static str {
        if cfg!(target_pointer_width = "64") {
            "x64"
        } else {
            "x86"
        }
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
        if opencl_lib.is_some() {
            break;
        }
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
        if clang_dll.is_some() {
            break;
        }
    }
    if clang_dll.is_none() {
        clang_dll = prompt_for_path("libclang.dll");
    }
    if let Some(dll_path) = &clang_dll {
        if let Some(parent) = dll_path.parent() {
            env::set_var("LIBCLANG_PATH", parent);
            println!(
                "LIBCLANG_PATH environment variable set to {}",
                parent.display()
            );
        }
    } else {
        println!("libclang.dll not found and not provided. LIBCLANG_PATH will not be set.");
    }

    #[cfg(feature = "opencl")]
    {
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
    }

    #[cfg(not(feature = "opencl"))]
    println!("OpenCL verification skipped; build installer with --features opencl to enable it.");

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
