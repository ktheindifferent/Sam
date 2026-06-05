pub mod games;
pub mod image;
pub mod snapcast;
pub mod snapcast_api;
pub mod youtube;

use rouille::Request;
use rouille::Response;

pub fn handle(
    current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url().contains("/image") {
        return image::handle(current_session, request);
    }

    if request.url().contains("/games") {
        return games::handle(current_session, request);
    }

    // Snapcast media control API (no session required for local control)
    if request.url().contains("/snapcast") {
        return Ok(snapcast_api::handle(request));
    }

    Ok(Response::empty_404())
}

/// Initialize the media service
pub async fn initialize() -> anyhow::Result<()> {
    log::info!("Media service initialized");

    // Check for librespot availability and log status
    match snapcast::check_librespot() {
        Ok(path) => {
            log::info!("Spotify support enabled (librespot at {})", path);
        }
        Err(_) => {
            log::warn!("Spotify support disabled - librespot not installed");
            log::info!("To enable Spotify: cargo install librespot");
        }
    }

    Ok(())
}

/// Check if media service is running
pub fn status() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// Start the media service
pub async fn start() -> anyhow::Result<()> {
    log::info!("Media service started");
    Ok(())
}

/// Stop the media service
pub async fn stop() -> anyhow::Result<()> {
    log::info!("Media service stopped");
    Ok(())
}

/// Check if media service is running
pub async fn is_running() -> bool {
    true
}
