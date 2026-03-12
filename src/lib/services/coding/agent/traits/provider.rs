//! Provider traits

use async_trait::async_trait;
use anyhow::Result;
use std::time::Duration;

/// Unified LLM provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a response
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;

    /// Stream a response
    async fn stream(&self, request: GenerateRequest) -> Result<ResponseStream>;

    /// List available models
    async fn list_models(&self) -> Result<Vec<Model>>;

    /// Get provider info
    fn info(&self) -> ProviderInfo;

    /// Check if provider is available
    async fn is_available(&self) -> bool;

    /// Get provider metrics
    async fn metrics(&self) -> Result<ProviderMetrics>;

    /// Generate a response with simple parameters (backward compatibility)
    async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
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
}

#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub prompt: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Function,
}

#[derive(Debug, Clone)]
pub struct GenerateResponse {
    pub text: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FinishReason {
    Complete,
    MaxTokens,
    StopSequence,
    ContentFilter,
    Error,
}

pub struct ResponseStream {
    receiver: tokio::sync::mpsc::Receiver<StreamChunk>,
}

impl ResponseStream {
    pub fn new(receiver: tokio::sync::mpsc::Receiver<StreamChunk>) -> Self {
        Self { receiver }
    }

    pub async fn next(&mut self) -> Option<StreamChunk> {
        self.receiver.recv().await
    }
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub is_final: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub context_length: usize,
    pub capabilities: Vec<ModelCapability>,
}

#[derive(Debug, Clone)]
pub enum ModelCapability {
    Chat,
    Completion,
    Embeddings,
    FineTuning,
    FunctionCalling,
    Vision,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub version: String,
    pub provider_type: ProviderType,
    pub base_url: Option<String>,
    pub requires_auth: bool,
}

#[derive(Debug, Clone)]
pub enum ProviderType {
    Cloud,
    Local,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency: Duration,
    pub tokens_processed: u64,
    pub uptime: Duration,
    pub error_rate: f32,
}

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for text
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;

    /// Get maximum input length
    fn max_input_length(&self) -> usize;
}

/// Trait for completion providers
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    /// Complete code
    async fn complete(&self, request: CompletionRequest) -> Result<Vec<CompletionOption>>;

    /// Get completion capabilities
    fn capabilities(&self) -> CompletionCapabilities;
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub prefix: String,
    pub suffix: Option<String>,
    pub language: String,
    pub max_completions: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct CompletionOption {
    pub text: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CompletionCapabilities {
    pub languages: Vec<String>,
    pub features: Vec<CompletionFeature>,
    pub max_context: usize,
}

#[derive(Debug, Clone)]
pub enum CompletionFeature {
    Syntax,
    Semantic,
    Documentation,
    TypeInference,
    MultiLine,
}