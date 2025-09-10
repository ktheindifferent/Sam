use log::{error, info, warn};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context};
use super::environment::get_env_config;
use crate::monitoring::report_service_error;

/// Install Docker if not present and ensure daemon is running.
pub async fn install() {
    let env_config = get_env_config();
    
    // Skip Docker management in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - Docker management disabled");
        return;
    }
    
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
    let env_config = get_env_config();
    
    // Skip Docker management in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - Docker management disabled");
        return;
    }
    
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("open").arg("-a").arg("Docker").output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker Desktop."),
            _ => {
                let err = anyhow::anyhow!("Failed to start Docker Desktop. Please start it manually.");
                error!("{}", err);
                report_service_error("docker", &err, None);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("sudo")
            .args(&["systemctl", "start", "docker"])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker daemon."),
            _ => {
                let err = anyhow::anyhow!("Failed to start Docker daemon. Please start it manually.");
                error!("{}", err);
                report_service_error("docker", &err, None);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(&[
                "-Command",
                "Start-Process -FilePath 'C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe'",
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => info!("Started Docker Desktop."),
            _ => {
                let err = anyhow::anyhow!("Failed to start Docker Desktop. Please start it manually.");
                error!("{}", err);
                report_service_error("docker", &err, None);
            }
        }
    }
}

/// Stop the Docker daemon/service.
pub async fn stop() {
    let env_config = get_env_config();
    
    // Skip Docker management in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - Docker management disabled");
        return;
    }
    
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args(["-e", "quit app \"Docker\""])
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
            .args(&["-Command", "Stop-Process -Name 'Docker Desktop' -Force"])
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

// Async versions for orchestrator compatibility
pub async fn is_running_async() -> Result<bool> {
    tokio::task::spawn_blocking(is_running)
        .await
        .context("Failed to check Docker status")
}

pub async fn ensure_running() -> Result<()> {
    let env_config = get_env_config();
    
    // Skip Docker check in CapRover mode
    if env_config.is_caprover {
        info!("Running in CapRover mode - skipping Docker check");
        return Ok(());
    }
    
    if !is_running() {
        start().await;
        // Wait for Docker to fully start
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if is_running() {
                info!("Docker daemon is now running");
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Docker failed to start within 30 seconds"))
    } else {
        Ok(())
    }
}

// Container management helpers for orchestrator
pub async fn start_postgres() -> Result<()> {
    let env_config = get_env_config();
    
    // Skip in CapRover mode - use external PostgreSQL
    if env_config.is_caprover {
        info!("Running in CapRover mode - using external PostgreSQL");
        return Ok(());
    }
    
    let output = tokio::process::Command::new("docker")
        .args([
            "run", "-d",
            "--name", "sam-postgres",
            "-e", "POSTGRES_PASSWORD=sampassword",
            "-e", "POSTGRES_DB=sam",
            "-p", "5432:5432",
            "--restart", "unless-stopped",
            "postgres:14-alpine"
        ])
        .output()
        .await
        .context("Failed to start PostgreSQL container")?;
    
    if !output.status.success() {
        // Check if container already exists
        let check = tokio::process::Command::new("docker")
            .args(["start", "sam-postgres"])
            .output()
            .await;
        
        if check.is_ok() && check.unwrap().status.success() {
            info!("Started existing PostgreSQL container");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to start PostgreSQL: {}", 
                String::from_utf8_lossy(&output.stderr)))
        }
    } else {
        info!("Created and started new PostgreSQL container");
        Ok(())
    }
}

pub async fn stop_postgres() -> Result<()> {
    let output = tokio::process::Command::new("docker")
        .args(["stop", "sam-postgres"])
        .output()
        .await
        .context("Failed to stop PostgreSQL container")?;
    
    if output.status.success() {
        info!("Stopped PostgreSQL container");
        Ok(())
    } else {
        warn!("PostgreSQL container may not be running");
        Ok(())
    }
}

pub async fn start_redis() -> Result<()> {
    let env_config = get_env_config();
    
    // Skip in CapRover mode - use external Redis
    if env_config.is_caprover {
        info!("Running in CapRover mode - using external Redis");
        return Ok(());
    }
    
    let output = tokio::process::Command::new("docker")
        .args([
            "run", "-d",
            "--name", "sam-redis",
            "-p", "6379:6379",
            "--restart", "unless-stopped",
            "redis:7-alpine"
        ])
        .output()
        .await
        .context("Failed to start Redis container")?;
    
    if !output.status.success() {
        // Check if container already exists
        let check = tokio::process::Command::new("docker")
            .args(["start", "sam-redis"])
            .output()
            .await;
        
        if check.is_ok() && check.unwrap().status.success() {
            info!("Started existing Redis container");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to start Redis: {}", 
                String::from_utf8_lossy(&output.stderr)))
        }
    } else {
        info!("Created and started new Redis container");
        Ok(())
    }
}

pub async fn stop_redis() -> Result<()> {
    let output = tokio::process::Command::new("docker")
        .args(["stop", "sam-redis"])
        .output()
        .await
        .context("Failed to stop Redis container")?;
    
    if output.status.success() {
        info!("Stopped Redis container");
        Ok(())
    } else {
        warn!("Redis container may not be running");
        Ok(())
    }
}

// Clean up all SAM containers
pub async fn cleanup_containers() -> Result<()> {
    let containers = vec!["sam-postgres", "sam-redis"];
    
    for container in containers {
        let _ = tokio::process::Command::new("docker")
            .args(["rm", "-f", container])
            .output()
            .await;
    }
    
    info!("Cleaned up SAM Docker containers");
    Ok(())
}
