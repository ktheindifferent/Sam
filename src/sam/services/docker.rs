use std::process::{Command, Stdio};
use log::{info, error};
use std::time::Duration;
use std::thread;

/// Install Docker if not present and ensure daemon is running.
pub async fn install() {
    if !is_installed() {
        info!("Docker is not installed. Installing...");
        install_docker();
    } else {
        info!("Docker is already installed.");
    }

    if !is_running() {
        info!("Docker daemon is not running. Attempting to start...");
        start().await;
    } else {
        info!("Docker daemon is running.");
    }
}

/// Start the Docker daemon/service.
pub async fn start() {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("open")
            .arg("-a")
            .arg("Docker")
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker Desktop."),
            _ => error!("Failed to start Docker Desktop. Please start it manually."),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("sudo")
            .args(&["systemctl", "start", "docker"])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker daemon."),
            _ => error!("Failed to start Docker daemon. Please start it manually."),
        }
    }
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(&[
                "-Command",
                "Start-Process -FilePath 'C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe'"
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker Desktop."),
            _ => error!("Failed to start Docker Desktop. Please start it manually."),
        }
    }
}

/// Stop the Docker daemon/service.
pub async fn stop() {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args(&["-e", "quit app \"Docker\""])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Stopped Docker Desktop."),
            _ => error!("Failed to stop Docker Desktop. Please stop it manually."),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("sudo")
            .args(&["systemctl", "stop", "docker"])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Stopped Docker daemon."),
            _ => error!("Failed to stop Docker daemon. Please stop it manually."),
        }
    }
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(&[
                "-Command",
                "Stop-Process -Name 'Docker Desktop' -Force"
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Stopped Docker Desktop."),
            _ => error!("Failed to stop Docker Desktop. Please stop it manually."),
        }
    }
}

/// Return the status of the Docker daemon: "running", "stopped", or "not installed"
pub fn status() -> &'static str {
    if is_running() {
        "running"
    } else if is_installed() {
        "stopped"
    } else {
        "not installed"
    }
}

/// Check if Docker is installed
pub fn is_installed() -> bool {
    let mut child = match Command::new("docker")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(2) {
                    let _ = child.kill();
                    log::warn!("Timeout waiting for 'docker --version' (is Docker installed?)");
                    return false;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

/// Check if Docker daemon is running
pub fn is_running() -> bool {
    let mut child = match Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(2) {
                    let _ = child.kill();
                    log::warn!("Timeout waiting for 'docker info' (is Docker running?)");
                    return false;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

// Platform-specific install logic
#[cfg(target_os = "macos")]
fn install_docker() {
    Command::new("brew")
        .args(["install", "--cask", "docker"])
        .status()
        .expect("Failed to install Docker via Homebrew");
}

#[cfg(target_os = "linux")]
fn install_docker() {
    Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://get.docker.com | sh")
        .status()
        .expect("Failed to install Docker on Linux");
}

#[cfg(target_os = "windows")]
fn install_docker() {
    Command::new("powershell")
        .args(&["-Command", "winget install -e --id Docker.DockerDesktop"])
        .status()
        .expect("Failed to install Docker via winget");
}