use anyhow::Result;
use log::{info, warn};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Auto-configuration for Ollama servers and model selection
pub struct OllamaAutoConfig {
    client: Client,
}

impl OllamaAutoConfig {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Scan for available Ollama servers
    pub async fn scan_servers(&self) -> Vec<(String, Vec<String>)> {
        let mut servers = vec![];

        // Common Ollama endpoints to check
        let endpoints = vec![
            ("localhost", "http://localhost:11434"),
            ("127.0.0.1", "http://127.0.0.1:11434"),
            ("Remote GPU", "http://172.16.0.125:11434"),
            // Add more known servers here
        ];

        for (name, endpoint) in endpoints {
            if let Ok(models) = self.check_server(endpoint).await {
                info!(
                    "Found Ollama server at {} with {} models",
                    endpoint,
                    models.len()
                );
                servers.push((name.to_string(), models));
            }
        }

        servers
    }

    /// Check a specific server and return available models
    async fn check_server(&self, endpoint: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", endpoint);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Server returned error status"));
        }

        let data: Value = response.json().await?;

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

    /// Select the best model based on availability and preferences
    pub fn select_best_model(available_models: &[String], task_type: &str) -> Option<String> {
        // Priority order for different task types
        let model_priorities = match task_type {
            "coding" => vec![
                "gpt-oss:120b",
                "gpt-oss:20b",
                "qwen2.5-coder:latest",
                "codellama:latest",
                "llama3.1:latest",
                "mistrallite:latest",
            ],
            "general" => vec![
                "gpt-oss:120b",
                "gpt-oss:20b",
                "llama3.1:latest",
                "gemma3:1b",
                "mistrallite:latest",
            ],
            _ => vec![
                "gpt-oss:20b",
                "llama3.1:latest",
                "codellama:latest",
                "mistrallite:latest",
            ],
        };

        // Find the first available model from the priority list
        for preferred_model in model_priorities {
            if available_models.iter().any(|m| m == preferred_model) {
                info!("Selected model: {}", preferred_model);
                return Some(preferred_model.to_string());
            }
        }

        // Fallback to first available model
        available_models.first().cloned()
    }

    /// Test model performance
    pub async fn test_model_speed(&self, endpoint: &str, model: &str) -> Result<Duration> {
        let start = std::time::Instant::now();

        let request = serde_json::json!({
            "model": model,
            "prompt": "Write hello world",
            "stream": false,
            "options": {
                "temperature": 0.0,
                "max_tokens": 10
            }
        });

        let response = self
            .client
            .post(format!("{}/api/generate", endpoint))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Model test failed"));
        }

        let _: Value = response.json().await?;

        Ok(start.elapsed())
    }

    /// Get recommended configuration based on scanning
    pub async fn get_recommended_config(&self) -> Result<(String, String)> {
        let servers = self.scan_servers().await;

        if servers.is_empty() {
            return Err(anyhow::anyhow!("No Ollama servers found"));
        }

        // Prefer remote GPU server if available
        for (name, models) in &servers {
            if name.contains("GPU") || name.contains("Remote") {
                if let Some(model) = Self::select_best_model(&models, "coding") {
                    return Ok((name.clone(), model));
                }
            }
        }

        // Fallback to local server
        for (name, models) in &servers {
            if let Some(model) = Self::select_best_model(&models, "coding") {
                return Ok((name.clone(), model));
            }
        }

        Err(anyhow::anyhow!("No suitable models found"))
    }
}
