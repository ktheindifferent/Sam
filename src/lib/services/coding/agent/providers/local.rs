//! Local LLM provider implementation

use super::UnifiedProvider;
use crate::services::coding::agent::traits::provider::{
    FinishReason, GenerateRequest, GenerateResponse, Model, ModelCapability, ProviderInfo,
    ProviderMetrics, ProviderType, TokenUsage,
};
use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;

/// Local provider for running models locally
pub struct LocalProvider {
    model_path: Option<String>,
    model_name: String,
}

impl LocalProvider {
    pub fn new(model_path: Option<String>) -> Self {
        Self {
            model_path,
            model_name: "local-llm".to_string(),
        }
    }
}

#[async_trait]
impl UnifiedProvider for LocalProvider {
    async fn generate(&self, _request: GenerateRequest) -> Result<GenerateResponse> {
        // TODO: Implement actual local model inference
        Ok(GenerateResponse {
            text: "Local model response placeholder".to_string(),
            model: self.model_name.clone(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            finish_reason: FinishReason::Complete,
            metadata: Default::default(),
        })
    }

    async fn is_available(&self) -> bool {
        self.model_path.is_some()
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Local".to_string(),
            version: "1.0.0".to_string(),
            provider_type: ProviderType::Local,
            base_url: None,
            requires_auth: false,
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        Ok(vec![Model {
            id: self.model_name.clone(),
            name: "Local LLM".to_string(),
            description: Some("Local language model".to_string()),
            context_length: 2048,
            capabilities: vec![ModelCapability::Chat, ModelCapability::Completion],
        }])
    }

    async fn get_metrics(&self) -> Result<ProviderMetrics> {
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
