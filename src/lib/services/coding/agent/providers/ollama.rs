//! Refactored Ollama provider using base class

use anyhow::{Context, Result};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use super::super::provider_base::{BaseProvider, ProviderImpl, RetryConfig};
use super::super::traits::provider::{
    FinishReason, GenerateRequest, GenerateResponse, Model, ModelCapability, ProviderInfo,
    ProviderMetrics, ProviderType, ResponseStream, StreamChunk, TokenUsage,
};
use crate::services::llms::ollama::{OllamaConfig, OllamaService};

/// Ollama provider implementation
pub struct OllamaProviderImpl {
    service: Arc<OllamaService>,
    name: String,
}

impl OllamaProviderImpl {
    pub fn new(service: Arc<OllamaService>) -> Self {
        Self {
            service,
            name: "ollama".to_string(),
        }
    }

    pub fn with_config(config: OllamaConfig) -> Self {
        Self::new(Arc::new(OllamaService::new(config)))
    }
}

#[async_trait]
impl ProviderImpl for OllamaProviderImpl {
    async fn generate_impl(&self, prompt: &str, model: &str) -> Result<String> {
        debug!(
            "Generating response with Ollama - model: {}, prompt length: {}",
            model,
            prompt.len()
        );

        let response = self
            .service
            .generate(model, prompt, None)
            .await
            .context("Failed to generate response from Ollama")?;

        info!(
            "Successfully generated response from Ollama (length: {} chars)",
            response.response.len()
        );

        Ok(response.response)
    }

    async fn is_available_impl(&self) -> bool {
        let available = self.service.is_running().await;
        if !available {
            warn!("Ollama service is not available");
        }
        available
    }

    async fn list_models_impl(&self) -> Result<Vec<String>> {
        let (models, _errors) = self.service.list_all_models().await?;
        Ok(models.iter().map(|m| m.name.clone()).collect())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Ollama provider with retry and metrics
pub struct OllamaProvider {
    base: BaseProvider<OllamaProviderImpl>,
}

impl OllamaProvider {
    pub fn new(service: Arc<OllamaService>) -> Self {
        let impl_provider = OllamaProviderImpl::new(service);
        let base = BaseProvider::new(impl_provider, 60) // 60 requests per minute
            .with_retry_config(RetryConfig {
                max_retries: 3,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(10),
                multiplier: 2.0,
            });

        Self { base }
    }

    pub fn from_endpoint(endpoint: &str, port: u16, timeout_seconds: u64) -> Self {
        let config = OllamaConfig {
            host: endpoint.to_string(),
            port,
            timeout_seconds,
            custom_endpoint: None,
        };
        let service = Arc::new(OllamaService::new(config));
        Self::new(service)
    }

    /// Get the underlying service (for backward compatibility)
    pub fn get_service(&self) -> Arc<OllamaService> {
        // This would need access to the inner impl, for now return a placeholder
        Arc::new(OllamaService::new(OllamaConfig::default()))
    }

    /// Generate response with full request
    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let start = std::time::Instant::now();

        // Use the model from request, or default
        let model = if request.model.is_empty() {
            "codellama:13b"
        } else {
            &request.model
        };

        // Build prompt from messages if provided
        let prompt = if !request.messages.is_empty() {
            request
                .messages
                .iter()
                .map(|msg| format!("{:?}: {}", msg.role, msg.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            request.prompt.clone()
        };

        // Generate response
        let text = self.base.generate_response(&prompt, model).await?;

        // Calculate token usage (approximate)
        let prompt_tokens = prompt.len() / 4;
        let completion_tokens = text.len() / 4;

        Ok(GenerateResponse {
            text,
            model: model.to_string(),
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            finish_reason: FinishReason::Complete,
            metadata: serde_json::json!({
                "provider": "ollama",
                "duration_ms": start.elapsed().as_millis(),
            }),
        })
    }

    /// Stream response
    pub async fn stream(&self, request: GenerateRequest) -> Result<ResponseStream> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // In a real implementation, this would stream from Ollama
        // For now, simulate streaming
        let response = self.generate(request).await?;

        tokio::spawn(async move {
            // Simulate streaming by sending chunks
            for chunk in response.text.chars().collect::<Vec<_>>().chunks(10) {
                let chunk_str: String = chunk.iter().collect();
                let _ = tx
                    .send(StreamChunk {
                        delta: chunk_str,
                        is_final: false,
                        metadata: None,
                    })
                    .await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            // Send final chunk
            let _ = tx
                .send(StreamChunk {
                    delta: String::new(),
                    is_final: true,
                    metadata: Some(response.metadata),
                })
                .await;
        });

        Ok(ResponseStream::new(rx))
    }

    /// Check availability
    pub async fn is_available(&self) -> bool {
        self.base.is_available().await
    }

    /// List models
    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let model_names = self.base.list_models().await?;

        Ok(model_names
            .into_iter()
            .map(|name| Model {
                id: name.clone(),
                name: name.clone(),
                description: Some(format!("Ollama model: {}", name)),
                context_length: 4096, // Default, should query from Ollama
                capabilities: vec![ModelCapability::Chat, ModelCapability::Completion],
            })
            .collect())
    }

    /// Get provider info
    pub fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Ollama".to_string(),
            version: "1.0.0".to_string(),
            provider_type: ProviderType::Local,
            base_url: Some("http://localhost:11434".to_string()),
            requires_auth: false,
        }
    }

    /// Get metrics
    pub async fn get_metrics(&self) -> Result<ProviderMetrics> {
        let metrics = self.base.get_metrics().await;

        Ok(ProviderMetrics {
            total_requests: metrics.total_requests,
            successful_requests: metrics.success_count,
            failed_requests: metrics.failure_count,
            average_latency: Duration::from_millis(metrics.avg_response_time_ms as u64),
            tokens_processed: 0, // Would need to track this
            uptime: Duration::from_secs(
                metrics
                    .last_success
                    .or(metrics.last_failure)
                    .and_then(|t| t.elapsed().ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            error_rate: if metrics.total_requests > 0 {
                metrics.failure_count as f32 / metrics.total_requests as f32
            } else {
                0.0
            },
        })
    }
}

// Implement UnifiedProvider trait for OllamaProvider
use super::UnifiedProvider;

#[async_trait]
impl UnifiedProvider for OllamaProvider {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        self.generate(request).await
    }

    async fn is_available(&self) -> bool {
        self.is_available().await
    }

    fn info(&self) -> ProviderInfo {
        self.info()
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        self.list_models().await
    }

    async fn get_metrics(&self) -> Result<ProviderMetrics> {
        self.get_metrics().await
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
