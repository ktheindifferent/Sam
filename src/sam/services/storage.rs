// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// Files can be stored in many places: Local(SQL), Local(NAS), Cloud(Dropbox, OneDrive, Etc.)

use rouille::{post_input, Request, Response};
use std::{fs::File, path::Path, thread, time::Duration};

/// Initializes the storage service by setting up cache and creating necessary folders.
pub fn init() {
    let storage_init_thread = thread::Builder::new()
        .name("storage_init".to_string())
        .spawn(move || {
            init_cache();
            crate::sam::services::dropbox::create_folder("/Sam");
        });

    match storage_init_thread {
        Ok(_) => log::info!("storage_init started successfully"),
        Err(e) => log::error!("failed to initialize storage_init: {}", e),
    }
}

/// Initializes the cache system for file storage.
pub fn init_cache() {
    let cache_thread = thread::Builder::new()
        .name("cache".to_string())
        .spawn(move || {
            loop {
                let _ = crate::sam::memory::storage::File::cache_all();
                thread::sleep(Duration::from_secs(100)); // Adjusted to seconds for clarity
            }
        });

    match cache_thread {
        Ok(_) => log::info!("cache started successfully"),
        Err(e) => log::error!("failed to initialize cache: {}", e),
    }
}

/// Handles storage-related API requests.
///
/// Supported endpoints:
/// - `/api/services/storage/locations`
/// - `/api/services/storage/files`
/// - `/api/services/storage/file/{oid}`
pub fn handle(
    _current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/storage/locations" {
        // Handle storage locations
        if request.method() == "GET" {
            let locations = crate::sam::memory::config::FileStorageLocation::select(None, None, None, None)?;
            return Ok(Response::json(&locations));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                storage_type: String,
                endpoint: String,
                username: String,
                password: String,
            })?;

            let mut location = crate::sam::memory::config::FileStorageLocation::new();
            location.storage_type = input.storage_type;
            location.endpoint = input.endpoint;
            location.username = input.username;
            location.password = input.password;
            location.save()?;

            return Ok(Response::json(&location));
        }
    }

    if request.url() == "/api/services/storage/files" {
        // Handle file storage
        if request.method() == "GET" {
            let files = crate::sam::memory::storage::File::select_lite(None, None, None, None)?;
            return Ok(Response::json(&files));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                file_data: rouille::input::post::BufferedFile,
                file_folder_tree: Option<Vec<String>>,
                storage_location_oid: Option<String>,
            })?;

            let mut file = crate::sam::memory::storage::File::new();
            file.file_name = input.file_data.filename.ok_or("unknown filename")?;
            file.file_type = input.file_data.mime;
            file.file_data = Some(input.file_data.data);
            file.file_folder_tree = input.file_folder_tree;
            file.storage_location_oid = input.storage_location_oid.unwrap_or_else(|| "SQL".to_string());
            file.save()?;

            return Ok(Response::json(&file));
        }
    }

    if request.url().contains("/api/services/storage/file/") {
        // Handle file retrieval by OID
        if let Some(oid) = request.url().split('/').nth(5) { // Fixed lifetime issue
            let file_path = format!("/opt/sam/files/{}", oid);
            if Path::new(&file_path).exists() {
                let file = File::open(&file_path)?;
                return Ok(Response::from_file("", file));
            }

            // Query file from database
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(oid.to_string()));
            pg_query.query_columns.push("oid =".to_string());

            let files = crate::sam::memory::storage::File::select(None, None, None, Some(pg_query))?;
            if let Some(file) = files.first() {
                return Ok(Response::from_data(file.file_type.clone(), file.file_data.clone().unwrap()));
            } else {
                return Ok(Response::empty_404());
            }
        }
    }

    Ok(Response::empty_404())
}
