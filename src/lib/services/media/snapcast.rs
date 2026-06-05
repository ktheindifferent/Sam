// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// ██     ██ ███████ ███████      ███████  ██████  ███    ██  █████  ██████
// ██     ██ ██      ██           ██      ██    ██ ████   ██ ██   ██ ██   ██
// ██  █  ██ █████   ███████      ███████ ██    ██ ██ ██  ██ ███████ ██████
// ██ ███ ██ ██           ██           ██ ██    ██ ██  ██ ██ ██   ██ ██   ██
//  ███ ███  ███████ ███████      ███████  ██████  ██   ████ ██   ██ ██   ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/**
 * Librespot Integration for Spotify Support
 *
 * This module provides installation and configuration helpers for librespot,
 * the open-source Spotify client used as a Snapcast source.
 *
 * Setup Instructions:
 * 1. Install librespot: cargo install librespot
 * 2. Copy to system path: sudo cp ~/.cargo/bin/librespot /usr/local/bin/
 * 3. Configure credentials via environment variables:
 *    - SNAPCAST_SPOTIFY_USERNAME
 *    - SNAPCAST_SPOTIFY_PASSWORD
 *    - SNAPCAST_SPOTIFY_DEVICE_NAME
 *
 * Security Notes:
 * - Credentials are read from environment variables only (never hardcoded)
 * - Config file permissions are set to 0640 (owner/group read, owner write)
 * - Default bind address is localhost for security
 */
use log::{debug, error, info, warn};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::thread;

/// Snapcast service error types
#[derive(Debug, thiserror::Error)]
pub enum SnapcastError {
    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Configuration failed: {0}")]
    ConfigurationFailed(String),

    #[error("Librespot not found: {0}")]
    LibrespotNotFound(String),

    #[error("Service start failed: {0}")]
    ServiceStartFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

static SNAPCAST_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Initialize the Snapcast server
///
/// This function:
/// 1. Checks if snapserver is installed, installs if missing
/// 2. Starts snapserver in a background thread
/// 3. Handles platform-specific quirks (e.g., Debian Bullseye)
pub async fn init() -> Result<(), SnapcastError> {
    info!("Initializing Snapcast server");

    // Check and install if needed
    if !Path::new("/usr/bin/snapserver").exists() {
        info!("snapserver not found, attempting installation");
        install().map_err(|e| SnapcastError::InstallationFailed(e.to_string()))?;
    } else {
        debug!("snapserver already installed at /usr/bin/snapserver");
    }

    // Start snapserver in background thread
    // Note: Debian Bullseye requires manual launch instead of service
    let snap_cast_thread = thread::Builder::new()
        .name("snapserver".to_string())
        .spawn(move || {
            debug!("Starting snapserver process");
            crate::tools::safe_uinx_cmd("snapserver", &[]);
        })
        .map_err(|e| SnapcastError::ServiceStartFailed(e.to_string()))?;

    SNAPCAST_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
    info!(
        "snapcast server started successfully on thread: {:?}",
        snap_cast_thread.thread().name()
    );
    Ok(())
}

/// Stop the Snapcast server
pub async fn deinit() -> Result<(), SnapcastError> {
    info!("Stopping Snapcast server");
    crate::tools::safe_uinx_cmd("pkill", &["snapserver"]);
    SNAPCAST_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Check if Snapcast is running
pub async fn is_running() -> bool {
    SNAPCAST_RUNNING.load(std::sync::atomic::Ordering::SeqCst)
}

/// Configure Snapcast server with security settings
///
/// This function:
/// 1. Reads credentials from environment variables (never hardcoded)
/// 2. Generates secure random password if not provided
/// 3. Checks for librespot availability
/// 4. Creates secure configuration with proper file permissions
/// 5. Binds to localhost by default (explicit config required for external access)
pub fn configure() -> Result<(), SnapcastError> {
    info!("Configuring Snapcast server with security settings");

    // Get credentials from environment variables only
    let (username, password_generated) = match std::env::var("SNAPCAST_USERNAME") {
        Ok(user) => (user, false),
        Err(_) => {
            warn!("SNAPCAST_USERNAME not set, using default");
            ("sam_user".to_string(), false)
        }
    };

    let password = match std::env::var("SNAPCAST_PASSWORD") {
        Ok(pass) => pass,
        Err(_) => {
            warn!("SNAPCAST_PASSWORD not set, generating random password");
            let new_pass = generate_secure_password();
            info!("Generated password: {}", &new_pass[..8]); // Log first 8 chars only
            new_pass
        }
    };

    let device_name = std::env::var("SNAPCAST_DEVICE_NAME").unwrap_or_else(|_| "Sam".to_string());

    // Security settings: bind to localhost by default
    let bind_address =
        std::env::var("SNAPCAST_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());

    if bind_address != "127.0.0.1" {
        warn!("Snapcast will bind to external address: {}", bind_address);
        warn!("Ensure firewall rules are properly configured");
    }

    // Check if librespot is available
    let (librespot_available, librespot_path) = match check_librespot() {
        Ok(path) => {
            info!("librespot found at: {}", path);
            (true, path)
        }
        Err(_) => {
            warn!("librespot not found - Spotify integration disabled");
            info!("Install with: cargo install librespot");
            (false, "/usr/local/bin/librespot".to_string())
        }
    };

    // Build Spotify source URL if librespot is available
    let spotify_source = if librespot_available && !password_generated {
        debug!("Configuring Spotify source with provided credentials");
        format!("source = librespot://{}?name={}&username={}&password={}&devicename={}&bitrate=320&normalize=true\n",
            librespot_path, device_name, username, password, device_name)
    } else if librespot_available {
        warn!("Spotify source configured but password was auto-generated");
        warn!("Set SNAPCAST_PASSWORD environment variable for consistent credentials");
        format!(
            "source = librespot://{}?name={}&devicename={}&bitrate=320&normalize=true\n",
            librespot_path, device_name, device_name
        )
    } else {
        debug!("Spotify source disabled (librespot not available)");
        format!("# Spotify source disabled\n# Install librespot: cargo install librespot\n# Then copy to: sudo cp ~/.cargo/bin/librespot /usr/local/bin/\n")
    };

    // Build secure configuration
    let cfg = format!(
        r#"[server]
threads = -1
pidfile = /var/run/snapserver/pid
user = snapserver
group = audio

[http]
enabled = true
bind_to_address = {}
port = 1780
doc_root = /usr/share/snapserver/snapweb
# For HTTPS in production, set:
# ssl_certificate = /path/to/cert.pem
# ssl_certificate_key = /path/to/key.pem

[tcp]
enabled = true
bind_to_address = {}
port = 1705

[stream]
bind_to_address = {}
port = 1704
{}source = pipe:///tmp/snapfifo?name=samfifo&mode=0666

[logging]
loglevel = info
logfile = /var/log/snapserver.log"#,
        bind_address, bind_address, bind_address, spotify_source
    );

    // Write configuration with secure permissions
    std::fs::write("/etc/snapserver.conf", &cfg)
        .map_err(|e| SnapcastError::ConfigurationFailed(e.to_string()))?;

    // Set secure file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::set_permissions(
            "/etc/snapserver.conf",
            std::fs::Permissions::from_mode(0o640),
        ) {
            Ok(_) => debug!("Config file permissions set to 0640"),
            Err(e) => warn!("Failed to set config file permissions: {}", e),
        }
    }

    info!("Snapcast configuration written successfully");
    info!("  - Bind address: {}", bind_address);
    info!(
        "  - Spotify integration: {}",
        if librespot_available {
            "enabled"
        } else {
            "disabled"
        }
    );
    info!("  - Device name: {}", device_name);

    Ok(())
}

/// Generate a secure random password
fn generate_secure_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789!@#$%^&*";
    let mut rng = rand::thread_rng();

    (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Check if librespot is installed and available
pub fn check_librespot() -> Result<String, String> {
    // Check common installation paths
    let paths = [
        "/usr/local/bin/librespot",
        "/usr/bin/librespot",
        "/bin/librespot",
        &std::env::var("LIBRESPOT_PATH").unwrap_or_default(),
    ];

    for path in &paths {
        if !path.is_empty() && Path::new(path).exists() {
            log::info!("librespot found at {}", path);
            return Ok(path.to_string());
        }
    }

    // Try to find via which command
    let output = std::process::Command::new("which")
        .arg("librespot")
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            log::info!("librespot found at {}", path);
            return Ok(path);
        }
    }

    Err("librespot not found. Install with: cargo install librespot".to_string())
}

/// Install librespot helper - provides instructions for the user
pub fn get_installation_instructions() -> &'static str {
    r#"
To enable Spotify support in Snapcast, you need to install librespot:

1. Install via Cargo (recommended):
   cargo install librespot

2. Copy to system path:
   sudo cp ~/.cargo/bin/librespot /usr/local/bin/

3. Verify installation:
   librespot --version

4. Configure credentials (optional - can also use env vars):
   export SNAPCAST_SPOTIFY_USERNAME="your_spotify_username"
   export SNAPCAST_SPOTIFY_PASSWORD="your_spotify_password"
   export SNAPCAST_SPOTIFY_DEVICE_NAME="Sam"
   export LIBRESPOT_PATH="/usr/local/bin/librespot"

5. Restart Snapcast server:
   sudo service snapserver restart

Alternatively, set the environment variables before starting Sam:
   SNAPCAST_SPOTIFY_USERNAME=... SNAPCAST_SPOTIFY_PASSWORD=... sam
"#
}

// Only one install() definition per compilation
#[cfg(not(target_os = "linux"))]
pub fn install() -> std::io::Result<()> {
    log::info!("OS not supported");
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn install() -> std::io::Result<()> {
    install_snapcast_server_arm64()
}

#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub fn install() -> std::io::Result<()> {
    install_snapcast_server_arm()
}

#[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "x86")))]
pub fn install() -> std::io::Result<()> {
    install_snapcast_server_amd64()
}

pub fn install_snapcast_server_arm64() -> std::io::Result<()> {
    let data = include_bytes!("../../../../packages/snapcast/0.26.0/arm64/bullseye/snapserver.deb");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/tmp/snapserver.deb")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    crate::tools::safe_uinx_cmd(
        "dpkg",
        &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"],
    );
    crate::tools::safe_uinx_cmd("service", &["snapserver", "start"]);
    Ok(())
}

pub fn install_snapcast_server_arm() -> std::io::Result<()> {
    let data = include_bytes!("../../../../packages/snapcast/0.26.0/snapserver_0.26.0-1_armhf.deb");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/tmp/snapserver.deb")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    crate::tools::safe_uinx_cmd(
        "dpkg",
        &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"],
    );
    crate::tools::safe_uinx_cmd("service", &["snapserver", "start"]);
    Ok(())
}

// Backup: https://github.com/badaix/snapcast/releases/download/v0.27.0/snapserver_0.27.0-1_amd64.deb
pub fn install_snapcast_server_amd64() -> std::io::Result<()> {
    let data = include_bytes!("../../../../packages/snapcast/0.27.0/snapserver_0.27.0-1_amd64.deb");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/tmp/snapserver.deb")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    crate::tools::safe_uinx_cmd(
        "dpkg",
        &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"],
    );
    crate::tools::safe_uinx_cmd("service", &["snapserver", "start"]);
    Ok(())
}
