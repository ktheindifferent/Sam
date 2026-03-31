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
    Ok(())
}
