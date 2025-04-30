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
    cmd.arg("apt-get").arg("install").arg("-y");
    for pkg in &packages {
        cmd.arg(pkg);
    }
    let status = cmd.status().await?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("apt-get install failed with status: {}", status))
    }
}

