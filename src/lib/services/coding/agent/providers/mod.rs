//! Refactored provider implementations using base class

use anyhow::Result;
use async_trait::async_trait;

pub mod local;
pub mod manager;
pub mod ollama;
pub mod openai;

pub use local::LocalProvider;
pub use manager::ProviderManager;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;

use super::traits::provider::{
    GenerateRequest, GenerateResponse, LLMProvider as LLMProviderTrait, Model, ProviderInfo,
    ProviderMetrics,
};

// Re-export LLMProvider for backward compatibility
pub use super::traits::provider::LLMProvider;

/// Unified provider interface
#[async_trait]
pub trait UnifiedProvider: Send + Sync {
    /// Generate response with unified interface
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;

    /// Check availability
    async fn is_available(&self) -> bool;

    /// Get provider information
    fn info(&self) -> ProviderInfo;

    /// List available models
    async fn list_models(&self) -> Result<Vec<Model>>;

    /// Get metrics
    async fn get_metrics(&self) -> Result<ProviderMetrics>;

    /// Downcast to Any for type checking
    fn as_any(&self) -> &dyn std::any::Any;

    /// Downcast to mutable Any for type checking
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Convert old LLMProvider trait to new UnifiedProvider
pub struct LegacyProviderAdapter<T: LLMProviderTrait> {
    inner: T,
}

impl<T: LLMProviderTrait> LegacyProviderAdapter<T> {
    pub fn new(provider: T) -> Self {
        Self { inner: provider }
    }
}

#[async_trait]
impl<T: LLMProviderTrait + 'static> UnifiedProvider for LegacyProviderAdapter<T> {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        self.inner.generate(request).await
    }

    async fn is_available(&self) -> bool {
        self.inner.is_available().await
    }

    fn info(&self) -> ProviderInfo {
        self.inner.info()
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        self.inner.list_models().await
    }

    async fn get_metrics(&self) -> Result<ProviderMetrics> {
        self.inner.metrics().await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.inner as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        &mut self.inner as &mut dyn std::any::Any
    }
}
