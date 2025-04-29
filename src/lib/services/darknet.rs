// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use scraper::{Html, Selector};
use tokio::fs::metadata;
use tokio::process::Command;

pub async fn build_darknet() -> Result<(), String> {
    let darknet_dir = "./scripts/darknet";
    let output_dir = "/opt/sam/bin";

    // Run `make` in the darknet directory (async)
    let status = Command::new("make")
        .current_dir(darknet_dir)
        .status()
        .await
        .map_err(|e| format!("Failed to start make: {e}"))?;

    if !status.success() {
        return Err(format!("Make failed with status: {status}"));
    }

    // Copy the compiled binary to /opt/sam/bin (async)
    let src_bin = format!("{darknet_dir}/darknet");
    let dest_bin = format!("{output_dir}/darknet");

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create output dir: {e}"))?;

    // Read the binary file asynchronously
    let bin_bytes = async_fs::read(&src_bin)
        .await
        .map_err(|e| format!("Failed to read binary: {e}"))?;

    let mut dest_file = async_fs::File::create(&dest_bin)
        .await
        .map_err(|e| format!("Failed to create dest binary: {e}"))?;

    dest_file.write_all(&bin_bytes)
        .await
        .map_err(|e| format!("Failed to write binary: {e}"))?;

    // Optionally, make the binary executable (async)
    let status = Command::new("chmod")
        .arg("+x")
        .arg(&dest_bin)
        .status()
        .await
        .map_err(|e| format!("Failed to chmod binary: {e}"))?;

    if !status.success() {
        return Err(format!("Chmod failed with status: {status}"));
    }

    Ok(())
}

pub async fn download_cfg_index() -> Result<(), String> {

    let url = "https://github.com/ktheindifferent/Darknet/tree/master/cfg";
    let output_dir = "./cfg";

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {e}"))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch cfg index: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch cfg index, status: {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| format!("Failed to read body: {e}"))?;

    // Parse HTML to extract file names
    let document = Html::parse_document(&body);
    let selector = Selector::parse("a.Link--primary").unwrap();

    let mut file_list = Vec::new();
    for element in document.select(&selector) {
        if let Some(file_name) = element.value().attr("title") {
            if file_name.ends_with(".cfg") || file_name.ends_with(".data") {
                file_list.push(file_name.to_string());
            }
        }
    }

    let index_path = format!("{output_dir}/index.txt");
    let mut file = async_fs::File::create(&index_path)
        .await
        .map_err(|e| format!("Failed to create index file: {e}"))?;

    for name in &file_list {
        // Write the filename to index.txt
        file.write_all(format!("{name}\n").as_bytes())
            .await
            .map_err(|e| format!("Failed to write to index file: {e}"))?;

        // Download each .cfg/.data file only if it doesn't already exist
        let cfg_url = format!(
            "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/{name}"
        );
        let cfg_path = format!("{output_dir}/{name}");

        if metadata(&cfg_path).await.is_ok() {
            // File already exists, skip download
            continue;
        }

        let resp = reqwest::get(&cfg_url)
            .await
            .map_err(|e| format!("Failed to download {name}: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Failed to download {}, status: {}", name, resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| format!("Failed to read {name} bytes: {e}"))?;

        let mut out_file = async_fs::File::create(&cfg_path)
            .await
            .map_err(|e| format!("Failed to create {cfg_path}: {e}"))?;

        out_file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write {cfg_path}: {e}"))?;
    }

    Ok(())
}

pub async fn download_yolov3_cfg() -> Result<(), String> {
    let url = "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/yolov3.cfg";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{output_dir}/yolov3.cfg");

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {e}"))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download cfg: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to download cfg, status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read cfg bytes: {e}"))?;

    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create cfg file: {e}"))?;

    out_file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write cfg file: {e}"))?;

    Ok(())
}

pub async fn download_yolov3_model() -> Result<(), String> {
    let url = "https://github.com/patrick013/Object-Detection---Yolov3/raw/refs/heads/master/model/yolov3.weights";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{output_dir}/yolov3.weights");

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create model directory: {e}"))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download model: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to download model, status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read model bytes: {e}"))?;

    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create model file: {e}"))?;

    out_file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write model file: {e}"))?;

    Ok(())
}

pub async fn install() -> std::io::Result<()> {
    build_darknet().await.map_err(std::io::Error::other)?;
    download_yolov3_model().await.map_err(std::io::Error::other)?;
    download_yolov3_cfg().await.map_err(std::io::Error::other)?;
    download_cfg_index().await.map_err(std::io::Error::other)?;
    let _ = crate::cmd_async("chmod +x /opt/sam/bin/darknet");
    Ok(())
}


