use std::process::Command;
use log::{info, error};
use std::time::Duration;
use std::thread;

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
        .output();

    match pull {
        Ok(status) if status.status.success() => info!("Redis Docker image pulled successfully."),
        Ok(status) => {
            error!("Failed to pull Redis image, exit code: {:?}", status);
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
        .output(); // changed from .status() to .output()

    match run {
        Ok(output) if output.status.success() => {
            info!("Redis Docker container started as 'sam-redis'.");
            // Optionally log container id: String::from_utf8_lossy(&output.stdout)
        }
        Ok(output) => error!(
            "Failed to start Redis container, exit code: {}. Stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ),
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
        .output();

    match stop {
        Ok(status) if status.status.success() => info!("Redis Docker container stopped."),
        Ok(status) => error!("Failed to stop Redis container, exit code: {}", status.status),
        Err(e) => error!("Failed to stop Redis container: {}", e),
    }
    // Optionally remove the container after stopping
    let rm = Command::new("docker")
        .args(&["rm", "sam-redis"])
        .output();
    match rm {
        Ok(status) if status.status.success() => info!("Redis Docker container removed."),
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
pub fn is_running() -> bool {
    let mut check = Command::new("docker")
        .args(&["ps", "--filter", "name=sam-redis", "--format", "{{.Names}}"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();

    match check {
        Ok(mut child) => {
            let pid = child.id();
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() {
                            let mut output = String::new();
                            if let Some(mut out) = child.stdout.take() {
                                use std::io::Read;
                                let _ = out.read_to_string(&mut output);
                            }
                            return output.contains("sam-redis");
                        }
                        return false;
                    }
                    Ok(None) => {
                        if start.elapsed() > Duration::from_secs(2) {
                            let _ = child.kill();
                            log::warn!("Timeout waiting for 'docker ps' (is Docker running?)");
                            return false;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => return false,
                }
            }
        }
        Err(_) => false,
    }
}

/// Helper: check if the Redis Docker container exists (installed)
fn is_installed() -> bool {
    let mut check = Command::new("docker")
        .args(&["ps", "-a", "--filter", "name=sam-redis", "--format", "{{.Names}}"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();

    match check {
        Ok(mut child) => {
            let pid = child.id();
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() {
                            let mut output = String::new();
                            if let Some(mut out) = child.stdout.take() {
                                use std::io::Read;
                                let _ = out.read_to_string(&mut output);
                            }
                            return output.contains("sam-redis");
                        }
                        return false;
                    }
                    Ok(None) => {
                        if start.elapsed() > Duration::from_secs(2) {
                            let _ = child.kill();
                            log::warn!("Timeout waiting for 'docker ps -a' (is Docker running?)");
                            return false;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => return false,
                }
            }
        }
        Err(_) => false,
    }
}
