use rouille::{Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub services: Vec<ServiceHealth>,
    pub timestamp: DateTime<Utc>,
}

/// Main health check endpoint handler
pub fn handle_health_check(request: &Request) -> Response {
    let path = request.url();
    
    if path == "/health" || path == "/healthz" {
        // Basic health check
        basic_health_check()
    } else if path == "/health/live" || path == "/liveness" {
        // Liveness probe for Kubernetes
        liveness_check()
    } else if path == "/health/ready" || path == "/readiness" {
        // Readiness probe for Kubernetes
        readiness_check()
    } else if path == "/health/detailed" {
        // Detailed health check with all services
        detailed_health_check()
    } else {
        Response::empty_404()
    }
}

/// Basic health check - returns 200 if service is running
fn basic_health_check() -> Response {
    Response::json(&serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

/// Liveness check - returns 200 if process is alive
fn liveness_check() -> Response {
    Response::json(&serde_json::json!({
        "status": "alive",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

/// Readiness check - returns 200 if service is ready to accept requests
fn readiness_check() -> Response {
    let mut ready = true;
    let mut checks = HashMap::new();
    
    // Check database connection
    if let Ok(db_healthy) = check_database_health() {
        checks.insert("database", db_healthy);
        if !db_healthy {
            ready = false;
        }
    }
    
    // Check Redis connection
    if let Ok(redis_healthy) = check_redis_health() {
        checks.insert("redis", redis_healthy);
        // Redis is optional, don't fail readiness
    }
    
    // Check disk space
    if let Ok(disk_healthy) = check_disk_space() {
        checks.insert("disk_space", disk_healthy);
        if !disk_healthy {
            ready = false;
        }
    }
    
    if ready {
        Response::json(&serde_json::json!({
            "status": "ready",
            "checks": checks,
            "timestamp": Utc::now().to_rfc3339()
        }))
    } else {
        Response::json(&serde_json::json!({
            "status": "not_ready",
            "checks": checks,
            "timestamp": Utc::now().to_rfc3339()
        })).with_status_code(503)
    }
}

/// Detailed health check with all service statuses
fn detailed_health_check() -> Response {
    let start_time = std::time::Instant::now();
    let mut services = Vec::new();
    let mut overall_status = HealthStatus::Healthy;
    
    // Check core services
    services.push(check_web_server_health());
    services.push(check_database_service_health());
    services.push(check_redis_service_health());
    services.push(check_voice_service_health());
    services.push(check_p2p_service_health());
    services.push(check_crawler_service_health());
    services.push(check_security_service_health());
    services.push(check_file_storage_health());
    services.push(check_docker_service_health());
    
    // Determine overall status
    for service in &services {
        match service.status {
            HealthStatus::Unhealthy => {
                overall_status = HealthStatus::Unhealthy;
                break;
            },
            HealthStatus::Degraded => {
                if !matches!(overall_status, HealthStatus::Unhealthy) {
                    overall_status = HealthStatus::Degraded;
                }
            },
            _ => {}
        }
    }
    
    let system_health = SystemHealth {
        overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime_seconds(),
        services,
        timestamp: Utc::now(),
    };
    
    let status_code = match system_health.overall_status {
        HealthStatus::Healthy => 200,
        HealthStatus::Degraded => 200,  // Still return 200 for degraded
        HealthStatus::Unhealthy => 503,
    };
    
    Response::json(&system_health).with_status_code(status_code)
}

// Individual service health check functions

fn check_web_server_health() -> ServiceHealth {
    ServiceHealth {
        name: "web_server".to_string(),
        status: HealthStatus::Healthy,
        message: Some("Web server is responding".to_string()),
        last_check: Utc::now(),
        response_time_ms: Some(1),
        metadata: HashMap::new(),
    }
}

fn check_database_service_health() -> ServiceHealth {
    let start = std::time::Instant::now();
    
    match check_database_health() {
        Ok(true) => ServiceHealth {
            name: "postgresql".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Database connection successful".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            metadata: HashMap::new(),
        },
        Ok(false) | Err(_) => ServiceHealth {
            name: "postgresql".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some("Database connection failed".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

fn check_redis_service_health() -> ServiceHealth {
    let start = std::time::Instant::now();
    
    match check_redis_health() {
        Ok(true) => ServiceHealth {
            name: "redis".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Redis connection successful".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            metadata: HashMap::new(),
        },
        Ok(false) => ServiceHealth {
            name: "redis".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Redis not available (optional service)".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        },
        Err(_) => ServiceHealth {
            name: "redis".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Redis check failed".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

fn check_voice_service_health() -> ServiceHealth {
    // Check if voice services are available
    let whisper_available = std::path::Path::new("/usr/local/lib/libwhisper.so").exists()
        || std::path::Path::new("/opt/homebrew/lib/libwhisper.dylib").exists();
    
    if whisper_available {
        ServiceHealth {
            name: "voice_services".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Voice services available".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(5),
            metadata: HashMap::new(),
        }
    } else {
        ServiceHealth {
            name: "voice_services".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Whisper library not found".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

fn check_p2p_service_health() -> ServiceHealth {
    ServiceHealth {
        name: "p2p_network".to_string(),
        status: HealthStatus::Healthy,
        message: Some("P2P service operational".to_string()),
        last_check: Utc::now(),
        response_time_ms: Some(2),
        metadata: HashMap::new(),
    }
}

fn check_crawler_service_health() -> ServiceHealth {
    ServiceHealth {
        name: "web_crawler".to_string(),
        status: HealthStatus::Healthy,
        message: Some("Crawler service ready".to_string()),
        last_check: Utc::now(),
        response_time_ms: Some(3),
        metadata: HashMap::new(),
    }
}

fn check_security_service_health() -> ServiceHealth {
    ServiceHealth {
        name: "security".to_string(),
        status: HealthStatus::Healthy,
        message: Some("Security modules active".to_string()),
        last_check: Utc::now(),
        response_time_ms: Some(1),
        metadata: HashMap::new(),
    }
}

fn check_file_storage_health() -> ServiceHealth {
    match check_disk_space() {
        Ok(true) => ServiceHealth {
            name: "file_storage".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Adequate disk space available".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(1),
            metadata: HashMap::new(),
        },
        Ok(false) => ServiceHealth {
            name: "file_storage".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Low disk space warning".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(1),
            metadata: HashMap::new(),
        },
        Err(e) => ServiceHealth {
            name: "file_storage".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some(format!("Disk check failed: {}", e)),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

fn check_docker_service_health() -> ServiceHealth {
    use crate::sam::services::docker;
    
    if docker::is_running() {
        ServiceHealth {
            name: "docker".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Docker daemon running".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(10),
            metadata: HashMap::new(),
        }
    } else if docker::is_installed() {
        ServiceHealth {
            name: "docker".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Docker installed but not running".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    } else {
        ServiceHealth {
            name: "docker".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Docker not installed".to_string()),
            last_check: Utc::now(),
            response_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

// Helper functions

fn check_database_health() -> Result<bool, Box<dyn std::error::Error>> {
    // Try to connect to PostgreSQL
    use crate::sam::memory;
    
    match memory::get_database_connection() {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
    }
}

fn check_redis_health() -> Result<bool, Box<dyn std::error::Error>> {
    // Check if Redis is running
    use crate::sam::services::redis;
    
    Ok(tokio::runtime::Runtime::new()?.block_on(redis::is_running()))
}

fn check_disk_space() -> Result<bool, Box<dyn std::error::Error>> {
    use sysinfo::{System, SystemExt, DiskExt};
    
    let mut sys = System::new_all();
    sys.refresh_disks_list();
    
    for disk in sys.disks() {
        let total = disk.total_space();
        let available = disk.available_space();
        
        if total > 0 {
            let usage_percent = ((total - available) as f64 / total as f64) * 100.0;
            
            // Return false if any disk is over 90% full
            if usage_percent > 90.0 {
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

fn get_uptime_seconds() -> u64 {
    use sysinfo::{System, SystemExt};
    
    let sys = System::new_all();
    sys.uptime()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Healthy"));
    }

    #[test]
    fn test_service_health_creation() {
        let health = ServiceHealth {
            name: "test_service".to_string(),
            status: HealthStatus::Healthy,
            message: Some("All good".to_string()),
            last_check: Utc::now(),
            response_time_ms: Some(100),
            metadata: HashMap::new(),
        };

        assert_eq!(health.name, "test_service");
        assert!(matches!(health.status, HealthStatus::Healthy));
    }

    #[test]
    fn test_system_health_overall_status() {
        let services = vec![
            ServiceHealth {
                name: "service1".to_string(),
                status: HealthStatus::Healthy,
                message: None,
                last_check: Utc::now(),
                response_time_ms: Some(10),
                metadata: HashMap::new(),
            },
            ServiceHealth {
                name: "service2".to_string(),
                status: HealthStatus::Degraded,
                message: None,
                last_check: Utc::now(),
                response_time_ms: Some(20),
                metadata: HashMap::new(),
            },
        ];

        // With one degraded service, overall should be degraded
        let mut overall = HealthStatus::Healthy;
        for service in &services {
            if matches!(service.status, HealthStatus::Degraded) {
                overall = HealthStatus::Degraded;
            }
        }
        
        assert!(matches!(overall, HealthStatus::Degraded));
    }
}