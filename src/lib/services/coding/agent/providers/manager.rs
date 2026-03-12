//! Provider manager for handling multiple LLM providers

use std::sync::Arc;
use std::collections::HashMap;
use std::any::Any;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::RwLock;
use super::{UnifiedProvider, OllamaProvider, OpenAIProvider, LocalProvider};
use crate::services::coding::agent::traits::provider::{
    GenerateRequest, GenerateResponse, Model, ProviderInfo,
    ProviderMetrics,
};

/// Manager for multiple LLM providers
pub struct ProviderManager {
    providers: Arc<RwLock<HashMap<String, Box<dyn UnifiedProvider>>>>,
    active_provider: Arc<RwLock<Option<String>>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            active_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a provider
    pub async fn register(&self, name: String, provider: Box<dyn UnifiedProvider>) {
        let mut providers = self.providers.write().await;
        providers.insert(name.clone(), provider);

        // Set as active if first provider
        let mut active = self.active_provider.write().await;
        if active.is_none() {
            *active = Some(name);
        }
    }

    /// Add a provider (backward compatibility alias)
    pub fn add_provider(&mut self, name: String, provider: Box<dyn UnifiedProvider>) {
        let providers = self.providers.clone();
        let active = self.active_provider.clone();

        tokio::spawn(async move {
            let mut providers = providers.write().await;
            let was_empty = providers.is_empty();
            providers.insert(name.clone(), provider);

            if was_empty {
                let mut active = active.write().await;
                *active = Some(name);
            }
        });
    }

    /// Set default provider (backward compatibility)
    pub fn set_default_provider(&mut self, name: String) {
        let active = self.active_provider.clone();
        tokio::spawn(async move {
            let mut active = active.write().await;
            *active = Some(name);
        });
    }

    /// Set active provider
    pub async fn set_active(&self, name: String) -> Result<()> {
        let providers = self.providers.read().await;
        if !providers.contains_key(&name) {
            return Err(anyhow!("Provider {} not found", name));
        }

        let mut active = self.active_provider.write().await;
        *active = Some(name);
        Ok(())
    }

    /// Get active provider
    async fn get_active_provider(&self) -> Result<String> {
        let active = self.active_provider.read().await;
        active.clone().ok_or_else(|| anyhow!("No active provider set"))
    }

    /// Check if current provider is available
    pub async fn is_current_provider_available(&self) -> bool {
        if let Ok(provider_name) = self.get_active_provider().await {
            if let Ok(providers) = self.providers.try_read() {
                if let Some(provider) = providers.get(&provider_name) {
                    return provider.is_available().await;
                }
            }
        }
        false
    }

    /// Generate response using current provider
    pub async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
        let request = GenerateRequest {
            prompt: prompt.to_string(),
            model: model.to_string(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: vec![],
            system_prompt: None,
            messages: vec![],
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }

    /// List available models from all providers
    pub async fn list_available_models(&self) -> Result<Vec<String>> {
        let models = self.list_models().await?;
        Ok(models.iter().map(|m| m.id.clone()).collect())
    }

    /// Get current provider name
    pub async fn get_current_provider_name(&self) -> Option<String> {
        let active = self.active_provider.read().await;
        active.clone()
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Get provider status
    pub async fn get_provider_status(&self, name: &str) -> Result<bool> {
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(name) {
            Ok(provider.is_available().await)
        } else {
            Err(anyhow!("Provider {} not found", name))
        }
    }

    /// Get a specific provider
    pub fn get_provider(&self, name: &str) -> Option<Box<dyn UnifiedProvider>> {
        // This is a simplified version - in reality we'd need to handle the async lock
        None
    }

    /// Initialize default providers
    pub async fn init_defaults(&self) {
        // Register Ollama provider using from_endpoint
        self.register(
            "ollama".to_string(),
            Box::new(OllamaProvider::from_endpoint("localhost", 11434, 60)),
        ).await;

        // Register OpenAI provider if API key is available
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.register(
                "openai".to_string(),
                Box::new(OpenAIProvider::new(Some(api_key))),
            ).await;
        }

        // Register local provider if model path is available
        if let Ok(model_path) = std::env::var("LOCAL_MODEL_PATH") {
            self.register(
                "local".to_string(),
                Box::new(LocalProvider::new(Some(model_path))),
            ).await;
        }
    }
}

#[async_trait]
impl UnifiedProvider for ProviderManager {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let provider_name = self.get_active_provider().await?;
        let providers = self.providers.read().await;
        let provider = providers.get(&provider_name)
            .ok_or_else(|| anyhow!("Provider {} not found", provider_name))?;
        provider.generate(request).await
    }

    async fn is_available(&self) -> bool {
        if let Ok(provider_name) = self.get_active_provider().await {
            if let Ok(providers) = self.providers.try_read() {
                if let Some(provider) = providers.get(&provider_name) {
                    return provider.is_available().await;
                }
            }
        }
        false
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Provider Manager".to_string(),
            version: "1.0.0".to_string(),
            provider_type: crate::services::coding::agent::traits::provider::ProviderType::Cloud,
            base_url: None,
            requires_auth: false,
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let mut all_models = Vec::new();
        let providers = self.providers.read().await;
        for provider in providers.values() {
            if let Ok(models) = provider.list_models().await {
                all_models.extend(models);
            }
        }
        Ok(all_models)
    }

    async fn get_metrics(&self) -> Result<ProviderMetrics> {
        let provider_name = self.get_active_provider().await?;
        let providers = self.providers.read().await;
        let provider = providers.get(&provider_name)
            .ok_or_else(|| anyhow!("Provider {} not found", provider_name))?;
        provider.get_metrics().await
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}