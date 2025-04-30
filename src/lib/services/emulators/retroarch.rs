use std::process::Stdio;
use tokio::process::Command;

/// Installs RetroArch using the appropriate package manager for the current OS.
pub async fn install() -> Result<(), anyhow::Error> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        // Homebrew
        let mut c = Command::new("sudo");
        c.arg("-u").arg(&crate::get_human().await).arg("brew").arg("install").arg("retroarch");
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        // Try winget, fallback to choco
        if Command::new("winget").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("winget");
            c.args(["install", "Libretro.RetroArch", "-e", "--accept-package-agreements", "--accept-source-agreements"]);
            c
        } else {
            let mut c = Command::new("choco");
            c.args(["install", "retroarch", "-y"]);
            c
        }
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        // Try apt, dnf, yum, pacman, zypper
        if Command::new("apt").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("sudo");
            c.args(["apt", "update"]);
            c.status().await.ok(); // update first
            let mut c = Command::new("sudo");
            c.args(["apt", "install", "-y", "retroarch"]);
            c
        } else if Command::new("dnf").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("sudo");
            c.args(["dnf", "install", "-y", "retroarch"]);
            c
        } else if Command::new("yum").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("sudo");
            c.args(["yum", "install", "-y", "retroarch"]);
            c
        } else if Command::new("pacman").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("sudo");
            c.args(["pacman", "-Sy", "retroarch", "--noconfirm"]);
            c
        } else if Command::new("zypper").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok() {
            let mut c = Command::new("sudo");
            c.args(["zypper", "install", "-y", "retroarch"]);
            c
        } else {
            return Err(anyhow::anyhow!("No supported package manager found for RetroArch installation."));
        }
    };

    let status = cmd.status().await.map_err(|e| anyhow::anyhow!("Failed to run install command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("RetroArch install command failed with status: {status}"))
    }
}
