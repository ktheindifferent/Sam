use tokio::process::Command;
use tokio::fs;
use std::path::Path;

pub async fn install() -> Result<(), anyhow::Error> {
    // Step 1: Clone the repository
    if !Path::new("rustation-ng").exists() {
        let status = Command::new("git")
            .arg("clone")
            .arg("https://github.com/ktheindifferent/px1-sam.git")
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to clone repository"));
        }
    }

    // Step 2: Build with cargo
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir("rustation-ng")
        .status()
        .await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to build project"));
    }
    // Step 3: Copy the built binaries to /opt/sam/bin
    let bin_names = if cfg!(target_os = "macos") {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.dylib"]
    } else if cfg!(target_os = "windows") {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.dll"]
    } else {
        vec!["librustation_ng_retro.d", "librustation_ng_retro.so"]
    };

    let dest_dir = "/opt/sam/bin";
    // Create destination directory if it doesn't exist
    if !Path::new(dest_dir).exists() {
        fs::create_dir_all(dest_dir).await?;
    }

    for bin_name in bin_names {
        let src_path = format!("rustation-ng/target/release/{}", bin_name);
        let dest_path = format!("{}/{}", dest_dir, bin_name);
        // Only copy if the source exists (some platforms may not build all files)
        if Path::new(&src_path).exists() {
            fs::copy(&src_path, &dest_path).await?;
        }
    }
    Ok(())
}
