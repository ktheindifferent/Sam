use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use anyhow::Result;
use log::{debug, warn};
use std::collections::HashMap;

/// Resource limits enforcement
pub struct ResourceLimits {
    file_limiter: Arc<FileLimiter>,
    request_limiter: Arc<RequestLimiter>,
    memory_limiter: Arc<MemoryLimiter>,
}

impl ResourceLimits {
    /// Create new resource limits enforcer
    pub fn new(
        file_limits: FileLimits,
        request_limits: RequestLimits,
        memory_limits: MemoryLimits,
    ) -> Self {
        ResourceLimits {
            file_limiter: Arc::new(FileLimiter::new(file_limits)),
            request_limiter: Arc::new(RequestLimiter::new(request_limits)),
            memory_limiter: Arc::new(MemoryLimiter::new(memory_limits)),
        }
    }
    
    /// Check file upload limits
    pub async fn check_file_upload(
        &self,
        user_id: &str,
        file_size: usize,
        extension: &str,
    ) -> Result<FileUploadCheck> {
        self.file_limiter.check_upload(user_id, file_size, extension).await
    }
    
    /// Check request limits
    pub async fn check_request(
        &self,
        client_ip: &str,
        body_size: usize,
        headers_size: usize,
    ) -> Result<RequestCheck> {
        self.request_limiter.check_request(client_ip, body_size, headers_size).await
    }
    
    /// Check memory limits
    pub async fn check_memory_allocation(&self, size: usize) -> Result<MemoryCheck> {
        self.memory_limiter.check_allocation(size).await
    }
}

/// File upload limits
#[derive(Debug, Clone)]
pub struct FileLimits {
    pub max_file_size: usize,
    pub max_concurrent_uploads: usize,
    pub max_user_storage: usize,
    pub allowed_extensions: Vec<String>,
    pub blocked_extensions: Vec<String>,
}

/// File limiter
pub struct FileLimiter {
    limits: FileLimits,
    user_uploads: Arc<RwLock<HashMap<String, UserUploadState>>>,
    user_storage: Arc<RwLock<HashMap<String, usize>>>,
}

#[derive(Debug)]
struct UserUploadState {
    semaphore: Arc<Semaphore>,
    current_uploads: usize,
    last_upload: Instant,
}

impl FileLimiter {
    /// Create new file limiter
    pub fn new(limits: FileLimits) -> Self {
        FileLimiter {
            limits,
            user_uploads: Arc::new(RwLock::new(HashMap::new())),
            user_storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if upload is allowed
    pub async fn check_upload(
        &self,
        user_id: &str,
        file_size: usize,
        extension: &str,
    ) -> Result<FileUploadCheck> {
        // Check file size
        if file_size > self.limits.max_file_size {
            return Ok(FileUploadCheck::Rejected {
                reason: FileRejectionReason::FileTooLarge {
                    size: file_size,
                    max_size: self.limits.max_file_size,
                },
            });
        }
        
        // Check extension
        if !self.is_extension_allowed(extension) {
            return Ok(FileUploadCheck::Rejected {
                reason: FileRejectionReason::ExtensionBlocked {
                    extension: extension.to_string(),
                },
            });
        }
        
        // Check user storage quota
        let mut storage = self.user_storage.write().await;
        let current_storage = storage.get(user_id).copied().unwrap_or(0);
        
        if current_storage + file_size > self.limits.max_user_storage {
            return Ok(FileUploadCheck::Rejected {
                reason: FileRejectionReason::StorageQuotaExceeded {
                    current: current_storage,
                    requested: file_size,
                    max: self.limits.max_user_storage,
                },
            });
        }
        
        // Check concurrent uploads
        let mut uploads = self.user_uploads.write().await;
        let upload_state = uploads.entry(user_id.to_string()).or_insert_with(|| {
            UserUploadState {
                semaphore: Arc::new(Semaphore::new(self.limits.max_concurrent_uploads)),
                current_uploads: 0,
                last_upload: Instant::now(),
            }
        });
        
        // Try to acquire upload slot
        match upload_state.semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                upload_state.current_uploads += 1;
                upload_state.last_upload = Instant::now();
                storage.insert(user_id.to_string(), current_storage + file_size);
                
                Ok(FileUploadCheck::Allowed {
                    permit: Some(permit),
                    remaining_storage: self.limits.max_user_storage - (current_storage + file_size),
                })
            }
            Err(_) => Ok(FileUploadCheck::Rejected {
                reason: FileRejectionReason::TooManyConcurrentUploads {
                    current: upload_state.current_uploads,
                    max: self.limits.max_concurrent_uploads,
                },
            }),
        }
    }
    
    /// Check if extension is allowed
    fn is_extension_allowed(&self, extension: &str) -> bool {
        let ext_lower = extension.to_lowercase();
        
        // Check blocked extensions first
        if self.limits.blocked_extensions.iter()
            .any(|blocked| blocked.to_lowercase() == ext_lower) {
            return false;
        }
        
        // If allowed list is empty, all non-blocked are allowed
        if self.limits.allowed_extensions.is_empty() {
            return true;
        }
        
        // Check allowed list
        self.limits.allowed_extensions.iter()
            .any(|allowed| allowed.to_lowercase() == ext_lower)
    }
    
    /// Release storage quota
    pub async fn release_storage(&self, user_id: &str, size: usize) {
        let mut storage = self.user_storage.write().await;
        if let Some(current) = storage.get_mut(user_id) {
            *current = current.saturating_sub(size);
        }
    }
}

/// File upload check result
#[derive(Debug)]
pub enum FileUploadCheck {
    Allowed {
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
        remaining_storage: usize,
    },
    Rejected {
        reason: FileRejectionReason,
    },
}

/// File rejection reason
#[derive(Debug)]
pub enum FileRejectionReason {
    FileTooLarge {
        size: usize,
        max_size: usize,
    },
    ExtensionBlocked {
        extension: String,
    },
    StorageQuotaExceeded {
        current: usize,
        requested: usize,
        max: usize,
    },
    TooManyConcurrentUploads {
        current: usize,
        max: usize,
    },
}

/// Request processing limits
#[derive(Debug, Clone)]
pub struct RequestLimits {
    pub max_body_size: usize,
    pub max_header_size: usize,
    pub max_processing_time: Duration,
    pub max_concurrent_per_ip: usize,
}

/// Request limiter
pub struct RequestLimiter {
    limits: RequestLimits,
    ip_requests: Arc<RwLock<HashMap<String, IpRequestState>>>,
}

#[derive(Debug)]
struct IpRequestState {
    semaphore: Arc<Semaphore>,
    current_requests: usize,
    last_request: Instant,
}

impl RequestLimiter {
    /// Create new request limiter
    pub fn new(limits: RequestLimits) -> Self {
        RequestLimiter {
            limits,
            ip_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if request is allowed
    pub async fn check_request(
        &self,
        client_ip: &str,
        body_size: usize,
        headers_size: usize,
    ) -> Result<RequestCheck> {
        // Check body size
        if body_size > self.limits.max_body_size {
            return Ok(RequestCheck::Rejected {
                reason: RequestRejectionReason::BodyTooLarge {
                    size: body_size,
                    max_size: self.limits.max_body_size,
                },
            });
        }
        
        // Check header size
        if headers_size > self.limits.max_header_size {
            return Ok(RequestCheck::Rejected {
                reason: RequestRejectionReason::HeadersTooLarge {
                    size: headers_size,
                    max_size: self.limits.max_header_size,
                },
            });
        }
        
        // Check concurrent requests
        let mut ip_states = self.ip_requests.write().await;
        let ip_state = ip_states.entry(client_ip.to_string()).or_insert_with(|| {
            IpRequestState {
                semaphore: Arc::new(Semaphore::new(self.limits.max_concurrent_per_ip)),
                current_requests: 0,
                last_request: Instant::now(),
            }
        });
        
        // Try to acquire request slot
        match ip_state.semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                ip_state.current_requests += 1;
                ip_state.last_request = Instant::now();
                
                Ok(RequestCheck::Allowed {
                    permit: Some(permit),
                    timeout: self.limits.max_processing_time,
                })
            }
            Err(_) => Ok(RequestCheck::Rejected {
                reason: RequestRejectionReason::TooManyConcurrentRequests {
                    current: ip_state.current_requests,
                    max: self.limits.max_concurrent_per_ip,
                },
            }),
        }
    }
    
    /// Process request with timeout
    pub async fn process_with_timeout<F, T>(
        &self,
        processing_fn: F,
        timeout_duration: Duration,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        match timeout(timeout_duration, processing_fn).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Request processing timeout exceeded")),
        }
    }
}

/// Request check result
#[derive(Debug)]
pub enum RequestCheck {
    Allowed {
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
        timeout: Duration,
    },
    Rejected {
        reason: RequestRejectionReason,
    },
}

/// Request rejection reason
#[derive(Debug)]
pub enum RequestRejectionReason {
    BodyTooLarge {
        size: usize,
        max_size: usize,
    },
    HeadersTooLarge {
        size: usize,
        max_size: usize,
    },
    TooManyConcurrentRequests {
        current: usize,
        max: usize,
    },
}

/// Memory limits
#[derive(Debug, Clone)]
pub struct MemoryLimits {
    pub max_allocation: usize,
    pub max_buffer_size: usize,
    pub warning_threshold: f32,
    pub critical_threshold: f32,
}

/// Memory limiter
pub struct MemoryLimiter {
    limits: MemoryLimits,
    current_usage: Arc<RwLock<usize>>,
}

impl MemoryLimiter {
    /// Create new memory limiter
    pub fn new(limits: MemoryLimits) -> Self {
        MemoryLimiter {
            limits,
            current_usage: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Check if memory allocation is allowed
    pub async fn check_allocation(&self, size: usize) -> Result<MemoryCheck> {
        if size > self.limits.max_allocation {
            return Ok(MemoryCheck::Rejected {
                reason: MemoryRejectionReason::AllocationTooLarge {
                    requested: size,
                    max: self.limits.max_allocation,
                },
            });
        }
        
        let mut current = self.current_usage.write().await;
        let total_after = *current + size;
        
        // Check system memory
        let system_memory = get_system_memory_usage();
        
        if system_memory > self.limits.critical_threshold {
            return Ok(MemoryCheck::Rejected {
                reason: MemoryRejectionReason::SystemMemoryCritical {
                    usage: system_memory,
                    threshold: self.limits.critical_threshold,
                },
            });
        }
        
        if system_memory > self.limits.warning_threshold {
            warn!("System memory usage is high: {:.2}%", system_memory * 100.0);
        }
        
        *current = total_after;
        
        Ok(MemoryCheck::Allowed {
            allocated: size,
            current_usage: total_after,
        })
    }
    
    /// Release allocated memory
    pub async fn release(&self, size: usize) {
        let mut current = self.current_usage.write().await;
        *current = current.saturating_sub(size);
        debug!("Released {} bytes, current usage: {}", size, *current);
    }
    
    /// Get current memory usage
    pub async fn get_usage(&self) -> usize {
        *self.current_usage.read().await
    }
}

/// Memory check result
#[derive(Debug)]
pub enum MemoryCheck {
    Allowed {
        allocated: usize,
        current_usage: usize,
    },
    Rejected {
        reason: MemoryRejectionReason,
    },
}

/// Memory rejection reason
#[derive(Debug)]
pub enum MemoryRejectionReason {
    AllocationTooLarge {
        requested: usize,
        max: usize,
    },
    SystemMemoryCritical {
        usage: f32,
        threshold: f32,
    },
}

/// Get system memory usage as percentage
fn get_system_memory_usage() -> f32 {
    // Use sysinfo crate to get actual memory usage
    use sysinfo::System;
    
    let mut sys = System::new();
    sys.refresh_memory();
    
    let total = sys.total_memory();
    let used = sys.used_memory();
    
    if total > 0 {
        (used as f32) / (total as f32)
    } else {
        0.0
    }
}

/// Streaming buffer with size limits
pub struct LimitedBuffer {
    buffer: Vec<u8>,
    max_size: usize,
    current_size: usize,
}

impl LimitedBuffer {
    /// Create new limited buffer
    pub fn new(max_size: usize) -> Self {
        LimitedBuffer {
            buffer: Vec::with_capacity(max_size.min(8192)),
            max_size,
            current_size: 0,
        }
    }
    
    /// Write data to buffer
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let available = self.max_size - self.current_size;
        let to_write = data.len().min(available);
        
        if to_write == 0 {
            return Err(anyhow::anyhow!("Buffer size limit exceeded"));
        }
        
        self.buffer.extend_from_slice(&data[..to_write]);
        self.current_size += to_write;
        
        Ok(to_write)
    }
    
    /// Read and clear buffer
    pub fn read(&mut self) -> Vec<u8> {
        let data = std::mem::take(&mut self.buffer);
        self.current_size = 0;
        data
    }
    
    /// Get current size
    pub fn size(&self) -> usize {
        self.current_size
    }
    
    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.current_size >= self.max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_limiter() {
        let limits = FileLimits {
            max_file_size: 1024 * 1024, // 1MB
            max_concurrent_uploads: 2,
            max_user_storage: 10 * 1024 * 1024, // 10MB
            allowed_extensions: vec!["jpg".to_string(), "png".to_string()],
            blocked_extensions: vec!["exe".to_string()],
        };
        
        let limiter = FileLimiter::new(limits);
        
        // Test allowed upload
        let check = limiter.check_upload("user1", 512 * 1024, "jpg").await.unwrap();
        assert!(matches!(check, FileUploadCheck::Allowed { .. }));
        
        // Test blocked extension
        let check = limiter.check_upload("user1", 512 * 1024, "exe").await.unwrap();
        assert!(matches!(check, FileUploadCheck::Rejected { .. }));
        
        // Test file too large
        let check = limiter.check_upload("user1", 2 * 1024 * 1024, "jpg").await.unwrap();
        assert!(matches!(check, FileUploadCheck::Rejected { .. }));
    }

    #[test]
    fn test_limited_buffer() {
        let mut buffer = LimitedBuffer::new(10);
        
        // Write within limit
        assert_eq!(buffer.write(b"hello").unwrap(), 5);
        assert_eq!(buffer.size(), 5);
        
        // Write up to limit
        assert_eq!(buffer.write(b"world").unwrap(), 5);
        assert_eq!(buffer.size(), 10);
        assert!(buffer.is_full());
        
        // Try to exceed limit
        assert!(buffer.write(b"!").is_err());
        
        // Read and clear
        let data = buffer.read();
        assert_eq!(&data, b"helloworld");
        assert_eq!(buffer.size(), 0);
    }
}