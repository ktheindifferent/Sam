// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::post_input;
use rouille::Request;
use rouille::Response;

pub fn handle(
    current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/api/services/dropbox") {
        return crate::sam::services::dropbox::handle(current_session, request);
    }

    if request.url().contains("/api/services/lifx") {
        return crate::sam::services::lifx::handle(current_session, request);
    }

    if request.url().contains("/api/services/notifications") {
        return crate::sam::services::notifications::handle(current_session, request);
    }

    if request.url().contains("/api/services/osf") {
        return crate::sam::services::osf::handle(current_session, request);
    }

    if request.url().contains("/api/services/media/youtube") {
        return crate::sam::services::media::youtube::handle(current_session, request);
    }

    if request.url().contains("/api/services/tts") {
        return crate::sam::services::tts::handle(current_session, request);
    }

    if request.url().contains("/api/services/stt") {
        return crate::sam::services::stt::handle(current_session, request);
    }

    if request.url().contains("/api/services/jupiter") {
        return crate::sam::services::jupiter::handle(current_session, request);
    }

    if request.url().contains("/api/services/storage") {
        return crate::sam::services::storage::handle(current_session, request);
    }

    if request.url().contains("/api/services/media") {
        return crate::sam::services::media::handle(current_session, request);
    }

    if request.url() == "/api/services" || request.url() == "/api/services.json" {
        if request.method() == "GET" {
            let objects = crate::sam::memory::config::Service::select(None, None, None, None)?;
            return Ok(Response::json(&objects));
        }

        if request.method() == "POST" {
            // Collect input params from post request
            let input = post_input!(request, {
                identifier: String,
                secret: String,
                key: String,
                endpoint: String,
                username: Option<String>,
                password: Option<String>,
            })?;

            // Save Service
            let mut service = crate::sam::memory::config::Service::new();
            service.identifier = input.identifier;
            service.key = input.key;
            service.secret = input.secret;
            service.endpoint = input.endpoint;

            if let Some(username) = input.username {
                service.username = username;
            }
            if let Some(password) = input.password {
                service.password = password;
            }

            service.save()?;

            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::sam::memory::PGCol::String(service.oid.clone()));
            pg_query.query_columns.push("oid =".to_string());
            let objects =
                crate::sam::memory::config::Service::select(None, None, None, Some(pg_query))?;
            if !objects.is_empty() {
                if request.url().contains(".json") {
                    return Ok(Response::json(&objects[0]));
                } else {
                    let response = Response::redirect_302("/services.html");
                    return Ok(response);
                }
            } else {
                return Ok(Response::empty_404());
            }
        }
    }

    Ok(Response::empty_404())
}
