use rouille::{Request, Response};
use serde::{Deserialize, Serialize};
use log::{info, error, warn};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub status_text: String,
    pub metrics: Option<ServiceMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub connections: Option<u32>,
    pub memory: Option<u64>,
    pub keys: Option<u32>,
    pub pages_crawled: Option<u32>,
    pub queue_size: Option<u32>,
    pub last_run: Option<i64>,
    pub containers: Option<u32>,
    pub images: Option<u32>,
    pub version: Option<String>,
    pub db_size: Option<u64>,
    pub sessions: Option<u32>,
    pub messages_per_sec: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct EnvironmentInfo {
    pub is_caprover: bool,
    pub redis_configured: bool,
    pub postgres_configured: bool,
    pub tts_configured: bool,
    pub stt_configured: bool,
}

/// Handle service control API endpoints
pub fn handle(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();
    
    // Environment check endpoint
    if url == "/api/environment" {
        return handle_environment_check();
    }
    
    // Redis service endpoints
    if url.contains("/api/services/redis") {
        return handle_redis_service(request);
    }
    
    // Crawler service endpoints
    if url.contains("/api/services/crawler") {
        return handle_crawler_service(request);
    }
    
    // Docker service endpoints
    if url.contains("/api/services/docker") {
        return handle_docker_service(request);
    }
    
    // PostgreSQL service endpoints
    if url.contains("/api/services/postgres") {
        return handle_postgres_service(request);
    }
    
    // Voice service endpoints
    if url.contains("/api/services/voice") {
        return handle_voice_service(request);
    }
    
    // WebSocket service endpoints
    if url.contains("/api/services/websocket") {
        return handle_websocket_service(request);
    }
    
    // Get all services status
    if url == "/api/services/status" {
        return handle_all_services_status();
    }
    
    Ok(Response::empty_404())
}

/// Check environment configuration
fn handle_environment_check() -> Result<Response, crate::sam::http::Error> {
    use crate::sam::services::environment::get_env_config;
    
    let env_config = get_env_config();
    
    let info = EnvironmentInfo {
        is_caprover: env_config.is_caprover,
        redis_configured: env_config.redis_url.is_some(),
        postgres_configured: env_config.postgres_url.is_some(),
        tts_configured: env_config.tts_url.is_some(),
        stt_configured: env_config.stt_url.is_some(),
    };
    
    Ok(Response::json(&info))
}

/// Handle Redis service control
fn handle_redis_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();
    
    if url.ends_with("/status") {
        // Get Redis status
        let rt = tokio::runtime::Handle::current();
        let is_running = rt.block_on(async {
            crate::sam::services::redis::is_running().await
        });
        
        let status = ServiceStatus {
            running: is_running,
            status_text: if is_running { "running" } else { "stopped" }.to_string(),
            metrics: if is_running {
                // Try to get Redis metrics
                let pool_status = rt.block_on(async {
                    crate::sam::services::redis::get_pool_status().await.ok()
                });
                
                Some(ServiceMetrics {
                    connections: pool_status.as_ref().and_then(|s| {
                        // Parse connections from status string
                        s.split("Available: ").nth(1)
                            .and_then(|s| s.split(',').next())
                            .and_then(|s| s.parse().ok())
                    }),
                    memory: None, // Would need Redis INFO command
                    keys: None,    // Would need Redis DBSIZE command
                    ..Default::default()
                })
            } else {
                None
            },
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        info!("Starting Redis service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            crate::sam::services::redis::start().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Stopping Redis service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            crate::sam::services::redis::stop().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }
    
    Ok(Response::empty_404())
}

/// Handle Crawler service control
fn handle_crawler_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();
    
    if url.ends_with("/status") {
        // Get Crawler status
        let crawler_status = crate::sam::services::crawler::service_status();
        
        let is_running = crawler_status.contains("running");
        
        let status = ServiceStatus {
            running: is_running,
            status_text: crawler_status.clone(),
            metrics: if is_running {
                Some(ServiceMetrics {
                    pages_crawled: Some(0), // Would need actual crawler stats
                    queue_size: Some(0),
                    last_run: None,
                    ..Default::default()
                })
            } else {
                None
            },
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        info!("Starting Crawler service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            crate::sam::services::crawler::start_service_async().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Stopping Crawler service...");
        crate::sam::services::crawler::stop_service();
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }
    
    Ok(Response::empty_404())
}

/// Handle Docker service control
fn handle_docker_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    use crate::sam::services::environment::get_env_config;
    let env_config = get_env_config();
    
    let url = request.url();
    
    if url.ends_with("/status") {
        // Check if we're in CapRover mode
        if env_config.is_caprover {
            let status = ServiceStatus {
                running: false,
                status_text: "Disabled in CapRover mode".to_string(),
                metrics: None,
            };
            return Ok(Response::json(&status));
        }
        
        // Get Docker status
        let is_running = crate::sam::services::docker::is_running();
        
        let status = ServiceStatus {
            running: is_running,
            status_text: if is_running { "running" } else { "stopped" }.to_string(),
            metrics: None, // Docker metrics would require docker API calls
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "Docker management disabled in CapRover mode"
            })));
        }
        
        info!("Starting Docker service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            crate::sam::services::docker::start().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "Docker management disabled in CapRover mode"
            })));
        }
        
        info!("Stopping Docker service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            crate::sam::services::docker::stop().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }
    
    Ok(Response::empty_404())
}

/// Handle PostgreSQL service control
fn handle_postgres_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    use crate::sam::services::environment::get_env_config;
    let env_config = get_env_config();
    
    let url = request.url();
    
    if url.ends_with("/status") {
        // In CapRover mode, check external PostgreSQL
        if env_config.is_caprover {
            let status = ServiceStatus {
                running: env_config.postgres_url.is_some(),
                status_text: if env_config.postgres_url.is_some() { 
                    "External PostgreSQL configured" 
                } else { 
                    "Not configured" 
                }.to_string(),
                metrics: None,
            };
            return Ok(Response::json(&status));
        }
        
        // Check local PostgreSQL container
        let rt = tokio::runtime::Handle::current();
        let is_running = rt.block_on(async {
            // Check if PostgreSQL container is running
            tokio::process::Command::new("docker")
                .args(&["ps", "--filter", "name=sam-postgres", "--format", "{{.Names}}"])
                .output()
                .await
                .map(|output| String::from_utf8_lossy(&output.stdout).contains("sam-postgres"))
                .unwrap_or(false)
        });
        
        let status = ServiceStatus {
            running: is_running,
            status_text: if is_running { "running" } else { "stopped" }.to_string(),
            metrics: None,
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "PostgreSQL is externally managed in CapRover mode"
            })));
        }
        
        info!("Starting PostgreSQL service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            if let Err(e) = crate::sam::services::docker::start_postgres().await {
                error!("Failed to start PostgreSQL: {}", e);
            }
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "PostgreSQL is externally managed in CapRover mode"
            })));
        }
        
        info!("Stopping PostgreSQL service...");
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            if let Err(e) = crate::sam::services::docker::stop_postgres().await {
                error!("Failed to stop PostgreSQL: {}", e);
            }
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }
    
    Ok(Response::empty_404())
}

/// Handle Voice service control
fn handle_voice_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();
    
    if url.ends_with("/status") {
        // Voice service status would depend on STT/TTS availability
        let status = ServiceStatus {
            running: false, // Would need actual voice service check
            status_text: "stopped".to_string(),
            metrics: None,
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        info!("Voice service start requested (not implemented)");
        return Ok(Response::json(&serde_json::json!({"status": "not_implemented"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Voice service stop requested (not implemented)");
        return Ok(Response::json(&serde_json::json!({"status": "not_implemented"})));
    }
    
    Ok(Response::empty_404())
}

/// Handle WebSocket service control
fn handle_websocket_service(request: &Request) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();
    
    if url.ends_with("/status") {
        // Check WebSocket server status
        let status = ServiceStatus {
            running: true, // WebSocket runs with HTTP server
            status_text: "running".to_string(),
            metrics: Some(ServiceMetrics {
                connections: Some(0), // Would need actual connection tracking
                messages_per_sec: Some(0),
                ..Default::default()
            }),
        };
        
        return Ok(Response::json(&status));
    }
    
    if url.ends_with("/start") && request.method() == "POST" {
        info!("WebSocket service is always running with HTTP server");
        return Ok(Response::json(&serde_json::json!({"status": "already_running"})));
    }
    
    if url.ends_with("/stop") && request.method() == "POST" {
        info!("WebSocket service cannot be stopped independently");
        return Ok(Response::json(&serde_json::json!({"status": "cannot_stop"})));
    }
    
    Ok(Response::empty_404())
}

/// Get status of all services
fn handle_all_services_status() -> Result<Response, crate::sam::http::Error> {
    let mut statuses = HashMap::new();
    
    // Check each service
    let rt = tokio::runtime::Handle::current();
    
    // Redis
    let redis_running = rt.block_on(async {
        crate::sam::services::redis::is_running().await
    });
    statuses.insert("redis", ServiceStatus {
        running: redis_running,
        status_text: if redis_running { "running" } else { "stopped" }.to_string(),
        metrics: None,
    });
    
    // Crawler
    let crawler_status = crate::sam::services::crawler::service_status();
    let crawler_running = crawler_status.contains("running");
    statuses.insert("crawler", ServiceStatus {
        running: crawler_running,
        status_text: crawler_status,
        metrics: None,
    });
    
    // Docker
    let docker_running = crate::sam::services::docker::is_running();
    statuses.insert("docker", ServiceStatus {
        running: docker_running,
        status_text: if docker_running { "running" } else { "stopped" }.to_string(),
        metrics: None,
    });
    
    // WebSocket (always running with HTTP)
    statuses.insert("websocket", ServiceStatus {
        running: true,
        status_text: "running".to_string(),
        metrics: None,
    });
    
    Ok(Response::json(&statuses))
}

impl Default for ServiceMetrics {
    fn default() -> Self {
        Self {
            connections: None,
            memory: None,
            keys: None,
            pages_crawled: None,
            queue_size: None,
            last_run: None,
            containers: None,
            images: None,
            version: None,
            db_size: None,
            sessions: None,
            messages_per_sec: None,
        }
    }
}