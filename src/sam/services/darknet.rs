// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs::{self};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use std::path::Path;
use scraper::{Html, Selector};
use tokio::fs::metadata;


pub fn build_darknet() -> Result<(), String> {

    let darknet_dir = "./scripts/darknet";
    let output_dir = "/opt/sam/bin";

    // Run `make` in the darknet directory
    let status = Command::new("make")
        .current_dir(darknet_dir)
        .status()
        .map_err(|e| format!("Failed to start make: {}", e))?;

    if !status.success() {
        return Err(format!("Make failed with status: {}", status));
    }

    // Copy the compiled binary to /opt/sam/bin
    let src_bin = format!("{}/darknet", darknet_dir);
    let dest_bin = format!("{}/darknet", output_dir);

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    std::fs::copy(&src_bin, &dest_bin)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Optionally, make the binary executable
    Command::new("chmod")
        .arg("+x")
        .arg(&dest_bin)
        .status()
        .map_err(|e| format!("Failed to chmod binary: {}", e))?;

    Ok(())
}

pub async fn download_cfg_index() -> Result<(), String> {

    let url = "https://github.com/ktheindifferent/Darknet/tree/master/cfg";
    let output_dir = "./cfg";

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {}", e))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch cfg index: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch cfg index, status: {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| format!("Failed to read body: {}", e))?;

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

    let index_path = format!("{}/index.txt", output_dir);
    let mut file = async_fs::File::create(&index_path)
        .await
        .map_err(|e| format!("Failed to create index file: {}", e))?;

    for name in &file_list {
        // Write the filename to index.txt
        file.write_all(format!("{}\n", name).as_bytes())
            .await
            .map_err(|e| format!("Failed to write to index file: {}", e))?;

        // Download each .cfg/.data file only if it doesn't already exist
        let cfg_url = format!(
            "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/{}",
            name
        );
        let cfg_path = format!("{}/{}", output_dir, name);

        if metadata(&cfg_path).await.is_ok() {
            // File already exists, skip download
            continue;
        }

        let resp = reqwest::get(&cfg_url)
            .await
            .map_err(|e| format!("Failed to download {}: {}", name, e))?;

        if !resp.status().is_success() {
            return Err(format!("Failed to download {}, status: {}", name, resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| format!("Failed to read {} bytes: {}", name, e))?;

        let mut out_file = async_fs::File::create(&cfg_path)
            .await
            .map_err(|e| format!("Failed to create {}: {}", cfg_path, e))?;

        out_file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write {}: {}", cfg_path, e))?;
    }

    Ok(())
}

pub async fn download_yolov3_cfg() -> Result<(), String> {
    let url = "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/yolov3.cfg";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{}/yolov3.cfg", output_dir);

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {}", e))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download cfg: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to download cfg, status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read cfg bytes: {}", e))?;

    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create cfg file: {}", e))?;

    out_file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write cfg file: {}", e))?;

    Ok(())
}

pub async fn download_yolov3_model() -> Result<(), String> {
    let url = "https://github.com/patrick013/Object-Detection---Yolov3/raw/refs/heads/master/model/yolov3.weights";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{}/yolov3.weights", output_dir);

    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create model directory: {}", e))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download model: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to download model, status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read model bytes: {}", e))?;

    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create model file: {}", e))?;

    out_file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write model file: {}", e))?;

    Ok(())
}


// ./darknet detect cfg/yolov3-tiny.cfg yolov3-tiny.weights data/dog.jpg
// {id: "dog", probability: 0.570732, left: 129, right: 369, top: 186, bottom: 517}
// {id: "car", probability: 0.517267, left: 533, right: 621, top: 94, bottom: 157}
// {id: "car", probability: 0.615291, left: 465, right: 679, top: 71, bottom: 169}
// {id: "bicycle", probability: 0.585022, left: 206, right: 575, top: 150, bottom: 450}
pub fn darknet_image_with_gpu(file_path: String) -> Result<String, String> {


    let _observation_file = fs::read(file_path.as_str()).unwrap();


    let child = Command::new("sh")
    .arg("-c")
    .arg(format!("cd /opt/sam/bin/darknet/ && ./darknet-gpu detect /opt/sam/bin/darknet/cfg/yolov3-tiny.cfg /opt/sam/bin/darknet/yolov3-tiny.weights {}", file_path.clone()))
    .stdout(Stdio::piped())
    .spawn()
    .expect("failed to execute child");

    let output = child
        .wait_with_output()
        .expect("failed to wait on child");
    let darknet = String::from_utf8_lossy(&output.stdout).to_string().replace("\n", "");

    if darknet.to_lowercase().contains("error") || darknet.is_empty() {
        return Err(format!("deepvision_scan_image_with_cpu_error: {}", darknet))
    }

    Ok(darknet)

}


#[derive(Debug, Serialize, Deserialize)]
pub struct DetectionCoordinates {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectionObject {
    pub class_id: usize,
    pub name: String,
    pub coordinates: DetectionCoordinates,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectionResult {
    pub frame_id: usize,
    pub filename: String,
    pub objects: Vec<DetectionObject>,
}

pub async fn darknet_detect(image_path: &str) -> Result<DetectionResult, String> {
    let darknet_bin = "/opt/sam/bin/darknet";
    let cfg = "/opt/sam/models/yolov3.cfg";
    let weights = "/opt/sam/models/yolov3.weights";
    if !Path::new(cfg).exists() {
        download_yolov3_cfg().await.map_err(|e| format!("Failed to download cfg: {}", e))?;
    }
    if !Path::new(weights).exists() {
        download_yolov3_model().await.map_err(|e| format!("Failed to download weights: {}", e))?;
    }
    if !Path::new(image_path).exists() {
        return Err(format!("Image file does not exist: {}", image_path));
    }
    // let image_filename = Path::new(image_path)
    //     .file_name()
    //     .and_then(|n| n.to_str())
    //     .unwrap_or(image_path);

    let output = Command::new(darknet_bin)
        .arg("detect")
        .arg(cfg)
        .arg(weights)
        .arg(image_path)
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run darknet: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "darknet failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout.find('{').ok_or("No JSON output from darknet")?;
    let json = &stdout[json_start..];

    serde_json::from_str::<DetectionResult>(json)
        .map_err(|e| format!("Failed to parse darknet output: {}", e))
}

// Fix error handling for install()
pub async fn install() -> std::io::Result<()> {
    build_darknet().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    download_yolov3_model().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    download_yolov3_cfg().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    download_cfg_index().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let _ = crate::sam::tools::uinx_cmd("chmod +x /opt/sam/bin/darknet");
    Ok(())
}
