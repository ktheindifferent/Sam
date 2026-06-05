//! Base executor functionality

use anyhow::Result;
use std::path::Path;
use tokio::time::Duration;

/// Base executor trait
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    /// Execute a command
    async fn execute(&self, command: &str, working_dir: &Path) -> Result<String>;

    /// Execute with timeout
    async fn execute_with_timeout(
        &self,
        command: &str,
        working_dir: &Path,
        timeout: Duration,
    ) -> Result<String>;

    /// Check if executor is available
    async fn is_available(&self) -> bool;
}
