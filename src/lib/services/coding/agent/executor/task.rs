//! Task executor module

use std::future::Future;
use anyhow::Result;

/// Task executor for managing async tasks
pub struct TaskExecutor;

impl TaskExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a future as a task
    pub async fn execute<F, T>(&self, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        future.await
    }
}