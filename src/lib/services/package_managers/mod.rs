pub mod linux;
pub mod osx;
pub mod windows;
pub mod pip;

use anyhow::Result;

#[cfg(target_os = "windows")]
pub async fn install_package(package: &str) -> Result<()> {
    windows::install_package(package).await
}

#[cfg(target_os = "windows")]
pub async fn install_packages(packages: Vec<&str>) -> Result<()> {
    windows::install_packages(packages).await
}

#[cfg(target_os = "macos")]
pub async fn install_package(package: &str) -> Result<()> {
    osx::install_package(package).await
}

#[cfg(target_os = "macos")]
pub async fn install_packages(packages: Vec<&str>) -> Result<()> {
    osx::install_packages(packages).await
}

#[cfg(target_os = "linux")]
pub async fn install_package(package: &str) -> Result<()> {
    linux::install_package(package).await
}

#[cfg(target_os = "linux")]
pub async fn install_packages(packages: Vec<&str>) -> Result<()> {
    linux::install_packages(packages).await
}