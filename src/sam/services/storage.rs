// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2023 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

// Files can be stored in many places: Local(SQL), Local(NAS), Cloud(Dropbox, OneDrive, Etc.)

use rouille::post_input;
use rouille::Request;
use rouille::Response;
use std::{thread, time::Duration};
use std::fs::File;
use std::path::Path;

pub fn sql_get(){

}

pub fn sql_store(){

}

pub fn init(){
    let storage_init_thread = thread::Builder::new().name("storage_init".to_string()).spawn(move || {
        init_cache();
        crate::sam::services::dropbox::create_folder("/Sam");
    });
    
    match storage_init_thread{
        Ok(_) => {
            log::info!("storage_init started successfully");
        },
        Err(e) => {
            log::error!("failed to initialize storage_init: {}", e);
        }
    }
}

pub fn init_cache(){
    let cache_thread = thread::Builder::new().name("cache".to_string()).spawn(move || {
        loop{
            crate::sam::memory::FileStorage::cache_all();
            thread::sleep(Duration::from_millis(4000));
        }
    });
    
    match cache_thread{
        Ok(_) => {
            log::info!("cache started successfully");
        },
        Err(e) => {
            log::error!("failed to initialize cache: {}", e);
        }
    }
}


pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/storage/locations" {

        if request.method() == "GET" {
            let locations = crate::sam::memory::StorageLocation::select(None, None, None, None)?;
            return Ok(Response::json(&locations));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                storge_type: String,
                endpoint: String,
                username: String,
                password: String,
            })?;

            let mut location = crate::sam::memory::StorageLocation::new();
            location.storge_type = input.storge_type;
            location.endpoint = input.endpoint;
            location.username = input.username;
            location.password = input.password;
            location.save()?;

            return Ok(Response::json(&location));
        }
    }

    if request.url() == "/api/services/storage/files" {

        if request.method() == "GET" {
            let files = crate::sam::memory::FileStorage::select_lite(None, None, None, None)?;
            return Ok(Response::json(&files));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                file_data: rouille::input::post::BufferedFile,
                file_folder_tree: Option<Vec<String>>,
                storage_location_oid: Option<String>,
            })?;

            let mut file = crate::sam::memory::FileStorage::new();
            file.file_name = input.file_data.filename.ok_or("unknown")?;
            file.file_type = input.file_data.mime;
            file.file_data = Some(input.file_data.data);
            file.file_folder_tree = input.file_folder_tree;
            file.storage_location_oid = format!("SQL");
            file.save()?;

            return Ok(Response::json(&file));
        }
    }


    if request.url().contains("/api/services/storage/file/") {
        // Get file:oid
        let url = request.url();
        let split = url.split("/");
        let vec: Vec<&str> = split.collect();
        let oid = vec[5];

        // TODO: check cache first
        if Path::new(format!("/opt/sam/files/{}", oid).as_str()).exists(){
            let file = File::open(format!("/opt/sam/files/{}", oid).as_str()).unwrap();
            return Ok(Response::from_file("", file));
        }

        // Build query
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(oid.clone().to_string()));
        pg_query.query_coulmns.push(format!("oid ="));

        // Select file by oid using query
        let files = crate::sam::memory::FileStorage::select(None, None, None, Some(pg_query)).unwrap();
        
        // Clone file into memory
        let file = files[0].clone();

        // Return file to client
        return Ok(Response::from_data(file.file_type, file.file_data.unwrap()));
    }

    return Ok(Response::empty_404());
}
