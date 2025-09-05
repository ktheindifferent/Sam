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

pub fn stop_service() -> anyhow::Result<()> {
    // TODO: Implement proper service stopping
    Ok(())
}

pub fn status_service() -> anyhow::Result<String> {
    // TODO: Implement proper service status check
    Ok("Running".to_string())
}

pub fn handle(session: Option<String>, request: &rouille::Request) -> rouille::Response {
    // TODO: Implement proper LIFX HTTP API handling
    rouille::Response::text("LIFX service not fully implemented")
}

pub async fn start_server() -> anyhow::Result<()> {
    log::info!("LIFX server started");
    Ok(())
}