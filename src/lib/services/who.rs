use tokio::fs::File;
use tokio::io::{AsyncWriteExt, Result};

/// Writes the provided data to the specified file path asynchronously.
async fn write_file(data: &[u8], path: &str) -> Result<()> {
    let mut pos = 0;
    let mut buffer = File::create(path).await?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..]).await?;
        pos += bytes_written;
    }
    buffer.flush().await?;
    Ok(())
}

/// Installs necessary scripts and datasets for the "who" service asynchronously.
pub async fn install() -> Result<()> {
    // Write the Python script
    write_file(
        include_bytes!("../../../scripts/who.io/who2.py"),
        "/opt/sam/scripts/who.io/who2.py",
    )
    .await?;

    // Write the trained KNN model
    write_file(
        include_bytes!("../../../scripts/who.io/trained_knn_model.clf"),
        "/opt/sam/scripts/who.io/trained_knn_model.clf",
    )
    .await?;

    // Write and extract Barack Obama dataset
    let obama_zip = "/opt/sam/scripts/who.io/dataset/barack_obama.zip";
    write_file(
        include_bytes!("../../../scripts/who.io/dataset/barack_obama.zip"),
        obama_zip,
    )
    .await?;
    crate::extract_zip_async(obama_zip, "/opt/sam/scripts/who.io/dataset/").await?; // If extract_zip is async, add .await
    let _ = crate::cmd_async(&format!("rm -rf {obama_zip}")).await?;

    // Write and extract Donald Trump dataset
    let trump_zip = "/opt/sam/scripts/who.io/dataset/donald_trump.zip";
    write_file(
        include_bytes!("../../../scripts/who.io/dataset/donald_trump.zip"),
        trump_zip,
    )
    .await?;
    crate::extract_zip_async(trump_zip, "/opt/sam/scripts/who.io/dataset/").await?; // If extract_zip is async, add .await
    let _ = crate::cmd_async(&format!("rm -rf {trump_zip}")).await?;

    Ok(())
}
