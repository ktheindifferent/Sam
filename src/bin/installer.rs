// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.



// Required dependencies
use serde::{Serialize, Deserialize};
use error_chain::error_chain;
use opencl3::device::{get_all_devices, CL_DEVICE_TYPE_GPU};
use tokio::fs as async_fs;

use std::env;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use git2::{Repository, FetchOptions, Cred, RemoteCallbacks};
use dialoguer::Confirm;


// Define error handling
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
        Git(git2::Error);
    }
}


#[cfg(target_os = "windows")]
const OS: &str = "windows";
#[cfg(target_os = "macos")]
const OS: &str = "macos";
#[cfg(target_os = "linux")]
const OS: &str = "linux";

#[tokio::main]
async fn main() {

    // Initialize logger with color, warning level, and timestamps
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Info)
        .with_timestamps(true)
        .init()
        .unwrap();

    // Store the current username in the SAM_USER environment variable
    let whoami = whoami::username();
    let opt_sam_path = Path::new("/opt/sam/");
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
        ]).unwrap();
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




    log::info!("Checking for GPU devices...");
    // await check_gpu_devices().await;
    log::info!("Compiling snapcast...");
    libsam::services::snapcast::install().await;
    log::info!("Installing darknet...");
    libsam::services::darknet::install().await;
    log::info!("Installing SPREC...");
    libsam::services::sprec::install().await;
}
//     // install_services();
//     log::info!("Setting up default settings...");
//     // crate::sam::http::api::settings::set_defaults();
//     log::info!("Configuring Snapcast...");
//     // crate::sam::services::media::snapcast::configure();
//     log::info!("Installation complete!");


// }



// Pre-installation setup: Install required packages and create directories
async fn pre_install() -> Result<()> {
    match OS {
        "windows" => {
            log::debug!("Installing system dependencies for Windows...");
            let _ = libsam::cmd_async("choco install ffmpeg git git-lfs boost opencv python3").await?;

            log::debug!("Installing Python packages for Windows...");
            let _ = libsam::cmd_async("pip3 install rivescript pexpect").await?;
        },
        "linux" => {
            log::debug!("Installing system dependencies for Linux...");
            let _ = libsam::cmd_async("apt install libx264-dev libssl-dev unzip libavcodec-extra58 python3 pip git git-lfs wget libboost-dev libopencv-dev python3-opencv ffmpeg iputils-ping libasound2-dev libpulse-dev libvorbisidec-dev libvorbis-dev libopus-dev libflac-dev libsoxr-dev alsa-utils libavahi-client-dev avahi-daemon libexpat1-dev libfdk-aac-dev -y").await?;
        
            log::debug!("Installing Python packages for Linux...");
            let _ = libsam::cmd_async("pip3 install rivescript pexpect").await?;
        },
        "macos" => {
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
            let _ = libsam::cmd_async("pip3 install rivescript pexpect --break-system-packages").await?;
        },
        &_ => {
            log::error!("Unsupported OS: {}", OS);
            return Err(io::Error::other("Unsupported OS").into());
        }
    }


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
        if let Err(e) = async_fs::create_dir_all(dir).await {
            log::warn!("Failed to create directory {}: {}", dir, e);
        }
    }

    // Set permissions for Linux and MacOS
    match OS {
        "windows" => {},
        _ => {
            let _ = libsam::cmd_async("chmod -R 777 /opt/sam").await;
            let _ = libsam::cmd_async("chown 1000 -R /opt/sam").await;
        }
    }

    Ok(())
}

// Check for GPU devices and create a marker file if found
fn check_gpu_devices() {
    let devices = get_all_devices(CL_DEVICE_TYPE_GPU);
    if devices.is_err() {
        log::info!("No GPU devices found!");
    } else {
        let _ = libsam::cmd_async("touch /opt/sam/gpu");
    }
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
    let local_oid = head.target().ok_or_else(|| std::io::Error::other("No HEAD found"))?;
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
        log::warn!("A new revision is available for Sam!\nCurrent: {}\nLatest: {}", local_short, remote_short);
        if Confirm::new().with_prompt("Would you like to update Sam using git?").interact().unwrap_or(false) {
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
