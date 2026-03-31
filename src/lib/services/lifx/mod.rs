pub mod api_server;
pub mod bulb;
pub mod config;
pub mod discovery;
pub mod handlers;
pub mod protocol;
pub mod traits;

pub use api_server::start;
pub use config::Config;
pub use traits::{LightControl, LightDevice};

// Alias functions for CLI compatibility
pub use start as start_service;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);
static BULB_COUNT: AtomicUsize = AtomicUsize::new(0);
static API_PORT: AtomicUsize = AtomicUsize::new(0);

pub fn stop_service() -> anyhow::Result<()> {
    SERVICE_RUNNING.store(false, Ordering::SeqCst);
    log::info!("LIFX service stopped");
    Ok(())
}

pub fn status_service() -> anyhow::Result<String> {
    if SERVICE_RUNNING.load(Ordering::SeqCst) {
        let bulb_count = BULB_COUNT.load(Ordering::SeqCst);
        let port = API_PORT.load(Ordering::SeqCst);
        Ok(format!("Running - {} bulbs discovered (port {})", bulb_count, port))
    } else {
        Ok("Stopped".to_string())
    }
}

pub fn get_status_json() -> serde_json::Value {
    let running = SERVICE_RUNNING.load(Ordering::SeqCst);
    let bulb_count = BULB_COUNT.load(Ordering::SeqCst);
    let port = API_PORT.load(Ordering::SeqCst);

    serde_json::json!({
        "running": running,
        "bulb_count": bulb_count,
        "api_port": port,
        "status": if running { "healthy" } else { "stopped" }
    })
}

pub fn set_service_state(running: bool, bulb_count: usize, port: u16) {
    SERVICE_RUNNING.store(running, Ordering::SeqCst);
    BULB_COUNT.store(bulb_count, Ordering::SeqCst);
    API_PORT.store(port as usize, Ordering::SeqCst);
}

pub fn handle(session: Option<String>, request: &rouille::Request) -> rouille::Response {
    use rouille::Response;

    // Handle status endpoint
    if request.url().contains("/api/services/lifx/status") {
        return Response::json(&get_status_json());
    }

    // Handle other LIFX API endpoints
    if request.url().contains("/api/services/lifx/") {
        // Check authorization for protected endpoints
        let auth_header = request.header("Authorization");
        if auth_header.is_none() && !request.url().contains("/status") {
            return Response::text("Unauthorized").with_status_code(401);
        }

        // Delegate to API server handler
        return handlers::handle_api_request(request);
    }

    Response::empty_404()
}

pub async fn start_server() -> anyhow::Result<()> {
    log::info!("LIFX server started");
    Ok(())
}