// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;

pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/humans" {
        let objects = crate::sam::memory::Human::select(None, None, Some("email ASC".to_string()), None)?;
        return Ok(Response::json(&objects));
    }

    if request.url().contains("/api/humans") && request.url().contains("/observations"){
       
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let oid = vec[3];

        if request.method() == "GET" {
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(oid.to_string()));
            pg_query.query_coulmns.push("oid =".to_string());

            let humans = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;
        
            if !humans.is_empty(){
                return Ok(Response::json(&humans[0].clone()));
            } else {
                return Ok(Response::empty_404());
            }

        }
    }


    if request.url().contains("/api/humans") {
       
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let oid = vec[3];

        if request.method() == "GET" {
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(oid.to_string()));
            pg_query.query_coulmns.push("oid =".to_string());

            let humans = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;
        
            if !humans.is_empty(){
                return Ok(Response::json(&humans[0].clone()));
            } else {
                return Ok(Response::empty_404());
            }

        }
    }


    Ok(Response::empty_404())
 
}