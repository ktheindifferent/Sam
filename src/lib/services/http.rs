use std::fs::File;

use std::io::Write;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;


pub async fn install() -> std::io::Result<()> {

    let data = include_bytes!("../../../packages/www.zip");
    let mut pos = 0;
    let mut buffer = TokioFile::create("/opt/sam/www.zip").await?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..]).await?;
        pos += bytes_written;
    }

    let _ = crate::extract_zip_async("/opt/sam/www.zip", "/opt/sam/").await?;

    Ok(())
}
