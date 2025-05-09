// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// www.rs is for external network communications to the home
// runs on port :8000

// TODO:
// 1. Authentication api and sessions support (DONE)
// 2. Build Human/Location/Pet/Service/Thing/User management API's
// 3. Sam web console app (DONE)
// 4. User management api

// use tch::{Device};

// use error_chain::error_chain;
use thiserror::Error;
use anyhow::Result;

use rouille::post_input;
use rouille::session;
use rouille::Request;
use rouille::Response;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("Post input error: {0}")]
    PostError(#[from] rouille::input::post::PostError),
    #[error("RustTube error: {0}")]
    RustTubeError(#[from] rustube::Error),
    #[error("Internal service error: {0}")]
    InternalServiceError(#[from] crate::sam::services::Error),
    #[error("Sam memory error: {0}")]
    SamMemoryError(#[from] crate::sam::memory::Error),
    #[error("Other error: {0}")]
    Other(String),
}

// Add these implementations:
impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Other(err)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

pub mod api;

// TODO - Authenticate connections using a one time key and expiring Sessions
// WW
/// Handles an incoming HTTP request and returns a response.
///
/// # Arguments
/// * `request` - Reference to the incoming `Request`.
///
/// # Returns
/// * `Result<Response>` - The HTTP response or an error.
pub fn handle(request: &Request) -> Result<Response, Error> {
    // Asset Pre Router
    if request.url().contains("setup.html")
        || request.url().contains(".webmanifest")
        || request.url().contains(".svg")
        || request.url().contains(".gif")
        || request.url().contains(".wav")
        || request.url().contains(".mp4")
        || request.url().contains(".css")
        || request.url().contains(".js")
        || request.url().contains(".min.js")
        || request.url().contains(".map")
        || request.url().contains(".png")
        || request.url().contains(".jpg")
        || request.url().contains(".svg")
        || request.url().contains(".ico")
        || request.url().contains(".tff")
        || request.url().contains(".woff")
        || request.url().contains(".woff2")
    {
        // Special handling for .mp4 files to support HTTP Range requests (Safari compatibility)
        if request.url().contains(".mp4") {
            use std::fs::File;
            use std::io::{Read, Seek, SeekFrom};
            use std::borrow::Cow;
            use rouille::ResponseBody;

            #[cfg(debug_assertions)]
            let file_path = format!("./www{}", request.url());
            #[cfg(not(debug_assertions))]
            let file_path = format!("/opt/sam/www{}", request.url());

            if let Ok(mut file) = File::open(&file_path) {
                if let Ok(metadata) = file.metadata() {
                    let file_size = metadata.len();
                    let range_header = request.header("Range");
                    if let Some(range_header) = range_header {
                        // Example: Range: bytes=0-1023
                        if let Some(range) = range_header.strip_prefix("bytes=") {
                            let mut parts = range.split('-');
                            let start = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                            let end = parts.next().and_then(|e| e.parse::<u64>().ok()).unwrap_or(file_size - 1);
                            let end = end.min(file_size - 1);
                            let chunk_size = end - start + 1;
                            if file.seek(SeekFrom::Start(start)).is_ok() {
                                let mut buffer = vec![0u8; chunk_size as usize];
                                if file.read_exact(&mut buffer).is_ok() {
                                    let content_range = format!("bytes {}-{}/{}", start, end, file_size);
                                    let content_length = chunk_size.to_string();
                                    return Ok(Response::from_data("video/mp4", buffer)
                                        .with_status_code(206)
                                        .with_additional_header("Content-Range", Cow::Owned(content_range))
                                        .with_additional_header("Accept-Ranges", "bytes")
                                        .with_additional_header("Content-Length", Cow::Owned(content_length))
                                        .with_additional_header("Access-Control-Allow-Origin", "*")
                                    );
                                }
                            }
                        }
                    } else {
                        // No Range header, serve the whole file
                        return Ok(Response::from_file("video/mp4", file)
                            .with_additional_header("Accept-Ranges", "bytes")
                            .with_additional_header("Access-Control-Allow-Origin", "*")
                        );
                    }
                }
            }
            // If file not found or error, fall through to match_assets
        }

        #[cfg(debug_assertions)]
        {
            let xresponse = rouille::match_assets(request, "./www/");
            if xresponse.is_success() {
                return Ok(xresponse
                    .with_additional_header("Access-Control-Allow-Origin", "*")
                    .with_no_cache());
            }
        }

        #[cfg(not(debug_assertions))]
        {
            let xresponse = rouille::match_assets(&request, "/opt/sam/www/");
            if xresponse.is_success() {
                return Ok(xresponse
                    .with_additional_header("Access-Control-Allow-Origin", "*")
                    .with_no_cache());
            }
        }
    }

    // TODO: Limit by tiimestamp field
    let sessions = crate::sam::memory::cache::WebSessions::select(None, None, None, None)?;

    Ok(session::session(
        request,
        "SID",
        99999999999999999,
        |session| {
            // Setup/Restore Current Session
            let mut current_session =
                crate::sam::memory::cache::WebSessions::new(session.id().to_string());
            for s in sessions {
                if s.sid == current_session.sid {
                    current_session = s;
                    break;
                }
            }

            match handle_with_session(current_session, request) {
                Ok(x) => x,
                Err(err) => {
                    log::error!("HTTP_SESSION_ERROR: {}", err);
                    Response::empty_404()
                }
            }
        },
    ))
}

pub fn handle_with_session(
    current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, Error> {
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
    if request.url() == "/auth" {
        let input = post_input!(request, {
            email: String,
            password: String,
        })?;

        let mut editable_session = current_session.clone();

        // Search for OID matches
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(input.email.clone()));
        pg_query.query_columns.push("email ilike".to_string());
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(input.password.clone()));
        pg_query.query_columns.push(" AND password =".to_string());

        let humans = crate::sam::memory::Human::select(None, None, None, Some(pg_query))?;

        if !humans.is_empty() {
            editable_session.authenticated = true;
            editable_session.human_oid = humans[0].oid.clone();
            for header in request.headers() {
                if header.0.contains("X-Forwarded-For") {
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
    let locations: Vec<crate::sam::memory::Location> =
        crate::sam::memory::Location::select(None, None, None, None)?;
    if request.url() != "/setup.html" && locations.is_empty() {
        let response = Response::redirect_302("/setup.html");
        return Ok(response);
    }

    // Is Authenticated?
    if request.url() != "/login.html" && !current_session.authenticated {
        let response = Response::redirect_302("/login.html");
        return Ok(response);
    }

    // Is Authenticated?
    if request.url() == "/login.html" && current_session.authenticated {
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

    if request.url().contains("/api") {
        return api::handle_api_request(current_session, request);
    }

    if request.url().contains("/streams") {
        let xresponse = rouille::match_assets(request, "/opt/sam/");
        if xresponse.is_success() {
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_no_cache());
        }
    }

    if request.url().contains("/files")
        || request.url().contains("/tmp")
        || request.url().contains("/games")
    {
        let xresponse = rouille::match_assets(request, "/opt/sam/");
        if xresponse.is_success() {
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_no_cache());
        }
    }

    #[cfg(debug_assertions)]
    {
        let xresponse = rouille::match_assets(request, "./www/");
        if xresponse.is_success() {
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_no_cache());
        }
    }

    #[cfg(not(debug_assertions))]
    {
        let xresponse = rouille::match_assets(&request, "/opt/sam/www/");
        if xresponse.is_success() {
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_no_cache());
        }
    }

    let response = Response::redirect_302("/index.html");
    Ok(response)
}

// use std::fs::File;

// use std::io::Write;

// pub fn install() -> std::io::Result<()> {
//     let data = include_bytes!("../../packages/www.zip");
//     let mut pos = 0;
//     let mut buffer = File::create("/opt/sam/www.zip")?;
//     while pos < data.len() {
//         let bytes_written = buffer.write(&data[pos..])?;
//         pos += bytes_written;
//     }

//     let _ = crate::sam::tools::extract_zip("/opt/sam/www.zip", "/opt/sam/");

//     Ok(())
// }
