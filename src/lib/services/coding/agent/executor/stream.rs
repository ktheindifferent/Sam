//! Streaming executor module

/// Streaming executor for handling streams
pub struct StreamingExecutor;

impl StreamingExecutor {
    pub fn new() -> Self {
        Self
    }
}

/// Re-export stream chunk from main module
pub use super::StreamChunk;