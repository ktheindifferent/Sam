pub mod ps1;
pub mod retroarch;

pub async fn install() -> Result<(), anyhow::Error> {
    retroarch::install().await?;
    ps1::install().await?;
    Ok(())
}