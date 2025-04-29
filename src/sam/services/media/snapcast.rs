// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// TODO: Install librespot in the root bin folder
// cargo install librespot
// cp $HOME/.cargo/bin/librespot /bin/librespot
// cp $HOME/.cargo/bin/librespot /usr/bin/librespot

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
            crate::sam::tools::uinx_cmd("snapserver");
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

// TODO - Automatically apply security settings and config
// /etc/snapserver.conf
pub fn configure() {
    let cfg = "[server]
threads = -1
pidfile = /var/run/snapserver/pid

[http]
enabled = true
bind_to_address = 0.0.0.0
port = 1780
doc_root = /usr/share/snapserver/snapweb

[tcp]
enabled = true
bind_to_address = 0.0.0.0
port = 1705

[stream]
bind_to_address = 0.0.0.0
port = 1704
source = librespot:///bin/librespot?name=Sam&username=calebsmithdev&password=Nofear1234&devicename=Sam&bitrate=320&nomalize=true
source = pipe:///tmp/snapfifo?name=samfifo
[logging]".to_string();
    log::info!("cfg: {:?}", cfg);
    std::fs::write("/etc/snapserver.conf", &cfg).expect("Unable to write file");
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

    crate::sam::tools::uinx_cmd("dpkg --force-all -i /opt/sam/tmp/snapserver.deb");
    crate::sam::tools::uinx_cmd("service snapserver start");
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

    crate::sam::tools::uinx_cmd("dpkg --force-all -i /opt/sam/tmp/snapserver.deb");
    crate::sam::tools::uinx_cmd("service snapserver start");
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

    crate::sam::tools::uinx_cmd("dpkg --force-all -i /opt/sam/tmp/snapserver.deb");
    crate::sam::tools::uinx_cmd("service snapserver start");
    Ok(())
}
