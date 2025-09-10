// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// http.rs is for external network communications to the home
// runs on port :8000

// use tch::{Device};

// use error_chain::error_chain;
use anyhow::Result;
use thiserror::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use rouille::post_input;
use rouille::session;
use rouille::Request;
use rouille::Response;
use serde_json;

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
    InternalServiceError(#[from] crate::services::Error),
    #[error("Sam memory error: {0}")]
    SamMemoryError(#[from] crate::memory::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    InternalServerError(String),
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

impl Error {
    pub fn new(msg: &str) -> Self {
        Error::Other(msg.to_string())
    }
}

pub mod api;
pub mod csrf;

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
        || request.url().contains("login.html")
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
            use std::borrow::Cow;
            use std::fs::File;
            use std::io::{Read, Seek, SeekFrom};

            #[cfg(debug_assertions)]
            let file_path = format!("./www{}", request.url());
            #[cfg(not(debug_assertions))]
            let file_path = {
                // Try /app/www first (Docker/CapRover), then /opt/sam/www
                let docker_path = format!("/app/www{}", request.url());
                if std::path::Path::new(&docker_path).exists() {
                    docker_path
                } else {
                    format!("/opt/sam/www{}", request.url())
                }
            };

            if let Ok(mut file) = File::open(&file_path) {
                if let Ok(metadata) = file.metadata() {
                    let file_size = metadata.len();
                    let range_header = request.header("Range");
                    if let Some(range_header) = range_header {
                        // Example: Range: bytes=0-1023
                        if let Some(range) = range_header.strip_prefix("bytes=") {
                            let mut parts = range.split('-');
                            let start = parts
                                .next()
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(0);
                            let end = parts
                                .next()
                                .and_then(|e| e.parse::<u64>().ok())
                                .unwrap_or(file_size - 1);
                            let end = end.min(file_size - 1);
                            let chunk_size = end - start + 1;
                            if file.seek(SeekFrom::Start(start)).is_ok() {
                                let mut buffer = vec![0u8; chunk_size as usize];
                                if file.read_exact(&mut buffer).is_ok() {
                                    let content_range =
                                        format!("bytes {}-{}/{}", start, end, file_size);
                                    let content_length = chunk_size.to_string();
                                    return Ok(Response::from_data("video/mp4", buffer)
                                        .with_status_code(206)
                                        .with_additional_header(
                                            "Content-Range",
                                            Cow::Owned(content_range),
                                        )
                                        .with_additional_header("Accept-Ranges", "bytes")
                                        .with_additional_header(
                                            "Content-Length",
                                            Cow::Owned(content_length),
                                        )
                                        .with_additional_header(
                                            "Access-Control-Allow-Origin",
                                            request.header("Origin").unwrap_or("http://localhost:8080").to_string(),
                                        ));
                                }
                            }
                        }
                    } else {
                        // No Range header, serve the whole file
                        return Ok(Response::from_file("video/mp4", file)
                            .with_additional_header("Accept-Ranges", "bytes")
                            .with_additional_header("Access-Control-Allow-Origin", request.header("Origin").unwrap_or("http://localhost:8080").to_string()));
                    }
                }
            }
            // If file not found or error, fall through to match_assets
        }

        // (CORS) Security for debug mode
        #[cfg(debug_assertions)]
        {
            let xresponse = rouille::match_assets(request, "./www/");
            if xresponse.is_success() {
                let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
                return Ok(xresponse
                    .with_additional_header("Access-Control-Allow-Origin", origin)
                    .with_no_cache());
            }
        }

        // (CORS) Security for production mode
        #[cfg(not(debug_assertions))]
        {
            // Try /app/www first (Docker/CapRover path)
            let xresponse = rouille::match_assets(&request, "/app/www/");
            if xresponse.is_success() {
                let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
                return Ok(xresponse
                    .with_additional_header("Access-Control-Allow-Origin", origin)
                    .with_no_cache());
            }
            
            // Fallback to /opt/sam/www for traditional installations
            let xresponse = rouille::match_assets(&request, "/opt/sam/www/");
            if xresponse.is_success() {
                let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
                return Ok(xresponse
                    .with_additional_header("Access-Control-Allow-Origin", origin)
                    .with_no_cache());
            }
        }
    }

    // Limit by timestamp field to 24 hours ago to improve query performance
    let right_now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64;
    let mut pg_query = crate::memory::PostgresQueries::default();
    pg_query.queries.push(crate::memory::PGCol::Timestamp(right_now - 86400)); // 24 hours ago
    pg_query.query_columns.push("timestamp <".to_string());

    // Fetch sessions
    let sessions = crate::memory::cache::WebSessions::select(None, None, None, Some(pg_query))?;

    // 24 hours limit
    const SESSION_DURATION: u64 = 86400;
    Ok(session::session(
        request,
        "SID",
        SESSION_DURATION,
        |session| -> Response {
            // Setup/Restore Current Session
            let mut current_session =
                crate::memory::cache::WebSessions::new(session.id().to_string());
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
        }
    ))
}

pub fn handle_with_session(
    current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, Error> {
    // =================================================================
    // Core Web Functions: setup, auth, deauth, etc.
    // =================================================================

    // Is Setup?
    let locations: Vec<crate::memory::Location> =
        crate::memory::Location::select(None, None, None, None)?;
    let is_initial_setup = locations.is_empty();


    // Setup: POST
    if request.url() == "/setup" && is_initial_setup {
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

        // Validate password confirmation
        if input.password != input.password_confirm {
            let response = Response::json(&serde_json::json!({
                "error": "Passwords do not match"
            }))
            .with_status_code(400);
            return Ok(response);
        }

        // Hash password before saving
        let hashed_password = crate::security::Auth::hash_password(&input.password)
            .map_err(|e| Error::Other(format!("Failed to hash password: {}", e)))?;

        // Save Human with hashed password
        let mut human = crate::memory::Human::new();
        human.name = input.name;
        human.email = Some(input.email);
        human.password = Some(hashed_password);
        human.save()?;

        // Save Location
        let mut location = crate::memory::Location::new();
        location.name = input.location_name;
        location.address = input.location_address;
        location.city = input.location_city;
        location.state = input.location_state;
        location.zip_code = input.location_zip;
        location.save()?;

        // TODO - Save Services

        // TODO - Authenticate
        
        // Redirect to login page after successful setup
        let response = Response::redirect_302("/login.html");
        return Ok(response);
    }

    // Secure authentication with password hashing and rate limiting
    if request.url() == "/auth" {
        let input = post_input!(request, {
            email: String,
            password: String,
        })?;

        // Get IP address for rate limiting
        let ip_address = request.headers()
            .find(|h| h.0.contains("X-Forwarded-For"))
            .map(|h| h.1.to_string())
            .unwrap_or_else(|| request.remote_addr().to_string());

        // Check rate limit
        let rate_limit_key = format!("auth:{}:{}", ip_address, input.email.to_lowercase());
        if !crate::security::Auth::check_auth_rate_limit(&rate_limit_key) {
            let wait_time = crate::security::Auth::get_wait_time(&rate_limit_key)
                .unwrap_or(60);
            let response = Response::json(&serde_json::json!({
                "error": "Too many authentication attempts. Please try again later.",
                "wait_seconds": wait_time
            }))
            .with_status_code(429);
            return Ok(response);
        }

        let mut editable_session = current_session.clone();

        // Search for user by email using parameterized query (case-insensitive)
        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String(input.email.to_lowercase()));
        pg_query.query_columns.push("LOWER(email)".to_string());

        let humans = crate::memory::Human::select(None, None, None, Some(pg_query))?;

        let mut auth_success = false;
        if !humans.is_empty() {
            // Verify password hash
            if let Some(stored_hash) = &humans[0].password {
                match crate::security::Auth::verify_password(&input.password, stored_hash) {
                    Ok(true) => {
                        auth_success = true;
                        editable_session.authenticated = true;
                        editable_session.human_oid = humans[0].oid.clone();
                        editable_session.ip_address = ip_address.clone();
                        
                        // Clear rate limit on successful authentication
                        crate::security::Auth::clear_auth_rate_limit(&rate_limit_key);
                    }
                    Ok(false) => {
                        // Invalid password
                    }
                    Err(_) => {
                        // Hash verification error - treat as failed auth
                    }
                }
            }
        }

        editable_session.save()?;

        if auth_success {
            let response = Response::redirect_302("/index.html");
            return Ok(response);
        } else {
            let response = Response::redirect_302("/login.html?error=invalid_credentials");
            return Ok(response);
        }
    }

    // =================================================================
    // Checkpoint -- Redirect the user as required
    // =================================================================


    
    // During initial setup, redirect to setup page UNLESS already on setup-related URLs
    if is_initial_setup && 
       request.url() != "/setup.html" && 
       request.url() != "/setup" {
        let response = Response::redirect_302("/setup.html");
        return Ok(response);
    }

    // Is Authenticated?
    // Skip authentication check for login page, auth endpoint, and setup pages
    if request.url() != "/login.html" && 
       request.url() != "/auth" &&
       request.url() != "/setup.html" &&
       request.url() != "/setup" &&
       !current_session.authenticated {
        let response = Response::redirect_302("/login.html");
        return Ok(response);
    }

    // Is Authenticated?
    if request.url() == "/login.html" && current_session.authenticated {
        let response = Response::redirect_302("/index.html");
        return Ok(response);
    }

    // Health check endpoints (no auth required - moved before auth check)
    if request.url() == "/health" {
        return Ok(Response::json(&serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })));
    }
    
    if request.url() == "/health/detailed" {
        let cpu_usage = crate::tools::get_cpu_usage().unwrap_or(0.0);
        let memory_usage = crate::tools::get_memory_usage().unwrap_or(0.0);
        
        return Ok(Response::json(&serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metrics": {
                "cpu_usage": cpu_usage,
                "memory_usage": memory_usage
            }
        })));
    }
    
    // WebSocket endpoint - return proper upgrade response (no auth required)
    if request.url() == "/ws" {
        // WebSocket connections need special handling
        // For now, return a message indicating WebSocket is not available through HTTP
        return Ok(Response::json(&serde_json::json!({
            "error": "WebSocket endpoint - use WebSocket protocol",
            "message": "Connect using ws:// or wss:// protocol on port 8080"
        })).with_status_code(426)); // 426 Upgrade Required
    }
    
    // Service control API endpoints (allow without auth for dashboard functionality)
    if request.url().starts_with("/api/services/") {
        return api::handle_api_request(current_session, request);
    }
    
    // Environment API endpoint (no auth required)
    if request.url() == "/api/environment" {
        return api::handle_api_request(current_session, request);
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
            let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", origin)
                .with_no_cache());
        }
    }

    if request.url().contains("/files")
        || request.url().contains("/tmp")
        || request.url().contains("/games")
    {
        let xresponse = rouille::match_assets(request, "/opt/sam/");
        if xresponse.is_success() {
            let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
            return Ok(xresponse
                .with_additional_header("Access-Control-Allow-Origin", origin)
                .with_no_cache());
        }
    }


    let xresponse = rouille::match_assets(request, "./www/");
    if xresponse.is_success() {
        let origin = request.header("Origin").unwrap_or("http://localhost:8080").to_string();
        return Ok(xresponse
            .with_additional_header("Access-Control-Allow-Origin", origin)
            .with_no_cache());
    }
    
    // For unmatched API routes, return 404 JSON instead of redirecting
    if request.url().starts_with("/api/") || request.url().starts_with("/ws") {
        return Ok(Response::json(&serde_json::json!({
            "error": "Not found",
            "path": request.url()
        })).with_status_code(404));
    }

    // Only redirect to index.html for non-API routes
    let response = Response::redirect_302("/index.html");
    Ok(response)
}