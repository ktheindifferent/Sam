use super::types::{JobError, JobResult};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, payload: Value) -> Result<JobResult, JobError>;

    fn max_retries(&self) -> u32 {
        3
    }

    fn retry_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff: 2^attempt seconds
        Duration::from_secs(2_u64.pow(attempt.min(10)))
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(300)) // 5 minutes default
    }

    fn name(&self) -> &str;

    async fn on_success(&self, _payload: &Value, _result: &JobResult) -> Result<(), JobError> {
        Ok(())
    }

    async fn on_failure(&self, _payload: &Value, _error: &JobError) -> Result<(), JobError> {
        Ok(())
    }

    async fn on_retry(
        &self,
        _payload: &Value,
        _attempt: u32,
        _error: &JobError,
    ) -> Result<(), JobError> {
        Ok(())
    }

    async fn validate_payload(&self, _payload: &Value) -> Result<(), JobError> {
        Ok(())
    }
}
