use std::collections::HashMap;
use std::process::Stdio;
use anyhow::{Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;
use log::{info, debug};

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: OllamaModelDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaListResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaGenerateRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaGenerateResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaPullRequest {
    pub name: String,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaPullResponse {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub host: String,
    pub port: u16,
    pub timeout_seconds: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 11434,
            timeout_seconds: 300,
        }
    }
}

#[derive(Clone)]
pub struct OllamaService {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl OllamaService {
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, client }
    }

    pub fn new_with_defaults() -> Self {
        Self::new(OllamaConfig::default())
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }

    /// Check if Ollama is installed on the system
    pub async fn is_installed(&self) -> bool {
        self.check_ollama_command().await.is_ok()
    }

    /// Check if Ollama service is running
    pub async fn is_running(&self) -> bool {
        match self.get_version().await {
            Ok(_) => true,
            Err(e) => {
                debug!("Ollama service check failed: {}", e);
                false
            }
        }
    }

    /// Install Ollama automatically based on the operating system
    pub async fn install(&self) -> Result<String> {
        info!("Installing Ollama...");
        
        let os = std::env::consts::OS;
        match os {
            "macos" => self.install_macos().await,
            "linux" => self.install_linux().await,
            "windows" => self.install_windows().await,
            _ => Err(anyhow::anyhow!("Unsupported operating system: {}", os))
        }
    }

    async fn install_macos(&self) -> Result<String> {
        info!("Installing Ollama on macOS...");
        
        // Check if Homebrew is available
        if AsyncCommand::new("brew").arg("--version").output().await.is_ok() {
            info!("Using Homebrew to install Ollama...");
            let output = AsyncCommand::new("brew")
                .arg("install")
                .arg("ollama")
                .output()
                .await
                .context("Failed to run brew install")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Brew install failed: {}", stderr));
            }

            Ok("Ollama installed successfully via Homebrew".to_string())
        } else {
            // Fallback to curl installation
            info!("Using curl to install Ollama...");
            let output = AsyncCommand::new("curl")
                .arg("-fsSL")
                .arg("https://ollama.ai/install.sh")
                .stdout(Stdio::piped())
                .output()
                .await
                .context("Failed to download Ollama install script")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to download install script: {}", stderr));
            }

            let install_script = String::from_utf8_lossy(&output.stdout);
            
            // Execute the install script
            let mut child = AsyncCommand::new("sh")
                .arg("-c")
                .arg(&*install_script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Failed to execute install script")?;

            let status = child.wait().await.context("Failed to wait for install script")?;
            
            if !status.success() {
                return Err(anyhow::anyhow!("Install script failed with status: {}", status));
            }

            Ok("Ollama installed successfully via install script".to_string())
        }
    }

    async fn install_linux(&self) -> Result<String> {
        info!("Installing Ollama on Linux...");
        
        let output = AsyncCommand::new("curl")
            .arg("-fsSL")
            .arg("https://ollama.ai/install.sh")
            .stdout(Stdio::piped())
            .output()
            .await
            .context("Failed to download Ollama install script")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to download install script: {}", stderr));
        }

        let install_script = String::from_utf8_lossy(&output.stdout);
        
        // Execute the install script
        let mut child = AsyncCommand::new("sh")
            .arg("-c")
            .arg(&*install_script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to execute install script")?;

        let status = child.wait().await.context("Failed to wait for install script")?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Install script failed with status: {}", status));
        }

        Ok("Ollama installed successfully on Linux".to_string())
    }

    async fn install_windows(&self) -> Result<String> {
        info!("Installing Ollama on Windows...");
        
        // Download the Windows installer
        let installer_url = "https://ollama.ai/download/OllamaSetup.exe";
        let installer_path = std::env::temp_dir().join("OllamaSetup.exe");
        
        info!("Downloading Ollama installer...");
        let response = self.client
            .get(installer_url)
            .send()
            .await
            .context("Failed to download Ollama installer")?;

        let bytes = response.bytes().await.context("Failed to read installer bytes")?;
        
        tokio::fs::write(&installer_path, &bytes)
            .await
            .context("Failed to save installer")?;

        info!("Running Ollama installer...");
        let output = AsyncCommand::new(&installer_path)
            .arg("/S") // Silent install
            .output()
            .await
            .context("Failed to run installer")?;

        // Clean up installer file
        let _ = tokio::fs::remove_file(&installer_path).await;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Installer failed: {}", stderr));
        }

        Ok("Ollama installed successfully on Windows".to_string())
    }

    /// Start the Ollama service
    pub async fn start_service(&self) -> Result<String> {
        info!("Starting Ollama service...");
        
        if self.is_running().await {
            return Ok("Ollama service is already running".to_string());
        }

        let mut child = AsyncCommand::new("ollama")
            .arg("serve")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start Ollama service")?;

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Check if it's now running
        if self.is_running().await {
            Ok("Ollama service started successfully".to_string())
        } else {
            Err(anyhow::anyhow!("Failed to start Ollama service"))
        }
    }

    /// Stop the Ollama service (platform specific)
    pub async fn stop_service(&self) -> Result<String> {
        info!("Stopping Ollama service...");
        
        #[cfg(unix)]
        {
            let output = AsyncCommand::new("pkill")
                .arg("-f")
                .arg("ollama")
                .output()
                .await;
            
            match output {
                Ok(output) if output.status.success() => {
                    Ok("Ollama service stopped successfully".to_string())
                },
                _ => Err(anyhow::anyhow!("Failed to stop Ollama service"))
            }
        }
        
        #[cfg(windows)]
        {
            let output = AsyncCommand::new("taskkill")
                .arg("/F")
                .arg("/IM")
                .arg("ollama.exe")
                .output()
                .await;
            
            match output {
                Ok(output) if output.status.success() => {
                    Ok("Ollama service stopped successfully".to_string())
                },
                _ => Err(anyhow::anyhow!("Failed to stop Ollama service"))
            }
        }
    }

    async fn check_ollama_command(&self) -> Result<()> {
        let output = AsyncCommand::new("ollama")
            .arg("--version")
            .output()
            .await
            .context("Failed to check ollama command")?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Ollama command not found"))
        }
    }

    /// Get Ollama version information
    pub async fn get_version(&self) -> Result<String> {
        let url = format!("{}/api/version", self.base_url());
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to get version from Ollama API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API request failed with status: {}", response.status()));
        }

        let version_info = response.text().await.context("Failed to read version response")?;
        Ok(version_info)
    }

    /// List all available models
    pub async fn list_models(&self) -> Result<OllamaListResponse> {
        let url = format!("{}/api/tags", self.base_url());
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to list models")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list models: {}", response.status()));
        }

        let models = response
            .json::<OllamaListResponse>()
            .await
            .context("Failed to parse models response")?;

        Ok(models)
    }

    /// Pull/download a model
    pub async fn pull_model(&self, model_name: &str) -> Result<String> {
        info!("Pulling model: {}", model_name);
        
        let url = format!("{}/api/pull", self.base_url());
        let request = OllamaPullRequest {
            name: model_name.to_string(),
            stream: false,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to pull model")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to pull model: {}", response.status()));
        }

        let pull_response = response
            .json::<OllamaPullResponse>()
            .await
            .context("Failed to parse pull response")?;

        Ok(format!("Model {} pulled successfully: {}", model_name, pull_response.status))
    }

    /// Remove a model
    pub async fn remove_model(&self, model_name: &str) -> Result<String> {
        info!("Removing model: {}", model_name);
        
        let url = format!("{}/api/delete", self.base_url());
        let request = serde_json::json!({
            "name": model_name
        });

        let response = self.client
            .delete(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to remove model")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to remove model: {}", response.status()));
        }

        Ok(format!("Model {} removed successfully", model_name))
    }

    /// Generate text using a model
    pub async fn generate(&self, model: &str, prompt: &str, options: Option<HashMap<String, serde_json::Value>>) -> Result<OllamaGenerateResponse> {
        debug!("Generating text with model: {}, prompt: {}", model, prompt);
        
        let url = format!("{}/api/generate", self.base_url());
        let request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to generate text")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("Generation failed: {}", error_text));
        }

        let generate_response = response
            .json::<OllamaGenerateResponse>()
            .await
            .context("Failed to parse generation response")?;

        Ok(generate_response)
    }

    /// Generate text with streaming response
    pub async fn generate_stream(&self, model: &str, prompt: &str, options: Option<HashMap<String, serde_json::Value>>) -> Result<impl futures::Stream<Item = Result<OllamaGenerateResponse>>> {
        let url = format!("{}/api/generate", self.base_url());
        let request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: true,
            options,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to start streaming generation")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Streaming request failed: {}", response.status()));
        }

        use futures_util::StreamExt;
        
        let stream = response.bytes_stream()
            .map(|chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // Parse each line as JSON
                        for line in text.lines() {
                            if !line.trim().is_empty() {
                                match serde_json::from_str::<OllamaGenerateResponse>(line) {
                                    Ok(response) => return Ok(response),
                                    Err(e) => return Err(anyhow::anyhow!("Failed to parse streaming response: {}", e)),
                                }
                            }
                        }
                        Err(anyhow::anyhow!("No valid JSON in chunk"))
                    },
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e))
                }
            });

        Ok(stream)
    }

    /// Get available model tags from Ollama library
    pub async fn search_models(&self, query: &str) -> Result<Vec<String>> {
        // This would typically call the Ollama library API, but for now we return popular models
        let popular_models = vec![
            "llama3.2:latest".to_string(),
            "llama3.1:latest".to_string(),
            "llama3:latest".to_string(),
            "llama2:latest".to_string(),
            "mistral:latest".to_string(),
            "codellama:latest".to_string(),
            "vicuna:latest".to_string(),
            "wizardcoder:latest".to_string(),
            "neural-chat:latest".to_string(),
            "starling-lm:latest".to_string(),
        ];
        
        let filtered_models: Vec<String> = popular_models
            .into_iter()
            .filter(|model| model.to_lowercase().contains(&query.to_lowercase()))
            .collect();
            
        Ok(filtered_models)
    }

    /// Get model information
    pub async fn show_model(&self, model_name: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/show", self.base_url());
        let request = serde_json::json!({
            "name": model_name
        });

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to get model info")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get model info: {}", response.status()));
        }

        let model_info = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse model info response")?;

        Ok(model_info)
    }
}

// Convenience functions for common operations
impl OllamaService {
    /// Quick text generation with default options
    pub async fn quick_generate(&self, model: &str, prompt: &str) -> Result<String> {
        let response = self.generate(model, prompt, None).await?;
        Ok(response.response)
    }

    /// Check if a specific model is installed
    pub async fn is_model_installed(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.models.iter().any(|m| m.name == model_name))
    }

    /// Get installed model names only
    pub async fn get_installed_model_names(&self) -> Result<Vec<String>> {
        let models = self.list_models().await?;
        Ok(models.models.into_iter().map(|m| m.name).collect())
    }

    /// Install a recommended set of models
    pub async fn install_recommended_models(&self) -> Result<String> {
        let recommended = vec![
            "llama3.2:latest",
            "codellama:latest",
            "mistral:latest"
        ];

        let mut results = Vec::new();
        for model in recommended {
            match self.pull_model(model).await {
                Ok(msg) => results.push(msg),
                Err(e) => results.push(format!("Failed to install {}: {}", model, e)),
            }
        }

        Ok(results.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_service_creation() {
        let service = OllamaService::new_with_defaults();
        assert_eq!(service.config.host, "127.0.0.1");
        assert_eq!(service.config.port, 11434);
    }

    #[tokio::test]
    async fn test_base_url() {
        let service = OllamaService::new_with_defaults();
        assert_eq!(service.base_url(), "http://127.0.0.1:11434");
    }
}