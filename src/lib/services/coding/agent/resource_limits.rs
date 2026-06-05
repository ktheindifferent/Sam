use super::constants::*;
use crate::services::coding::agent::errors::{CodingAgentError, CodingAgentResult};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU time per operation in seconds
    pub max_cpu_seconds: u64,
    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,
    /// Maximum output size in bytes
    pub max_output_bytes: usize,
    /// Maximum execution time per command in seconds
    pub max_command_duration_seconds: u64,
    /// Maximum context size in tokens (approximate)
    pub max_context_tokens: usize,
    /// Maximum file size for reading in bytes
    pub max_file_size_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: MAX_MEMORY_BYTES as u64,
            max_cpu_seconds: MAX_CPU_SECONDS,
            max_concurrent_operations: MAX_CONCURRENT_OPERATIONS,
            max_output_bytes: MAX_OUTPUT_BYTES,
            max_command_duration_seconds: DEFAULT_TIMEOUT_SECONDS * 2,
            max_context_tokens: MAX_CONVERSATION_TOKENS,
            max_file_size_bytes: MAX_FILE_SIZE_BYTES as u64,
        }
    }
}

/// Resource monitor for tracking resource usage
pub struct ResourceMonitor {
    limits: ResourceLimits,
    current_memory: Arc<AtomicU64>,
    current_operations: Arc<AtomicUsize>,
    operation_semaphore: Arc<Semaphore>,
    start_time: Instant,
}

impl ResourceMonitor {
    pub fn new(limits: ResourceLimits) -> Self {
        let max_ops = limits.max_concurrent_operations;
        Self {
            limits,
            current_memory: Arc::new(AtomicU64::new(0)),
            current_operations: Arc::new(AtomicUsize::new(0)),
            operation_semaphore: Arc::new(Semaphore::new(max_ops)),
            start_time: Instant::now(),
        }
    }

    /// Check if memory limit would be exceeded
    pub fn check_memory_limit(&self, additional_bytes: u64) -> CodingAgentResult<()> {
        let current = self.current_memory.load(Ordering::Relaxed);
        let projected = current + additional_bytes;

        if projected > self.limits.max_memory_bytes {
            Err(CodingAgentError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                limit: format!("{} MB", self.limits.max_memory_bytes / (1024 * 1024)),
                current: format!("{} MB", projected / (1024 * 1024)),
            })
        } else {
            Ok(())
        }
    }

    /// Allocate memory tracking
    pub fn allocate_memory(&self, bytes: u64) -> CodingAgentResult<MemoryAllocation> {
        self.check_memory_limit(bytes)?;
        self.current_memory.fetch_add(bytes, Ordering::SeqCst);
        Ok(MemoryAllocation {
            bytes,
            monitor: self.current_memory.clone(),
        })
    }

    /// Check if output size is within limits
    pub fn check_output_size(&self, output: &str) -> CodingAgentResult<()> {
        let size = output.len();
        if size > self.limits.max_output_bytes {
            Err(CodingAgentError::ResourceLimitExceeded {
                resource: "output size".to_string(),
                limit: format!("{} KB", self.limits.max_output_bytes / 1024),
                current: format!("{} KB", size / 1024),
            })
        } else {
            Ok(())
        }
    }

    /// Truncate output if it exceeds limits
    pub fn truncate_output(&self, output: String) -> String {
        if output.len() > self.limits.max_output_bytes {
            let truncated = &output[..self.limits.max_output_bytes];
            format!(
                "{}\n... (output truncated, exceeded {} KB limit)",
                truncated,
                self.limits.max_output_bytes / 1024
            )
        } else {
            output
        }
    }

    /// Check if file size is within limits
    pub fn check_file_size(&self, size: u64) -> CodingAgentResult<()> {
        if size > self.limits.max_file_size_bytes {
            Err(CodingAgentError::ResourceLimitExceeded {
                resource: "file size".to_string(),
                limit: format!("{} MB", self.limits.max_file_size_bytes / (1024 * 1024)),
                current: format!("{} MB", size / (1024 * 1024)),
            })
        } else {
            Ok(())
        }
    }

    /// Check if context size is within limits (approximate token count)
    pub fn check_context_size(&self, text: &str) -> CodingAgentResult<()> {
        // Approximate token count (roughly 4 chars per token)
        let estimated_tokens = text.len() / 4;

        if estimated_tokens > self.limits.max_context_tokens {
            Err(CodingAgentError::ResourceLimitExceeded {
                resource: "context tokens".to_string(),
                limit: format!("{} tokens", self.limits.max_context_tokens),
                current: format!("{} tokens", estimated_tokens),
            })
        } else {
            Ok(())
        }
    }

    /// Acquire permit for concurrent operation
    pub async fn acquire_operation_permit(&self) -> CodingAgentResult<OperationPermit> {
        match self.operation_semaphore.clone().acquire_owned().await {
            Ok(permit) => {
                self.current_operations.fetch_add(1, Ordering::SeqCst);
                Ok(OperationPermit {
                    _permit: permit,
                    counter: self.current_operations.clone(),
                })
            }
            Err(_) => Err(CodingAgentError::ResourceLimitExceeded {
                resource: "concurrent operations".to_string(),
                limit: format!("{}", self.limits.max_concurrent_operations),
                current: format!("{}", self.current_operations.load(Ordering::Relaxed)),
            }),
        }
    }

    /// Get current resource usage statistics
    pub fn get_usage_stats(&self) -> ResourceUsageStats {
        ResourceUsageStats {
            memory_used_bytes: self.current_memory.load(Ordering::Relaxed),
            memory_limit_bytes: self.limits.max_memory_bytes,
            concurrent_operations: self.current_operations.load(Ordering::Relaxed),
            max_concurrent_operations: self.limits.max_concurrent_operations,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Create a timeout duration for command execution
    pub fn command_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.max_command_duration_seconds)
    }
}

/// RAII guard for memory allocation
pub struct MemoryAllocation {
    bytes: u64,
    monitor: Arc<AtomicU64>,
}

impl Drop for MemoryAllocation {
    fn drop(&mut self) {
        self.monitor.fetch_sub(self.bytes, Ordering::SeqCst);
    }
}

/// RAII guard for operation permit
pub struct OperationPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    counter: Arc<AtomicUsize>,
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsageStats {
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: u64,
    pub concurrent_operations: usize,
    pub max_concurrent_operations: usize,
    pub uptime_seconds: u64,
}

impl ResourceUsageStats {
    /// Get memory usage as percentage
    pub fn memory_usage_percent(&self) -> f32 {
        if self.memory_limit_bytes == 0 {
            0.0
        } else {
            (self.memory_used_bytes as f32 / self.memory_limit_bytes as f32) * 100.0
        }
    }

    /// Get concurrent operations usage as percentage
    pub fn operations_usage_percent(&self) -> f32 {
        if self.max_concurrent_operations == 0 {
            0.0
        } else {
            (self.concurrent_operations as f32 / self.max_concurrent_operations as f32) * 100.0
        }
    }

    /// Check if resources are under pressure
    pub fn is_under_pressure(&self) -> bool {
        self.memory_usage_percent() > 80.0 || self.operations_usage_percent() > 80.0
    }
}
