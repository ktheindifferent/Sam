use std::process::Command;
use log::{info, error};

/// Install and start Redis using Docker if not already running.
/// This is intended to be called from setup/install.
pub async fn install() {
    info!("Checking for running Redis Docker container...");
    if is_running() {
        info!("Redis Docker container 'sam-redis' is already running.");
        return;
    }
    info!("Pulling Redis Docker image...");
    let pull = Command::new("docker")
        .args(&["pull", "redis:7"])
        .status();

    match pull {
        Ok(status) if status.success() => info!("Redis Docker image pulled successfully."),
        Ok(status) => {
            error!("Failed to pull Redis image, exit code: {}", status);
            return;
        }
        Err(e) => {
            error!("Failed to pull Redis image: {}", e);
            return;
        }
    }

    start().await;
}

/// Start the Redis Docker container (if not running)
pub async fn start() {
    if is_running() {
        info!("Redis Docker container 'sam-redis' is already running.");
        return;
    }
    info!("Starting Redis Docker container...");
    let run = Command::new("docker")
        .args(&[
            "run", "-d",
            "--name", "sam-redis",
            "-p", "6379:6379",
            "--restart", "unless-stopped",
            "redis:7",
        ])
        .status();

    match run {
        Ok(status) if status.success() => info!("Redis Docker container started as 'sam-redis'."),
        Ok(status) => error!("Failed to start Redis container, exit code: {}", status),
        Err(e) => error!("Failed to start Redis container: {}", e),
    }
}

/// Stop the Redis Docker container (if running)
pub async fn stop() {
    if !is_running() {
        info!("Redis Docker container 'sam-redis' is not running.");
        return;
    }
    info!("Stopping Redis Docker container...");
    let stop = Command::new("docker")
        .args(&["stop", "sam-redis"])
        .status();

    match stop {
        Ok(status) if status.success() => info!("Redis Docker container stopped."),
        Ok(status) => error!("Failed to stop Redis container, exit code: {}", status),
        Err(e) => error!("Failed to stop Redis container: {}", e),
    }
    // Optionally remove the container after stopping
    let rm = Command::new("docker")
        .args(&["rm", "sam-redis"])
        .status();
    match rm {
        Ok(status) if status.success() => info!("Redis Docker container removed."),
        Ok(_) => {} // ignore errors if already removed
        Err(_) => {}
    }
}

/// Return the status of the Redis Docker container: "running", "stopped", or "not installed"
pub fn status() -> &'static str {
    if is_running() {
        "running"
    } else if is_installed() {
        "stopped"
    } else {
        "not installed"
    }
}

/// Helper: check if the Redis Docker container is running
fn is_running() -> bool {
    let check = Command::new("docker")
        .args(&["ps", "--filter", "name=sam-redis", "--format", "{{.Names}}"])
        .output();
    match check {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains("sam-redis"),
        Err(_) => false,
    }
}

/// Helper: check if the Redis Docker container exists (installed)
fn is_installed() -> bool {
    let check = Command::new("docker")
        .args(&["ps", "-a", "--filter", "name=sam-redis", "--format", "{{.Names}}"])
        .output();
    match check {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains("sam-redis"),
        Err(_) => false,
    }
}
