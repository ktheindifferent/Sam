//! # SAM Services Module
//! 
//! This module provides a collection of services for the SAM system, including:
//! - External API integrations (OpenAI, GitHub, Dropbox, etc.)
//! - Media services (YouTube, Spotify, image processing)
//! - Communication services (SMS, notifications)
//! - Infrastructure services (Docker, SSH, database connections)
//! - Home automation (LIFX, Matter protocol)
//! 
//! ## Architecture
//! 
//! Services follow a consistent pattern:
//! - Trait-based abstractions for testability
//! - Configuration management through centralized config system
//! - Error handling with common error types
//! - Retry logic for resilient operations
//! - HTTP client configuration for external APIs

// Core infrastructure modules
pub mod config;
pub mod environment;
pub mod errors;
pub mod http_client;
pub mod retry;
pub mod traits;
pub mod validation;

// Service-specific modules
pub mod backup;
pub mod backup_enhanced;
pub mod cache;
pub mod clamav;
pub mod copilot;
pub mod crawler;
pub mod darknet;
pub mod database;
pub mod docker;
pub mod dropbox;
pub mod error_handling;
pub mod file_storage;
pub mod git;
pub mod github;
pub mod jupiter;
pub mod lifx;
pub mod llama;
pub mod matter;
pub mod mdns;
pub mod media;
pub mod monitoring;
pub mod notifications;
pub mod openai;
pub mod orchestrator;
pub mod osf;
pub mod restart;
pub mod p2p;
pub mod password_manager;
pub mod pg;
pub mod redis;
pub mod rivescript;
pub mod rtsp;
pub mod rtsp_dl_simple;
pub mod rtsp_recording;
pub mod sms;
pub mod socket;
pub mod sound;
pub mod spotify;
pub mod sprec;
pub mod ssh;
pub mod storage;
pub mod stt;
pub mod thread_manager;
pub mod tts;

#[cfg(test)]
mod thread_safety_test;
pub mod voice;
pub mod vulnerability_scanner;
pub mod who;

// Re-export commonly used types
pub use config::{ConfigManager, GlobalConfig, ServiceConfig};
pub use errors::{CommonError, ErrorContext, Result};
pub type Error = CommonError;
pub use http_client::{ApiClient, HttpClientConfig, SharedHttpClient};
pub use retry::{ExponentialBackoffRetry, RetryConfig, RetryStrategy};
pub use traits::{Service, ServiceFactory, ServiceHealth, ServiceRegistry};
pub use validation::{ConfigValidator, ValidationError, Validator};