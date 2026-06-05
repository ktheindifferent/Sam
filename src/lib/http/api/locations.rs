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
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/locations" {
        let objects = crate::memory::Location::select(None, None, None, None)?;
        return Ok(Response::json(&objects));
    }

    if request.url().contains("/api/locations") && request.url().contains("/rooms") {
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let Some(location_oid) = vec.get(3).filter(|oid| !oid.is_empty()) else {
            return Ok(Response::empty_404());
        };

        if request.method() == "GET" {
            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(location_oid.to_string()));
            pg_query.query_columns.push("location_oid =".to_string());

            let rooms = crate::memory::Room::select(None, None, None, Some(pg_query))?;

            return Ok(Response::json(&rooms));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                name: String
            })?;

            let mut room = crate::memory::Room::new();
            room.name = input.name;
            room.location_oid = (*location_oid).to_string();
            room.save()?;

            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(room.oid.clone()));
            pg_query.query_columns.push("oid =".to_string());

            let objects = crate::memory::Room::select(None, None, None, Some(pg_query))?;
            if let Some(room) = objects.first() {
                if request.url().contains(".json") {
                    return Ok(Response::json(room));
                } else {
                    let response = Response::redirect_302("/locations.html");
                    return Ok(response);
                }
            } else {
                return Ok(Response::empty_404());
            }
        }
    }

    Ok(Response::empty_404())
}
