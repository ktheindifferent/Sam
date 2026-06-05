// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use super::validation::{sanitize_output_json, validate_id_param, validate_query_params};
use rouille::Request;
use rouille::Response;

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/humans" {
        // Validate query parameters
        let params = validate_query_params(request)
            .map_err(|_| crate::http::Error::new("Invalid query parameters"))?;

        let objects =
            crate::memory::Human::select(None, None, Some("email ASC".to_string()), None)?;

        // Sanitize output before sending
        let json_output = serde_json::to_value(&objects)
            .map_err(|_| crate::http::Error::new("Serialization error"))?;
        let sanitized = sanitize_output_json(&json_output);

        return Ok(Response::json(&sanitized));
    }

    if request.url().contains("/api/humans") && request.url().contains("/observations") {
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let Some(raw_oid) = vec.get(3) else {
            return Ok(Response::empty_404());
        };

        // Validate OID parameter
        let oid = validate_id_param(raw_oid)
            .map_err(|_| crate::http::Error::new("Invalid OID parameter"))?;

        if request.method() == "GET" {
            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(oid.to_string()));
            pg_query.query_columns.push("oid =".to_string());

            let humans = crate::memory::Human::select(None, None, None, Some(pg_query))?;

            let Some(human) = humans.first() else {
                return Ok(Response::empty_404());
            };
            return Ok(Response::json(human));
        }
    }

    if request.url().contains("/api/humans") {
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let Some(raw_oid) = vec.get(3) else {
            return Ok(Response::empty_404());
        };

        // Validate OID parameter
        let oid = validate_id_param(raw_oid)
            .map_err(|_| crate::http::Error::new("Invalid OID parameter"))?;

        if request.method() == "GET" {
            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(oid.to_string()));
            pg_query.query_columns.push("oid =".to_string());

            let humans = crate::memory::Human::select(None, None, None, Some(pg_query))?;

            let Some(human) = humans.first() else {
                return Ok(Response::empty_404());
            };
            return Ok(Response::json(human));
        }
    }

    Ok(Response::empty_404())
}
