// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use crate::services::thread_manager;
use rouille::post_input;
use rouille::Request;
use rouille::Response;

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/settings" {
        if request.method() == "GET" {
            let objects = crate::memory::config::Setting::select(None, None, None, None)?;
            return Ok(Response::json(&objects));
        }

        if request.method() == "POST" {
            let input = post_input!(request, {
                key: String,
                values: Vec<String>
            })?;

            let mut obj = crate::memory::config::Setting::new();
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
        let Some(identifier) = vec.get(3) else {
            return Ok(Response::empty_404());
        };

        if request.method() == "GET" && identifier.contains("key:") {
            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(identifier.replace("key:", "")));
            pg_query.query_columns.push("key =".to_string());
            let objects = crate::memory::config::Setting::select(None, None, None, Some(pg_query))?;
            let Some(setting) = objects.first() else {
                return Ok(Response::empty_404());
            };
            let Some(value) = setting.values.first() else {
                return Ok(Response::empty_404());
            };
            return Ok(Response::text(value));
        }
    }

    if request.url().contains("/api/settings") {
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let Some(identifier) = vec.get(3) else {
            return Ok(Response::empty_404());
        };

        if request.method() == "GET" && identifier.contains("key:") {
            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(identifier.replace("key:", "")));
            pg_query.query_columns.push("key =".to_string());
            let objects = crate::memory::config::Setting::select(None, None, None, Some(pg_query))?;
            let Some(setting) = objects.first() else {
                return Ok(Response::empty_404());
            };
            return Ok(Response::json(setting));
        }
    }

    Ok(Response::empty_404())
}

pub fn set_defaults() {
    thread_manager::spawn("settings-defaults", move |_shutdown_signal, _health_rx| {
        let objects = match crate::memory::config::Setting::select(None, None, None, None) {
            Ok(objects) => objects,
            Err(e) => {
                log::error!("Failed to load settings defaults: {}", e);
                return;
            }
        };
        if objects.is_empty() {
            // enable_embedded_lifx_server
            let mut enable_embedded_lifx_server = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_lifx_server.key = "enable_embedded_lifx_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_lifx_server.values = setting_vec;
            if let Err(e) = enable_embedded_lifx_server.save() {
                log::error!("Failed to save enable_embedded_lifx_server default: {}", e);
            }

            // enable_embedded_stt_server
            let mut enable_embedded_stt_server = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_stt_server.key = "enable_embedded_stt_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_stt_server.values = setting_vec;
            if let Err(e) = enable_embedded_stt_server.save() {
                log::error!("Failed to save enable_embedded_stt_server default: {}", e);
            }

            // enable_embedded_tts_server
            let mut enable_embedded_tts_server = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_tts_server.key = "enable_embedded_tts_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_tts_server.values = setting_vec;
            if let Err(e) = enable_embedded_tts_server.save() {
                log::error!("Failed to save enable_embedded_tts_server default: {}", e);
            }

            // enable_embedded_snapcast_server
            let mut enable_embedded_snapcast_server = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            enable_embedded_snapcast_server.key = "enable_embedded_snapcast_server".to_string();
            setting_vec.push("false".to_string());
            enable_embedded_snapcast_server.values = setting_vec;
            if let Err(e) = enable_embedded_snapcast_server.save() {
                log::error!(
                    "Failed to save enable_embedded_snapcast_server default: {}",
                    e
                );
            }

            // microphone_threshold
            let mut microphone_threshold = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            microphone_threshold.key = "microphone_threshold".to_string();
            setting_vec.push("14000".to_string());
            microphone_threshold.values = setting_vec;
            if let Err(e) = microphone_threshold.save() {
                log::error!("Failed to save microphone_threshold default: {}", e);
            }

            // default_file_storage_location
            let mut default_file_storage_location = crate::memory::config::Setting::new();
            let mut setting_vec: Vec<String> = Vec::new();
            default_file_storage_location.key = "default_file_storage_location".to_string();
            setting_vec.push("SQL".to_string());
            default_file_storage_location.values = setting_vec;
            if let Err(e) = default_file_storage_location.save() {
                log::error!(
                    "Failed to save default_file_storage_location default: {}",
                    e
                );
            }
        };
    });
}
