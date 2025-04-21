// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use rouille::post_input;
use rouille::Request;
use rouille::Response;
use std::thread;

pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/settings" {

        if request.method() == "GET" {
            let objects = crate::sam::memory::Setting::select(None, None, None, None)?;
            return Ok(Response::json(&objects));
        }

        if request.method() == "POST" {

            let input = post_input!(request, {
                key: String,
                values: Vec<String>
            })?;

            let mut obj = crate::sam::memory::Setting::new();
            obj.key = input.key;
            obj.values = input.values;
            obj.save()?;
            return Ok(Response::json(&obj));

        }
    }

    if request.url().contains("/api/settings") && request.url().contains("/value") {
       
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let identifier = vec[3];

        if request.method() == "GET" && identifier.contains("key:") {
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(identifier.replace("key:", "")));
            pg_query.query_columns.push("key =".to_string());
            let objects = crate::sam::memory::Setting::select(None, None, None, Some(pg_query))?;
            return Ok(Response::text(&objects[0].clone().values[0]));
        }

    }


    if request.url().contains("/api/settings") {
       
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let identifier = vec[3];

        if request.method() == "GET" && identifier.contains("key:") {
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(identifier.replace("key:", "")));
            pg_query.query_columns.push("key =".to_string());
            let objects = crate::sam::memory::Setting::select(None, None, None, Some(pg_query))?;
            return Ok(Response::json(&objects[0]));
        }

    }


    Ok(Response::empty_404())
}


pub fn set_defaults(){
    thread::spawn(move || {
        let objects = crate::sam::memory::Setting::select(None, None, None, None).unwrap();
        if objects.is_empty() {

            // enable_embedded_lifx_server
            let mut enable_embedded_lifx_server = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_lifx_server.key = "enable_embedded_lifx_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_lifx_server.values = setting_vec;
            enable_embedded_lifx_server.save().unwrap();

            // enable_embedded_stt_server
            let mut enable_embedded_stt_server = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_stt_server.key = "enable_embedded_stt_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_stt_server.values = setting_vec;
            enable_embedded_stt_server.save().unwrap();

            // enable_embedded_tts_server
            let mut enable_embedded_tts_server = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_tts_server.key = "enable_embedded_tts_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_tts_server.values = setting_vec;
            enable_embedded_tts_server.save().unwrap();

            // enable_embedded_snapcast_server
            let mut enable_embedded_snapcast_server = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_snapcast_server.key = "enable_embedded_snapcast_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_snapcast_server.values = setting_vec;
            enable_embedded_snapcast_server.save().unwrap();

            // microphone_threshold
            let mut microphone_threshold = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            microphone_threshold.key = "microphone_threshold".to_string();
            setting_vec.push("14000".to_string());
            microphone_threshold.values = setting_vec;
            microphone_threshold.save().unwrap();

            // default_file_storage_location
            let mut default_file_storage_location = crate::sam::memory::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            default_file_storage_location.key = "default_file_storage_location".to_string();
            setting_vec.push("SQL".to_string());
            default_file_storage_location.values = setting_vec;
            default_file_storage_location.save().unwrap();



        };
    });
}