use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use log::{info, warn, error};

use crate::services::llms::ollama::OllamaService;
use crate::services::llms::openai::{OpenAIClient, ChatMessage};

/// Model provider enumeration for different LLM backends
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelProvider {
    Ollama,
    OpenAI,
    Local,      // For llamafile, LM Studio, etc.
    Custom(String),
}

/// Model configuration for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub model_name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Model performance metrics
#[derive(Debug, Clone, Default)]
pub struct ModelPerformanceMetrics {
    pub avg_response_time: f64,
    pub success_rate: f32,
    pub user_satisfaction_score: f32,
    pub task_type_performance: HashMap<String, f32>,
}

/// LLM Provider trait for abstracting different language model providers
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate_response(&self, prompt: &str, model: &str) -> Result<String>;
    async fn is_available(&self) -> bool;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Performance tracking metrics for providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPerformance {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub last_request_time: Option<std::time::SystemTime>,
}

impl Default for ProviderPerformance {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            last_request_time: None,
        }
    }
}

/// Ollama provider implementation
pub struct OllamaProvider {
    service: Arc<OllamaService>,
}

impl OllamaProvider {
    pub fn new(service: Arc<OllamaService>) -> Self {
        Self { service }
    }

    /// Get the underlying Ollama service
    pub fn get_service(&self) -> Arc<OllamaService> {
        self.service.clone()
    }

    /// Helper method to get as Any trait for downcasting
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait::async_trait]
impl LLMProvider for OllamaProvider {
    async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
        info!("Generating response with Ollama - model: {}, prompt length: {}", model, prompt.len());

        match self.service.generate(model, prompt, None).await {
            Ok(response) => {
                info!("Successfully generated response from Ollama (length: {} chars)", response.response.len());
                Ok(response.response)
            }
            Err(e) => {
                error!("Failed to generate response from Ollama: {}", e);
                Err(e)
            }
        }
    }

    async fn is_available(&self) -> bool {
        let available = self.service.is_running().await;
        if !available {
            warn!("Ollama service is not available");
        }
        available
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let (models, _errors) = self.service.list_all_models().await?;
        Ok(models.iter().map(|m| m.name.clone()).collect())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OpenAI provider implementation
pub struct OpenAIProvider {
    client: Arc<OpenAIClient>,
    api_key: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Arc::new(OpenAIClient::new(api_key.clone()));
        Self {
            client,
            api_key,
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
        // Create messages for the OpenAI API
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful coding assistant. Provide clear, concise, and accurate responses with executable commands when appropriate.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ];

        // Use the client to make the API call
        match self.client.chat(model, messages).await {
            Ok(response) => {
                // Extract the response content
                if let Some(choice) = response.choices.first() {
                    Ok(choice.message.content.clone())
                } else {
                    Err(anyhow::anyhow!("No response from OpenAI"))
                }
            }
            Err(e) => {
                Err(anyhow::anyhow!("OpenAI API error: {}", e))
            }
        }
    }

    async fn is_available(&self) -> bool {
        // Check if the API key is set and valid format
        !self.api_key.is_empty() && self.api_key.starts_with("sk-")
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        // Return commonly available OpenAI models
        Ok(vec![
            "gpt-4-turbo-preview".to_string(),
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
        ])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Local provider implementation (placeholder)
#[derive(Debug)]
pub struct LocalProvider {
    provider_name: String,
    base_url: String,
}

impl LocalProvider {
    pub fn new(provider_name: String, base_url: String) -> Self {
        Self {
            provider_name,
            base_url,
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for LocalProvider {
    async fn generate_response(&self, prompt: &str, model: &str) -> Result<String> {
        // TODO: Implement local provider API integration
        Err(anyhow::anyhow!("Local provider not yet implemented"))
    }

    async fn is_available(&self) -> bool {
        false // TODO: Implement availability check
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        // TODO: Implement model listing
        Ok(vec![])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Circuit breaker state for provider failover
#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open(SystemTime),  // Time when circuit was opened
    HalfOpen,
}

/// Circuit breaker for managing provider failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    timeout: Duration,
    last_failure_time: Option<SystemTime>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            timeout,
            last_failure_time: None,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(SystemTime::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open(SystemTime::now());
        }
    }

    pub fn is_available(&mut self) -> bool {
        match &self.state {
            CircuitState::Closed => true,
            CircuitState::Open(opened_at) => {
                // Check if timeout has elapsed
                if SystemTime::now().duration_since(*opened_at).unwrap_or(Duration::ZERO) > self.timeout {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

/// Provider manager for handling multiple LLM providers
pub struct ProviderManager {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    performance: HashMap<String, ProviderPerformance>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    default_provider: Option<String>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            performance: HashMap::new(),
            circuit_breakers: HashMap::new(),
            default_provider: None,
        }
    }

    pub fn add_provider(&mut self, name: String, provider: Box<dyn LLMProvider>) {
        self.performance.insert(name.clone(), ProviderPerformance::default());
        self.circuit_breakers.insert(
            name.clone(),
            CircuitBreaker::new(3, Duration::from_secs(30)),
        );
        self.providers.insert(name, provider);
    }

    pub fn set_default_provider(&mut self, name: String) {
        if self.providers.contains_key(&name) {
            self.default_provider = Some(name);
        }
    }

    pub async fn generate_response(&mut self, prompt: &str, model: &str, provider_name: Option<&str>) -> Result<String> {
        info!("ProviderManager::generate_response called with model: {}, provider: {:?}", model, provider_name);

        let provider_name = provider_name
            .or(self.default_provider.as_deref())
            .ok_or_else(|| anyhow::anyhow!("No provider specified and no default provider set"))?;

        info!("Using provider: {}", provider_name);

        // Check circuit breaker status and find alternative if needed
        let use_alternative = if let Some(circuit_breaker) = self.circuit_breakers.get_mut(provider_name) {
            !circuit_breaker.is_available()
        } else {
            false
        };

        if use_alternative {
            // Find an alternative provider
            let mut alternative_found = None;
            for (alt_name, alt_breaker) in &mut self.circuit_breakers {
                if alt_name != provider_name && alt_breaker.is_available() {
                    if let Some(alt_provider) = self.providers.get(alt_name) {
                        if alt_provider.is_available().await {
                            alternative_found = Some(alt_name.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(alt_name) = alternative_found {
                log::warn!("Provider {} circuit open, falling back to {}", provider_name, alt_name);
                // Use the alternative provider
                if let Some(alt_provider) = self.providers.get(&alt_name) {
                    let start_time = Instant::now();
                    let result = alt_provider.generate_response(prompt, model).await;
                    let duration = start_time.elapsed();

                    // Update metrics for alternative provider
                    if let Some(alt_breaker) = self.circuit_breakers.get_mut(&alt_name) {
                        if result.is_ok() {
                            alt_breaker.record_success();
                        } else {
                            alt_breaker.record_failure();
                        }
                    }

                    if let Some(perf) = self.performance.get_mut(&alt_name) {
                        perf.total_requests += 1;
                        if result.is_ok() {
                            perf.successful_requests += 1;
                        } else {
                            perf.failed_requests += 1;
                        }
                        let new_avg = if perf.total_requests == 1 {
                            duration.as_millis() as f64
                        } else {
                            (perf.average_response_time_ms * (perf.total_requests - 1) as f64 + duration.as_millis() as f64) / perf.total_requests as f64
                        };
                        perf.average_response_time_ms = new_avg;
                        perf.last_request_time = Some(SystemTime::now());
                    }

                    return result;
                }
            }

            return Err(anyhow::anyhow!("Provider {} is unavailable (circuit open)", provider_name));
        }

        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_name))?;

        info!("Calling provider {} generate_response", provider_name);
        let start_time = Instant::now();
        let result = provider.generate_response(prompt, model).await;
        let duration = start_time.elapsed();
        info!("Provider {} response took {:?}, success: {}", provider_name, duration, result.is_ok());

        // Update circuit breaker and performance metrics
        if let Some(circuit_breaker) = self.circuit_breakers.get_mut(provider_name) {
            if result.is_ok() {
                circuit_breaker.record_success();
            } else {
                circuit_breaker.record_failure();
            }
        }

        // Update performance metrics
        if let Some(perf) = self.performance.get_mut(provider_name) {
            perf.total_requests += 1;
            if result.is_ok() {
                perf.successful_requests += 1;
            } else {
                perf.failed_requests += 1;
            }

            // Update average response time
            let new_avg = if perf.total_requests == 1 {
                duration.as_millis() as f64
            } else {
                (perf.average_response_time_ms * (perf.total_requests - 1) as f64 + duration.as_millis() as f64) / perf.total_requests as f64
            };
            perf.average_response_time_ms = new_avg;
            perf.last_request_time = Some(SystemTime::now());
        }

        result
    }

    pub async fn is_provider_available(&self, provider_name: &str) -> bool {
        if let Some(provider) = self.providers.get(provider_name) {
            provider.is_available().await
        } else {
            false
        }
    }

    pub async fn list_models(&self, provider_name: Option<&str>) -> Result<Vec<String>> {
        let provider_name = provider_name
            .or(self.default_provider.as_deref())
            .ok_or_else(|| anyhow::anyhow!("No provider specified and no default provider set"))?;

        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_name))?;

        provider.list_models().await
    }

    pub fn get_performance(&self, provider_name: &str) -> Option<&ProviderPerformance> {
        self.performance.get(provider_name)
    }

    pub fn list_providers(&self) -> Vec<&String> {
        self.providers.keys().collect()
    }

    /// Check if current default provider is available
    pub async fn is_current_provider_available(&self) -> bool {
        if let Some(provider_name) = &self.default_provider {
            if let Some(provider) = self.providers.get(provider_name) {
                return provider.is_available().await;
            }
        }
        false
    }

    /// List available models from current provider
    pub async fn list_available_models(&self) -> Result<Vec<String>> {
        if let Some(provider_name) = &self.default_provider {
            if let Some(provider) = self.providers.get(provider_name) {
                return provider.list_models().await;
            }
        }
        Ok(vec![])
    }

    /// Get current provider name
    pub fn get_current_provider_name(&self) -> Option<String> {
        self.default_provider.clone()
    }

    /// Get status of all providers
    pub async fn get_provider_status(&self) -> HashMap<String, bool> {
        let mut status = HashMap::new();
        for (name, provider) in &self.providers {
            status.insert(name.clone(), provider.is_available().await);
        }
        status
    }

    /// Get performance metrics for all providers
    pub fn get_performance_metrics(&self) -> &HashMap<String, ProviderPerformance> {
        &self.performance
    }

    /// Check if a provider exists
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, name: &str) -> Result<()> {
        if self.providers.contains_key(name) {
            self.default_provider = Some(name.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", name))
        }
    }

    /// Set current provider (alias for switch_provider)
    pub fn set_current_provider(&mut self, name: &str) -> Result<()> {
        self.switch_provider(name)
    }

    /// Get a specific provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Box<dyn LLMProvider>> {
        self.providers.get(name)
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}