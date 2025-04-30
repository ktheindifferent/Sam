pub mod brew;
pub mod macports;

use anyhow::Result;

/// Detects the available package manager on macOS (brew or macports).
async fn detect_package_manager() -> Result<PackageManager> {
    if which::which("brew").is_ok() {
        return Ok(PackageManager::Brew);
    }
    if which::which("port").is_ok() {
        return Ok(PackageManager::MacPorts);
    }
    Err(anyhow::anyhow!("No supported package manager found (brew or macports)"))
}

enum PackageManager {
    Brew,
    MacPorts,
}

/// Installs a single package using the detected package manager.
pub async fn install_package(package: &str) -> Result<()> {
    match detect_package_manager().await? {
        PackageManager::Brew => brew::install_package(package).await,
        PackageManager::MacPorts => macports::install_package(package).await,
    }
}

/// Installs multiple packages using the detected package manager.
pub async fn install_packages(packages: Vec<&str>) -> Result<()> {
    match detect_package_manager().await? {
        PackageManager::Brew => brew::install_packages(packages).await,
        PackageManager::MacPorts => macports::install_packages(packages).await,
    }
}
