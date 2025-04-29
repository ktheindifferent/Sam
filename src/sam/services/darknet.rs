// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs::{self};

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

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

    let output = child.wait_with_output().expect("failed to wait on child");
    let darknet = String::from_utf8_lossy(&output.stdout)
        .to_string()
        .replace("\n", "");

    if darknet.to_lowercase().contains("error") || darknet.is_empty() {
        return Err(format!("deepvision_scan_image_with_cpu_error: {darknet}"));
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
        libsam::services::darknet::download_yolov3_cfg(None)
            .await
            .map_err(|e| format!("Failed to download cfg: {e}"))?;
    }
    if !Path::new(weights).exists() {
        libsam::services::darknet::download_yolov3_model(None)
            .await
            .map_err(|e| format!("Failed to download weights: {e}"))?;
    }
    if !Path::new(image_path).exists() {
        return Err(format!("Image file does not exist: {image_path}"));
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
        .map_err(|e| format!("Failed to run darknet: {e}"))?;

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
        .map_err(|e| format!("Failed to parse darknet output: {e}"))
}
