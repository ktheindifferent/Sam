use log;

pub async fn set_path() -> Result<(), anyhow::Error> {
    // Winget is usually in PATH by default on modern Windows, but we can check
    log::info!("Checking if winget is in PATH");
    // No-op for now, but could add logic if needed
    Ok(())
}

pub async fn install() -> Result<(), anyhow::Error> {
    // Winget is included by default on Windows 10 1709+ and Windows 11
    log::info!("Checking for winget installation");
    let winget_path = r"C:\\Program Files\\WindowsApps\\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\\winget.exe";
    let exists = tokio::fs::metadata(winget_path).await.is_ok() || crate::run_and_log_async("winget", &["--version"]).await.is_ok();
    if exists {
        log::info!("winget is already installed.");
        return Ok(());
    }

    log::info!("winget is not installed. Attempting to install App Installer from Microsoft Store...");

    // Try to launch the Microsoft Store to the App Installer page
    let store_uri = "ms-windows-store://pdp/?productid=9NBLGGH4NNS1";
    let result = crate::run_and_log_async("start", &["", store_uri]).await;

    match result {
        Ok(_) => {
            log::info!("Launched Microsoft Store to App Installer page. Please complete the installation manually.");
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "winget not installed. Please install App Installer from the Microsoft Store.",
            )
            .into())
        }
        Err(e) => {
            log::error!("Failed to launch Microsoft Store: {}", e);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to launch Microsoft Store for App Installer.",
            )
            .into())
        }
    }
}

pub async fn install_package(package: &str) -> Result<(), anyhow::Error> {
    log::info!("Installing winget package: {}", package);
    let args = ["install", package, "--accept-source-agreements", "--accept-package-agreements", "--silent"];
    let result = crate::run_and_log_async("winget", &args).await;
    match result {
        Ok(_) => log::info!("winget package installed: {}", package),
        Err(e) => {
            log::error!("Failed to install winget package {}: {}", package, e);
            return Err(e.into());
        }
    }
    Ok(())
}

pub async fn install_packages(packages: Vec<&str>) -> Result<(), anyhow::Error> {
    log::info!("Installing winget packages: {:?}", packages);
    for pkg in packages {
        let args = ["install", pkg, "--accept-source-agreements", "--accept-package-agreements", "--silent"];
        let result = crate::run_and_log_async("winget", &args).await;
        match result {
            Ok(_) => log::info!("winget package installed: {}", pkg),
            Err(e) => log::error!("Failed to install winget package {}: {}", pkg, e),
        }
    }
    Ok(())
}

pub async fn verify() -> Result<(), anyhow::Error> {
    log::info!("Verifying winget installation...");
    let result = crate::run_and_log_async("winget", &["--version"]).await;
    match result {
        Ok(_) => log::info!("winget is available."),
        Err(e) => {
            log::error!("winget is not available: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "winget not found").into());
        }
    }
    Ok(())
}
