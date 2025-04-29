// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use scraper::{Html, Selector};
use tokio::fs::metadata;
use tokio::process::Command;

// Helper: Run a command and stream output lines
async fn run_command_stream_lines(mut cmd: Command, output_lines: Option<&Arc<Mutex<Vec<String>>>>, prefix: &str) -> Result<(), String> {
    let mut child = cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", prefix, e))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut lines = vec![];
    if let Some(stdout) = stdout {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await.map_err(|e| format!("{} stdout error: {}", prefix, e))? {
            let msg = format!("{}: {}", prefix, line);
            crate::println(output_lines, msg.clone()).await;
            if output_lines.is_none() {
                println!("{}", msg);
            }
            lines.push(msg);
        }
    }
    if let Some(stderr) = stderr {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await.map_err(|e| format!("{} stderr error: {}", prefix, e))? {
            let msg = format!("{}: {}", prefix, line);
            crate::println(output_lines, msg.clone()).await;
            if output_lines.is_none() {
                println!("{}", msg);
            }
            lines.push(msg);
        }
    }
    let status = child.wait().await.map_err(|e| format!("{} wait error: {}", prefix, e))?;
    if !status.success() {
        return Err(format!("{} failed: {:?}", prefix, lines));
    }
    Ok(())
}

pub async fn build_darknet(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<(), String> {
    let darknet = PathBuf::from("/opt/sam/bin/darknet");
    if darknet.exists() {
        crate::println(output_lines, "darknet binary already exists.".to_string()).await;
        return Ok(());
    }
    let darknet_dir = "./scripts/darknet";
    let output_dir = "/opt/sam/bin";
    crate::println(output_lines, "Building darknet...".to_string()).await;
    let mut make_cmd = Command::new("make");
    make_cmd.current_dir(darknet_dir);
    run_command_stream_lines(make_cmd, output_lines, "make").await?;
    let src_bin = format!("{darknet_dir}/darknet");
    let dest_bin = format!("{output_dir}/darknet");
    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create output dir: {e}"))?;
    let bin_bytes = async_fs::read(&src_bin)
        .await
        .map_err(|e| format!("Failed to read binary: {e}"))?;
    let mut dest_file = async_fs::File::create(&dest_bin)
        .await
        .map_err(|e| format!("Failed to create dest binary: {e}"))?;
    dest_file
        .write_all(&bin_bytes)
        .await
        .map_err(|e| format!("Failed to write binary: {e}"))?;
    let mut chmod_cmd = Command::new("chmod");
    chmod_cmd.arg("+x").arg(&dest_bin);
    run_command_stream_lines(chmod_cmd, output_lines, "chmod").await?;
    crate::println(output_lines, "darknet build: done.".to_string()).await;
    Ok(())
}

pub async fn download_cfg_index(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<(), String> {
    let url = "https://github.com/ktheindifferent/Darknet/tree/master/cfg";
    let output_dir = "./cfg";
    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {e}"))?;
    crate::println(output_lines, "Downloading cfg index...".to_string()).await;
    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch cfg index: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Failed to fetch cfg index, status: {}",
            resp.status()
        ));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read body: {e}"))?;
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
        file.write_all(format!("{name}\n").as_bytes())
            .await
            .map_err(|e| format!("Failed to write to index file: {e}"))?;
        let cfg_url = format!(
            "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/{name}"
        );
        let cfg_path = format!("{output_dir}/{name}");
        if metadata(&cfg_path).await.is_ok() {
            continue;
        }
        crate::println(output_lines, format!("Downloading {name}...")).await;
        let resp = reqwest::get(&cfg_url)
            .await
            .map_err(|e| format!("Failed to download {name}: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Failed to download {}, status: {}",
                name,
                resp.status()
            ));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read {name} bytes: {e}"))?;
        let mut out_file = async_fs::File::create(&cfg_path)
            .await
            .map_err(|e| format!("Failed to create {cfg_path}: {e}"))?;
        out_file
            .write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write {cfg_path}: {e}"))?;
    }
    crate::println(output_lines, "cfg index download: done.".to_string()).await;
    Ok(())
}

pub async fn download_yolov3_cfg(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<(), String> {
    let url = "https://raw.githubusercontent.com/ktheindifferent/Darknet/refs/heads/master/cfg/yolov3.cfg";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{output_dir}/yolov3.cfg");
    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create cfg directory: {e}"))?;
    crate::println(output_lines, "Downloading yolov3.cfg...".to_string()).await;
    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download cfg: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Failed to download cfg, status: {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read cfg bytes: {e}"))?;
    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create cfg file: {e}"))?;
    out_file
        .write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write cfg file: {e}"))?;
    crate::println(output_lines, "yolov3.cfg download: done.".to_string()).await;
    Ok(())
}

pub async fn download_yolov3_model(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> Result<(), String> {
    let url = "https://github.com/patrick013/Object-Detection---Yolov3/raw/refs/heads/master/model/yolov3.weights";
    let output_dir = "/opt/sam/models";
    let output_path = format!("{output_dir}/yolov3.weights");
    async_fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create model directory: {e}"))?;
    crate::println(output_lines, "Downloading yolov3.weights...".to_string()).await;
    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download model: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Failed to download model, status: {}",
            resp.status()
        ));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read model bytes: {e}"))?;
    let mut out_file = async_fs::File::create(&output_path)
        .await
        .map_err(|e| format!("Failed to create model file: {e}"))?;
    out_file
        .write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write model file: {e}"))?;
    crate::println(output_lines, "yolov3.weights download: done.".to_string()).await;
    Ok(())
}

pub async fn install(output_lines: Option<&Arc<Mutex<Vec<String>>>>) -> std::io::Result<()> {
    build_darknet(output_lines).await.map_err(std::io::Error::other)?;
    download_yolov3_model(output_lines).await.map_err(std::io::Error::other)?;
    download_yolov3_cfg(output_lines).await.map_err(std::io::Error::other)?;
    download_cfg_index(output_lines).await.map_err(std::io::Error::other)?;
    let mut chmod_cmd = Command::new("chmod");
    chmod_cmd.arg("+x").arg("/opt/sam/bin/darknet");
    let _ = run_command_stream_lines(chmod_cmd, output_lines, "chmod").await;
    Ok(())
}
