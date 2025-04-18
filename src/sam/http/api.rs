pub mod humans;
pub mod io;
pub mod locations;
pub mod observations;
pub mod pets;
pub mod services;
pub mod things;
pub mod rooms;
pub mod settings;

use rouille::Request;
use rouille::Response;

pub fn handle_api_request(current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
   
    if request.url() == "/api/sid" {
        return Ok(Response::text(current_session.sid));
    }
    
    // TODO: Fetch Human and append to response
    if request.url() == "/api/current_session" {
        return Ok(Response::json(&current_session));
    }

    if request.url() == "/api/current_human" {

        // Search for OID matches
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(current_session.human_oid.clone()));
        pg_query.query_coulmns.push("oid =".to_string());

        let human = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;
        return Ok(Response::json(&human[0]));
    }

    // IO: GET
    if request.url().contains("/api/io"){
        return io::handle(current_session, request);
    }

    if request.url().contains("/api/humans"){
        return humans::handle(current_session, request);
    }

    if request.url().contains("/api/locations"){
        return locations::handle(current_session, request);
    }

    if request.url().contains("/api/observations"){
        return observations::handle(current_session, request);
    }

    if request.url().contains("/api/rooms"){
        return rooms::handle(current_session, request);
    }

    if request.url().contains("/api/services"){
        return services::handle(current_session, request);
    }

    if request.url().contains("/api/settings"){
        return settings::handle(current_session, request);
    }

    if request.url().contains("/api/things"){
        return things::handle(current_session, request);
    }
    

    Ok(Response::empty_404())
}