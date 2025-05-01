pub mod ps1;
pub mod retroarch;

pub async fn install() -> Result<(), anyhow::Error> {
    log::info!("Installing emulators...");
    // Install emulators
    log::info!("Installing RetroArch emulator...");
    retroarch::install().await?;
    log::info!("Installing PS1 emulator...");
    ps1::install().await?;
    Ok(())
}