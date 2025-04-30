use std::process::Stdio;
use tokio::process::Command;

/// Installs RetroArch using the appropriate package manager for the current OS.
pub async fn install() -> Result<(), anyhow::Error> {
    crate::services::package_managers::install_package("retroarch").await
}

// retroarch -L librustation_ng_retro.so my_game.cue
// /Applications/RetroArch.app/Contents/MacOS/RetroArch -L ~/Library/Application\ Support/RetroArch/cores/mesen_libretro.dylib ~/Downloads/zelda.nes -v
pub async fn run_retroarch_with_psx_core(game: &str) -> Result<(), anyhow::Error> {

    let bin_names = if cfg!(target_os = "macos") {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.dylib"]
    } else if cfg!(target_os = "windows") {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.dll"]
    } else {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.so"]
    };

    let mut cmd = Command::new("retroarch");
    cmd.arg("-L").arg(bin_names[1]).arg(game);
    let status = cmd.status().await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to run RetroArch with libretro core"));
    }
    Ok(())
}