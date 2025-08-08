pub mod humans;
pub mod io;
pub mod locations;
pub mod observations;
pub mod pets;
pub mod rooms;
pub mod services;
pub mod settings;
pub mod things;

use rouille::Request;
use rouille::Response;

pub fn handle_api_request(
    current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    let url = request.url();

    // Handle exact route matches
    if let Some(response) = handle_exact_routes(&current_session, url)? {
        return Ok(response);
    }

    // Handle prefix-based routes
    if let Some(response) = handle_prefix_routes(current_session, request, url)? {
        return Ok(response);
    }

    Ok(Response::empty_404())
}

fn handle_exact_routes(
    session: &crate::sam::memory::cache::WebSessions,
    url: &str,
) -> Result<Option<Response>, crate::sam::http::Error> {
    match url {
        "/api/sid" => Ok(Some(Response::text(session.sid.clone()))),
        "/api/current_session" => Ok(Some(Response::json(session))),
        "/api/current_human" => handle_current_human(session),
        _ => Ok(None),
    }
}

fn handle_current_human(
    session: &crate::sam::memory::cache::WebSessions,
) -> Result<Option<Response>, crate::sam::http::Error> {
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query
        .queries
        .push(crate::sam::memory::PGCol::String(session.human_oid.clone()));
    pg_query.query_columns.push("oid =".to_string());

    let human = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;
    Ok(Some(Response::json(&human[0])))
}

fn handle_prefix_routes(
    session: crate::sam::memory::cache::WebSessions,
    request: &Request,
    url: &str,
) -> Result<Option<Response>, crate::sam::http::Error> {
    const ROUTE_HANDLERS: &[(
        &str,
        fn(
            crate::sam::memory::cache::WebSessions,
            &Request,
        ) -> Result<Response, crate::sam::http::Error>,
    )] = &[
        ("/api/io", |s, r| io::handle(s, r)),
        ("/api/humans", |s, r| humans::handle(s, r)),
        ("/api/locations", |s, r| locations::handle(s, r)),
        ("/api/observations", |s, r| observations::handle(s, r)),
        ("/api/rooms", |s, r| rooms::handle(s, r)),
        ("/api/services", |s, r| services::handle(s, r)),
        ("/api/settings", |s, r| settings::handle(s, r)),
        ("/api/things", |s, r| things::handle(s, r)),
    ];

    for (prefix, handler) in ROUTE_HANDLERS {
        if url.contains(prefix) {
            return handler(session, request).map(Some);
        }
    }

    Ok(None)
}
