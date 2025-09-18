use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;
use reqwest::Client;
use std::time::Duration;

/// Ollama server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaServerConfig {
    pub name: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub is_default: bool,
    pub is_local: bool,
    pub gpu_provider: Option<GpuProviderConfig>,
    pub tags: Vec<String>,
    pub max_concurrent_requests: usize,
    pub timeout_seconds: u64,
}

/// GPU provider configuration (e.g., Salad)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProviderConfig {
    pub provider_type: GpuProviderType,
    pub api_key: Option<String>,
    pub instance_type: Option<String>,
    pub cost_per_hour: Option<f64>,
    pub auto_start: bool,
    pub auto_stop_after_idle_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuProviderType {
    Salad,
    Vast,
    RunPod,
    Lambda,
    Custom,
}

/// Complete Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfiguration {
    pub servers: Vec<OllamaServerConfig>,
    pub selected_server: Option<String>,
    pub selected_model: Option<String>,
    pub auto_discover_local: bool,
    pub fallback_enabled: bool,
    pub model_preferences: HashMap<String, ModelPreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreference {
    pub preferred_server: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub context_window: u32,
}

impl Default for OllamaConfiguration {
    fn default() -> Self {
        Self {
            servers: vec![
                OllamaServerConfig {
                    name: "Local".to_string(),
                    endpoint: "http://localhost:11434".to_string(),
                    models: vec![],
                    is_default: true,
                    is_local: true,
                    gpu_provider: None,
                    tags: vec!["local".to_string(), "development".to_string()],
                    max_concurrent_requests: 1,
                    timeout_seconds: 300,
                }
            ],
            selected_server: Some("Local".to_string()),
            selected_model: Some("codellama:latest".to_string()),
            auto_discover_local: true,
            fallback_enabled: true,
            model_preferences: HashMap::new(),
        }
    }
}

/// Ollama configuration manager
pub struct OllamaConfigManager {
    config: OllamaConfiguration,
    config_path: PathBuf,
    client: Client,
}

impl OllamaConfigManager {
    /// Create a new configuration manager
    pub async fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_or_create_config(&config_path).await?;

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut manager = Self {
            config,
            config_path,
            client,
        };

        // Auto-discover local Ollama if enabled
        if manager.config.auto_discover_local {
            let _ = manager.discover_local_ollama().await;
        }

        Ok(manager)
    }

    /// Get the configuration file path
    fn get_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

        let config_dir = PathBuf::from(home).join(".sam").join("coding_agent");
        std::fs::create_dir_all(&config_dir)?;

        Ok(config_dir.join("ollama_config.json"))
    }

    /// Load or create configuration
    async fn load_or_create_config(path: &Path) -> Result<OllamaConfiguration> {
        if path.exists() {
            let content = fs::read_to_string(path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let config = OllamaConfiguration::default();
            let content = serde_json::to_string_pretty(&config)?;
            fs::write(path, content).await?;
            Ok(config)
        }
    }

    /// Save configuration
    pub async fn save_config(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content).await?;
        Ok(())
    }

    /// Add a new Ollama server
    pub async fn add_server(&mut self, mut server: OllamaServerConfig) -> Result<()> {
        // Test the connection first
        if !self.test_server_connection(&server.endpoint).await? {
            return Err(anyhow::anyhow!("Could not connect to server at {}", server.endpoint));
        }

        // Fetch available models
        server.models = self.fetch_server_models(&server.endpoint).await?;

        // If this is the first server or marked as default, set it as selected
        if self.config.servers.is_empty() || server.is_default {
            // Clear other defaults if this is the new default
            if server.is_default {
                for existing in &mut self.config.servers {
                    existing.is_default = false;
                }
            }
            self.config.selected_server = Some(server.name.clone());
        }

        // Check for duplicate names
        if self.config.servers.iter().any(|s| s.name == server.name) {
            return Err(anyhow::anyhow!("Server with name '{}' already exists", server.name));
        }

        self.config.servers.push(server);
        self.save_config().await?;
        Ok(())
    }

    /// Remove an Ollama server
    pub async fn remove_server(&mut self, name: &str) -> Result<()> {
        let initial_count = self.config.servers.len();
        self.config.servers.retain(|s| s.name != name);

        if self.config.servers.len() == initial_count {
            return Err(anyhow::anyhow!("Server '{}' not found", name));
        }

        // If we removed the selected server, select another
        if self.config.selected_server.as_deref() == Some(name) {
            self.config.selected_server = self.config.servers.first().map(|s| s.name.clone());
        }

        self.save_config().await?;
        Ok(())
    }

    /// Discover local Ollama installation
    pub async fn discover_local_ollama(&mut self) -> Result<bool> {
        let local_endpoints = vec![
            "http://localhost:11434",
            "http://127.0.0.1:11434",
            "http://0.0.0.0:11434",
        ];

        for endpoint in local_endpoints {
            if self.test_server_connection(endpoint).await? {
                // Check if we already have this server
                if !self.config.servers.iter().any(|s| s.endpoint == endpoint) {
                    let models = self.fetch_server_models(endpoint).await?;

                    let server = OllamaServerConfig {
                        name: "Local (Auto-discovered)".to_string(),
                        endpoint: endpoint.to_string(),
                        models,
                        is_default: self.config.servers.is_empty(),
                        is_local: true,
                        gpu_provider: None,
                        tags: vec!["local".to_string(), "auto-discovered".to_string()],
                        max_concurrent_requests: 1,
                        timeout_seconds: 300,
                    };

                    self.config.servers.push(server);
                    self.save_config().await?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Test connection to an Ollama server
    pub async fn test_server_connection(&self, endpoint: &str) -> Result<bool> {
        let url = format!("{}/api/tags", endpoint);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Fetch available models from a server
    pub async fn fetch_server_models(&self, endpoint: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", endpoint);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: serde_json::Value = response.json().await?;

        let models = data["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    /// Select a server
    pub async fn select_server(&mut self, name: &str) -> Result<()> {
        if !self.config.servers.iter().any(|s| s.name == name) {
            return Err(anyhow::anyhow!("Server '{}' not found", name));
        }

        self.config.selected_server = Some(name.to_string());

        // Auto-select first available model from this server
        if let Some(server) = self.config.servers.iter().find(|s| s.name == name) {
            if !server.models.is_empty() {
                self.config.selected_model = Some(server.models[0].clone());
            }
        }

        self.save_config().await?;
        Ok(())
    }

    /// Select a model
    pub async fn select_model(&mut self, model: &str) -> Result<()> {
        self.config.selected_model = Some(model.to_string());
        self.save_config().await?;
        Ok(())
    }

    /// Get current server configuration
    pub fn get_current_server(&self) -> Option<&OllamaServerConfig> {
        self.config.selected_server.as_ref().and_then(|name| {
            self.config.servers.iter().find(|s| s.name == *name)
        })
    }

    /// Get all servers
    pub fn get_servers(&self) -> &[OllamaServerConfig] {
        &self.config.servers
    }

    /// Get selected model
    pub fn get_selected_model(&self) -> Option<&str> {
        self.config.selected_model.as_deref()
    }

    /// Refresh models for a server
    pub async fn refresh_server_models(&mut self, server_name: &str) -> Result<Vec<String>> {
        let endpoint = self.config.servers.iter()
            .find(|s| s.name == server_name)
            .map(|s| s.endpoint.clone())
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        let models = self.fetch_server_models(&endpoint).await?;

        // Update the server's model list
        if let Some(server) = self.config.servers.iter_mut().find(|s| s.name == server_name) {
            server.models = models.clone();
        }

        self.save_config().await?;
        Ok(models)
    }

    /// Add model preference
    pub async fn add_model_preference(&mut self, model: String, preference: ModelPreference) -> Result<()> {
        self.config.model_preferences.insert(model, preference);
        self.save_config().await?;
        Ok(())
    }

    /// Get model preference
    pub fn get_model_preference(&self, model: &str) -> Option<&ModelPreference> {
        self.config.model_preferences.get(model)
    }

    /// Quick setup for common scenarios
    pub async fn quick_setup_remote_server(&mut self, ip: &str, port: u16, name: Option<String>) -> Result<()> {
        let endpoint = format!("http://{}:{}", ip, port);
        let name = name.unwrap_or_else(|| format!("Remote ({})", ip));

        let server = OllamaServerConfig {
            name,
            endpoint: endpoint.clone(),
            models: vec![],
            is_default: false,
            is_local: false,
            gpu_provider: None,
            tags: vec!["remote".to_string()],
            max_concurrent_requests: 4,
            timeout_seconds: 600,
        };

        self.add_server(server).await?;
        Ok(())
    }

    /// Setup Salad GPU provider
    pub async fn setup_salad_provider(
        &mut self,
        server_name: &str,
        api_key: String,
        instance_type: String,
        cost_per_hour: f64,
    ) -> Result<()> {
        if let Some(server) = self.config.servers.iter_mut().find(|s| s.name == server_name) {
            server.gpu_provider = Some(GpuProviderConfig {
                provider_type: GpuProviderType::Salad,
                api_key: Some(api_key),
                instance_type: Some(instance_type),
                cost_per_hour: Some(cost_per_hour),
                auto_start: true,
                auto_stop_after_idle_minutes: Some(30),
            });

            self.save_config().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Server '{}' not found", server_name))
        }
    }

    /// Get endpoint for current selection
    pub fn get_current_endpoint(&self) -> Option<String> {
        self.get_current_server().map(|s| s.endpoint.clone())
    }

    /// Get summary of configuration
    pub fn get_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("Configured Servers: {}\n", self.config.servers.len()));

        if let Some(current) = self.get_current_server() {
            summary.push_str(&format!("Current Server: {} ({})\n", current.name, current.endpoint));
            summary.push_str(&format!("Available Models: {}\n", current.models.len()));

            if let Some(gpu) = &current.gpu_provider {
                summary.push_str(&format!("GPU Provider: {:?}\n", gpu.provider_type));
                if let Some(cost) = gpu.cost_per_hour {
                    summary.push_str(&format!("Cost: ${:.2}/hour\n", cost));
                }
            }
        }

        if let Some(model) = &self.config.selected_model {
            summary.push_str(&format!("Selected Model: {}\n", model));
        }

        summary
    }
}

/// Interactive configuration builder
pub struct OllamaConfigBuilder {
    config: OllamaServerConfig,
}

impl OllamaConfigBuilder {
    pub fn new(name: String) -> Self {
        Self {
            config: OllamaServerConfig {
                name,
                endpoint: String::new(),
                models: vec![],
                is_default: false,
                is_local: false,
                gpu_provider: None,
                tags: vec![],
                max_concurrent_requests: 1,
                timeout_seconds: 300,
            }
        }
    }

    pub fn endpoint(mut self, endpoint: String) -> Self {
        self.config.endpoint = endpoint;
        self
    }

    pub fn is_default(mut self, is_default: bool) -> Self {
        self.config.is_default = is_default;
        self
    }

    pub fn is_local(mut self, is_local: bool) -> Self {
        self.config.is_local = is_local;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.config.tags = tags;
        self
    }

    pub fn with_salad_gpu(
        mut self,
        api_key: String,
        instance_type: String,
        cost_per_hour: f64,
    ) -> Self {
        self.config.gpu_provider = Some(GpuProviderConfig {
            provider_type: GpuProviderType::Salad,
            api_key: Some(api_key),
            instance_type: Some(instance_type),
            cost_per_hour: Some(cost_per_hour),
            auto_start: true,
            auto_stop_after_idle_minutes: Some(30),
        });
        self
    }

    pub fn max_concurrent_requests(mut self, max: usize) -> Self {
        self.config.max_concurrent_requests = max;
        self
    }

    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.config.timeout_seconds = timeout;
        self
    }

    pub fn build(self) -> OllamaServerConfig {
        self.config
    }
}