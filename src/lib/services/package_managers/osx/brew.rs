use tokio::process::Command;

/// Asynchronously installs a single package using apt-get.
/// Returns Ok(()) if the package is installed successfully, otherwise returns an error.
pub async fn install_package(package: &str) -> Result<(), anyhow::Error> {
    install_packages(vec![package]).await
}

/// Asynchronously installs a batch of packages using apt-get.
/// Returns Ok(()) if all packages are installed successfully, otherwise returns an error.
pub async fn install_packages(packages: Vec<&str>) -> Result<(), anyhow::Error> {
    if packages.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new("sudo");
    cmd.arg("-u").arg(&crate::get_human().await).arg("brew").arg("install");
    for pkg in &packages {
        cmd.arg(pkg);
    }
    let status = cmd.status().await?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(format!("brew install failed with status: {}", status)))
    }
}

/// Asynchronously installs Homebrew if it is not already installed.
/// Returns Ok(()) if Homebrew is installed or was installed successfully, otherwise returns an error.
pub async fn install() -> Result<(), anyhow::Error> {

    // Check if brew is already installed
    let status = Command::new("sudo")
        .arg("-u")
        .arg(&crate::get_human().await)
        .arg("brew")
        .arg("--version")
        .status()
        .await;

    if let Ok(status) = status {
        if status.success() {
            return Ok(());
        }
    }

    // Install Homebrew using the official installation script
    let install_cmd = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#;
    let status = Command::new("sh")
        .arg("-c")
        .arg(install_cmd)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(format!("Homebrew install failed with status: {}", status)))
    }
}

