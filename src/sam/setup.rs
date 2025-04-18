// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

// Required dependencies
use serde::{Serialize, Deserialize};
use error_chain::error_chain;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};

// Define error handling
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
    }
}

// Main installation function
pub async fn install() {
    // Pre-installation setup
    pre_install();

    // Check for GPU devices
    check_gpu_devices();

    // Install various services
    install_services();

    // Initialize default settings
    crate::sam::http::api::settings::set_defaults();

    // Configure Snapcast
    crate::sam::services::media::snapcast::configure();
}

// Pre-installation setup: Install required packages and create directories
fn pre_install() {
    // Install system dependencies
    // Install system dependencies based on OS
    #[cfg(target_os = "linux")]
    crate::sam::tools::uinx_cmd("apt install libx264-dev libssl-dev unzip libavcodec-extra58 python3 pip git git-lfs wget libboost-dev libopencv-dev python3-opencv ffmpeg iputils-ping libasound2-dev libpulse-dev libvorbisidec-dev libvorbis-dev libopus-dev libflac-dev libsoxr-dev alsa-utils libavahi-client-dev avahi-daemon libexpat1-dev libfdk-aac-dev -y");

    #[cfg(target_os = "macos")]
    crate::sam::tools::uinx_cmd("brew install x264 openssl unzip ffmpeg python3 git git-lfs wget boost opencv ffmpeg libsndfile pulseaudio opus flac soxr alsa-lib avahi expat fdk-aac");
    crate::sam::tools::uinx_cmd("pip3 install rivescript pexpect");

    // Create necessary directories
    let directories = vec![
        "/opt/sam", "/opt/sam/bin", "/opt/sam/dat", "/opt/sam/streams", "/opt/sam/models",
        "/opt/sam/models/nst", "/opt/sam/files", "/opt/sam/fonts", "/opt/sam/games",
        "/opt/sam/scripts", "/opt/sam/scripts/rivescript", "/opt/sam/scripts/who.io",
        "/opt/sam/scripts/who.io/dataset", "/opt/sam/scripts/sprec", "/opt/sam/scripts/sprec/audio",
        "/opt/sam/scripts/sprec/noise", "/opt/sam/scripts/sprec/noise/_background_noise_",
        "/opt/sam/scripts/sprec/noise/other", "/opt/sam/tmp", "/opt/sam/tmp/youtube",
        "/opt/sam/tmp/youtube/downloads", "/opt/sam/tmp/sound", "/opt/sam/tmp/observations",
        "/opt/sam/tmp/observations/vwav",
    ];
    for dir in directories {
        crate::sam::tools::uinx_cmd(&format!("mkdir -p {}", dir));
    }

    // Set permissions
    crate::sam::tools::uinx_cmd("chmod -R 777 /opt/sam");
    crate::sam::tools::uinx_cmd("chown 1000 -R /opt/sam");
}

// Check for GPU devices and create a marker file if found
fn check_gpu_devices() {
    let devices = get_all_devices(CL_DEVICE_TYPE_GPU);
    if devices.is_err() {
        log::info!("No GPU devices found!");
    } else {
        crate::sam::tools::uinx_cmd("touch /opt/sam/gpu");
    }
}

// Install various services and log their status
fn install_services() {
    let services = vec![
        ("darknet", crate::sam::services::darknet::install as fn() -> std::result::Result<(), std::io::Error>),
        ("sprec", crate::sam::services::sprec::install as fn() -> std::result::Result<(), std::io::Error>),
        ("rivescript", crate::sam::services::rivescript::install as fn() -> std::result::Result<(), std::io::Error>),
        ("who.io", crate::sam::services::who::install as fn() -> std::result::Result<(), std::io::Error>),
        ("STT server", crate::sam::services::stt::install as fn() -> std::result::Result<(), std::io::Error>),
        ("Media service", crate::sam::services::media::install as fn() -> std::result::Result<(), std::io::Error>),
    ];

    for (name, install_fn) in services {
        match install_fn() {
            Ok(_) => log::info!("{} installed successfully", name),
            Err(e) => log::error!("Failed to install {}: {}", name, e),
        }
    }

    // Install HTTP server in release mode
    #[cfg(not(debug_assertions))]
    match crate::sam::http::install() {
        Ok(_) => log::info!("HTTP server installed successfully"),
        Err(e) => log::error!("Failed to install HTTP server: {}", e),
    }
}

// Check for updates from the Open Sam Foundation API
pub async fn update() -> Result<()> {
    let request = reqwest::Client::new()
        .get("https://osf.opensam.foundation/api/packages")
        .send()
        .await?;
    let packages = request.json::<Packages>().await?;

    for package in packages {
        if package.latest_version != crate::VERSION.ok_or("0.0.0")? && package.name == "sam" {
            log::warn!("UPDATE_CHECK: S.A.M. needs an update");
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
