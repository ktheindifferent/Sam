use rouille::{Request, Response};
use std::sync::Arc;
use tokio::runtime::Runtime;
use log::{debug, warn, error, info};
use anyhow::Result;
use lazy_static::lazy_static;

use crate::resource_management::{
    ResourceManager, ResourceConfig, FileLimits, RequestLimits, MemoryLimits,
    PoolConfig, CleanupConfig
};
use crate::http::rate_limiter::{RateLimiter, RateLimitConfig};

lazy_static! {
    // Global runtime for async operations
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create runtime");
    
    // Global resource manager
    static ref RESOURCE_MANAGER: Arc<ResourceManager> = {
        let config = ResourceConfig::from_env();
        let mut manager = ResourceManager::new(config);
        
        // Start background cleanup
        RUNTIME.block_on(async {
            manager.start_cleanup().await;
        });
        
        Arc::new(manager)
    };
    
    // Global rate limiter
    static ref RATE_LIMITER: Arc<RateLimiter> = {
        let config = RateLimitConfig::from_env();
        Arc::new(RateLimiter::new(config))
    };
}

impl ResourceConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        ResourceConfig {
            file_limits: FileLimits {
                max_file_size: std::env::var("MAX_FILE_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100 * 1024 * 1024), // 100MB default
                max_concurrent_uploads: std::env::var("MAX_CONCURRENT_UPLOADS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                max_user_storage: std::env::var("MAX_USER_STORAGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10 * 1024 * 1024 * 1024), // 10GB default
                allowed_extensions: std::env::var("ALLOWED_EXTENSIONS")
                    .ok()
                    .map(|s| s.split(',').map(|e| e.trim().to_string()).collect())
                    .unwrap_or_default(),
                blocked_extensions: std::env::var("BLOCKED_EXTENSIONS")
                    .ok()
                    .map(|s| s.split(',').map(|e| e.trim().to_string()).collect())
                    .unwrap_or_else(|| vec![
                        ".exe".to_string(),
                        ".dll".to_string(),
                        ".bat".to_string(),
                        ".cmd".to_string(),
                        ".scr".to_string(),
                    ]),
                enable_virus_scan: std::env::var("ENABLE_VIRUS_SCAN")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
                temp_cleanup_interval: std::env::var("TEMP_CLEANUP_INTERVAL")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3600),
                temp_max_age: std::env::var("TEMP_MAX_AGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(86400),
            },
            request_limits: RequestLimits {
                max_body_size: std::env::var("MAX_BODY_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10 * 1024 * 1024), // 10MB default
                max_processing_time: std::env::var("MAX_PROCESSING_TIME")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(300), // 5 minutes default
                max_concurrent_per_ip: std::env::var("MAX_CONCURRENT_PER_IP")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                max_header_size: std::env::var("MAX_HEADER_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8192),
                enable_cancellation: std::env::var("ENABLE_REQUEST_CANCELLATION")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
            },
            pool_config: PoolConfig::default(),
            cleanup_config: CleanupConfig {
                enable_auto_cleanup: std::env::var("ENABLE_AUTO_CLEANUP")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
                cleanup_interval: std::env::var("CLEANUP_INTERVAL")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3600),
                temp_dir: std::env::var("TEMP_DIR")
                    .ok()
                    .map(|s| std::path::PathBuf::from(s))
                    .unwrap_or_else(|| std::path::PathBuf::from("/opt/sam/tmp")),
                max_temp_size: std::env::var("MAX_TEMP_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10 * 1024 * 1024 * 1024), // 10GB default
                orphan_age_threshold: std::env::var("ORPHAN_AGE_THRESHOLD")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(86400),
            },
            memory_limits: MemoryLimits {
                max_memory_per_request: std::env::var("MAX_MEMORY_PER_REQUEST")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(512 * 1024 * 1024), // 512MB default
                max_buffer_size: std::env::var("MAX_BUFFER_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(64 * 1024), // 64KB default
                enable_monitoring: std::env::var("ENABLE_MEMORY_MONITORING")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
                warning_threshold: std::env::var("MEMORY_WARNING_THRESHOLD")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.8),
                critical_threshold: std::env::var("MEMORY_CRITICAL_THRESHOLD")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.95),
            },
        }
    }
}

impl RateLimitConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = RateLimitConfig::default();
        
        if let Ok(limit) = std::env::var("DEFAULT_AUTH_RATE_LIMIT") {
            if let Ok(val) = limit.parse() {
                config.default_authenticated_limit = val;
            }
        }
        
        if let Ok(limit) = std::env::var("DEFAULT_ANON_RATE_LIMIT") {
            if let Ok(val) = limit.parse() {
                config.default_anonymous_limit = val;
            }
        }
        
        if let Ok(window) = std::env::var("RATE_LIMIT_WINDOW_SECONDS") {
            if let Ok(val) = window.parse() {
                config.window_seconds = val;
            }
        }
        
        if let Ok(use_redis) = std::env::var("USE_REDIS_RATE_LIMIT") {
            if let Ok(val) = use_redis.parse() {
                config.use_redis = val;
            }
        }
        
        config
    }
}

/// Resource management middleware
pub async fn resource_middleware(request: &Request) -> Option<Response> {
    let client_ip = request.remote_addr().to_string();
    let endpoint = request.url();
    
    // Check rate limits first
    if let Some(response) = check_rate_limit(request).await {
        return Some(response);
    }
    
    // Check request size limits
    if let Some(response) = check_request_limits(request).await {
        return Some(response);
    }
    
    // Check file upload limits for upload endpoints
    if is_upload_endpoint(endpoint) {
        if let Some(response) = check_upload_limits(request).await {
            return Some(response);
        }
    }
    
    None // Allow request to proceed
}

/// Check rate limits
async fn check_rate_limit(request: &Request) -> Option<Response> {
    let endpoint = request.url();
    let client_id = get_client_id(request);
    let is_authenticated = is_authenticated_request(request);
    
    match RATE_LIMITER.check_rate_limit(endpoint, &client_id, is_authenticated).await {
        Ok(status) => {
            if !status.is_allowed() {
                let headers = status.to_headers();
                let mut response = Response::text("Rate limit exceeded")
                    .with_status_code(429);
                
                for (key, value) in headers {
                    response = response.with_additional_header(key, value);
                }
                
                warn!("Rate limit exceeded for client {} on endpoint {}", client_id, endpoint);
                Some(response)
            } else {
                None
            }
        }
        Err(e) => {
            error!("Rate limiting error: {}. Allowing request to proceed.", e);
            None
        }
    }
}

/// Check request size limits
async fn check_request_limits(request: &Request) -> Option<Response> {
    // Check Content-Length header
    if let Some(content_length) = request.header("Content-Length") {
        if let Ok(size) = content_length.parse::<usize>() {
            let limits = &RESOURCE_MANAGER.config.request_limits;
            
            if size > limits.max_body_size {
                warn!("Request body too large: {} bytes (max: {})", size, limits.max_body_size);
                return Some(Response::text("Request body too large")
                    .with_status_code(413)
                    .with_additional_header("X-Max-Body-Size", limits.max_body_size.to_string()));
            }
        }
    }
    
    // Check header size
    let header_size = estimate_header_size(request);
    let limits = &RESOURCE_MANAGER.config.request_limits;
    
    if header_size > limits.max_header_size {
        warn!("Request headers too large: {} bytes (max: {})", header_size, limits.max_header_size);
        return Some(Response::text("Request headers too large")
            .with_status_code(431));
    }
    
    None
}

/// Check file upload limits
async fn check_upload_limits(request: &Request) -> Option<Response> {
    // Extract user ID (from session, auth header, etc.)
    let user_id = get_user_id(request).unwrap_or_else(|| "anonymous".to_string());
    
    // Get file size from Content-Length
    let file_size = request.header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    // Get file extension from Content-Disposition or URL
    let file_extension = extract_file_extension(request);
    
    match RESOURCE_MANAGER.check_upload_allowed(&user_id, file_size, &file_extension).await {
        Ok(permission) => {
            match permission {
                crate::resource_management::UploadPermission::Allowed { .. } => None,
                crate::resource_management::UploadPermission::Denied { reason } => {
                    warn!("Upload denied for user {}: {}", user_id, reason);
                    Some(Response::text(format!("Upload denied: {}", reason))
                        .with_status_code(400))
                }
            }
        }
        Err(e) => {
            error!("Failed to check upload limits: {}", e);
            Some(Response::text("Internal server error")
                .with_status_code(500))
        }
    }
}

/// Check if endpoint is an upload endpoint
fn is_upload_endpoint(endpoint: &str) -> bool {
    endpoint.contains("/upload") ||
    endpoint.contains("/file") ||
    endpoint.contains("/attachment") ||
    endpoint.contains("/media") ||
    endpoint.contains("/import")
}

/// Get client ID from request
fn get_client_id(request: &Request) -> String {
    // Try to get authenticated user ID
    if let Some(user_id) = request.header("X-User-Id") {
        return user_id.to_string();
    }
    
    // Try to get session ID
    if let Some(session_id) = request.header("X-Session-Id") {
        return session_id.to_string();
    }
    
    // Fall back to IP address
    request.remote_addr().to_string()
}

/// Get user ID from request
fn get_user_id(request: &Request) -> Option<String> {
    request.header("X-User-Id")
        .or_else(|| request.header("X-Auth-User"))
        .map(|s| s.to_string())
}

/// Check if request is authenticated
fn is_authenticated_request(request: &Request) -> bool {
    request.header("Authorization").is_some() || 
    request.header("X-API-Key").is_some() ||
    request.header("X-User-Id").is_some()
}

/// Extract file extension from request
fn extract_file_extension(request: &Request) -> String {
    // Try Content-Disposition header
    if let Some(disposition) = request.header("Content-Disposition") {
        if let Some(filename) = extract_filename_from_disposition(disposition) {
            return std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
        }
    }
    
    // Try URL path
    let path = request.url();
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

/// Extract filename from Content-Disposition header
fn extract_filename_from_disposition(disposition: &str) -> Option<String> {
    // Parse Content-Disposition: form-data; name="file"; filename="example.txt"
    disposition.split(';')
        .find(|part| part.trim().starts_with("filename="))
        .and_then(|part| {
            let filename = part.trim().trim_start_matches("filename=");
            Some(filename.trim_matches('"').to_string())
        })
}

/// Estimate header size
fn estimate_header_size(request: &Request) -> usize {
    let mut size = 0;
    
    // Estimate request line size
    size += request.method().len();
    size += request.url().len();
    size += 12; // " HTTP/1.1\r\n"
    
    // Add header sizes
    for (name, value) in request.headers() {
        size += name.len() + value.len() + 4; // ": " and "\r\n"
    }
    
    size += 2; // Final "\r\n"
    
    size
}

/// Initialize resource management system
pub fn initialize() -> Result<()> {
    // Force lazy static initialization
    let _ = &*RESOURCE_MANAGER;
    let _ = &*RATE_LIMITER;
    
    info!("Resource management system initialized");
    info!("Max file size: {} MB", RESOURCE_MANAGER.config.file_limits.max_file_size / (1024 * 1024));
    info!("Max request body: {} MB", RESOURCE_MANAGER.config.request_limits.max_body_size / (1024 * 1024));
    info!("Rate limiting enabled: {}", RATE_LIMITER.config.use_redis);
    
    // Start periodic cleanup of rate limit buckets
    RUNTIME.spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            RATE_LIMITER.cleanup_old_buckets().await;
        }
    });
    
    Ok(())
}

/// Get resource metrics
pub async fn get_metrics() -> crate::resource_management::ResourceMetrics {
    RESOURCE_MANAGER.get_metrics().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_from_disposition() {
        let disposition = r#"form-data; name="file"; filename="test.txt""#;
        let filename = extract_filename_from_disposition(disposition);
        assert_eq!(filename, Some("test.txt".to_string()));
    }

    #[test]
    fn test_is_upload_endpoint() {
        assert!(is_upload_endpoint("/api/upload"));
        assert!(is_upload_endpoint("/api/file/new"));
        assert!(is_upload_endpoint("/api/media/upload"));
        assert!(!is_upload_endpoint("/api/users"));
    }
}