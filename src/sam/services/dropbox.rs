use dropbox_sdk::default_client::NoauthDefaultClient;
use dropbox_sdk::default_client::UserAuthDefaultClient;
use dropbox_sdk::{files, UserAuthClient};
use rouille::post_input;
use rouille::Request;
use rouille::Response;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::collections::HashMap;

use std::io::prelude::*;

pub fn get_db_obj() -> Result<crate::sam::memory::config::Service, crate::sam::services::Error> {
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query
        .queries
        .push(crate::sam::memory::PGCol::String("dropbox".to_string()));
    pg_query.query_columns.push("identifier =".to_string());
    let service = crate::sam::memory::config::Service::select(None, None, None, Some(pg_query))
        .map_err(|e| crate::sam::services::Error::Other(e.to_string()))?;
    Ok(service[0].clone())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DropboxAuth {
    pub url: String, // unique
    pub pkce: String,
}

pub fn get_auth_url() -> DropboxAuth {
    let pkce = dropbox_sdk::oauth2::PkceCode::new();
    let client_id = "ogyeqdms81svfke".to_string();
    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(pkce.clone());
    let url = dropbox_sdk::oauth2::AuthorizeUrlBuilder::new(&client_id, &oauth2_flow).build();
    DropboxAuth {
        url: url.to_string(),
        pkce: pkce.code.to_string(),
    }
}

pub fn finish_auth(pkce: String, auth_code: String) -> dropbox_sdk::oauth2::Authorization {
    let pkcee = dropbox_sdk::oauth2::PkceCode { code: pkce };

    let client_id = "ogyeqdms81svfke".to_string();
    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(pkcee);

    let auth = dropbox_sdk::oauth2::Authorization::from_auth_code(
        client_id,
        oauth2_flow,
        auth_code.trim().to_owned(),
        None,
    );

    auth
}

pub fn update_key(key: String, refresh: Option<String>) -> Result<(), crate::sam::services::Error> {
    let mut service = crate::sam::memory::config::Service::new();
    service.identifier = "dropbox".to_string();
    match refresh {
        Some(refr) => {
            if refr.len() > 2 {
                service.key = refr;
            } else {
                let existing = get_db_obj().map_err(|e| {
                    log::error!("Failed to get dropbox database object: {}", e);
                    crate::sam::services::Error::Other(format!("Failed to get dropbox database object: {}", e))
                })?;
                service.key = existing.key;
            }
        }
        None => {
            let existing = get_db_obj().map_err(|e| {
                log::error!("Failed to get dropbox database object: {}", e);
                crate::sam::services::Error::Other(format!("Failed to get dropbox database object: {}", e))
            })?;
            service.key = existing.key;
        }
    }
    service.secret = key;
    service.endpoint = String::new();
    service.save().map_err(|e| {
        log::error!("Failed to save dropbox service configuration: {}", e);
        crate::sam::services::Error::Other(format!("Failed to save dropbox service configuration: {}", e))
    })?;
    Ok(())
}

//  dropbox_sdk::files::delete_v2(&client, &dropbox_sdk::files::DeleteArg::new(path.clone()));

pub fn handle(
    _current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/dropbox" {
        let path_param = request.get_param("path");

        match path_param {
            Some(path) => {
                let files = get_paths(&path);
                return Ok(Response::json(&files));
            }
            None => {
                let files = get_paths("/");
                return Ok(Response::json(&files));
            }
        }
    }

    if request.url() == "/api/services/dropbox/download" {
        let path_param = request.get_param("path").ok_or_else(|| {
            log::error!("Missing required 'path' parameter for dropbox download");
            crate::sam::http::Error::BadRequest("Missing path parameter".to_string())
        })?;
        let data = download_file(&path_param).map_err(|e| {
            log::error!("Failed to download file from dropbox: {}", e);
            crate::sam::http::Error::InternalServerError(e.to_string())
        })?;

        let response = Response::from_data("", data);

        return Ok(response);
    }

    if request.url() == "/api/services/dropbox/auth/1" {
        let auth = get_auth_url();
        return Ok(Response::json(&auth));
    }

    if request.url() == "/api/services/dropbox/auth/2" {
        let input = post_input!(request, {
            pkce: String,
            auth_code: String
        })?;

        let mut auth = finish_auth(input.pkce, input.auth_code);

        let noc = NoauthDefaultClient::default();
        let new = auth.obtain_access_token(noc).map_err(|e| {
            log::error!("Failed to obtain access token from dropbox: {}", e);
            crate::sam::http::Error::InternalServerError("Failed to obtain access token".to_string())
        })?;
        let saved_key = auth.save().ok_or_else(|| {
            log::error!("Failed to save dropbox auth");
            crate::sam::http::Error::InternalServerError("Failed to save auth".to_string())
        })?;
        update_key(saved_key, Some(new.refresh_token)).map_err(|e| {
            log::error!("Failed to update dropbox key: {}", e);
            crate::sam::http::Error::InternalServerError("Failed to update key".to_string())
        })?;

        let response = Response::redirect_302("/services.html");
        return Ok(response);
    }

    Ok(Response::empty_404())
}

pub fn destroy_empty_directories() {
    let dropbox_destroy_empty_directories = thread::Builder::new()
        .name("dropbox_destroy_empty_directories".to_string())
        .spawn(move || {
            let empties = crate::sam::services::dropbox::empty_directories();
            for e in empties {
                if is_path_empty(&e.clone()) {
                    if let Err(err) = delete(&e.clone()) {
                        log::error!("Failed to delete empty directory '{}': {}", e, err);
                    }
                }
            }
        });

    match dropbox_destroy_empty_directories {
        Ok(_) => {
            log::info!("dropbox_destroy_empty_directories task started successfully");
        }
        Err(e) => {
            log::error!(
                "failed to initialize dropbox_destroy_empty_directories task: {}",
                e
            );
        }
    }
}

pub fn create_sam_folder() -> Result<(), crate::sam::services::Error> {
    create_folder("/Sam")
}

pub fn create_folder(path: &str) -> Result<(), crate::sam::services::Error> {
    let obj = get_db_obj().map_err(|e| {
        log::error!("Failed to get dropbox database object: {}", e);
        crate::sam::services::Error::Other(format!("Failed to get dropbox database object: {}", e))
    })?;
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret)
        .ok_or_else(|| {
            log::error!("Failed to load dropbox authorization");
            crate::sam::services::Error::Other("Failed to load dropbox authorization".to_string())
        })?;
    let client = UserAuthDefaultClient::new(auth.clone());
    dropbox_sdk::files::create_folder_v2(
        &client,
        &dropbox_sdk::files::CreateFolderArg::new(path.to_string()),
    ).map_err(|e| {
        log::error!("Failed to create dropbox folder '{}': {}", path, e);
        crate::sam::services::Error::Other(format!("Failed to create dropbox folder '{}': {}", path, e))
    })?;
    Ok(())
}

/// File cache entry with metadata
#[derive(Clone, Debug)]
struct CachedFile {
    data: Vec<u8>,
    hash: String,
    cached_at: u64,
    size: usize,
}

/// In-memory cache for Dropbox files with TTL
static FILE_CACHE: Lazy<Mutex<HashMap<String, CachedFile>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Cache configuration
const CACHE_TTL_SECONDS: u64 = 3600; // 1 hour TTL
const MAX_CACHE_SIZE_MB: usize = 100; // Maximum 100MB cache size
const MAX_FILE_SIZE_MB: usize = 10; // Don't cache files larger than 10MB

/// Calculate SHA256 hash of data
fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Clean expired entries from cache
fn clean_expired_cache() {
    let now = current_timestamp();
    if let Ok(mut cache) = FILE_CACHE.lock() {
        cache.retain(|_, entry| {
            now - entry.cached_at < CACHE_TTL_SECONDS
        });
    }
}

/// Get total cache size in bytes
fn get_cache_size() -> usize {
    if let Ok(cache) = FILE_CACHE.lock() {
        cache.values().map(|entry| entry.size).sum()
    } else {
        0
    }
}

/// Evict least recently cached files if cache is too large
fn evict_if_needed(new_size: usize) {
    let max_size_bytes = MAX_CACHE_SIZE_MB * 1024 * 1024;
    
    if let Ok(mut cache) = FILE_CACHE.lock() {
        let current_size = cache.values().map(|e| e.size).sum::<usize>();
        
        if current_size + new_size > max_size_bytes {
            // Sort by cached_at timestamp and remove oldest entries
            let mut entries: Vec<(String, u64, usize)> = cache
                .iter()
                .map(|(k, v)| (k.clone(), v.cached_at, v.size))
                .collect();
            entries.sort_by_key(|e| e.1);
            
            let mut removed_size = 0;
            for (key, _, size) in entries {
                if current_size - removed_size + new_size <= max_size_bytes {
                    break;
                }
                cache.remove(&key);
                removed_size += size;
                log::debug!("Evicted cached file: {} ({}KB)", key, size / 1024);
            }
        }
    }
}

/// Download file with caching support
pub fn download_file(dropbox_path: &str) -> Result<Vec<u8>, String> {
    // Clean expired entries periodically
    clean_expired_cache();
    
    // Check cache first
    if let Ok(cache) = FILE_CACHE.lock() {
        if let Some(entry) = cache.get(dropbox_path) {
            let now = current_timestamp();
            if now - entry.cached_at < CACHE_TTL_SECONDS {
                log::info!("Cache hit for file: {}", dropbox_path);
                return Ok(entry.data.clone());
            }
        }
    }
    
    log::info!("Cache miss for file: {}, downloading from Dropbox", dropbox_path);
    let obj = get_db_obj().map_err(|e| {
        log::error!("Failed to get dropbox database object: {}", e);
        format!("Failed to get dropbox database object: {}", e)
    })?;
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret)
        .ok_or_else(|| {
            log::error!("Failed to load dropbox authorization");
            "Failed to load dropbox authorization".to_string()
        })?;
    let client = UserAuthDefaultClient::new(auth.clone());
    let dropbox_file = dropbox_sdk::files::download(
        &client,
        &dropbox_sdk::files::DownloadArg::new(dropbox_path.to_string()),
        None,
        None,
    );

    let file_result = dropbox_file.map_err(|e| {
        log::error!("Failed to download file from dropbox: {}", e);
        format!("Failed to download file from dropbox: {}", e)
    })?;
    
    let file_data = file_result.map_err(|e| {
        log::error!("Dropbox API error: {}", e);
        format!("Dropbox API error: {}", e)
    })?;
    
    let mut body = file_data.body.ok_or_else(|| {
        log::error!("No body in dropbox download response");
        "No body in dropbox download response".to_string()
    })?;

    let mut data = Vec::new();
    body.read_to_end(&mut data).map_err(|e| {
        log::error!("Unable to read dropbox download data: {}", e);
        format!("Unable to read dropbox download data: {}", e)
    })?;

    // Cache the file if it's not too large
    let file_size = data.len();
    if file_size <= MAX_FILE_SIZE_MB * 1024 * 1024 {
        // Evict old entries if needed
        evict_if_needed(file_size);
        
        let cached_file = CachedFile {
            data: data.clone(),
            hash: calculate_hash(&data),
            cached_at: current_timestamp(),
            size: file_size,
        };
        
        if let Ok(mut cache) = FILE_CACHE.lock() {
            cache.insert(dropbox_path.to_string(), cached_file);
            log::info!("Cached file: {} ({}KB)", dropbox_path, file_size / 1024);
        }
    } else {
        log::info!("File too large to cache: {} ({}MB)", dropbox_path, file_size / 1024 / 1024);
    }
    
    Ok(data)

    // log::info!("dropbox_file: {:?}", );
}

pub fn delete(path: &str) -> Result<(), crate::sam::services::Error> {
    let obj = get_db_obj().map_err(|e| {
        log::error!("Failed to get dropbox database object: {}", e);
        crate::sam::services::Error::Other(format!("Failed to get dropbox database object: {}", e))
    })?;
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret)
        .ok_or_else(|| {
            log::error!("Failed to load dropbox authorization");
            crate::sam::services::Error::Other("Failed to load dropbox authorization".to_string())
        })?;
    let client = UserAuthDefaultClient::new(auth.clone());
    dropbox_sdk::files::delete_v2(
        &client,
        &dropbox_sdk::files::DeleteArg::new(path.to_string()),
    ).map_err(|e| {
        log::error!("Failed to delete dropbox path '{}': {}", path, e);
        crate::sam::services::Error::Other(format!("Failed to delete dropbox path '{}': {}", path, e))
    })?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DropboxObject {
    pub path: String, // unique
    pub object_type: String,
}

pub fn get_paths(path: &str) -> Vec<DropboxObject> {
    let obj = match get_db_obj() {
        Ok(obj) => obj,
        Err(e) => {
            log::error!("Failed to get dropbox database object: {}", e);
            return Vec::new();
        }
    };
    let auth = dropbox_sdk::oauth2::Authorization::from_refresh_token(
        "ogyeqdms81svfke".to_string(),
        obj.key,
    );
    let client = UserAuthDefaultClient::new(auth.clone());

    let mut paths: Vec<DropboxObject> = Vec::new();

    match list_directory(&client, path, false) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);
                        let obj = DropboxObject {
                            path,
                            object_type: "folder".to_string(),
                        };
                        paths.push(obj);
                    }
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);
                        let obj = DropboxObject {
                            path,
                            object_type: "file".to_string(),
                        };
                        paths.push(obj);
                    }
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    }
                    Ok(Err(_e)) => {
                        // log::error!("Error from files/list_folder_continue: {}", _e);
                        break;
                    }
                    Err(_e) => {
                        // log::error!("API request error: {}", _e);
                        break;
                    }
                }
            }
        }
        Ok(Err(_e)) => {
            log::error!("Error from files/list_folder");
        }
        Err(_e) => {
            log::error!("API request error");
        }
    }

    paths
}

pub fn empty_directories() -> Vec<String> {
    let obj = match get_db_obj() {
        Ok(obj) => obj,
        Err(e) => {
            log::error!("Failed to get dropbox database object: {}", e);
            return Vec::new();
        }
    };
    let auth = match dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret) {
        Some(auth) => auth,
        None => {
            log::error!("Failed to load dropbox authorization");
            return Vec::new();
        }
    };
    let client = UserAuthDefaultClient::new(auth.clone());

    let mut empty_directories: Vec<String> = Vec::new();

    match list_directory(&client, "/", true) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);

                        if is_path_empty(&path.clone()) {
                            empty_directories.push(path.clone());
                        }
                    }
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        // log::info!("File: {}", entry.path_display.unwrap_or(entry.name));
                    }
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    }
                    Ok(Err(_e)) => {
                        // log::error!("Error from files/list_folder_continue: {}", _e);
                        break;
                    }
                    Err(_e) => {
                        // log::error!("API request error: {}", _e);
                        break;
                    }
                }
            }
        }
        Ok(Err(_e)) => {
            log::error!("Error from files/list_folder");
        }
        Err(_e) => {
            log::error!("API request error");
        }
    }

    empty_directories
}

pub fn is_path_empty(path: &str) -> bool {
    log::info!("checking if dropbox path is empty: {}", path);

    let obj = match get_db_obj() {
        Ok(obj) => obj,
        Err(e) => {
            log::error!("Failed to get dropbox database object: {}", e);
            return false; // Conservative approach - assume not empty if we can't check
        }
    };
    let auth = match dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret) {
        Some(auth) => auth,
        None => {
            log::error!("Failed to load dropbox authorization");
            return false; // Conservative approach - assume not empty if we can't check
        }
    };
    let client = UserAuthDefaultClient::new(auth.clone());

    let mut empty = true;
    match list_directory(&client, path, true) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        // empty = false;
                    }
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        empty = false;
                        return empty;
                    }
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    }
                    Ok(Err(_e)) => {
                        // log::error!("Error from files/list_folder_continue: {}", _e);
                        // break;
                    }
                    Err(_e) => {
                        // log::error!("API request error: {}", _e);
                        // break;
                    }
                }
            }
        }
        Ok(Err(_e)) => {
            // log::error!("Error from files/list_folder: {}", _e);
        }
        Err(_e) => {
            // log::error!("API request error: {}", _e);
        }
    }

    empty
}

pub fn get_auth_from_env_or_prompt() -> dropbox_sdk::oauth2::Authorization {
    let client_id = String::new();

    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(dropbox_sdk::oauth2::PkceCode::new());
    let url = dropbox_sdk::oauth2::AuthorizeUrlBuilder::new(&client_id, &oauth2_flow).build();
    log::error!("Open this URL in your browser:");
    log::error!("{}", url);
    // log::error!();
    let auth_code = String::new();

    dropbox_sdk::oauth2::Authorization::from_auth_code(
        client_id,
        oauth2_flow,
        auth_code.trim().to_owned(),
        None,
    )
}

fn list_directory<'a, T: UserAuthClient>(
    client: &'a T,
    path: &str,
    recursive: bool,
) -> dropbox_sdk::Result<Result<DirectoryIterator<'a, T>, files::ListFolderError>> {
    assert!(
        path.starts_with('/'),
        "path needs to be absolute (start with a '/')"
    );
    let requested_path = if path == "/" {
        // Root folder should be requested as empty string
        String::new()
    } else {
        path.to_owned()
    };
    match files::list_folder(
        client,
        &files::ListFolderArg::new(requested_path).with_recursive(recursive),
    ) {
        Ok(Ok(result)) => {
            let cursor = if result.has_more {
                Some(result.cursor)
            } else {
                None
            };

            Ok(Ok(DirectoryIterator {
                client,
                cursor,
                buffer: result.entries.into(),
            }))
        }
        Ok(Err(e)) => Ok(Err(e)),
        Err(e) => Err(e),
    }
}

struct DirectoryIterator<'a, T: UserAuthClient> {
    client: &'a T,
    buffer: VecDeque<files::Metadata>,
    cursor: Option<String>,
}

impl<'a, T: UserAuthClient> Iterator for DirectoryIterator<'a, T> {
    type Item = dropbox_sdk::Result<Result<files::Metadata, files::ListFolderContinueError>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.buffer.pop_front() {
            Some(Ok(Ok(entry)))
        } else if let Some(cursor) = self.cursor.take() {
            match files::list_folder_continue(
                self.client,
                &files::ListFolderContinueArg::new(cursor),
            ) {
                Ok(Ok(result)) => {
                    self.buffer.extend(result.entries);
                    if result.has_more {
                        self.cursor = Some(result.cursor);
                    }
                    self.buffer.pop_front().map(|entry| Ok(Ok(entry)))
                }
                Ok(Err(e)) => Some(Ok(Err(e))),
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}
