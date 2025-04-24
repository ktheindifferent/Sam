/// ███████     █████     ███    ███    
/// ██         ██   ██    ████  ████    
/// ███████    ███████    ██ ████ ██    
///      ██    ██   ██    ██  ██  ██    
/// ███████ ██ ██   ██ ██ ██      ██ ██ 
/// Copyright 2021-2026 The Open Sam Foundation (OSF)
/// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
/// Licensed under GPLv3....see LICENSE file.

/// Jupiter.rs acts as a service module to connect sam to your jupiter server 
/// Jupiter is a rust based wether server devloped by The Open Sam Foundation 
/// and can be found in the foundations git repository.

use rouille::Request;
use rouille::Response;
use rouille::post_input;

pub fn handle(current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    
    
    
    if request.url() == "/api/services/notifications/unseen" && request.method() == "GET" {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::Boolean(false));
        pg_query.query_columns.push("seen =".to_string());

        pg_query.queries.push(crate::sam::memory::PGCol::String(current_session.human_oid));
        pg_query.query_columns.push(" AND human_oid =".to_string());

        pg_query.queries.push(crate::sam::memory::PGCol::String(current_session.sid));
        pg_query.query_columns.push(" AND sid =".to_string());

        let notifications = crate::sam::memory::human::Notification::select(None, None, Some("timestamp DESC".to_string()), Some(pg_query))?;
        
        return Ok(Response::json(&notifications));
    }

    if request.url() == "/api/services/notifications/seen" {
        let input = post_input!(request, {
            oid: String
        })?;
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(input.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        let notifications = crate::sam::memory::human::Notification::select(Some(20), None, Some("timestamp DESC".to_string()), Some(pg_query))?;
        let mut notification = notifications[0].clone();
        notification.seen = true;
        notification.save().unwrap();

        return Ok(Response::json(&notification));
    }
    
    
    if request.url() == "/api/services/notifications" {

        if request.method() == "GET" {
            let mut pg_query = crate::sam::memory::PostgresQueries::default();

            pg_query.queries.push(crate::sam::memory::PGCol::String(current_session.human_oid));
            pg_query.query_columns.push("human_oid =".to_string());

            let notifications = crate::sam::memory::human::Notification::select(Some(20), None, Some("timestamp DESC".to_string()), Some(pg_query))?;
            
            return Ok(Response::json(&notifications));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                message: String
            })?;


            let mut notification = crate::sam::memory::human::Notification::new();
            notification.message = input.message;
            notification.sid = current_session.sid;
            notification.human_oid = current_session.human_oid;
            notification.save().unwrap();

            return Ok(Response::json(&notification));

        }


    }
    Ok(Response::empty_404())
}