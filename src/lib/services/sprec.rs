use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn install() -> crate::Result<()> {
    const FILES: &[(&str, &str, &[u8])] = &[
        (
            "../../../scripts/sprec/build.py",
            "/opt/sam/scripts/sprec/build.py",
            include_bytes!("../../../scripts/sprec/build.py"),
        ),
        (
            "../../../scripts/sprec/predict.py",
            "/opt/sam/scripts/sprec/predict.py",
            include_bytes!("../../../scripts/sprec/predict.py"),
        ),
        (
            "../../../scripts/sprec/requirements.txt",
            "/opt/sam/scripts/sprec/requirements.txt",
            include_bytes!("../../../scripts/sprec/requirements.txt"),
        ),
        (
            "../../../scripts/sprec/model.h5",
            "/opt/sam/scripts/sprec/model.h5",
            include_bytes!("../../../scripts/sprec/model.h5"),
        ),
        (
            "../../../scripts/sprec/labels.pickle",
            "/opt/sam/scripts/sprec/labels.pickle",
            include_bytes!("../../../scripts/sprec/labels.pickle"),
        ),
        (
            "../../../scripts/sprec/audio/Unknown.zip",
            "/opt/sam/scripts/sprec/audio/Unknown.zip",
            include_bytes!("../../../scripts/sprec/audio/Unknown.zip"),
        ),
        (
            "../../../scripts/sprec/noise/other.zip",
            "/opt/sam/scripts/sprec/noise/other.zip",
            include_bytes!("../../../scripts/sprec/noise/other.zip"),
        ),
        (
            "../../../scripts/sprec/noise/_background_noise_.zip",
            "/opt/sam/scripts/sprec/noise/_background_noise_.zip",
            include_bytes!("../../../scripts/sprec/noise/_background_noise_.zip"),
        ),
    ];

    for &(_, destination, data) in FILES {
        let mut file = File::create(destination).await?;
        file.write_all(data).await?;
    }

    let _ = crate::extract_zip_async(
        "/opt/sam/scripts/sprec/audio/Unknown.zip",
        "/opt/sam/scripts/sprec/audio/",
    )
    .await;
    let _ = crate::extract_zip_async(
        "/opt/sam/scripts/sprec/noise/other.zip",
        "/opt/sam/scripts/sprec/noise/",
    )
    .await;
    let _ = crate::extract_zip_async(
        "/opt/sam/scripts/sprec/noise/_background_noise_.zip",
        "/opt/sam/scripts/sprec/noise/",
    )
    .await;

    fs::remove_file("/opt/sam/scripts/sprec/audio/Unknown.zip").await?;
    fs::remove_file("/opt/sam/scripts/sprec/noise/other.zip").await?;
    fs::remove_file("/opt/sam/scripts/sprec/noise/_background_noise_.zip").await?;

    log::info!("Installing requirements for SPREC...");
    let _ = crate::cmd_async("pip3 install -r /opt/sam/scripts/sprec/requirements.txt").await?;
    Ok(())
}
