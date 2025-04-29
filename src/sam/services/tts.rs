// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::{Request, Response};
use std::thread;
use std::time::Duration;
use std::process::Command;
use std::fs::File;
use std::io::{Read, Write};
use rand::{distributions::Alphanumeric, Rng};
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

const TTS_TMP_DIR: &str = "/opt/sam/tmp/tts";
const MANIFEST_PATH: &str = "/opt/sam/tmp/tts/manifest.json";

#[derive(Serialize, Deserialize, Debug, Default)]
struct TtsManifest {
    // text -> filename
    entries: HashMap<String, String>,
}

fn load_manifest() -> TtsManifest {
    if let Ok(mut file) = File::open(MANIFEST_PATH) {
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_ok() {
            if let Ok(manifest) = serde_json::from_str(&buf) {
                return manifest;
            }
        }
    }
    TtsManifest::default()
}

fn save_manifest(manifest: &TtsManifest) {
    if let Ok(mut file) = File::create(MANIFEST_PATH) {
        let _ = file.write_all(serde_json::to_string(manifest).unwrap().as_bytes());
    }
}

fn ensure_tts_tmp_dir() {
    let path = Path::new(TTS_TMP_DIR);
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
}

pub fn handle(_current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/tts" {
        let input = request.get_param("text").unwrap();
        return Ok(Response::from_data("audio/wav", crate::sam::services::tts::get(input).unwrap()));
        
    }
    Ok(Response::empty_404())
}

pub fn init(){

    let tts_thead = thread::Builder::new().name("mozillatts".to_string()).spawn(move || {
        crate::sam::tools::uinx_cmd("docker run -p 5002:5002 synesthesiam/mozillatts");
    });
    match tts_thead{
        Ok(_) => {
            log::info!("tts server started successfully");
        },
        Err(e) => {
            log::error!("failed to initialize tts server: {}", e);
        }
    }
}

pub fn get(text: String) -> Result<Vec<u8>, crate::sam::services::Error> {
    ensure_tts_tmp_dir();
    let mut manifest = load_manifest();
    if let Some(filename) = manifest.entries.get(&text) {
        let file_path = Path::new(TTS_TMP_DIR).join(filename);
        if file_path.exists() {
            let mut file = File::open(&file_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    match tts_cross_platform_wav(&text) {
        Ok(x) => {
            // Save to cache
            let rand_name: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();
            let file_name = format!("{rand_name}.wav");
            let file_path = Path::new(TTS_TMP_DIR).join(&file_name);
            let mut file = File::create(&file_path)?;
            file.write_all(&x)?;
            manifest.entries.insert(text.clone(), file_name);
            save_manifest(&manifest);
            Ok(x)
        },
        Err(_) => {
            match fetch_local(text.clone()) {
                Ok(x) => {
                    // Save to cache
                    let rand_name: String = rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(16)
                        .map(char::from)
                        .collect();
                    let file_name = format!("{rand_name}.wav");
                    let file_path = Path::new(TTS_TMP_DIR).join(&file_name);
                    let mut file = File::create(&file_path)?;
                    file.write_all(&x)?;
                    manifest.entries.insert(text.clone(), file_name);
                    save_manifest(&manifest);
                    Ok(x)
                },
                Err(_) => {
                    match fetch_online(text.clone()) {
                        Ok(x) => {
                            // Save to cache
                            let rand_name: String = rand::thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(16)
                                .map(char::from)
                                .collect();
                            let file_name = format!("{rand_name}.wav");
                            let file_path = Path::new(TTS_TMP_DIR).join(&file_name);
                            let mut file = File::create(&file_path)?;
                            file.write_all(&x)?;
                            manifest.entries.insert(text.clone(), file_name);
                            save_manifest(&manifest);
                            Ok(x)
                        },
                        Err(e) => Err(e),
                    }
                }
            }
        }
    }

}

pub fn fetch_online(text: String) -> Result<Vec<u8>, crate::sam::services::Error> {
    let client = reqwest::blocking::Client::new();
    let bytes = client.get(format!("https://tts.opensam.foundation/api/tts?text={text}&speaker_id=&style_wav="))
        .basic_auth("sam", Some("87654321"))
        .timeout(Duration::from_secs(5))
        .send()?.bytes()?;
    Ok(bytes.to_vec())
}

pub fn fetch_local(text: String) -> Result<Vec<u8>, crate::sam::services::Error> {
    let client = reqwest::blocking::Client::new();
    let bytes = client.get(format!("http://localhost:5002/api/tts?text={text}&speaker_id=&style_wav="))
        .timeout(Duration::from_secs(5))
        .send()?.bytes()?;
    Ok(bytes.to_vec())
}
pub fn tts_cross_platform_wav(text: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let tmp_path = Path::new(TTS_TMP_DIR).join(format!("{}.wav", rand_name));
        let script = format!(
            "Add-Type -AssemblyName System.speech; \
            $speak = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            $speak.SetOutputToWaveFile('{}'); \
            $speak.Speak('{}');",
            tmp_path.display(),
            text.replace("'", "''")
        );
        Command::new("powershell")
            .args(&["-Command", &script])
            .output()?;
        let mut file = File::open(&tmp_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        std::fs::remove_file(&tmp_path).ok();
        Ok(buf)
    }
    #[cfg(target_os = "macos")]
    {
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let tmp_path = Path::new(TTS_TMP_DIR).join(format!("{rand_name}.wav"));
        Command::new("say")
            .args(["-o", tmp_path.to_str().unwrap(), "--data-format=LEF32@22050", text])
            .output()?;
        let mut file = File::open(&tmp_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        std::fs::remove_file(&tmp_path).ok();
        Ok(buf)
    }
    #[cfg(target_os = "linux")]
    {
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let tmp_path = Path::new(TTS_TMP_DIR).join(format!("{}.wav", rand_name));
        // Try espeak first
        let espeak_output = Command::new("espeak")
            .args(&["-w", tmp_path.to_str().unwrap(), text])
            .output();
        if let Ok(status) = espeak_status {
            if status.success() {
                let mut file = File::open(&tmp_path)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                std::fs::remove_file(&tmp_path).ok();
                return Ok(buf);
            }
        }
        // Fallback to festival/text2wave
        let festival_status = Command::new("text2wave")
            .arg("-o")
            .arg(tmp_path.to_str().unwrap())
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            });
        if let Ok(status) = festival_status {
            if status.success() {
                let mut file = File::open(&tmp_path)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                std::fs::remove_file(&tmp_path).ok();
                return Ok(buf);
            }
        }
        Err("No TTS engine found (espeak or festival/text2wave required)".into())
    }
}
#[cfg(target_os = "windows")]
pub fn tts_cross_platform(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("powershell")
        .args(&["-Command", &format!("Add-Type –AssemblyName System.speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{}');", text)])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn tts_cross_platform(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("say")
        .arg(text)
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn tts_cross_platform(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try espeak, fallback to festival
    if Command::new("espeak").arg(text).spawn().is_ok() {
        Ok(())
    } else if Command::new("festival").arg("--tts").arg(text).spawn().is_ok() {
        Ok(())
    } else {
        Err("No TTS engine found (espeak or festival required)".into())
    }
}