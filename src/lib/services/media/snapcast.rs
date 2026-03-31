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
 */

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::thread;

pub fn init() {
    // Attempt to re-install snapserver if it doesn't already exist
    if !Path::new("/usr/bin/snapserver").exists() {
        match install() {
            Ok(_) => (),
            Err(e) => {
                log::error!("snapserver install failed: {}", e);
            }
        }
    }

    // Snapserver sevice doesn't work for debian bullsye so we need to launch manually.
    // Attempt to launch snapserver in new thread.....will fail if port are already in use by snapserver
    let snap_cast_thread = thread::Builder::new()
        .name("snapserver".to_string())
        .spawn(move || {
            crate::tools::safe_uinx_cmd("snapserver", &[]);
        });

    match snap_cast_thread {
        Ok(_) => {
            log::info!("snapcast server started successfully");
        }
        Err(e) => {
            log::error!("failed to initialize snapcast server: {}", e);
        }
    }
}

/// Configure Snapcast server with security settings
pub fn configure() {
    // Get credentials from environment or secure storage
    let username = std::env::var("SNAPCAST_USERNAME").unwrap_or_else(|_| {
        log::warn!("SNAPCAST_USERNAME not set, using default");
        "sam_user".to_string()
    });
    
    let password = std::env::var("SNAPCAST_PASSWORD").unwrap_or_else(|_| {
        log::warn!("SNAPCAST_PASSWORD not set, generating random password");
        generate_secure_password()
    });
    
    let device_name = std::env::var("SNAPCAST_DEVICE_NAME")
        .unwrap_or_else(|_| "Sam".to_string());
    
    // Security settings: bind to localhost by default, require explicit config for external
    let bind_address = std::env::var("SNAPCAST_BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    
    // Check if librespot is available
    let librespot_path = std::env::var("LIBRESPOT_PATH")
        .unwrap_or_else(|_| "/usr/local/bin/librespot".to_string());
    
    let librespot_available = Path::new(&librespot_path).exists();
    
    // Build Spotify source URL if librespot is available
    let spotify_source = if librespot_available {
        format!("source = librespot://{}?name={}&username={}&password={}&devicename={}&bitrate=320&normalize=true\n",
            librespot_path, device_name, username, password, device_name)
    } else {
        format!("# Spotify source disabled (librespot not found at {})\n# Install with: cargo install librespot\n# Then set LIBRESPOT_PATH environment variable\n", librespot_path)
    };
    
    // Build secure configuration
    let cfg = format!(r#"[server]
threads = -1
pidfile = /var/run/snapserver/pid
user = snapserver
group = audio

[http]
enabled = true
bind_to_address = {}
port = 1780
doc_root = /usr/share/snapserver/snapweb
# Enable HTTPS for production
ssl_certificate = 
ssl_certificate_key = 

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
        bind_address,
        bind_address,
        bind_address,
        spotify_source
    );
    
    log::info!("Applying security configuration for Snapcast server");
    
    // Write configuration with secure permissions
    match std::fs::write("/etc/snapserver.conf", &cfg) {
        Ok(_) => {
            // Set secure file permissions (readable only by root and snapserver user)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o640);
                if let Err(e) = std::fs::set_permissions("/etc/snapserver.conf", permissions) {
                    log::error!("Failed to set secure permissions on config file: {}", e);
                }
            }
            log::info!("Snapcast configuration written successfully with security settings");
        }
        Err(e) => {
            log::error!("Failed to write Snapcast configuration: {}", e);
        }
    }
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

    crate::tools::safe_uinx_cmd("dpkg", &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"]);
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

    crate::tools::safe_uinx_cmd("dpkg", &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"]);
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

    crate::tools::safe_uinx_cmd("dpkg", &["--force-all", "-i", "/opt/sam/tmp/snapserver.deb"]);
    crate::tools::safe_uinx_cmd("service", &["snapserver", "start"]);
    Ok(())
}
