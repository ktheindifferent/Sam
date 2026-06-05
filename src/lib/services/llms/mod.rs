//! # LLM Services Module
//!
//! This module provides Large Language Model (LLM) integration services for the SAM system, including:
//! - Local LLM services (Llama.cpp)
//! - Local LLM runtime services (Ollama)
//! - Cloud-based LLM APIs (OpenAI)
//!
//! ## Available Services
//!
//! ### Local LLM Services
//! - **Llama**: Direct integration with llama.cpp for running models locally
//! - **Ollama**: Integration with Ollama runtime for easier local model management
//!
//! ### Cloud LLM Services
//! - **OpenAI**: Integration with OpenAI's GPT models via their API
//!
//! ## Architecture
//!
//! All LLM services follow consistent patterns:
//! - Model management (download, install, list)
//! - Text generation with configurable parameters
//! - Streaming support where available
//! - Error handling and retry logic
//! - Service health checks

pub mod llama;
pub mod ollama;
pub mod openai;

// Re-export main service types for convenience
pub use llama::LlamaService;
pub use ollama::{OllamaConfig, OllamaGenerateResponse, OllamaModel, OllamaService};
pub use openai::{ChatChoice, ChatMessage, ChatResponse, OpenAIClient};
