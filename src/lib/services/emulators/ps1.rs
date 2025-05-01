use tokio::process::Command;
use tokio::fs;
use std::path::Path;

pub async fn install() -> Result<(), anyhow::Error> {
    let repo_path = "scripts/px1-sam";
    // Step 1: Clone or update the repository
    // Check if the directory exists or is empty
    if !Path::new(repo_path).exists() || fs::read_dir(repo_path).await?.next_entry().await?.is_none() {
        println!("Cloning px1-sam repository...");
        let status = Command::new("git")
            .arg("clone")
            .arg("https://github.com/ktheindifferent/px1-sam.git")
            .arg(repo_path)
            .status()
            .await?;
        println!("Cloned px1-sam repository to {}", repo_path);
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to clone repository"));
        }
    } else {
        // If the directory exists, pull the latest changes in the px1-sam repo
        let status = Command::new("git")
            .arg("pull")
            .current_dir(repo_path)
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to pull latest changes"));
        }
    }

    // Step 2: Build with cargo
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(repo_path)
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
        let src_path = format!("{}/target/release/{}", repo_path, bin_name);
        let dest_path = format!("{}/{}", dest_dir, bin_name);
        // Only copy if the source exists (some platforms may not build all files)
        if Path::new(&src_path).exists() {
            fs::copy(&src_path, &dest_path).await?;
        }
    }
    Ok(())
}
