use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// GPU provider types for offloading LLM inference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuProvider {
    Ollama, // Local/remote Ollama instance (e.g., your GPU desktop rig)
    Salad,
    Vast,
    RunPod,
    LambdaLabs,
    Custom(String),
}

/// GPU instance specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInstanceSpec {
    pub gpu_type: String,         // e.g., "RTX 4090", "A100", "H100"
    pub vram_gb: u32,             // Video RAM in GB
    pub cpu_cores: u32,           // Number of CPU cores
    pub ram_gb: u32,              // System RAM in GB
    pub cost_per_hour: f64,       // Cost in USD per hour
    pub min_billing_minutes: u32, // Minimum billing period in minutes
}

/// GPU instance state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceState {
    Provisioning,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

/// Active GPU instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInstance {
    pub id: String,
    pub provider: GpuProvider,
    pub spec: GpuInstanceSpec,
    pub state: InstanceState,
    pub endpoint: String,             // Ollama API endpoint
    pub ssh_endpoint: Option<String>, // SSH access if available
    pub api_key: Option<String>,
    pub started_at: SystemTime,
    pub stopped_at: Option<SystemTime>,
    pub session_id: String,
    pub container_id: Option<String>,
    pub region: String,
}

/// Cost tracking for GPU usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTracker {
    pub session_id: String,
    pub total_cost: f64,
    pub runtime_minutes: u64,
    pub gpu_hours: f64,
    pub cost_per_hour: f64,
    pub budget_limit: Option<f64>,
    pub budget_alert_threshold: f64, // Percentage (0.0-1.0)
}

/// Ollama API client for local/remote GPU desktop rigs
pub struct OllamaClient {
    client: Client,
    base_url: String,
    api_key: Option<String>, // Optional for authentication
}

impl OllamaClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    /// Check if Ollama is running and available
    pub async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list models"));
        }

        let data: serde_json::Value = response.json().await?;
        let models = data["models"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| m["name"].as_str().map(String::from))
            .collect();

        Ok(models)
    }

    /// Pull a model if not available
    pub async fn pull_model(&self, model_name: &str) -> Result<()> {
        let request_body = serde_json::json!({
            "name": model_name,
            "stream": false
        });

        let mut request = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&request_body);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to pull model: {}", error_text));
        }

        Ok(())
    }

    /// Generate completion using Ollama
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<serde_json::Value>,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": options.unwrap_or(serde_json::json!({
                "temperature": 0.7,
                "num_gpu": 99,  // Use all available GPUs
                "num_thread": 8,
            }))
        });

        let mut request = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request_body);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to generate: {}", error_text));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data["response"].as_str().unwrap_or("").to_string())
    }

    /// Chat completion using Ollama
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        options: Option<serde_json::Value>,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": options.unwrap_or(serde_json::json!({
                "temperature": 0.7,
                "num_gpu": 99,  // Use all available GPUs
                "num_thread": 8,
            }))
        });

        let mut request = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request_body);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to chat: {}", error_text));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Get GPU information from Ollama instance
    pub async fn get_gpu_info(&self) -> Result<serde_json::Value> {
        let response = self
            .client
            .get(format!("{}/api/ps", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get GPU info"));
        }

        response.json().await.map_err(Into::into)
    }
}

/// Salad.com API client
pub struct SaladClient {
    api_key: String,
    client: Client,
    base_url: String,
}

impl SaladClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            base_url: "https://api.salad.com/v1".to_string(),
        }
    }

    /// Create a new GPU container instance
    pub async fn create_instance(&self, spec: &GpuInstanceSpec) -> Result<GpuInstance> {
        let request_body = serde_json::json!({
            "name": format!("ollama-gpu-{}", uuid::Uuid::new_v4()),
            "image": "ollama/ollama:latest",
            "gpu_type": spec.gpu_type,
            "cpu": spec.cpu_cores,
            "memory": spec.ram_gb * 1024, // Convert to MB
            "gpu_count": 1,
            "env": {
                "OLLAMA_HOST": "0.0.0.0:11434"
            },
            "ports": {
                "11434": "http"
            },
            "command": ["serve"],
            "replicas": 1,
            "auto_scale": false
        });

        let response = self
            .client
            .post(format!("{}/containers", self.base_url))
            .header("API-Key", &self.api_key)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create instance: {}", error_text));
        }

        let response_data: serde_json::Value = response.json().await?;

        Ok(GpuInstance {
            id: response_data["id"].as_str().unwrap_or_default().to_string(),
            provider: GpuProvider::Salad,
            spec: spec.clone(),
            state: InstanceState::Provisioning,
            endpoint: response_data["endpoint"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            ssh_endpoint: response_data
                .get("ssh_endpoint")
                .and_then(|s| s.as_str())
                .map(String::from),
            api_key: Some(self.api_key.clone()),
            started_at: SystemTime::now(),
            stopped_at: None,
            session_id: uuid::Uuid::new_v4().to_string(),
            container_id: Some(
                response_data["container_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            ),
            region: response_data["region"]
                .as_str()
                .unwrap_or("us-east")
                .to_string(),
        })
    }

    /// Stop and delete a GPU instance
    pub async fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let response = self
            .client
            .delete(format!("{}/containers/{}", self.base_url, instance_id))
            .header("API-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to delete instance: {}", error_text));
        }

        Ok(())
    }

    /// Get instance status
    pub async fn get_instance_status(&self, instance_id: &str) -> Result<InstanceState> {
        let response = self
            .client
            .get(format!("{}/containers/{}", self.base_url, instance_id))
            .header("API-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(InstanceState::Failed("Instance not found".to_string()));
        }

        let data: serde_json::Value = response.json().await?;
        let status = data["status"].as_str().unwrap_or("unknown");

        Ok(match status {
            "pending" | "creating" => InstanceState::Provisioning,
            "running" | "active" => InstanceState::Running,
            "stopping" => InstanceState::Stopping,
            "stopped" | "terminated" => InstanceState::Stopped,
            _ => InstanceState::Failed(format!("Unknown status: {}", status)),
        })
    }

    /// Wait for instance to be ready
    pub async fn wait_for_ready(&self, instance_id: &str, timeout: Duration) -> Result<()> {
        let start = SystemTime::now();

        loop {
            if SystemTime::now().duration_since(start)? > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for instance to be ready"));
            }

            match self.get_instance_status(instance_id).await? {
                InstanceState::Running => return Ok(()),
                InstanceState::Failed(err) => {
                    return Err(anyhow::anyhow!("Instance failed: {}", err))
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

/// GPU offload manager for handling remote GPU instances
pub struct GpuOffloadManager {
    instances: Arc<RwLock<HashMap<String, GpuInstance>>>,
    cost_trackers: Arc<RwLock<HashMap<String, CostTracker>>>,
    salad_client: Option<Arc<SaladClient>>,
    ollama_client: Option<Arc<OllamaClient>>,
    config: GpuOffloadConfig,
}

/// Configuration for GPU offloading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuOffloadConfig {
    pub enabled: bool,
    pub provider: GpuProvider,
    pub auto_scale: bool,
    pub max_instances: u32,
    pub budget_limit: Option<f64>,
    pub budget_alert_threshold: f64,
    pub idle_timeout_minutes: u32,
    pub preferred_gpu_types: Vec<String>,
    pub preferred_regions: Vec<String>,
    pub min_vram_gb: u32,
    // Ollama-specific configuration
    pub ollama_endpoint: Option<String>, // e.g., "http://192.168.1.100:11434"
    pub ollama_api_key: Option<String>,  // Optional authentication
    pub ollama_models: Vec<String>,      // Preferred models to use
    pub ollama_gpu_layers: Option<u32>,  // Number of layers to offload to GPU
}

impl Default for GpuOffloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: GpuProvider::Ollama, // Default to Ollama for local GPU rigs
            auto_scale: true,
            max_instances: 1,
            budget_limit: Some(10.0),    // $10 default limit
            budget_alert_threshold: 0.8, // Alert at 80% of budget
            idle_timeout_minutes: 30,
            preferred_gpu_types: vec!["RTX 4090".to_string(), "RTX 3090".to_string()],
            preferred_regions: vec!["us-east".to_string(), "us-west".to_string()],
            min_vram_gb: 24,
            // Ollama defaults
            ollama_endpoint: std::env::var("OLLAMA_HOST")
                .ok()
                .or_else(|| Some("http://localhost:11434".to_string())),
            ollama_api_key: std::env::var("OLLAMA_API_KEY").ok(),
            ollama_models: vec![
                "deepseek-coder:33b".to_string(),
                "codellama:34b".to_string(),
                "llama3:70b".to_string(),
                "mixtral:8x7b".to_string(),
            ],
            ollama_gpu_layers: Some(99), // Use all available GPU layers
        }
    }
}

impl GpuOffloadManager {
    pub fn new(config: GpuOffloadConfig) -> Self {
        let salad_client = if config.provider == GpuProvider::Salad {
            std::env::var("SALAD_API_KEY")
                .ok()
                .map(|key| Arc::new(SaladClient::new(key)))
        } else {
            None
        };

        let ollama_client = if config.provider == GpuProvider::Ollama {
            config.ollama_endpoint.as_ref().map(|endpoint| {
                Arc::new(OllamaClient::new(
                    endpoint.clone(),
                    config.ollama_api_key.clone(),
                ))
            })
        } else {
            None
        };

        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            cost_trackers: Arc::new(RwLock::new(HashMap::new())),
            salad_client,
            ollama_client,
            config,
        }
    }

    /// Start a new GPU instance for a coding session
    pub async fn start_gpu_instance(&self, session_id: &str) -> Result<GpuInstance> {
        if !self.config.enabled {
            return Err(anyhow::anyhow!("GPU offloading is not enabled"));
        }

        // Check if we already have an instance for this session
        let instances = self.instances.read().await;
        if let Some(existing) = instances.get(session_id) {
            if matches!(
                existing.state,
                InstanceState::Running | InstanceState::Provisioning
            ) {
                return Ok(existing.clone());
            }
        }
        drop(instances);

        // Check budget limits
        if let Some(budget_limit) = self.config.budget_limit {
            let trackers = self.cost_trackers.read().await;
            let total_cost: f64 = trackers.values().map(|t| t.total_cost).sum();
            if total_cost >= budget_limit {
                return Err(anyhow::anyhow!(
                    "Budget limit exceeded: ${:.2} >= ${:.2}",
                    total_cost,
                    budget_limit
                ));
            }
        }

        // Select best GPU type based on preferences and availability
        let gpu_spec = self.select_best_gpu_spec().await?;

        // Create instance based on provider
        let instance = match &self.config.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    // Check if Ollama is running
                    if !client.health_check().await? {
                        return Err(anyhow::anyhow!(
                            "Ollama is not running at {}",
                            self.config
                                .ollama_endpoint
                                .as_ref()
                                .unwrap_or(&"unknown".to_string())
                        ));
                    }

                    // Get GPU info
                    let gpu_info = client.get_gpu_info().await?;

                    // Ensure preferred models are available
                    let available_models = client.list_models().await?;
                    let mut model_to_use = None;

                    for preferred_model in &self.config.ollama_models {
                        if available_models.iter().any(|m| m.contains(preferred_model)) {
                            model_to_use = Some(preferred_model.clone());
                            break;
                        }
                    }

                    // Pull the first preferred model if none are available
                    if model_to_use.is_none() && !self.config.ollama_models.is_empty() {
                        let first_model = &self.config.ollama_models[0];
                        log::info!("Pulling model {} for GPU offload", first_model);
                        client.pull_model(first_model).await?;
                        model_to_use = Some(first_model.clone());
                    }

                    // Create a virtual instance representation for Ollama
                    GpuInstance {
                        id: format!("ollama-{}", uuid::Uuid::new_v4()),
                        provider: GpuProvider::Ollama,
                        spec: gpu_spec.clone(),
                        state: InstanceState::Running,
                        endpoint: self
                            .config
                            .ollama_endpoint
                            .clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string()),
                        ssh_endpoint: None,
                        api_key: self.config.ollama_api_key.clone(),
                        started_at: SystemTime::now(),
                        stopped_at: None,
                        session_id: session_id.to_string(),
                        container_id: None,
                        region: "local".to_string(),
                    }
                } else {
                    return Err(anyhow::anyhow!("Ollama endpoint not configured"));
                }
            }
            GpuProvider::Salad => {
                if let Some(client) = &self.salad_client {
                    let mut instance = client.create_instance(&gpu_spec).await?;
                    instance.session_id = session_id.to_string();

                    // Wait for instance to be ready
                    client
                        .wait_for_ready(&instance.id, Duration::from_secs(300))
                        .await?;
                    instance.state = InstanceState::Running;

                    instance
                } else {
                    return Err(anyhow::anyhow!("Salad API key not configured"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Provider {:?} not yet implemented",
                    self.config.provider
                ))
            }
        };

        // Store instance and create cost tracker
        // Use separate scopes to avoid holding multiple write locks
        {
            let mut instances = self.instances.write().await;
            instances.insert(session_id.to_string(), instance.clone());
        }

        {
            let mut trackers = self.cost_trackers.write().await;
            trackers.insert(
                session_id.to_string(),
                CostTracker {
                    session_id: session_id.to_string(),
                    total_cost: 0.0,
                    runtime_minutes: 0,
                    gpu_hours: 0.0,
                    cost_per_hour: gpu_spec.cost_per_hour,
                    budget_limit: self.config.budget_limit,
                    budget_alert_threshold: self.config.budget_alert_threshold,
                },
            );
        }

        log::info!(
            "Started GPU instance {} for session {} at ${}/hour",
            instance.id,
            session_id,
            gpu_spec.cost_per_hour
        );

        Ok(instance)
    }

    /// Stop and cleanup GPU instance for a session
    pub async fn stop_gpu_instance(&self, session_id: &str) -> Result<()> {
        let mut instances = self.instances.write().await;

        if let Some(mut instance) = instances.remove(session_id) {
            instance.stopped_at = Some(SystemTime::now());

            // Calculate final cost
            if let Some(started) = SystemTime::UNIX_EPOCH
                .checked_add(instance.started_at.duration_since(SystemTime::UNIX_EPOCH)?)
            {
                let runtime = SystemTime::now().duration_since(started)?;
                let runtime_minutes = runtime.as_secs() / 60;
                let billable_minutes =
                    runtime_minutes.max(instance.spec.min_billing_minutes as u64);
                let cost = (billable_minutes as f64 / 60.0) * instance.spec.cost_per_hour;

                let mut trackers = self.cost_trackers.write().await;
                if let Some(tracker) = trackers.get_mut(session_id) {
                    tracker.runtime_minutes = runtime_minutes;
                    tracker.gpu_hours = billable_minutes as f64 / 60.0;
                    tracker.total_cost = cost;

                    log::info!(
                        "Session {} GPU usage: {} minutes, cost: ${:.2}",
                        session_id,
                        runtime_minutes,
                        cost
                    );
                }
            }

            // Delete the instance
            match &self.config.provider {
                GpuProvider::Salad => {
                    if let Some(client) = &self.salad_client {
                        client.delete_instance(&instance.id).await?;
                    }
                }
                _ => {}
            }

            instance.state = InstanceState::Stopped;
        }

        Ok(())
    }

    /// Get Ollama endpoint for a session
    pub async fn get_ollama_endpoint(&self, session_id: &str) -> Option<String> {
        let instances = self.instances.read().await;
        instances
            .get(session_id)
            .filter(|i| matches!(i.state, InstanceState::Running))
            .map(|i| i.endpoint.clone())
    }

    /// Update cost tracking for active sessions
    pub async fn update_cost_tracking(&self) {
        let instances = self.instances.read().await;
        let mut trackers = self.cost_trackers.write().await;

        for (session_id, instance) in instances.iter() {
            if !matches!(instance.state, InstanceState::Running) {
                continue;
            }

            if let Some(tracker) = trackers.get_mut(session_id) {
                let runtime = SystemTime::now()
                    .duration_since(instance.started_at)
                    .unwrap_or_default();

                let runtime_minutes = runtime.as_secs() / 60;
                let billable_minutes =
                    runtime_minutes.max(instance.spec.min_billing_minutes as u64);
                let current_cost = (billable_minutes as f64 / 60.0) * instance.spec.cost_per_hour;

                tracker.runtime_minutes = runtime_minutes;
                tracker.gpu_hours = billable_minutes as f64 / 60.0;
                tracker.total_cost = current_cost;

                // Check budget alerts
                if let Some(limit) = tracker.budget_limit {
                    let usage_ratio = current_cost / limit;
                    if usage_ratio >= tracker.budget_alert_threshold {
                        log::warn!(
                            "Session {} approaching budget limit: ${:.2} of ${:.2} ({:.0}%)",
                            session_id,
                            current_cost,
                            limit,
                            usage_ratio * 100.0
                        );
                    }
                }
            }
        }
    }

    /// Generate code using GPU-accelerated Ollama instance
    pub async fn generate_code(
        &self,
        session_id: &str,
        prompt: &str,
        model: Option<String>,
    ) -> Result<String> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No GPU instance found for session"))?;

        if !matches!(instance.state, InstanceState::Running) {
            return Err(anyhow::anyhow!("GPU instance is not running"));
        }

        match instance.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    let model_to_use = model.unwrap_or_else(|| {
                        self.config
                            .ollama_models
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "codellama:34b".to_string())
                    });

                    // Add coding-specific options for better results
                    let options = serde_json::json!({
                        "temperature": 0.2,  // Lower temperature for more focused code
                        "top_p": 0.95,
                        "num_gpu": self.config.ollama_gpu_layers.unwrap_or(99),
                        "num_thread": 8,
                        "repeat_penalty": 1.1,
                        "stop": ["```", "</code>", "// End"],
                    });

                    client.generate(&model_to_use, prompt, Some(options)).await
                } else {
                    Err(anyhow::anyhow!("Ollama client not configured"))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Code generation not supported for provider {:?}",
                instance.provider
            )),
        }
    }

    /// Chat with GPU-accelerated Ollama for coding assistance
    pub async fn chat_code(
        &self,
        session_id: &str,
        messages: Vec<serde_json::Value>,
        model: Option<String>,
    ) -> Result<String> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No GPU instance found for session"))?;

        if !matches!(instance.state, InstanceState::Running) {
            return Err(anyhow::anyhow!("GPU instance is not running"));
        }

        match instance.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    let model_to_use = model.unwrap_or_else(|| {
                        self.config
                            .ollama_models
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "codellama:34b".to_string())
                    });

                    let options = serde_json::json!({
                        "temperature": 0.2,
                        "top_p": 0.95,
                        "num_gpu": self.config.ollama_gpu_layers.unwrap_or(99),
                        "num_thread": 8,
                    });

                    client.chat(&model_to_use, messages, Some(options)).await
                } else {
                    Err(anyhow::anyhow!("Ollama client not configured"))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Chat not supported for provider {:?}",
                instance.provider
            )),
        }
    }

    /// Get available models for the current GPU instance
    pub async fn list_available_models(&self, session_id: &str) -> Result<Vec<String>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No GPU instance found for session"))?;

        match instance.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    client.list_models().await
                } else {
                    Err(anyhow::anyhow!("Ollama client not configured"))
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Pull a specific model to the GPU instance
    pub async fn pull_model(&self, session_id: &str, model_name: &str) -> Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No GPU instance found for session"))?;

        match instance.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    log::info!("Pulling model {} to GPU instance", model_name);
                    client.pull_model(model_name).await
                } else {
                    Err(anyhow::anyhow!("Ollama client not configured"))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Model pulling not supported for provider {:?}",
                instance.provider
            )),
        }
    }

    /// Get GPU utilization information
    pub async fn get_gpu_stats(&self, session_id: &str) -> Result<serde_json::Value> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No GPU instance found for session"))?;

        match instance.provider {
            GpuProvider::Ollama => {
                if let Some(client) = &self.ollama_client {
                    client.get_gpu_info().await
                } else {
                    Err(anyhow::anyhow!("Ollama client not configured"))
                }
            }
            _ => Ok(serde_json::json!({})),
        }
    }

    /// Auto-stop idle instances
    pub async fn cleanup_idle_instances(&self) -> Result<()> {
        let instances = self.instances.read().await;
        let now = SystemTime::now();
        let idle_duration = Duration::from_secs(self.config.idle_timeout_minutes as u64 * 60);

        let mut to_stop = Vec::new();
        for (session_id, instance) in instances.iter() {
            if matches!(instance.state, InstanceState::Running) {
                if let Ok(elapsed) = now.duration_since(instance.started_at) {
                    if elapsed > idle_duration {
                        to_stop.push(session_id.clone());
                    }
                }
            }
        }
        drop(instances);

        for session_id in to_stop {
            log::info!("Auto-stopping idle GPU instance for session {}", session_id);
            self.stop_gpu_instance(&session_id).await?;
        }

        Ok(())
    }

    /// Select best GPU specification based on preferences
    async fn select_best_gpu_spec(&self) -> Result<GpuInstanceSpec> {
        // For Salad, these are typical offerings
        let available_gpus = vec![
            GpuInstanceSpec {
                gpu_type: "RTX 4090".to_string(),
                vram_gb: 24,
                cpu_cores: 8,
                ram_gb: 32,
                cost_per_hour: 0.90,
                min_billing_minutes: 5,
            },
            GpuInstanceSpec {
                gpu_type: "RTX 3090".to_string(),
                vram_gb: 24,
                cpu_cores: 6,
                ram_gb: 24,
                cost_per_hour: 0.65,
                min_billing_minutes: 5,
            },
            GpuInstanceSpec {
                gpu_type: "RTX 3080".to_string(),
                vram_gb: 10,
                cpu_cores: 4,
                ram_gb: 16,
                cost_per_hour: 0.45,
                min_billing_minutes: 5,
            },
            GpuInstanceSpec {
                gpu_type: "A100".to_string(),
                vram_gb: 40,
                cpu_cores: 12,
                ram_gb: 48,
                cost_per_hour: 1.50,
                min_billing_minutes: 10,
            },
        ];

        // Filter by minimum VRAM requirement
        let suitable: Vec<_> = available_gpus
            .into_iter()
            .filter(|gpu| gpu.vram_gb >= self.config.min_vram_gb)
            .collect();

        if suitable.is_empty() {
            return Err(anyhow::anyhow!(
                "No suitable GPU found with {}GB+ VRAM",
                self.config.min_vram_gb
            ));
        }

        // Prefer GPUs in the preference list
        for pref in &self.config.preferred_gpu_types {
            if let Some(gpu) = suitable.iter().find(|g| &g.gpu_type == pref) {
                return Ok(gpu.clone());
            }
        }

        // Otherwise, choose the cheapest suitable option
        suitable
            .into_iter()
            .min_by(|a, b| {
                a.cost_per_hour
                    .partial_cmp(&b.cost_per_hour)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| anyhow::anyhow!("No GPU available"))
    }

    /// Get cost summary for all sessions
    pub async fn get_cost_summary(&self) -> HashMap<String, CostTracker> {
        let trackers = self.cost_trackers.read().await;
        trackers.clone()
    }

    /// Get total cost across all sessions
    pub async fn get_total_cost(&self) -> f64 {
        let trackers = self.cost_trackers.read().await;
        trackers.values().map(|t| t.total_cost).sum()
    }
}
