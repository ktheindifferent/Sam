//! # RTSP Services Module
//! 
//! This module provides comprehensive RTSP (Real-Time Streaming Protocol) services for the SAM system.
//! It includes functionality for:
//! 
//! - Stream management and processing
//! - Deep learning and computer vision analysis
//! - Recording and storage capabilities
//! - Motion detection and alerting
//! - Face recognition and object detection
//! 
//! ## Architecture
//! 
//! The RTSP module is organized into the following components:
//! 
//! - `manager`: Core RTSP stream management and orchestration
//! - `dl`: Full-featured deep learning module with OpenCV support
//! - `dl_simple`: Simplified deep learning module without OpenCV dependencies
//! - `dl_test`: Test implementations for development and testing
//! - `recording`: Recording, storage, and playback functionality
//! 
//! ## Usage
//! 
//! The main entry point is the `manager` module, which coordinates all RTSP services:
//! 
//! ```rust
//! use crate::services::rtsp;
//! 
//! // Initialize RTSP services
//! rtsp::init();
//! ```

pub mod dl;
pub mod dl_simple;
pub mod dl_test;
pub mod manager;
pub mod recording;

// Re-export commonly used types and functions
pub use manager::init;
pub use dl::{DetectionResult, Detection, FaceDetection, Alert, AlertType, AlertSeverity};
pub use dl_simple::{start_deep_learning_processor};
pub use recording::{RecordingManager, RecordingConfig, RecordingTrigger, create_recording_tables};

// Configuration and common types
use serde::{Deserialize, Serialize};

/// Common bounding box representation used across RTSP modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Common error types for RTSP operations
#[derive(Debug, thiserror::Error)]
pub enum RtspError {
    #[error("Stream connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Recording error: {0}")]
    RecordingError(String),
    
    #[error("Deep learning processing error: {0}")]
    ProcessingError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, RtspError>;
