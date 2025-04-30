use tokio::process::Command;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::path::Path;

/// Asynchronously installs a single package using MacPorts.
/// Returns Ok(()) if the package is installed successfully, otherwise returns an error.
pub async fn install_package(package: &str) -> Result<(), anyhow::Error> {
    install_packages(vec![package]).await
}

/// Asynchronously installs a batch of packages using MacPorts.
/// Returns Ok(()) if all packages are installed successfully, otherwise returns an error.
pub async fn install_packages(packages: Vec<&str>) -> Result<(), anyhow::Error> {
    if packages.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new("sudo");
    cmd.arg("port").arg("install");
    for pkg in &packages {
        cmd.arg(pkg);
    }
    let status = cmd.status().await?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(format!("port install failed with status: {}", status)))
    }
}

/// Asynchronously installs MacPorts on macOS.
/// Returns Ok(()) if MacPorts is installed successfully, otherwise returns an error.
pub async fn install() -> Result<(), anyhow::Error> {

    // Check if port is already installed
    if which::which("port").is_ok() {
        return Ok(());
    }

    // Download the MacPorts installer pkg
    let url = "https://github.com/macports/macports-base/releases/latest/download/MacPorts-2.8.1-14-Ventura.pkg";
    let pkg_path = "/tmp/MacPorts-latest.pkg";

    // Download the pkg file using curl
    let status = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(pkg_path)
        .arg(url)
        .status()
        .await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to download MacPorts installer"));
    }

    // Install the pkg using the installer command
    let status = Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(pkg_path)
        .arg("-target")
        .arg("/")
        .status()
        .await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to install MacPorts"));
    }

    // Optionally, remove the pkg file
    let _ = tokio::fs::remove_file(pkg_path).await;

    // Check if port is now installed
    if which::which("port").is_ok() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("MacPorts installation did not complete successfully"))
    }
}