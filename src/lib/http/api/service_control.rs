use log::{error, info};
use rouille::{Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub status_text: String,
    pub metrics: Option<ServiceMetrics>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
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
    pub crawler_threads: Option<String>,
    pub crawler_dns_threads: Option<String>,
    pub crawler_disabled: Option<String>,
}

/// Handle service control API endpoints
pub fn handle(request: &Request) -> Result<Response, crate::http::Error> {
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

    // Ollama service endpoints
    if url.contains("/api/services/ollama") {
        return handle_ollama_service(request);
    }

    // NextCloud service endpoints
    if url.contains("/api/services/nextcloud") {
        return handle_nextcloud_service(request);
    }

    // Get all services status
    if url == "/api/services/status" {
        return handle_all_services_status();
    }

    Ok(Response::empty_404())
}

/// Check environment configuration
fn handle_environment_check() -> Result<Response, crate::http::Error> {
    use crate::services::environment::get_env_config;

    let env_config = get_env_config();

    let info = EnvironmentInfo {
        is_caprover: env_config.is_caprover,
        redis_configured: env_config.redis_url.is_some(),
        postgres_configured: env_config.postgres_url.is_some(),
        tts_configured: env_config.tts_url.is_some(),
        stt_configured: env_config.stt_url.is_some(),
        crawler_threads: std::env::var("CRAWLER_THREADS").ok(),
        crawler_dns_threads: std::env::var("CRAWLER_DNS_THREADS").ok(),
        crawler_disabled: std::env::var("CRAWLER_DISABLED").ok(),
    };

    Ok(Response::json(&info))
}

/// Handle Redis service control
fn handle_redis_service(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();

    if url.ends_with("/status") {
        // Get Redis status
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        let is_running = rt.block_on(async { crate::services::redis::is_running().await });

        let status = ServiceStatus {
            running: is_running,
            status_text: if is_running { "running" } else { "stopped" }.to_string(),
            metrics: if is_running {
                // Try to get Redis metrics
                let pool_status =
                    rt.block_on(async { crate::services::redis::get_pool_status().await.ok() });

                Some(ServiceMetrics {
                    connections: pool_status.as_ref().and_then(|s| {
                        // Parse connections from status string
                        s.split("Available: ")
                            .nth(1)
                            .and_then(|s| s.split(',').next())
                            .and_then(|s| s.parse().ok())
                    }),
                    memory: None, // Would need Redis INFO command
                    keys: None,   // Would need Redis DBSIZE command
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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::redis::start().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }

    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Stopping Redis service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::redis::stop().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        info!("Restarting Redis service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::redis::stop().await;
            crate::services::redis::start().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "restarted"})));
    }

    Ok(Response::empty_404())
}

/// Handle Crawler service control
fn handle_crawler_service(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();

    if url.ends_with("/status") {
        // Get Crawler status
        let crawler_status = crate::services::crawler::service_status();

        let is_running = crawler_status.contains("running");

        let status = ServiceStatus {
            running: is_running,
            status_text: crawler_status.to_string(),
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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::crawler::start_service_async().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }

    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Stopping Crawler service...");
        crate::services::crawler::stop_service();
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        info!("Restarting Crawler service...");
        crate::services::crawler::stop_service();
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::crawler::start_service_async().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "restarted"})));
    }

    Ok(Response::empty_404())
}

/// Handle Docker service control
fn handle_docker_service(request: &Request) -> Result<Response, crate::http::Error> {
    use crate::services::environment::get_env_config;
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
        let is_running = crate::services::docker::is_running();

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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::docker::start().await;
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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::docker::stop().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "Docker management disabled in CapRover mode"
            }))
            .with_status_code(409));
        }

        info!("Restarting Docker service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::docker::stop().await;
            crate::services::docker::start().await;
        });
        return Ok(Response::json(&serde_json::json!({"status": "restarted"})));
    }

    Ok(Response::empty_404())
}

/// Handle PostgreSQL service control
fn handle_postgres_service(request: &Request) -> Result<Response, crate::http::Error> {
    use crate::services::environment::get_env_config;
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
                }
                .to_string(),
                metrics: None,
            };
            return Ok(Response::json(&status));
        }

        // Check local PostgreSQL container
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        let is_running = rt.block_on(async {
            // Check if PostgreSQL container is running
            tokio::process::Command::new("docker")
                .args([
                    "ps",
                    "--filter",
                    "name=sam-postgres",
                    "--format",
                    "{{.Names}}",
                ])
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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            if let Err(e) = crate::services::docker::start_postgres().await {
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
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            if let Err(e) = crate::services::docker::stop_postgres().await {
                error!("Failed to stop PostgreSQL: {}", e);
            }
        });
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        if env_config.is_caprover {
            return Ok(Response::json(&serde_json::json!({
                "error": "PostgreSQL is externally managed in CapRover mode"
            }))
            .with_status_code(409));
        }

        info!("Restarting PostgreSQL service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            if let Err(e) = crate::services::docker::stop_postgres().await {
                error!("Failed to stop PostgreSQL: {}", e);
            }
            if let Err(e) = crate::services::docker::start_postgres().await {
                error!("Failed to start PostgreSQL: {}", e);
            }
        });
        return Ok(Response::json(&serde_json::json!({"status": "restarted"})));
    }

    Ok(Response::empty_404())
}

/// Handle Voice service control
fn handle_voice_service(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();

    if url.ends_with("/status") {
        let running = crate::services::voice::is_initialized();
        let status = ServiceStatus {
            running,
            status_text: if running { "running" } else { "stopped" }.to_string(),
            metrics: None,
        };

        return Ok(Response::json(&status));
    }

    if url.ends_with("/start") && request.method() == "POST" {
        info!("Starting voice service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::voice::initialize()
                .await
                .map_err(|e| crate::http::Error::from(e.to_string()))
        })?;
        return Ok(Response::json(&serde_json::json!({"status": "started"})));
    }

    if url.ends_with("/stop") && request.method() == "POST" {
        info!("Stopping voice service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::voice::shutdown()
                .await
                .map_err(|e| crate::http::Error::from(e.to_string()))
        })?;
        return Ok(Response::json(&serde_json::json!({"status": "stopped"})));
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        info!("Restarting voice service...");
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
        rt.block_on(async {
            crate::services::voice::shutdown()
                .await
                .map_err(|e| crate::http::Error::from(e.to_string()))?;
            crate::services::voice::initialize()
                .await
                .map_err(|e| crate::http::Error::from(e.to_string()))
        })?;
        return Ok(Response::json(&serde_json::json!({"status": "restarted"})));
    }

    Ok(Response::empty_404())
}

/// Handle WebSocket service control
fn handle_websocket_service(request: &Request) -> Result<Response, crate::http::Error> {
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
        return Ok(Response::json(
            &serde_json::json!({"status": "already_running"}),
        ));
    }

    if url.ends_with("/stop") && request.method() == "POST" {
        info!("WebSocket service cannot be stopped independently");
        return Ok(Response::json(
            &serde_json::json!({"status": "cannot_stop"}),
        ));
    }

    Ok(Response::empty_404())
}

/// Handle Ollama service control
fn handle_ollama_service(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();
    let rt = tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
    let service = crate::services::llms::ollama::OllamaService::new_with_defaults();

    if url.ends_with("/status") {
        return Ok(Response::json(&ollama_status(&rt, &service)));
    }

    if url.ends_with("/start") && request.method() == "POST" {
        let result = rt.block_on(async { service.start_service().await });
        return service_action_response("started", result);
    }

    if url.ends_with("/stop") && request.method() == "POST" {
        let result = rt.block_on(async { service.stop_service().await });
        return service_action_response("stopped", result);
    }

    if url.ends_with("/restart") && request.method() == "POST" {
        let _ = rt.block_on(async { service.stop_service().await });
        let result = rt.block_on(async { service.start_service().await });
        return service_action_response("restarted", result);
    }

    Ok(Response::empty_404())
}

/// Handle NextCloud service status
fn handle_nextcloud_service(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();

    if url.ends_with("/status") {
        return Ok(Response::json(&nextcloud_status()));
    }

    Ok(Response::empty_404())
}

fn service_action_response(
    status: &str,
    result: anyhow::Result<String>,
) -> Result<Response, crate::http::Error> {
    match result {
        Ok(message) => Ok(Response::json(&serde_json::json!({
            "status": status,
            "message": message
        }))),
        Err(e) => Ok(Response::json(&serde_json::json!({
            "error": e.to_string()
        }))
        .with_status_code(500)),
    }
}

fn ollama_status(
    rt: &tokio::runtime::Runtime,
    service: &crate::services::llms::ollama::OllamaService,
) -> ServiceStatus {
    let installed = rt.block_on(async { service.is_installed().await });
    let running = if installed {
        rt.block_on(async { service.is_running().await })
    } else {
        false
    };
    let version = if running {
        rt.block_on(async { service.get_version().await.ok() })
    } else {
        None
    };

    ServiceStatus {
        running,
        status_text: if !installed {
            "Ollama not installed".to_string()
        } else if running {
            "running".to_string()
        } else {
            "installed but stopped".to_string()
        },
        metrics: Some(ServiceMetrics {
            version,
            ..Default::default()
        }),
    }
}

fn nextcloud_status() -> ServiceStatus {
    let env_configured = std::env::var("NEXTCLOUD_URL").is_ok()
        || std::env::var("NEXTCLOUD_ENDPOINT").is_ok()
        || std::env::var("NEXTCLOUD_SERVER_URL").is_ok();

    let storage_configured =
        crate::memory::config::FileStorageLocation::select(None, None, None, None)
            .map(|locations| {
                locations.iter().any(|location| {
                    location.storage_type.eq_ignore_ascii_case("nextcloud")
                        || location.endpoint.to_lowercase().contains("nextcloud")
                })
            })
            .unwrap_or(false);

    let service_configured = crate::memory::config::Service::select(None, None, None, None)
        .map(|services| {
            services.iter().any(|service| {
                service.identifier.eq_ignore_ascii_case("nextcloud")
                    || service.endpoint.to_lowercase().contains("nextcloud")
            })
        })
        .unwrap_or(false);

    let configured = env_configured || storage_configured || service_configured;

    ServiceStatus {
        running: configured,
        status_text: if configured {
            "configured".to_string()
        } else {
            "not configured".to_string()
        },
        metrics: None,
    }
}

/// Get status of all services
fn handle_all_services_status() -> Result<Response, crate::http::Error> {
    let mut statuses = HashMap::new();

    // Check each service
    let rt = tokio::runtime::Runtime::new().map_err(|e| crate::http::Error::from(e.to_string()))?;
    let env_config = crate::services::environment::get_env_config();

    // Redis
    let redis_running = rt.block_on(async { crate::services::redis::is_running().await });
    statuses.insert(
        "redis",
        ServiceStatus {
            running: redis_running,
            status_text: if env_config.is_caprover && redis_running {
                "CapRover Redis configured".to_string()
            } else if env_config.is_caprover {
                "Redis not configured".to_string()
            } else if redis_running {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            metrics: None,
        },
    );

    // Crawler
    let crawler_status = crate::services::crawler::service_status();
    let crawler_running = crawler_status.contains("running");
    statuses.insert(
        "crawler",
        ServiceStatus {
            running: crawler_running,
            status_text: crawler_status.to_string(),
            metrics: None,
        },
    );

    // Docker
    let docker_running = if env_config.is_caprover {
        false
    } else {
        crate::services::docker::is_running()
    };
    statuses.insert(
        "docker",
        ServiceStatus {
            running: docker_running,
            status_text: if env_config.is_caprover {
                "Disabled in CapRover mode".to_string()
            } else if docker_running {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            metrics: None,
        },
    );

    // PostgreSQL
    statuses.insert(
        "postgres",
        ServiceStatus {
            running: if env_config.is_caprover {
                env_config.postgres_url.is_some()
            } else {
                env_config.should_use_postgres()
            },
            status_text: if env_config.is_caprover && env_config.postgres_url.is_some() {
                "External PostgreSQL configured".to_string()
            } else if env_config.should_use_postgres() {
                "configured".to_string()
            } else {
                "not configured".to_string()
            },
            metrics: None,
        },
    );

    // Ollama
    let ollama = crate::services::llms::ollama::OllamaService::new_with_defaults();
    statuses.insert("ollama", ollama_status(&rt, &ollama));

    // Voice
    let voice_running = if env_config.is_caprover {
        env_config.tts_url.is_some() || env_config.stt_url.is_some()
    } else {
        crate::services::voice::is_initialized()
    };
    statuses.insert(
        "voice",
        ServiceStatus {
            running: voice_running,
            status_text: if env_config.is_caprover && voice_running {
                "CapRover voice endpoint configured".to_string()
            } else if env_config.is_caprover {
                "voice endpoint not configured".to_string()
            } else if voice_running {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            metrics: None,
        },
    );

    // NextCloud
    statuses.insert("nextcloud", nextcloud_status());

    // WebSocket (always running with HTTP)
    statuses.insert(
        "websocket",
        ServiceStatus {
            running: true,
            status_text: "running".to_string(),
            metrics: None,
        },
    );

    Ok(Response::json(&statuses))
}
