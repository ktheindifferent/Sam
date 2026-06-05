pub mod humans;
pub mod io;
pub mod jobs;
pub mod locations;
pub mod observations;
pub mod ollama;
pub mod pets;
pub mod rooms;
pub mod service_control;
pub mod services;
pub mod settings;
pub mod telemetry;
pub mod things;
pub mod validation;

use rouille::Request;
use rouille::Response;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ConsoleFileList {
    path: String,
    parent: Option<String>,
    entries: Vec<ConsoleFileEntry>,
}

#[derive(Serialize)]
struct ConsoleFileEntry {
    name: String,
    path: String,
    kind: String,
    size: u64,
    modified: Option<i64>,
}

pub fn handle_api_request(
    current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    let url = request.url();

    // Handle exact route matches
    if let Some(response) = handle_exact_routes(&current_session, &url)? {
        return Ok(response);
    }

    // Handle prefix-based routes
    if let Some(response) = handle_prefix_routes(current_session, request, &url)? {
        return Ok(response);
    }

    Ok(Response::empty_404())
}

fn handle_exact_routes(
    session: &crate::memory::cache::WebSessions,
    url: &str,
) -> Result<Option<Response>, crate::http::Error> {
    match url {
        "/api/sid" => Ok(Some(Response::text(session.sid.clone()))),
        "/api/current_session" => Ok(Some(Response::json(session))),
        "/api/current_human" => handle_current_human(session),
        "/api/system/metrics" => handle_system_metrics(),
        _ => Ok(None),
    }
}

fn handle_current_human(
    session: &crate::memory::cache::WebSessions,
) -> Result<Option<Response>, crate::http::Error> {
    let mut pg_query = crate::memory::PostgresQueries::default();
    pg_query
        .queries
        .push(crate::memory::PGCol::String(session.human_oid.clone()));
    pg_query.query_columns.push("oid =".to_string());

    let human = crate::memory::Human::select(None, None, None, Some(pg_query))?;
    Ok(human.first().map(Response::json))
}

fn handle_prefix_routes(
    session: crate::memory::cache::WebSessions,
    request: &Request,
    url: &str,
) -> Result<Option<Response>, crate::http::Error> {
    // Handle service control endpoints first (more specific)
    if url.contains("/api/services/redis")
        || url.contains("/api/services/crawler")
        || url.contains("/api/services/docker")
        || url.contains("/api/services/postgres")
        || url.contains("/api/services/voice")
        || url.contains("/api/services/websocket")
        || url == "/api/services/status"
        || url == "/api/environment"
    {
        return service_control::handle(request).map(Some);
    }

    // Handle Ollama API endpoints
    if url.contains("/api/ollama") {
        return ollama::handle(request).map(Some);
    }

    if url.contains("/api/stt") || url.contains("/api/services/stt") {
        return Ok(Some(crate::services::stt::handle(
            Some(session.sid.clone()),
            request,
        )));
    }

    if url == "/api/console/files" {
        return handle_console_files(request).map(Some);
    }

    const ROUTE_HANDLERS: &[(
        &str,
        fn(crate::memory::cache::WebSessions, &Request) -> Result<Response, crate::http::Error>,
    )] = &[
        ("/api/io", |s, r| io::handle(s, r)),
        ("/api/humans", |s, r| humans::handle(s, r)),
        ("/api/locations", |s, r| locations::handle(s, r)),
        ("/api/observations", |s, r| observations::handle(s, r)),
        ("/api/rooms", |s, r| rooms::handle(s, r)),
        ("/api/services", |s, r| services::handle(s, r)),
        ("/api/settings", |s, r| settings::handle(s, r)),
        ("/api/things", |s, r| things::handle(s, r)),
    ];

    // Handle public telemetry endpoints (no session required)
    if url.contains("/api/telemetry") {
        return telemetry::handle(request).map(Some);
    }

    for (prefix, handler) in ROUTE_HANDLERS {
        if url.contains(prefix) {
            return handler(session, request).map(Some);
        }
    }

    Ok(None)
}

fn handle_system_metrics() -> Result<Option<Response>, crate::http::Error> {
    use crate::resource_management::monitoring::ResourceMonitor;

    // Create a simple runtime block to handle async call
    let rt = tokio::runtime::Runtime::new()?;

    let metrics = rt.block_on(async { ResourceMonitor::collect_metrics().await });

    Ok(Some(
        Response::json(&metrics).with_additional_header("Cache-Control", "no-cache"),
    ))
}

fn handle_console_files(request: &Request) -> Result<Response, crate::http::Error> {
    if request.method() != "GET" {
        return Ok(Response::empty_404());
    }

    let requested_path = request.get_param("path").unwrap_or_else(|| "~".to_string());
    let resolved = resolve_console_path(&requested_path)?;
    let canonical = std::fs::canonicalize(&resolved)
        .map_err(|e| crate::http::Error::from(format!("Failed to resolve path: {}", e)))?;
    let metadata = std::fs::metadata(&canonical)
        .map_err(|e| crate::http::Error::from(format!("Failed to read path: {}", e)))?;

    if !metadata.is_dir() {
        return Err(crate::http::Error::from(format!(
            "{} is not a directory",
            canonical.display()
        )));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&canonical)
        .map_err(|e| crate::http::Error::from(format!("Failed to list directory: {}", e)))?
    {
        let entry = entry.map_err(|e| {
            crate::http::Error::from(format!("Failed to read directory entry: {}", e))
        })?;
        let entry_path = entry.path();
        let entry_metadata = entry.metadata().map_err(|e| {
            crate::http::Error::from(format!("Failed to read entry metadata: {}", e))
        })?;
        let kind = if entry_metadata.is_dir() {
            "directory"
        } else if entry_metadata.is_file() {
            "file"
        } else if entry_metadata.file_type().is_symlink() {
            "symlink"
        } else {
            "other"
        };
        let modified = entry_metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64);

        entries.push(ConsoleFileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry_path.display().to_string(),
            kind: kind.to_string(),
            size: entry_metadata.len(),
            modified,
        });
    }

    entries.sort_by(|a, b| {
        let a_is_dir = a.kind == "directory";
        let b_is_dir = b.kind == "directory";
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let parent = canonical.parent().map(|path| path.display().to_string());
    Ok(Response::json(&ConsoleFileList {
        path: canonical.display().to_string(),
        parent,
        entries,
    }))
}

fn resolve_console_path(path: &str) -> Result<PathBuf, crate::http::Error> {
    if path == "~" || path.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| crate::http::Error::from("HOME is not configured".to_string()))?;
        let suffix = path.strip_prefix("~/").unwrap_or("");
        return Ok(Path::new(&home).join(suffix));
    }

    let requested = PathBuf::from(path);
    if requested.is_absolute() {
        Ok(requested)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(requested))
            .map_err(|e| crate::http::Error::from(format!("Failed to read current dir: {}", e)))
    }
}
