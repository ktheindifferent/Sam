// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub voice: String,
    pub language: String,
    pub speed: f32,
    pub pitch: f32,
    pub volume: f32,
    pub cache_dir: PathBuf,
    pub cache_enabled: bool,
    pub engine: TtsEngine,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TtsEngine {
    System,
    MozillaTts,
    External(String),
    Coqui,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            voice: "default".to_string(),
            language: "en-US".to_string(),
            speed: 1.0,
            pitch: 1.0,
            volume: 1.0,
            cache_dir: PathBuf::from("/opt/sam/tmp/tts"),
            cache_enabled: true,
            engine: TtsEngine::System,
            timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
    pub language: Option<String>,
    pub speed: Option<f32>,
    pub pitch: Option<f32>,
    pub volume: Option<f32>,
    pub format: AudioFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum AudioFormat {
    #[default]
    Wav,
    Mp3,
    Ogg,
    Flac,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio_data: Vec<u8>,
    pub format: AudioFormat,
    pub duration_ms: u128,
    pub cached: bool,
}

struct TtsCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
    current_size: usize,
}

#[derive(Clone)]
struct CacheEntry {
    file_path: PathBuf,
    access_count: usize,
    last_accessed: std::time::Instant,
    size: usize,
}

impl TtsCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            current_size: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.access_count += 1;
            entry.last_accessed = std::time::Instant::now();
            
            if let Ok(mut file) = File::open(&entry.file_path) {
                let mut data = Vec::new();
                if file.read_to_end(&mut data).is_ok() {
                    return Some(data);
                }
            }
        }
        None
    }

    fn put(&mut self, key: String, data: &[u8], path: PathBuf) -> Result<(), std::io::Error> {
        let size = data.len();
        
        while self.current_size + size > self.max_size && !self.entries.is_empty() {
            self.evict_lru();
        }
        
        let entry = CacheEntry {
            file_path: path.clone(),
            access_count: 0,
            last_accessed: std::time::Instant::now(),
            size,
        };
        
        if let Some(old_entry) = self.entries.insert(key, entry) {
            self.current_size -= old_entry.size;
        }
        self.current_size += size;
        
        let mut file = File::create(path)?;
        file.write_all(data)?;
        
        Ok(())
    }

    fn evict_lru(&mut self) {
        if let Some((key, _)) = self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, e)| (k.clone(), e.clone()))
        {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.size;
                let _ = fs::remove_file(&entry.file_path);
            }
        }
    }
}

pub struct TtsService {
    config: TtsConfig,
    cache: Arc<Mutex<TtsCache>>,
}

impl TtsService {
    pub fn new(config: TtsConfig) -> Result<Self, crate::services::Error> {
        if config.cache_enabled {
            fs::create_dir_all(&config.cache_dir).map_err(|e| {
                crate::services::Error::from(format!("Failed to create cache dir: {}", e))
            })?;
        }
        
        let cache = Arc::new(Mutex::new(TtsCache::new(100 * 1024 * 1024))); // 100MB cache
        
        Ok(Self { config, cache })
    }

    pub fn synthesize(&self, request: TtsRequest) -> Result<TtsResult, crate::services::Error> {
        let start_time = std::time::Instant::now();
        
        let cache_key = self.generate_cache_key(&request);
        
        if self.config.cache_enabled {
            if let Ok(mut cache) = self.cache.lock() {
                if let Some(cached_data) = cache.get(&cache_key) {
                    return Ok(TtsResult {
                        audio_data: cached_data,
                        format: request.format.clone(),
                        duration_ms: start_time.elapsed().as_millis(),
                        cached: true,
                    });
                }
            }
        }
        
        let audio_data = match &self.config.engine {
            TtsEngine::System => self.synthesize_system(&request)?,
            TtsEngine::MozillaTts => self.synthesize_mozilla(&request)?,
            TtsEngine::External(url) => self.synthesize_external(&request, url)?,
            TtsEngine::Coqui => self.synthesize_coqui(&request)?,
        };
        
        if self.config.cache_enabled {
            let cache_path = self.config.cache_dir.join(format!("{}.audio", cache_key));
            if let Ok(mut cache) = self.cache.lock() {
                let _ = cache.put(cache_key, &audio_data, cache_path);
            }
        }
        
        Ok(TtsResult {
            audio_data,
            format: request.format,
            duration_ms: start_time.elapsed().as_millis(),
            cached: false,
        })
    }

    fn generate_cache_key(&self, request: &TtsRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        request.text.hash(&mut hasher);
        request.voice.hash(&mut hasher);
        request.language.hash(&mut hasher);
        request.speed.map(|s| (s * 100.0) as i32).hash(&mut hasher);
        request.pitch.map(|p| (p * 100.0) as i32).hash(&mut hasher);
        request.volume.map(|v| (v * 100.0) as i32).hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    fn synthesize_system(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        #[cfg(target_os = "windows")]
        {
            self.synthesize_windows(request)
        }
        #[cfg(target_os = "macos")]
        {
            self.synthesize_macos(request)
        }
        #[cfg(target_os = "linux")]
        {
            self.synthesize_linux(request)
        }
    }

    #[cfg(target_os = "windows")]
    fn synthesize_windows(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        use rand::{distributions::Alphanumeric, Rng};
        use std::process::Command;
        
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let tmp_path = self.config.cache_dir.join(format!("{}.wav", rand_name));
        
        let rate = ((request.speed.unwrap_or(1.0) - 1.0) * 10.0) as i32;
        let volume = (request.volume.unwrap_or(1.0) * 100.0) as i32;
        
        let script = format!(
            r#"
            Add-Type -AssemblyName System.speech
            $speak = New-Object System.Speech.Synthesis.SpeechSynthesizer
            $speak.Rate = {}
            $speak.Volume = {}
            $speak.SetOutputToWaveFile('{}')
            $speak.Speak('{}')
            "#,
            rate,
            volume,
            tmp_path.display(),
            request.text.replace("'", "''")
        );
        
        let output = Command::new("powershell")
            .args(&["-Command", &script])
            .output()
            .map_err(|e| crate::services::Error::from(format!("PowerShell error: {}", e)))?;
        
        if !output.status.success() {
            return Err(crate::services::Error::from("Windows TTS failed"));
        }
        
        let mut file = File::open(&tmp_path).map_err(|e| {
            crate::services::Error::from(format!("Failed to read TTS output: {}", e))
        })?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        fs::remove_file(&tmp_path).ok();
        
        Ok(data)
    }

    #[cfg(target_os = "macos")]
    fn synthesize_macos(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        use rand::{distributions::Alphanumeric, Rng};
        use std::process::Command;
        
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let tmp_path = self.config.cache_dir.join(format!("{}.wav", rand_name));
        
        let mut args = vec![
            "-o".to_string(),
            tmp_path.to_str()
                .ok_or_else(|| crate::services::Error::from("Invalid path"))?
                .to_string(),
            "--data-format=LEF32@22050".to_string(),
        ];
        
        if let Some(voice) = &request.voice {
            args.push("-v".to_string());
            args.push(voice.clone());
        }
        
        if let Some(rate) = request.speed {
            args.push("-r".to_string());
            args.push((rate * 200.0).to_string());
        }
        
        args.push(request.text.clone());
        
        let output = Command::new("say")
            .args(&args)
            .output()
            .map_err(|e| crate::services::Error::from(format!("macOS say error: {}", e)))?;
        
        if !output.status.success() {
            return Err(crate::services::Error::from("macOS TTS failed"));
        }
        
        let mut file = File::open(&tmp_path).map_err(|e| {
            crate::services::Error::from(format!("Failed to read TTS output: {}", e))
        })?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        fs::remove_file(&tmp_path).ok();
        
        Ok(data)
    }

    #[cfg(target_os = "linux")]
    fn synthesize_linux(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        use rand::{distributions::Alphanumeric, Rng};
        use std::process::Command;
        
        let rand_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let tmp_path = self.config.cache_dir.join(format!("{}.wav", rand_name));
        
        let speed = ((request.speed.unwrap_or(1.0) * 175.0) as i32).max(80).min(450);
        let pitch = ((request.pitch.unwrap_or(1.0) * 50.0) as i32).max(0).min(99);
        let volume = ((request.volume.unwrap_or(1.0) * 200.0) as i32).max(0).min(200);
        
        let path_str = tmp_path.to_str()
            .ok_or_else(|| crate::services::Error::from("Invalid path"))?;
        
        let output = Command::new("espeak")
            .args(&[
                "-w", path_str,
                "-s", &speed.to_string(),
                "-p", &pitch.to_string(),
                "-a", &volume.to_string(),
                &request.text,
            ])
            .output();
        
        match output {
            Ok(result) if result.status.success() => {
                let mut file = File::open(&tmp_path)?;
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                fs::remove_file(&tmp_path).ok();
                return Ok(data);
            }
            _ => {}
        }
        
        let output = Command::new("pico2wave")
            .args(&[
                "-w", path_str,
                "-l", request.language.as_ref().unwrap_or(&"en-US".to_string()),
                &request.text,
            ])
            .output();
        
        match output {
            Ok(result) if result.status.success() => {
                let mut file = File::open(&tmp_path)?;
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                fs::remove_file(&tmp_path).ok();
                return Ok(data);
            }
            _ => {}
        }
        
        Err(crate::services::Error::from(
            "No TTS engine available on Linux (espeak or pico2wave required)"
        ))
    }

    fn synthesize_mozilla(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()?;
        
        let response = client
            .get(format!(
                "http://localhost:5002/api/tts?text={}&speaker_id={}&style_wav=",
                urlencoding::encode(&request.text),
                request.voice.as_deref().unwrap_or("")
            ))
            .send()
            .map_err(|e| crate::services::Error::from(format!("Mozilla TTS error: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(crate::services::Error::from("Mozilla TTS request failed"));
        }
        
        let bytes = response.bytes()?;
        Ok(bytes.to_vec())
    }

    fn synthesize_external(&self, request: &TtsRequest, url: &str) -> Result<Vec<u8>, crate::services::Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()?;
        
        let mut params = HashMap::new();
        params.insert("text", request.text.clone());
        if let Some(voice) = &request.voice {
            params.insert("voice", voice.clone());
        }
        if let Some(lang) = &request.language {
            params.insert("language", lang.clone());
        }
        
        let response = client
            .post(url)
            .json(&params)
            .send()
            .map_err(|e| crate::services::Error::from(format!("External TTS error: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(crate::services::Error::from("External TTS request failed"));
        }
        
        let bytes = response.bytes()?;
        Ok(bytes.to_vec())
    }

    fn synthesize_coqui(&self, request: &TtsRequest) -> Result<Vec<u8>, crate::services::Error> {
        Err(crate::services::Error::from(
            "Coqui TTS integration not yet implemented"
        ))
    }

    pub fn list_voices(&self) -> Result<Vec<String>, crate::services::Error> {
        match &self.config.engine {
            TtsEngine::System => {
                #[cfg(target_os = "macos")]
                {
                    use std::process::Command;
                    let output = Command::new("say")
                        .args(["-v", "?"])
                        .output()
                        .map_err(|e| crate::services::Error::from(format!("Failed to list voices: {}", e)))?;
                    
                    let text = String::from_utf8_lossy(&output.stdout);
                    let voices: Vec<String> = text
                        .lines()
                        .filter_map(|line| line.split_whitespace().next())
                        .map(|s| s.to_string())
                        .collect();
                    Ok(voices)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok(vec!["default".to_string()])
                }
            }
            _ => Ok(vec!["default".to_string()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_config_default() {
        let config = TtsConfig::default();
        assert_eq!(config.speed, 1.0);
        assert_eq!(config.language, "en-US");
        assert!(config.cache_enabled);
    }

    #[test]
    fn test_cache_key_generation() {
        let config = TtsConfig::default();
        let service = match TtsService::new(config) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create TTS service: {}", e);
                return;
            }
        };
        
        let request1 = TtsRequest {
            text: "Hello world".to_string(),
            voice: Some("default".to_string()),
            language: Some("en-US".to_string()),
            speed: Some(1.0),
            pitch: Some(1.0),
            volume: Some(1.0),
            format: AudioFormat::Wav,
        };
        
        let request2 = TtsRequest {
            text: "Hello world".to_string(),
            voice: Some("default".to_string()),
            language: Some("en-US".to_string()),
            speed: Some(1.0),
            pitch: Some(1.0),
            volume: Some(1.0),
            format: AudioFormat::Wav,
        };
        
        let key1 = service.generate_cache_key(&request1);
        let key2 = service.generate_cache_key(&request2);
        
        assert_eq!(key1, key2);
    }
}