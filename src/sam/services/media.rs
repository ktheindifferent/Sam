pub mod games;
pub mod image;
pub mod snapcast;
pub mod youtube;

use rouille::Request;
use rouille::Response;

pub fn handle(
    current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/image") {
        return image::handle(current_session, request);
    }

    if request.url().contains("/games") {
        return games::handle(current_session, request);
    }

    Ok(Response::empty_404())
}
