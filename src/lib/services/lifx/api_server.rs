use super::config::Config;
use super::discovery::DiscoveryService;
use super::handlers::HttpHandlers;
use rouille::Response;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use crate::services::thread_manager::{self, ThreadConfig};

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
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        config.validate()?;
        
        let source = 0x72757374; // "rust" in hex
        let discovery = Arc::new(Mutex::new(DiscoveryService::new(source)?));
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

        // Background refresh thread
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
            log::info!("LIFX API refresh thread started");
            while !stop_flag_bg.load(Ordering::SeqCst) && !shutdown_signal.load(Ordering::Relaxed) {
                if let Ok(discovery) = discovery_clone.lock() {
                    discovery.refresh_devices();
                }
                thread::sleep(refresh_interval);
            }
            log::info!("LIFX API refresh thread stopped");
        });

        // HTTP server thread
        let discovery_clone = self.discovery.clone();
        let handlers = self.handlers.clone();
        let config = self.config.clone();
        let stop_flag_http2 = stop_flag_http.clone();

        let http_thread = thread::Builder::new()
            .name("lifx_api_http".to_string())
            .spawn(move || {
            let server = rouille::Server::new(
                format!("0.0.0.0:{}", config.port),
                move |request| {
                    if stop_flag_http.load(Ordering::SeqCst) {
                        return Response::empty_404();
                    }

                    // Check authorization
                    let auth_header = request.header("Authorization");
                    if auth_header.is_none() {
                        return Response::empty_404();
                    } else if *auth_header.unwrap() != format!("Bearer {}", config.secret_key) {
                        return Response::empty_404();
                    }

                    // Parse URL
                    let urls = request.url().to_string();
                    let split = urls.split("/");
                    let vec: Vec<&str> = split.collect();

                    let selector = if vec.len() >= 3 {
                        vec[3]
                    } else {
                        "all"
                    };

                    // Handle requests
                    let discovery = match discovery_clone.lock() {
                        Ok(d) => d,
                        Err(_) => return Response::text("Internal Server Error").with_status_code(500),
                    };

                    let bulbs_arc = discovery.get_bulbs();
                    let bulbs = match bulbs_arc.lock() {
                        Ok(b) => b,
                        Err(_) => return Response::text("Internal Server Error").with_status_code(500),
                    };

                    if request.url().contains("/lights/states") {
                        // Implement bulk state changes using existing handler with "all" selector
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
                },
            );

            match server {
                Ok(server) => {
                    log::info!("LIFX API server started on port {}", config.port);
                    while !stop_flag_http2.load(Ordering::SeqCst) {
                        server.poll();
                        thread::sleep(Duration::from_millis(10));
                    }
                }
                Err(e) => {
                    log::error!("Failed to bind LIFX API server: {}", e);
                }
            }
        }).unwrap();

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