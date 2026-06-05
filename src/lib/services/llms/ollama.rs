use anyhow::{Context, Result};
use log::{debug, info};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;

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
    pub custom_endpoint: Option<String>, // Full URL endpoint (e.g., "http://172.16.0.125:11434")
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 11434,
            timeout_seconds: 300,
            custom_endpoint: None,
        }
    }
}

impl OllamaConfig {
    /// Create config from endpoint URL
    pub fn from_endpoint(endpoint: &str, timeout_seconds: u64) -> Self {
        Self {
            host: "127.0.0.1".to_string(), // Not used when custom_endpoint is set
            port: 11434,                   // Not used when custom_endpoint is set
            timeout_seconds,
            custom_endpoint: Some(endpoint.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
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
        if let Some(ref endpoint) = self.config.custom_endpoint {
            endpoint.clone()
        } else {
            format!("http://{}:{}", self.config.host, self.config.port)
        }
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
            _ => Err(anyhow::anyhow!("Unsupported operating system: {}", os)),
        }
    }

    async fn install_macos(&self) -> Result<String> {
        info!("Installing Ollama on macOS...");

        // Check if Homebrew is available
        if AsyncCommand::new("brew")
            .arg("--version")
            .output()
            .await
            .is_ok()
        {
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
                return Err(anyhow::anyhow!(
                    "Failed to download install script: {}",
                    stderr
                ));
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

            let status = child
                .wait()
                .await
                .context("Failed to wait for install script")?;

            if !status.success() {
                return Err(anyhow::anyhow!(
                    "Install script failed with status: {}",
                    status
                ));
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
            return Err(anyhow::anyhow!(
                "Failed to download install script: {}",
                stderr
            ));
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

        let status = child
            .wait()
            .await
            .context("Failed to wait for install script")?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Install script failed with status: {}",
                status
            ));
        }

        Ok("Ollama installed successfully on Linux".to_string())
    }

    async fn install_windows(&self) -> Result<String> {
        info!("Installing Ollama on Windows...");

        // Download the Windows installer
        let installer_url = "https://ollama.ai/download/OllamaSetup.exe";
        let installer_path = std::env::temp_dir().join("OllamaSetup.exe");

        info!("Downloading Ollama installer...");
        let response = self
            .client
            .get(installer_url)
            .send()
            .await
            .context("Failed to download Ollama installer")?;

        let bytes = response
            .bytes()
            .await
            .context("Failed to read installer bytes")?;

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
                }
                _ => Err(anyhow::anyhow!("Failed to stop Ollama service")),
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
                }
                _ => Err(anyhow::anyhow!("Failed to stop Ollama service")),
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

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get version from Ollama API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "API request failed with status: {}",
                response.status()
            ));
        }

        let version_info = response
            .text()
            .await
            .context("Failed to read version response")?;
        Ok(version_info)
    }

    /// List all installed models
    pub async fn list_models(&self) -> Result<OllamaListResponse> {
        let url = format!("{}/api/tags", self.base_url());

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list models")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to list models: {}",
                response.status()
            ));
        }

        let models = response
            .json::<OllamaListResponse>()
            .await
            .context("Failed to parse models response")?;

        Ok(models)
    }

    /// Get both installed and available models
    pub async fn list_all_models(&self) -> Result<(Vec<OllamaModel>, Vec<String>)> {
        let installed_models = match self.list_models().await {
            Ok(response) => response.models,
            Err(_) => Vec::new(),
        };

        let available_models = self.search_models("").await.unwrap_or_else(|_| Vec::new());

        Ok((installed_models, available_models))
    }

    /// Pull/download a model
    pub async fn pull_model(&self, model_name: &str) -> Result<String> {
        info!("Pulling model: {}", model_name);

        let url = format!("{}/api/pull", self.base_url());
        let request = OllamaPullRequest {
            name: model_name.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to pull model")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to pull model: {}",
                response.status()
            ));
        }

        let pull_response = response
            .json::<OllamaPullResponse>()
            .await
            .context("Failed to parse pull response")?;

        Ok(format!(
            "Model {} pulled successfully: {}",
            model_name, pull_response.status
        ))
    }

    /// Remove a model
    pub async fn remove_model(&self, model_name: &str) -> Result<String> {
        info!("Removing model: {}", model_name);

        let url = format!("{}/api/delete", self.base_url());
        let request = serde_json::json!({
            "name": model_name
        });

        let response = self
            .client
            .delete(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to remove model")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to remove model: {}",
                response.status()
            ));
        }

        Ok(format!("Model {} removed successfully", model_name))
    }

    /// Generate text using a model
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<OllamaGenerateResponse> {
        debug!("Generating text with model: {}, prompt: {}", model, prompt);

        let url = format!("{}/api/generate", self.base_url());
        let request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to generate text")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("Generation failed: {}", error_text));
        }

        let generate_response = response
            .json::<OllamaGenerateResponse>()
            .await
            .context("Failed to parse generation response")?;

        Ok(generate_response)
    }

    /// Generate text with streaming response
    pub async fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        options: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<impl futures::Stream<Item = Result<OllamaGenerateResponse>>> {
        let url = format!("{}/api/generate", self.base_url());
        let request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: true,
            options,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to start streaming generation")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Streaming request failed: {}",
                response.status()
            ));
        }

        use futures_util::StreamExt;

        let stream = response.bytes_stream().map(|chunk| {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse each line as JSON
                    for line in text.lines() {
                        if !line.trim().is_empty() {
                            match serde_json::from_str::<OllamaGenerateResponse>(line) {
                                Ok(response) => return Ok(response),
                                Err(e) => {
                                    return Err(anyhow::anyhow!(
                                        "Failed to parse streaming response: {}",
                                        e
                                    ))
                                }
                            }
                        }
                    }
                    Err(anyhow::anyhow!("No valid JSON in chunk"))
                }
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            }
        });

        Ok(stream)
    }

    /// Get available model tags from Ollama library
    pub async fn search_models(&self, query: &str) -> Result<Vec<String>> {
        // Comprehensive list of popular and available Ollama models
        let available_models = vec![
            // Llama models
            "llama3.2:latest".to_string(),
            "llama3.2:1b".to_string(),
            "llama3.2:3b".to_string(),
            "llama3.1:latest".to_string(),
            "llama3.1:8b".to_string(),
            "llama3.1:70b".to_string(),
            "llama3:latest".to_string(),
            "llama3:8b".to_string(),
            "llama3:70b".to_string(),
            "llama2:latest".to_string(),
            "llama2:7b".to_string(),
            "llama2:13b".to_string(),
            "llama2:70b".to_string(),
            // Code models
            "codellama:latest".to_string(),
            "codellama:7b".to_string(),
            "codellama:13b".to_string(),
            "codellama:34b".to_string(),
            "codeqwen:latest".to_string(),
            "codegemma:latest".to_string(),
            "codegemma:2b".to_string(),
            "codegemma:7b".to_string(),
            // Mistral models
            "mistral:latest".to_string(),
            "mistral:7b".to_string(),
            "mistral-nemo:latest".to_string(),
            "mixtral:latest".to_string(),
            "mixtral:8x7b".to_string(),
            "mixtral:8x22b".to_string(),
            "mistrallite:latest".to_string(),
            // Gemma models
            "gemma:latest".to_string(),
            "gemma:2b".to_string(),
            "gemma:7b".to_string(),
            "gemma2:latest".to_string(),
            "gemma2:2b".to_string(),
            "gemma2:9b".to_string(),
            "gemma2:27b".to_string(),
            "gemma3:latest".to_string(),
            "gemma3:1b".to_string(),
            "gemma3:27b".to_string(),
            // Specialized models
            "qwen2:latest".to_string(),
            "qwen2:0.5b".to_string(),
            "qwen2:1.5b".to_string(),
            "qwen2:7b".to_string(),
            "qwen2:72b".to_string(),
            "phi3:latest".to_string(),
            "phi3:mini".to_string(),
            "phi3:medium".to_string(),
            "deepseek-coder:latest".to_string(),
            "deepseek-coder:1.3b".to_string(),
            "deepseek-coder:6.7b".to_string(),
            "deepseek-coder:33b".to_string(),
            "deepseek-r1:latest".to_string(),
            "deepseek-r1:1.5b".to_string(),
            "deepseek-r1:7b".to_string(),
            "deepseek-r1:8b".to_string(),
            "deepseek-r1:14b".to_string(),
            "deepseek-r1:32b".to_string(),
            "deepseek-r1:70b".to_string(),
            // Chat and assistant models
            "openchat:latest".to_string(),
            "orca-mini:latest".to_string(),
            "vicuna:latest".to_string(),
            "vicuna:7b".to_string(),
            "vicuna:13b".to_string(),
            "wizardcoder:latest".to_string(),
            "wizardlm:latest".to_string(),
            "neural-chat:latest".to_string(),
            "starling-lm:latest".to_string(),
            "solar:latest".to_string(),
            "yi:latest".to_string(),
            "yi:6b".to_string(),
            "yi:34b".to_string(),
            // Function calling models
            "llama3-groq-tool-use:latest".to_string(),
            "mistral-nemo:12b".to_string(),
            // Embedding models
            "nomic-embed-text:latest".to_string(),
            "all-minilm:latest".to_string(),
            // Vision models
            "llava:latest".to_string(),
            "llava:7b".to_string(),
            "llava:13b".to_string(),
            "llava:34b".to_string(),
            "bakllava:latest".to_string(),
            // Math and reasoning
            "mathstral:latest".to_string(),
            "wizardmath:latest".to_string(),
        ];

        let filtered_models: Vec<String> = available_models
            .into_iter()
            .filter(|model| {
                query.is_empty() || model.to_lowercase().contains(&query.to_lowercase())
            })
            .collect();

        Ok(filtered_models)
    }

    /// Get model information
    pub async fn show_model(&self, model_name: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/show", self.base_url());
        let request = serde_json::json!({
            "name": model_name
        });

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to get model info")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get model info: {}",
                response.status()
            ));
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

    /// Get model names in a formatted display with sizes
    pub async fn get_formatted_model_list(&self) -> Result<Vec<String>> {
        let models = self.list_models().await?;
        let mut formatted = Vec::new();

        for model in models.models {
            let size_gb = model.size as f64 / (1024.0 * 1024.0 * 1024.0);
            formatted.push(format!("{} ({:.1} GB)", model.name, size_gb));
        }

        Ok(formatted)
    }

    /// Install a recommended set of models
    pub async fn install_recommended_models(&self) -> Result<String> {
        let recommended = vec![
            "llama3.2:latest",
            "codellama:latest",
            "mistral:latest",
            "gemma2:2b",
            "phi3:mini",
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
