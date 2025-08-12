# Resource Management Implementation

## Overview
Comprehensive resource management system has been implemented to prevent DoS attacks and resource exhaustion. The system provides multiple layers of protection including file upload restrictions, request rate limiting, connection pool management, memory limits, and automatic cleanup mechanisms.

## Key Components Implemented

### 1. File Upload Restrictions (`src/sam/resource_management/limits.rs`)
- **Maximum file size enforcement**: Default 100MB, configurable via `MAX_FILE_SIZE` env var
- **Concurrent upload limits**: Default 10 per user, prevents resource monopolization
- **User storage quotas**: Default 10GB per user, tracks cumulative usage
- **File type validation**: 
  - Configurable allowed extensions whitelist
  - Blocked extensions blacklist (exe, dll, bat, cmd, scr by default)
- **Virus scanning hooks**: Ready for ClamAV integration
- **Automatic orphaned file cleanup**: Removes temp files older than 24 hours

### 2. Request Limits (`src/sam/resource_management/limits.rs`)
- **Request body size limits**: Default 10MB, configurable via `MAX_BODY_SIZE`
- **Header size limits**: Default 8KB, prevents header bomb attacks
- **Request processing timeout**: Default 5 minutes, prevents hanging requests
- **Concurrent request limits per IP**: Default 100, prevents connection flooding
- **Request cancellation**: Automatic cleanup on client disconnect

### 3. Resource Cleanup with RAII (`src/sam/resource_management/cleanup.rs`)
- **TempFile struct**: Automatic file deletion on drop
  ```rust
  let temp_file = TempFile::new(&temp_dir)?;
  temp_file.write(&data).await?;
  // File automatically deleted when temp_file goes out of scope
  ```
- **CleanupGuard**: Generic RAII wrapper for any resource
- **ResourceCleanup**: Batch cleanup operations
- **ScopeGuard**: Ensures cleanup on all code paths

### 4. Connection Pool Management (`src/sam/resource_management/pool.rs`)
- **Connection pooling**: Reuses database connections efficiently
- **Health checks**: Periodic validation of idle connections
- **Circuit breaker**: Prevents cascading failures
  - Opens after 5 consecutive failures
  - Auto-resets after 60 seconds
- **Connection metrics**: Tracks usage, wait times, failures
- **Automatic cleanup**: Removes expired/invalid connections

### 5. Memory Management (`src/sam/resource_management/limits.rs`)
- **Per-request memory limits**: Default 512MB
- **Streaming buffer limits**: Default 64KB chunks
- **System memory monitoring**: Warns at 80%, critical at 95%
- **Memory usage tracking**: Per-request allocation tracking
- **Automatic memory release**: RAII-based cleanup

### 6. Rate Limiting (`src/sam/http/rate_limiter.rs`)
- **Endpoint-specific limits**: Different limits for different endpoints
- **User vs anonymous limits**: Higher limits for authenticated users
- **Distributed rate limiting**: Redis support for multi-instance deployments
- **Burst handling**: Token bucket algorithm with burst capacity
- **Automatic cleanup**: Removes old rate limit buckets

### 7. Resource Monitoring (`src/sam/resource_management/monitoring.rs`)
- **Real-time metrics collection**: CPU, memory, disk, network
- **Alert system**: Configurable thresholds for warnings/critical alerts
- **Historical data**: Maintains metrics history for analysis
- **Performance tracking**: Request times, success rates, resource usage

## Fixed Issues

### File Handle Leaks in observations.rs
**Before**: Files were created without guaranteed cleanup, leading to resource leaks
```rust
// OLD CODE - RESOURCE LEAK
std::fs::write(tmp_file_path.clone(), wav_data)?;
// ... processing ...
// Manual cleanup that might not run on error
crate::sam::tools::uinx_cmd(format!("rm {}", tmp_file_path));
```

**After**: Automatic cleanup using RAII pattern
```rust
// NEW CODE - GUARANTEED CLEANUP
let mut cleanup = ResourceCleanup::new();
cleanup.add_file(PathBuf::from(&wav_path));
let _guard = CleanupGuard::new(cleanup, |c| { /* cleanup */ });
// Files automatically cleaned up even on error
```

## Configuration

All limits are configurable via environment variables:

```bash
# File Upload Limits
MAX_FILE_SIZE=104857600           # 100MB in bytes
MAX_CONCURRENT_UPLOADS=10         # Per user
MAX_USER_STORAGE=10737418240      # 10GB in bytes
ALLOWED_EXTENSIONS=jpg,png,pdf    # Comma-separated
BLOCKED_EXTENSIONS=exe,dll,bat    # Comma-separated
ENABLE_VIRUS_SCAN=true            # Enable virus scanning

# Request Limits
MAX_BODY_SIZE=10485760            # 10MB in bytes
MAX_PROCESSING_TIME=300           # 5 minutes in seconds
MAX_CONCURRENT_PER_IP=100        # Max concurrent requests
MAX_HEADER_SIZE=8192              # 8KB in bytes

# Memory Limits
MAX_MEMORY_PER_REQUEST=536870912  # 512MB in bytes
MAX_BUFFER_SIZE=65536             # 64KB streaming buffer
MEMORY_WARNING_THRESHOLD=0.8      # 80% warning
MEMORY_CRITICAL_THRESHOLD=0.95    # 95% critical

# Cleanup Configuration
ENABLE_AUTO_CLEANUP=true          # Enable background cleanup
CLEANUP_INTERVAL=3600             # 1 hour in seconds
TEMP_DIR=/opt/sam/tmp            # Temp file directory
ORPHAN_AGE_THRESHOLD=86400        # 24 hours in seconds

# Rate Limiting
DEFAULT_AUTH_RATE_LIMIT=1000      # Authenticated users
DEFAULT_ANON_RATE_LIMIT=100       # Anonymous users
RATE_LIMIT_WINDOW_SECONDS=60      # Time window
USE_REDIS_RATE_LIMIT=true         # Use Redis for distributed limiting
```

## Usage Examples

### 1. File Upload with Limits
```rust
let manager = ResourceManager::new(config);

// Check if upload is allowed
match manager.check_upload_allowed(user_id, file_size, extension).await? {
    UploadPermission::Allowed { permit } => {
        // Process upload with automatic cleanup
        let processed = manager.process_upload(file_data, filename, user_id).await?;
        // permit automatically released when dropped
    }
    UploadPermission::Denied { reason } => {
        return Err(format!("Upload denied: {}", reason));
    }
}
```

### 2. Request Processing with Timeout
```rust
let limiter = RequestLimiter::new(limits);

// Process with timeout
let result = limiter.process_with_timeout(
    async { 
        // Your processing logic here
    },
    Duration::from_secs(300)
).await?;
```

### 3. Connection Pool Usage
```rust
let pool = ConnectionPool::new(config, factory).await?;

// Get connection with automatic return
let conn = pool.get().await?;
// Use connection...
// Automatically returned to pool when dropped
```

### 4. Temporary File with Cleanup
```rust
let temp_file = TempFile::new(&temp_dir)?;
temp_file.write(&data).await?;

// Option 1: Auto cleanup on drop
drop(temp_file);

// Option 2: Move to permanent location
temp_file.move_to(&permanent_path).await?;

// Option 3: Keep the file
let path = temp_file.persist();
```

## Testing

Comprehensive test suite implemented in `tests/resource_management_tests.rs`:
- File upload limit tests
- Concurrent upload limit tests
- Request size limit tests
- Memory allocation tests
- Connection pool tests
- Automatic cleanup tests
- RAII pattern tests
- Integration tests

Run tests:
```bash
cargo test resource_management
```

## Security Benefits

1. **DoS Prevention**: Rate limiting and resource limits prevent abuse
2. **Memory Protection**: Prevents memory exhaustion attacks
3. **Disk Protection**: File size limits and cleanup prevent disk filling
4. **Connection Protection**: Pool limits prevent connection exhaustion
5. **Process Protection**: Request timeouts prevent hanging processes
6. **Type Safety**: File extension validation prevents malicious uploads
7. **Cleanup Guarantee**: RAII ensures resources are always freed

## Performance Impact

- **Minimal overhead**: Most checks are O(1) operations
- **Efficient pooling**: Connection reuse reduces database load
- **Async operations**: Non-blocking I/O for better concurrency
- **Smart caching**: Reuses processed results when possible
- **Background cleanup**: Doesn't impact request processing

## Monitoring and Metrics

Access metrics via:
```rust
let metrics = resource_manager.get_metrics().await;
// Returns CPU, memory, disk, network, request metrics
```

## Future Enhancements

1. **ClamAV Integration**: Full virus scanning implementation
2. **S3 Storage**: Option to store files in S3 instead of local disk
3. **Rate Limit Customization**: Per-user custom limits
4. **Advanced Monitoring**: Prometheus/Grafana integration
5. **Machine Learning**: Anomaly detection for resource usage patterns

## Migration Guide

To use the improved observations handler:
```rust
// Replace in your routing:
// OLD: crate::sam::http::api::observations::handle
// NEW: crate::sam::http::api::observations_improved::handle
```

The new implementation is fully backward compatible while adding resource protection.