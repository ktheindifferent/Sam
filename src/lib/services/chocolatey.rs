use std::env;
use std::path::PathBuf;
use tokio::fs;
use log;





pub async fn set_path() -> Result<(), anyhow::Error> {
    let choco_bin = "C:\\ProgramData\\chocolatey\\bin";
    log::info!("Adding Chocolatey bin to PATH: {}", choco_bin);
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = std::env::split_paths(&paths).collect::<Vec<_>>();
    new_path.push(std::path::PathBuf::from(choco_bin));
    let joined = std::env::join_paths(new_path).unwrap();
    std::env::set_var("PATH", &joined);
    Ok(())
}

pub async fn install() -> Result<(), anyhow::Error> {
    let _ = set_path().await?;
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
    log::info!("Checking for Chocolatey at {}", choco_path);
    let choco_exists = tokio::fs::metadata(choco_path).await.is_ok();
    if !choco_exists {
        log::warn!("Chocolatey not found, attempting installation...");
        log::info!("Running Chocolatey install script via PowerShell...");
        let result = crate::run_and_log_async(
            "powershell",
            &[
                "-NoProfile",
                "-InputFormat",
                "None",
                "-ExecutionPolicy",
                "Bypass",
                "-Scope",
                "Process",
                "-Command",
                "[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))"
            ]
        ).await;
        match result {
            Ok(_) => log::info!("Chocolatey install script completed."),
            Err(e) => log::error!("Chocolatey install script failed: {}", e),
        }
        // After install, add to PATH again in case it was just created
        set_path().await?;
    }
    Ok(())
}
pub async fn verify() -> Result<(), anyhow::Error> {
    let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
    log::info!("Verifying Chocolatey installation...");
    if tokio::fs::metadata(choco_path).await.is_err() {
        log::error!("Chocolatey is still not available after attempted install. Please ensure C:\\ProgramData\\chocolatey\\bin is in your PATH and choco.exe exists.");
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Chocolatey not found after install").into());
    } else {
        log::info!("Chocolatey found at {}", choco_path);
    }
    Ok(())
}


    pub async fn install_packages(packages: Vec<&str>) -> Result<(), anyhow::Error> {
        let choco_path = "C:\\ProgramData\\chocolatey\\bin\\choco.exe";
        log::info!("Installing packages with Chocolatey: {:?}", packages);

        if packages.is_empty() {
            log::warn!("No packages specified for installation.");
            return Ok(());
        }

        let mut choco_args = vec!["install"];
        choco_args.extend(packages.iter().copied());
        choco_args.push("-y");

        log::info!("Running: {} {}", choco_path, choco_args.join(" "));
        let result = crate::run_and_log_async(choco_path, &choco_args).await;
        match result {
            Ok(_) => log::info!("Chocolatey package installation succeeded."),
            Err(e) => log::error!("Chocolatey package installation failed: {}", e),
        }
        Ok(())
    }