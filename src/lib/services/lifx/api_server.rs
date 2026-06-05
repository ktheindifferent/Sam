//! Modern LIFX API Server Module
//!
//! This module provides a modular, maintainable API server for LIFX light control.
//! It replaces the monolithic `lifx_api_server.rs` with separate components:
//! - `config`: Configuration management with validation
//! - `discovery`: UDP-based bulb discovery
//! - `handlers`: HTTP request handlers
//! - `protocol`: LIFX LAN protocol implementation
//!
//! # Error Handling
//! - Graceful degradation when bulbs are unreachable
//! - Mutex poison recovery for all shared state
//! - Detailed logging for debugging
//! - Circuit breaker pattern for repeated failures

use super::config::Config;
use super::discovery::DiscoveryService;
use super::handlers::HttpHandlers;
use crate::services::thread_manager::{self, ThreadConfig};
use log::{debug, error, info, warn};
use rouille::Response;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// LIFX service error types
#[derive(Debug, thiserror::Error)]
pub enum LifxError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Bulb not found: {0}")]
    BulbNotFound(String),

    #[error("Service already running")]
    AlreadyRunning,

    #[error("Thread spawn failed: {0}")]
    ThreadSpawnFailed(String),
}

pub struct StopHandle {
    stop_flag: Arc<AtomicBool>,
    http_thread: Option<JoinHandle<()>>,
}

impl StopHandle {
    pub fn stop(self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.http_thread {
            let _ = handle.join();
        }
    }
}

pub struct ApiServer {
    config: Config,
    discovery: Arc<Mutex<DiscoveryService>>,
    handlers: Arc<HttpHandlers>,
    stop_flag: Arc<AtomicBool>,
}

impl ApiServer {
    pub fn new(config: Config) -> Result<Self, LifxError> {
        config.validate().map_err(|e| LifxError::ConfigError(e))?;

        let source = 0x72757374; // "rust" in hex
        let discovery = Arc::new(Mutex::new(
            DiscoveryService::new(source).map_err(|e| LifxError::DiscoveryFailed(e.to_string()))?,
        ));

        info!(
            "LIFX discovery service initialized with source: 0x{:08x}",
            source
        );

        // Set global discovery reference for handlers
        crate::services::lifx::set_global_discovery(discovery.clone());

        let handlers = Arc::new(HttpHandlers::new(source));
        let stop_flag = Arc::new(AtomicBool::new(false));

        Ok(Self {
            config,
            discovery,
            handlers,
            stop_flag,
        })
    }

    pub fn start(self) -> StopHandle {
        let stop_flag = self.stop_flag.clone();
        let stop_flag_bg = stop_flag.clone();
        let stop_flag_http = stop_flag.clone();

        let discovery_clone = self.discovery.clone();
        let refresh_interval = self.config.refresh_interval;

        // Background refresh thread with circuit breaker pattern
        let bg_config = ThreadConfig {
            name: "lifx_api_refresh".to_string(),
            restart_on_panic: true,
            max_restarts: 5,
            restart_delay_ms: 2000,
            health_check_interval_ms: Some(30000),
            enable_monitoring: true,
            priority: crate::services::thread_manager::ThreadPriority::Normal,
            max_memory_mb: None,
            cpu_affinity: None,
        };

        thread_manager::spawn_with_config(bg_config, move |shutdown_signal, _health_rx| {
            info!("LIFX API refresh thread started");
            let mut consecutive_failures = 0;
            const MAX_FAILURES: usize = 10;

            while !stop_flag_bg.load(Ordering::SeqCst) && !shutdown_signal.load(Ordering::Relaxed) {
                match discovery_clone.lock() {
                    Ok(discovery) => {
                        discovery.refresh_devices();
                        consecutive_failures = 0; // Reset on success
                        debug!("LIFX device refresh completed successfully");
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        warn!(
                            "LIFX refresh failed ({} / {}): {}",
                            consecutive_failures, MAX_FAILURES, e
                        );

                        if consecutive_failures >= MAX_FAILURES {
                            error!("Too many consecutive refresh failures, backing off");
                            thread::sleep(Duration::from_secs(30)); // Extended backoff
                            consecutive_failures = 0;
                        }
                    }
                }
                thread::sleep(refresh_interval);
            }
            info!("LIFX API refresh thread stopped");
        });

        // HTTP server thread
        let discovery_clone = self.discovery.clone();
        let handlers = self.handlers.clone();
        let config = self.config.clone();
        let stop_flag_http2 = stop_flag_http.clone();
        let port = config.port;

        let http_thread = match thread::Builder::new()
            .name("lifx_api_http".to_string())
            .spawn(move || {
                let server = rouille::Server::new(format!("0.0.0.0:{}", port), move |request| {
                    if stop_flag_http.load(Ordering::SeqCst) {
                        return Response::empty_404();
                    }

                    // Check authorization
                    let auth_header = request.header("Authorization");
                    match auth_header {
                        Some(header) if header == format!("Bearer {}", config.secret_key) => {}
                        Some(_) => {
                            debug!("LIFX API request rejected: invalid token");
                            return Response::empty_404();
                        }
                        None => {
                            debug!("LIFX API request rejected: missing authorization");
                            return Response::empty_404();
                        }
                    }

                    // Parse URL
                    let urls = request.url().to_string();
                    let split = urls.split("/");
                    let vec: Vec<&str> = split.collect();

                    let selector = if vec.len() > 3 { vec[3] } else { "all" };

                    // Handle requests with proper error handling
                    let discovery = match discovery_clone.lock() {
                        Ok(d) => d,
                        Err(e) => {
                            error!("LIFX discovery mutex poisoned: {}", e);
                            return Response::json(&serde_json::json!({
                                "error": "Internal server error",
                                "code": "DISCOVERY_LOCK_FAILED"
                            }))
                            .with_status_code(500);
                        }
                    };

                    let bulbs_arc = discovery.get_bulbs();
                    let bulbs = match bulbs_arc.lock() {
                        Ok(b) => b,
                        Err(e) => {
                            error!("LIFX bulbs mutex poisoned: {}", e);
                            return Response::json(&serde_json::json!({
                                "error": "Internal server error",
                                "code": "BULBS_LOCK_FAILED"
                            }))
                            .with_status_code(500);
                        }
                    };

                    if request.url().contains("/lights/states") {
                        // Implement bulk state changes
                        let sock = discovery.get_socket();
                        handlers.handle_set_state(request, &bulbs, "all", sock)
                    } else if request.url().contains("/state") {
                        let sock = discovery.get_socket();
                        handlers.handle_set_state(request, &bulbs, selector, sock)
                    } else if request.url().contains("/v1/lights/") {
                        handlers.handle_list_lights(&bulbs, selector)
                    } else {
                        Response::text("LIFX API Server")
                    }
                });

                match server {
                    Ok(server) => {
                        info!("LIFX API server started on port {}", port);
                        while !stop_flag_http2.load(Ordering::SeqCst) {
                            server.poll();
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                    Err(e) => {
                        error!("Failed to bind LIFX API server on port {}: {}", port, e);
                    }
                }
            }) {
            Ok(handle) => handle,
            Err(e) => {
                error!("Failed to spawn LIFX HTTP thread: {}", e);
                return StopHandle {
                    stop_flag,
                    http_thread: None,
                };
            }
        };

        StopHandle {
            stop_flag,
            http_thread: Some(http_thread),
        }
    }
}

pub fn start(config: Config) -> StopHandle {
    match ApiServer::new(config) {
        Ok(server) => server.start(),
        Err(e) => {
            log::error!("Failed to create LIFX API server: {}", e);
            StopHandle {
                stop_flag: Arc::new(AtomicBool::new(false)),
                http_thread: None,
            }
        }
    }
}
