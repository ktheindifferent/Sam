// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;

pub fn handle(
    _current_session: crate::sam::memory::cache::WebSessions,
    _request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    Ok(Response::empty_404())
}
