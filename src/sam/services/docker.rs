pub fn install(){
    // Install Docker Desktop
    if !is_docker_installed() {
        log::info!("Docker is not installed. Installing...");
        install_docker();
    } else {
        log::info!("Docker is already installed.");
    }

    // Check if Docker daemon is running
    if !is_docker_daemon_running() {
        log::info!("Docker daemon is not running. Please start it.");
        start_docker_daemon();
    } else {
        log::info!("Docker daemon is running.");
    }
}

#[cfg(target_os = "macos")]
pub fn install_docker() {
    // Uses Homebrew to install Docker Desktop via cask
    std::process::Command::new("brew")
        .args(["install", "--cask", "docker"])
        .status()
        .expect("Failed to install Docker via Homebrew");
}

#[cfg(target_os = "linux")]
pub fn install_docker() {
    // Uses apt-get for Ubuntu/Debian systems
    // For production, add checks for distro and permissions
    std::process::Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://get.docker.com | sh")
        .status()
        .expect("Failed to install Docker on Linux");
}

#[cfg(target_os = "windows")]
pub fn install_docker() {
    // Uses winget to install Docker Desktop
    std::process::Command::new("powershell")
        .args(&["-Command", "winget install -e --id Docker.DockerDesktop"])
        .status()
        .expect("Failed to install Docker via winget");
}

pub fn is_docker_installed() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn is_docker_daemon_running() -> bool {
    let output = std::process::Command::new("docker")
        .arg("ps")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                !stderr.contains("Cannot connect to the Docker daemon")
            }
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
pub fn start_docker_daemon() {
    // Try to open Docker Desktop.app (starts the daemon)
    let output = std::process::Command::new("open")
        .arg("-a")
        .arg("Docker")
        .output();

    match output {
        Ok(o) if o.status.success() => log::info!("Started Docker Desktop."),
        _ => log::info!("Failed to start Docker Desktop. Please start it manually."),
    }
}

#[cfg(target_os = "linux")]
pub fn start_docker_daemon() {
    // Try to start the docker service (systemd)
    let output = std::process::Command::new("sudo")
        .args(&["systemctl", "start", "docker"])
        .output();

    match output {
        Ok(o) if o.status.success() => log::info!("Started Docker daemon."),
        _ => log::info!("Failed to start Docker daemon. Please start it manually."),
    }
}

#[cfg(target_os = "windows")]
pub fn start_docker_daemon() {
    // Try to start Docker Desktop via PowerShell
    let output = std::process::Command::new("powershell")
        .args(&[
            "-Command",
            "Start-Process -FilePath 'C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe'"
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => log::info!("Started Docker Desktop."),
        _ => log::info!("Failed to start Docker Desktop. Please start it manually."),
    }
}