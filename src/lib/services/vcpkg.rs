use std::path::PathBuf;
use log;

/// Adds vcpkg to the system PATH for the current process.
pub async fn set_path() -> Result<(), anyhow::Error> {
    let vcpkg_bin = "C:\\vcpkg";
    log::info!("Adding vcpkg to PATH: {}", vcpkg_bin);
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = std::env::split_paths(&paths).collect::<Vec<_>>();
    new_path.push(PathBuf::from(vcpkg_bin));
    let joined = std::env::join_paths(new_path).unwrap();
    std::env::set_var("PATH", &joined);
    Ok(())
}

/// Ensures vcpkg is installed at C:\vcpkg. Clones and bootstraps if missing.
pub async fn ensure_installed() -> Result<(), anyhow::Error> {
    set_path().await?;
    let vcpkg_exe = "C:\\vcpkg\\vcpkg.exe";
    let vcpkg_exists = tokio::fs::metadata(vcpkg_exe).await.is_ok();
    if !vcpkg_exists {
        log::warn!("vcpkg not found, cloning and bootstrapping...");
        let _ = crate::run_and_log_async("git", &["clone", "https://github.com/microsoft/vcpkg.git", "C:/vcpkg"]).await;
        let bootstrap = "C:\\vcpkg\\bootstrap-vcpkg.bat";
        let _ = crate::run_and_log_async(bootstrap, &[]).await;
    } else {
        log::info!("vcpkg already installed at {}", vcpkg_exe);
    }
    Ok(())
}

/// Installs a list of vcpkg packages for the specified triplet (e.g., x64-windows).
pub async fn install_packages(packages: &[&str], triplet: &str) -> Result<(), anyhow::Error> {
    ensure_installed().await?;
    let vcpkg_exe = "C:\\vcpkg\\vcpkg.exe";
    for pkg in packages {
        let arg = format!("{}:{}", pkg, triplet);
        log::info!("Starting vcpkg install for {}...", arg);
        let args = ["install", &arg];
        let result = crate::run_and_log_async(vcpkg_exe, &args).await;
        match result {
            Ok(_) => log::info!("Successfully installed {} via vcpkg.", arg),
            Err(e) => log::error!("Failed to install {} via vcpkg: {}", arg, e),
        }
    }
    Ok(())
}
