pub mod apt;
pub mod dnf;
pub mod pacman;
pub mod yum;
pub mod zypper;
use tokio::process::Command;
use std::process::Stdio;

/// Enum representing supported Linux package managers.
#[derive(Debug, Clone, Copy)]
enum LinuxPackageManager {
    Apt,
    Dnf,
    Yum,
    Zypper,
    Pacman,
}

/// Detects the available package manager on the system.
fn detect_package_manager() -> Option<LinuxPackageManager> {
    if which::which("apt-get").is_ok() {
        Some(LinuxPackageManager::Apt)
    } else if which::which("dnf").is_ok() {
        Some(LinuxPackageManager::Dnf)
    } else if which::which("yum").is_ok() {
        Some(LinuxPackageManager::Yum)
    } else if which::which("zypper").is_ok() {
        Some(LinuxPackageManager::Zypper)
    } else if which::which("pacman").is_ok() {
        Some(LinuxPackageManager::Pacman)
    } else {
        None
    }
}

/// Asynchronously installs a single package using the detected package manager.
/// Returns Ok(()) if the package is installed successfully, otherwise returns an error.
pub async fn install_package(package: &str) -> Result<(), anyhow::Error> {
    install_packages(vec![package]).await
}

/// Asynchronously installs a batch of packages using the detected package manager.
/// Returns Ok(()) if all packages are installed successfully, otherwise returns an error.
pub async fn install_packages(packages: Vec<&str>) -> Result<(), anyhow::Error> {
    let manager = detect_package_manager()
        .ok_or_else(|| anyhow::anyhow!("No supported package manager found"))?;

    match manager {
        LinuxPackageManager::Apt => {
            return apt::install_packages(packages).await;
        },
        LinuxPackageManager::Dnf => {
            return dnf::install_packages(packages).await;
        },
        LinuxPackageManager::Yum => {
            return yum::install_packages(packages).await;
        },
        LinuxPackageManager::Zypper => {
            return zypper::install_packages(packages).await;
        },
        LinuxPackageManager::Pacman => {
            return pacman::install_packages(packages).await;
        },
    };
}

