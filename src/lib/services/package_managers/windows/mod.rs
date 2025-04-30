pub mod chocolatey;
pub mod winget;

use anyhow::Result;
use tokio::process::Command;

/// Enum for Windows package managers
#[derive(Debug, Clone, Copy)]
enum WindowsPackageManager {
    Winget,
    Chocolatey,
}

/// Detects which Windows package manager is available (winget preferred)
async fn detect_package_manager() -> Result<WindowsPackageManager> {
    // Try winget first
    let winget_available = Command::new("winget").arg("--version").output().await.is_ok();
    if winget_available {
        return Ok(WindowsPackageManager::Winget);
    }
    // Fallback to chocolatey
    let choco_path = r"C:\\ProgramData\\chocolatey\\bin\\choco.exe";
    let choco_available = tokio::fs::metadata(choco_path).await.is_ok();
    if choco_available {
        return Ok(WindowsPackageManager::Chocolatey);
    }
    Err(anyhow::anyhow!("No supported Windows package manager found (winget or chocolatey)"))
}

/// Installs a single package using the detected package manager.
pub async fn install_package(package: &str) -> Result<()> {
    match detect_package_manager().await? {
        WindowsPackageManager::Winget => winget::install_package(&convertChocoToWinget(package)).await,
        WindowsPackageManager::Chocolatey => chocolatey::install_package(package).await,
    }
}

/// Installs multiple packages using the detected package manager.
pub async fn install_packages(packages: Vec<&str>) -> Result<()> {
    match detect_package_manager().await? {
        WindowsPackageManager::Winget => {
            let winget_packages: Vec<String> = packages.iter().map(|&pkg| convertChocoToWinget(pkg)).collect();
            winget::install_packages(winget_packages.iter().map(|s| s.as_str()).collect()).await
        },
        WindowsPackageManager::Chocolatey => chocolatey::install_packages(packages).await,
    }
}

pub fn convertChocoToWinget(choco: &str) -> String {
    // Convert Chocolatey package name to Winget package name
    match choco {
        "retroarch" => "Libretro.RetroArch".to_string(),
        _ => choco.to_string(),
    }
}