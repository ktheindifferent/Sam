// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

// www.rs is for external network communications to the home
// runs on port :8000

// TODO:
// 1. Authentication api and sessions support (DONE)
// 2. Build Human/Location/Pet/Service/Thing/User management API's
// 3. Sam web console app (DONE)
// 4. User management api

// use tch::{Device};

use rouille::Request;
use rouille::Response;
use rouille::post_input;
use rouille::session;
use error_chain::error_chain;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        PostError(rouille::input::post::PostError);
        RustTubeError(rustube::Error);
        InternalServiceError(crate::sam::services::Error);
        SamMemoryError(crate::sam::memory::Error);
    }
}

pub mod api;




// TODO - Authenticate connections using a one time key and expiring Sessions
// WW
pub fn handle(request: &Request) -> Result<Response> {

    // Asset Pre Router
    if request.url().contains("setup.html") || request.url().contains(".webmanifest") || request.url().contains(".svg") || request.url().contains(".gif") || request.url().contains(".wav") || request.url().contains(".mp4") || request.url().contains(".css") || request.url().contains(".js") || request.url().contains(".min.js") || request.url().contains(".map") || request.url().contains(".png") || request.url().contains(".jpg") || request.url().contains(".svg") || request.url().contains(".ico") || request.url().contains(".tff") || request.url().contains(".woff") || request.url().contains(".woff2") {
        #[cfg(debug_assertions)]{
            let xresponse = rouille::match_assets(request, "./www/");
            if xresponse.is_success() {
                return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
            } 
        }

        #[cfg(not(debug_assertions))]{
            let xresponse = rouille::match_assets(&request, "/opt/sam/www/");
            if xresponse.is_success() {
                return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
            } 
        }
    }

    // TODO: Limit by tiimestamp field
    let sessions = crate::sam::memory::WebSessions::select(None, None, None, None)?;


    return Ok(session::session(request, "SID", 99999999999999999, |session| {

        // Setup/Restore Current Session
        let mut current_session = crate::sam::memory::WebSessions::new(session.id().to_string());
        for s in sessions{
            if s.sid == current_session.sid{
                current_session = s;
                break;
            }
        }

        match handle_with_session(current_session, request){
            Ok(x) => {
                x
            },
            Err(err) => {
                log::error!("HTTP_SESSION_ERROR: {}", err);
                Response::empty_404()
            }
        }
    }));
}

pub fn handle_with_session(current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response> {


        // =================================================================
        // Core Web Functions: setup, auth, deauth, etc.
        // =================================================================

        // Setup: POST
        if request.url() == "/setup" {
            
            // Collect input params from post request
            let input = post_input!(request, {
                name: String,
                email: String,
                password: String,
                password_confirm: String,
                location_name: String,
                location_address: String,
                location_city: String,
                location_state: String,
                location_zip: String,
                lifx_api_key: Option<String>,
                spotify_api_key: Option<String>
            })?;

            // Save Human
            let mut human = crate::sam::memory::Human::new();
            human.name = input.name;
            human.email = Some(input.email);
            human.password = Some(input.password);
            human.save()?;

            // Save Location
            let mut location = crate::sam::memory::Location::new();
            location.name = input.location_name;
            location.address = input.location_address;
            location.city = input.location_city;
            location.state = input.location_state;
            location.zip_code = input.location_zip;
            location.save()?;

            // TODO - Save Services



            // TODO - Authenticate

            
        }

        // TODO: Fix SQL Injection vulnerability with wrapped params
        if request.url() == "/auth"{


            let input = post_input!(request, {
                email: String,
                password: String,
            })?;

            let mut editable_session = current_session.clone();

            // Search for OID matches
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(input.email.clone()));
            pg_query.query_columns.push("email ilike".to_string());
            pg_query.queries.push(crate::sam::memory::PGCol::String(input.password.clone()));
            pg_query.query_columns.push(" AND password =".to_string());

            let humans = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;

            if !humans.is_empty() {
                editable_session.authenticated = true;
                editable_session.human_oid = humans[0].oid.clone();
                for header in request.headers(){
                    if header.0.contains("X-Forwarded-For"){
                        editable_session.ip_address = header.1.to_string();
                    }
                }
            }


            editable_session.save()?;

            let response = Response::redirect_302("/index.html");
            return Ok(response);

        }


        // =================================================================
        // Checkpoint -- Redirect the user as required
        // =================================================================

        // Is Setup?
        let locations: Vec<crate::sam::memory::Location> = crate::sam::memory::Location::select(None, None, None, None)?;
        if request.url() != "/setup.html" && locations.is_empty(){
            let response = Response::redirect_302("/setup.html");
            return Ok(response);
        }

        // Is Authenticated?
        if request.url() != "/login.html" && !current_session.authenticated{
            let response = Response::redirect_302("/login.html");
            return Ok(response);
        }

        // Is Authenticated?
        if request.url() == "/login.html" && current_session.authenticated{
            let response = Response::redirect_302("/index.html");
            return Ok(response);
        }

        // =================================================================
        // End Checkpoint 
        // =================================================================


        // API Functions

  
        // if request.url().contains("/is_cuda"){
        //     let device = tch::Cuda::is_available();
        //     return Ok(Response::text(device.to_string()));
        // }

        // if request.url().contains("/is_cuda2"){
        //     let device = tch::Cuda::cudnn_is_available();
        //     return Ok(Response::text(device.to_string()));
        // }

        // if request.url().contains("/cudac"){
        //     let device = tch::Cuda::device_count();
        //     return Ok(Response::text(device.to_string()));
        // }


        

        if request.url().contains("/api"){
            return api::handle_api_request(current_session, request);
        }

        if request.url().contains("/streams"){
  
                let xresponse = rouille::match_assets(request, "/opt/sam/");
                if xresponse.is_success() {
                    return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
                } 
            
    

        }

        if request.url().contains("/files") || request.url().contains("/tmp") || request.url().contains("/games"){
            let xresponse = rouille::match_assets(request, "/opt/sam/");
            if xresponse.is_success() {
                return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
            } 
        }


        #[cfg(debug_assertions)]{
            let xresponse = rouille::match_assets(request, "./www/");
            if xresponse.is_success() {
                return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
            } 
        }

        #[cfg(not(debug_assertions))]{
            let xresponse = rouille::match_assets(&request, "/opt/sam/www/");
            if xresponse.is_success() {
                return Ok(xresponse.with_additional_header("Access-Control-Allow-Origin", "*").with_no_cache());
            } 
        }
            
        let response = Response::redirect_302("/index.html");
        Ok(response)
        
    
}


use std::fs::File;

use std::io::{Write};



pub fn install() -> std::io::Result<()> {
    let data = include_bytes!("../../packages/www.zip");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/www.zip")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let _ = crate::sam::tools::extract_zip("/opt/sam/www.zip", "/opt/sam/");

    Ok(())
}