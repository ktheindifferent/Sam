use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio::sync::RwLock;
use std::time::Duration;

use super::gpu_offload::{GpuOffloadManager, GpuInstance};
use super::traits::provider::LLMProvider;

/// Remote Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteOllamaConfig {
    pub local_endpoint: String,
    pub remote_endpoint: Option<String>,
    pub use_gpu_offload: bool,
    pub fallback_to_local: bool,
    pub health_check_interval_secs: u64,
    pub request_timeout_secs: u64,
    pub model_loading_timeout_secs: u64,
}

impl Default for RemoteOllamaConfig {
    fn default() -> Self {
        Self {
            local_endpoint: "http://localhost:11434".to_string(),
            remote_endpoint: None,
            use_gpu_offload: false,
            fallback_to_local: true,
            health_check_interval_secs: 30,
            request_timeout_secs: 300, // 5 minutes for large models
            model_loading_timeout_secs: 600, // 10 minutes for initial model load
        }
    }
}

/// Remote Ollama provider with GPU offloading support
pub struct RemoteOllamaProvider {
    config: RemoteOllamaConfig,
    client: Client,
    gpu_manager: Option<Arc<GpuOffloadManager>>,
    current_endpoint: Arc<RwLock<String>>,
    session_id: String,
    is_remote_healthy: Arc<RwLock<bool>>,
}

impl RemoteOllamaProvider {
    pub fn new(
        config: RemoteOllamaConfig,
        gpu_manager: Option<Arc<GpuOffloadManager>>,
        session_id: String,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        let current_endpoint = Arc::new(RwLock::new(
            config.remote_endpoint.clone()
                .unwrap_or_else(|| config.local_endpoint.clone())
        ));

        Self {
            config,
            client,
            gpu_manager,
            current_endpoint,
            session_id,
            is_remote_healthy: Arc::new(RwLock::new(false)),
        }
    }

    /// Start GPU instance if configured
    pub async fn start_gpu_instance(&self) -> Result<()> {
        if !self.config.use_gpu_offload {
            return Ok(());
        }

        if let Some(gpu_manager) = &self.gpu_manager {
            let instance = gpu_manager.start_gpu_instance(&self.session_id).await?;

            // Update endpoint to use remote GPU
            let mut endpoint = self.current_endpoint.write().await;
            *endpoint = instance.endpoint.clone();

            // Wait for Ollama to be ready on remote instance
            self.wait_for_ollama_ready(&instance.endpoint).await?;

            let mut healthy = self.is_remote_healthy.write().await;
            *healthy = true;

            log::info!("GPU instance started for session {}: {}", self.session_id, instance.endpoint);
        }

        Ok(())
    }

    /// Stop GPU instance
    pub async fn stop_gpu_instance(&self) -> Result<()> {
        if let Some(gpu_manager) = &self.gpu_manager {
            gpu_manager.stop_gpu_instance(&self.session_id).await?;

            // Revert to local endpoint
            let mut endpoint = self.current_endpoint.write().await;
            *endpoint = self.config.local_endpoint.clone();

            let mut healthy = self.is_remote_healthy.write().await;
            *healthy = false;

            log::info!("GPU instance stopped for session {}", self.session_id);
        }

        Ok(())
    }

    /// Wait for Ollama to be ready on the endpoint
    async fn wait_for_ollama_ready(&self, endpoint: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.config.model_loading_timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for Ollama to be ready"));
            }

            match self.check_health(endpoint).await {
                Ok(true) => {
                    log::info!("Ollama is ready at {}", endpoint);
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Check health of an Ollama endpoint
    async fn check_health(&self, endpoint: &str) -> Result<bool> {
        let response = self.client
            .get(format!("{}/api/tags", endpoint))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        Ok(response.map(|r| r.status().is_success()).unwrap_or(false))
    }

    /// Load a model on the remote instance
    pub async fn load_model(&self, model: &str) -> Result<()> {
        let endpoint = self.current_endpoint.read().await.clone();

        let request_body = serde_json::json!({
            "name": model,
            "stream": false
        });

        let response = self.client
            .post(format!("{}/api/pull", endpoint))
            .json(&request_body)
            .timeout(Duration::from_secs(self.config.model_loading_timeout_secs))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Failed to load model {}: {}", model, error));
        }

        log::info!("Model {} loaded on {}", model, endpoint);
        Ok(())
    }

    /// Generate response with automatic failover
    async fn generate_with_failover(&self, prompt: &str, model: &str) -> Result<String> {
        let remote_healthy = *self.is_remote_healthy.read().await;

        // Try remote first if healthy
        if remote_healthy {
            if let Some(endpoint) = self.config.remote_endpoint.as_ref() {
                match self.generate_from_endpoint(endpoint, prompt, model).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        log::warn!("Remote generation failed, falling back to local: {}", e);
                        let mut healthy = self.is_remote_healthy.write().await;
                        *healthy = false;
                    }
                }
            }
        }

        // Fallback to local if configured
        if self.config.fallback_to_local {
            self.generate_from_endpoint(&self.config.local_endpoint, prompt, model).await
        } else {
            Err(anyhow::anyhow!("Remote endpoint unavailable and fallback disabled"))
        }
    }

    /// Generate from specific endpoint
    async fn generate_from_endpoint(&self, endpoint: &str, prompt: &str, model: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "top_p": 0.9,
                "max_tokens": 4096
            }
        });

        let response = self.client
            .post(format!("{}/api/generate", endpoint))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Generation failed: {}", error));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data["response"].as_str().unwrap_or("").to_string())
    }

    /// Start background health monitoring
    pub fn start_health_monitoring(self: Arc<Self>) {
        let interval = Duration::from_secs(self.config.health_check_interval_secs);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                // Check remote endpoint health
                if let Some(remote) = &self.config.remote_endpoint {
                    let healthy = self.check_health(remote).await.unwrap_or(false);
                    let mut is_healthy = self.is_remote_healthy.write().await;
                    *is_healthy = healthy;

                    if !healthy {
                        log::warn!("Remote Ollama endpoint {} is unhealthy", remote);
                    }
                }

                // Update cost tracking if using GPU
                if let Some(gpu_manager) = &self.gpu_manager {
                    gpu_manager.update_cost_tracking().await;
                }
            }
        });
    }

    /// Get current cost information
    pub async fn get_session_cost(&self) -> Option<f64> {
        if let Some(gpu_manager) = &self.gpu_manager {
            let costs = gpu_manager.get_cost_summary().await;
            costs.get(&self.session_id).map(|t| t.total_cost)
        } else {
            None
        }
    }

    /// Get current endpoint being used
    pub async fn get_current_endpoint(&self) -> String {
        self.current_endpoint.read().await.clone()
    }

    /// Switch between local and remote endpoints
    pub async fn switch_endpoint(&self, use_remote: bool) -> Result<()> {
        let mut endpoint = self.current_endpoint.write().await;

        if use_remote {
            if let Some(gpu_manager) = &self.gpu_manager {
                if let Some(gpu_endpoint) = gpu_manager.get_ollama_endpoint(&self.session_id).await {
                    *endpoint = gpu_endpoint;
                } else if let Some(remote) = &self.config.remote_endpoint {
                    *endpoint = remote.clone();
                } else {
                    return Err(anyhow::anyhow!("No remote endpoint available"));
                }
            } else if let Some(remote) = &self.config.remote_endpoint {
                *endpoint = remote.clone();
            } else {
                return Err(anyhow::anyhow!("No remote endpoint configured"));
            }
        } else {
            *endpoint = self.config.local_endpoint.clone();
        }

        log::info!("Switched to endpoint: {}", *endpoint);
        Ok(())
    }
}

use super::traits::provider::{
    GenerateRequest, GenerateResponse, Model, ProviderInfo,
    ProviderMetrics, ProviderType, ResponseStream, StreamChunk,
    TokenUsage, FinishReason,
};

#[async_trait::async_trait]
impl LLMProvider for RemoteOllamaProvider {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        // Start GPU instance if needed and not already running
        if self.config.use_gpu_offload && !*self.is_remote_healthy.read().await {
            if let Err(e) = self.start_gpu_instance().await {
                log::warn!("Failed to start GPU instance: {}", e);
                if !self.config.fallback_to_local {
                    return Err(e);
                }
            }
        }

        let text = self.generate_with_failover(&request.prompt, &request.model).await?;

        Ok(GenerateResponse {
            text,
            model: request.model,
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            finish_reason: FinishReason::Complete,
            metadata: serde_json::json!({}),
        })
    }

    async fn stream(&self, request: GenerateRequest) -> Result<ResponseStream> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // For now, just generate and send as a single chunk
        let response = self.generate(request).await?;

        tokio::spawn(async move {
            let _ = tx.send(StreamChunk {
                delta: response.text,
                is_final: true,
                metadata: Some(response.metadata),
            }).await;
        });

        Ok(ResponseStream::new(rx))
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Remote Ollama".to_string(),
            version: "1.0.0".to_string(),
            provider_type: if self.config.use_gpu_offload {
                ProviderType::Hybrid
            } else {
                ProviderType::Local
            },
            base_url: Some(self.current_endpoint.blocking_read().clone()),
            requires_auth: false,
        }
    }

    async fn is_available(&self) -> bool {
        let endpoint = self.current_endpoint.read().await.clone();
        self.check_health(&endpoint).await.unwrap_or(false)
    }

    async fn metrics(&self) -> Result<ProviderMetrics> {
        Ok(ProviderMetrics {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_latency: std::time::Duration::from_millis(0),
            tokens_processed: 0,
            uptime: std::time::Duration::from_secs(0),
            error_rate: 0.0,
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let endpoint = self.current_endpoint.read().await.clone();

        let response = self.client
            .get(format!("{}/api/tags", endpoint))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: serde_json::Value = response.json().await?;
        let models = data["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        m["name"].as_str().map(|name| Model {
                            id: name.to_string(),
                            name: name.to_string(),
                            description: m["description"].as_str().map(String::from),
                            context_length: 4096,
                            capabilities: vec![],
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}

/// Session manager for remote Ollama with automatic GPU lifecycle
pub struct RemoteOllamaSessionManager {
    providers: Arc<RwLock<HashMap<String, Arc<RemoteOllamaProvider>>>>,
    gpu_manager: Arc<GpuOffloadManager>,
    config: RemoteOllamaConfig,
}

impl RemoteOllamaSessionManager {
    pub fn new(gpu_manager: Arc<GpuOffloadManager>, config: RemoteOllamaConfig) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            gpu_manager,
            config,
        }
    }

    /// Create or get a provider for a session
    pub async fn get_or_create_provider(&self, session_id: &str) -> Arc<RemoteOllamaProvider> {
        let mut providers = self.providers.write().await;

        if let Some(provider) = providers.get(session_id) {
            return provider.clone();
        }

        let provider = Arc::new(RemoteOllamaProvider::new(
            self.config.clone(),
            Some(self.gpu_manager.clone()),
            session_id.to_string(),
        ));

        // Start health monitoring
        provider.clone().start_health_monitoring();

        providers.insert(session_id.to_string(), provider.clone());
        provider
    }

    /// End a session and cleanup resources
    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        let mut providers = self.providers.write().await;

        if let Some(provider) = providers.remove(session_id) {
            // Stop GPU instance if running
            provider.stop_gpu_instance().await?;

            // Log final cost
            if let Some(cost) = provider.get_session_cost().await {
                log::info!("Session {} ended. Total GPU cost: ${:.2}", session_id, cost);
            }
        }

        Ok(())
    }

    /// Get total cost across all sessions
    pub async fn get_total_cost(&self) -> f64 {
        self.gpu_manager.get_total_cost().await
    }

    /// Cleanup idle sessions
    pub async fn cleanup_idle_sessions(&self) -> Result<()> {
        self.gpu_manager.cleanup_idle_instances().await
    }
}

use std::collections::HashMap;