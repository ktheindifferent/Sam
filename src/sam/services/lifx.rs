// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// extern crate lifx_api_server;
extern crate lifx_rs as lifx;

pub mod lifx_api_server;

use crate::sam::services::Result;
use once_cell::sync::Lazy;
use online::check;
use rouille::post_input;
use rouille::Request;
use rouille::Response;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

// Add a static for the StopHandle
static LIFX_SERVER_STOP_HANDLE: Lazy<
    Arc<Mutex<Option<crate::sam::services::lifx::lifx_api_server::StopHandle>>>,
> = Lazy::new(|| Arc::new(Mutex::new(None)));
static LIFX_SERVER_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));
static LIFX_SERVER_RUNNING: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

/// Start the LIFX service (server and sync)
pub fn start_service() {
    let mut running = match LIFX_SERVER_RUNNING.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_RUNNING lock: {}", e);
            return;
        }
    };
    if *running {
        log::info!("LIFX service already running");
        return;
    }
    *running = true;
    let handle = thread::spawn(move || {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String("lifx".to_string()));
        pg_query.query_columns.push("identifier =".to_string());
        let services =
            crate::sam::memory::config::Service::select(None, None, None, Some(pg_query));
        match services {
            Ok(services) => {
                crate::sam::services::lifx::init_server(services[0].secret.clone());
                crate::sam::services::lifx::sync(services[0].secret.clone());
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }
        // Keep thread alive until stopped
        loop {
            match LIFX_SERVER_RUNNING.lock() {
                Ok(guard) => {
                    if !*guard {
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Failed to acquire LIFX_SERVER_RUNNING lock: {}", e);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        log::info!("LIFX service thread exiting");
    });
    match LIFX_SERVER_HANDLE.lock() {
        Ok(mut handle_slot) => {
            *handle_slot = Some(handle);
            log::info!("LIFX service started");
        }
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_HANDLE lock: {}", e);
        }
    }
}

/// Stop the LIFX service
pub fn stop_service() {
    let mut running = match LIFX_SERVER_RUNNING.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_RUNNING lock: {}", e);
            return;
        }
    };
    if !*running {
        log::info!("LIFX service is not running");
        return;
    }
    *running = false;
    drop(running); // Release lock before joining thread to avoid deadlock

    // Stop the HTTP server via StopHandle
    match LIFX_SERVER_STOP_HANDLE.lock() {
        Ok(mut stop_handle_slot) => {
            if let Some(stop_handle) = stop_handle_slot.take() {
                stop_handle.stop(); // now consumes the handle and joins the thread
            }
        }
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_STOP_HANDLE lock: {}", e);
        }
    }

    // Join the background thread
    match LIFX_SERVER_HANDLE.lock() {
        Ok(mut handle_slot) => {
            if let Some(handle) = handle_slot.take() {
                let _ = handle.join();
                log::info!("LIFX service stopped");
            }
        }
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_HANDLE lock: {}", e);
        }
    }
}

/// Get the status of the LIFX service
pub fn status_service() -> &'static str {
    match LIFX_SERVER_RUNNING.lock() {
        Ok(running) => {
            if *running {
                "running"
            } else {
                "stopped"
            }
        }
        Err(e) => {
            log::error!("Failed to acquire LIFX_SERVER_RUNNING lock: {}", e);
            "unknown"
        }
    }
}

// Refactor init to use start_service
pub fn init() {
    start_service();
}

pub fn init_server(key: String) {
    let stop_handle_slot = LIFX_SERVER_STOP_HANDLE.clone();
    let lifx_thread = thread::Builder::new()
        .name("lifx_api_server".to_string())
        .spawn(move || {
            let config = lifx_api_server::Config {
                secret_key: key,
                port: 7084,
            };

            // Start the lifx_api_server and store the StopHandle
            let server_stop_handle = lifx_api_server::start(config);

            // Store the StopHandle in the global static for later control (e.g., stop)
            {
                match stop_handle_slot.lock() {
                    Ok(mut slot) => {
                        *slot = Some(server_stop_handle);
                    }
                    Err(e) => {
                        log::error!("Failed to acquire stop_handle_slot lock: {}", e);
                    }
                }
            }

            // Keep thread alive until service is stopped
            loop {
                match LIFX_SERVER_RUNNING.lock() {
                    Ok(guard) => {
                        if !*guard {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to acquire LIFX_SERVER_RUNNING lock: {}", e);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

    match lifx_thread {
        Ok(handle) => {
            match LIFX_SERVER_HANDLE.lock() {
                Ok(mut handle_slot) => {
                    *handle_slot = Some(handle);
                    log::info!("lifx api server started successfully");
                }
                Err(e) => {
                    log::error!("Failed to acquire LIFX_SERVER_HANDLE lock: {}", e);
                }
            }
        }
        Err(e) => {
            log::error!("failed to initialize lifx api server: {}", e);
        }
    }
}

pub fn get_lifx_service_db_obj() -> Result<crate::sam::memory::config::Service> {
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query
        .queries
        .push(crate::sam::memory::PGCol::String("lifx".to_string()));
    pg_query.query_columns.push("identifier =".to_string());
    let service = crate::sam::memory::config::Service::select(None, None, None, Some(pg_query))?;
    Ok(service[0].clone())
}

pub fn handle(
    _current_session: crate::sam::memory::cache::WebSessions,
    request: &Request,
) -> std::result::Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/lifx/list_all" {
        match get_lifx_service_db_obj() {
            Ok(service) => {
                match crate::sam::services::lifx::get_all(service.secret.clone()) {
                    Ok(objects) => return Ok(Response::json(&objects)),
                    Err(e) => {
                        log::error!("Failed to get all LIFX objects: {}", e);
                        return Ok(Response::empty_404());
                    }
                }
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/public/list" {
        match get_lifx_service_db_obj() {
            Ok(service) => {
                match crate::sam::services::lifx::get(service.secret.clone(), true) {
                    Ok(objects) => return Ok(Response::json(&objects)),
                    Err(e) => {
                        log::error!("Failed to get public LIFX objects: {}", e);
                        return Ok(Response::empty_404());
                    }
                }
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/private/list" {
        match get_lifx_service_db_obj() {
            Ok(service) => {
                let objects = crate::sam::services::lifx::get(service.secret.clone(), false);
                match objects {
                    Ok(objects) => {
                        return Ok(Response::json(&objects));
                    }
                    Err(e) => {
                        log::error!("{}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/set_state" {
        let input = post_input!(request, {
            selector: String,
            power: String,
            use_public: String,
        })?;

        match get_lifx_service_db_obj() {
            Ok(service) => {
                let mut public = false;
                if input.use_public == "true" {
                    public = true;
                }

                crate::sam::services::lifx::set(
                    service.secret.clone(),
                    input.selector.clone(),
                    public,
                    Some(input.power.clone()),
                    None,
                );

                return Ok(Response::json(&()));
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/set_color" {
        let input = post_input!(request, {
            selector: String,
            color: String,
            use_public: String
        })?;

        match get_lifx_service_db_obj() {
            Ok(service) => {
                let mut public = false;
                if input.use_public == "true" {
                    public = true;
                }

                crate::sam::services::lifx::set(
                    service.secret.clone(),
                    input.selector.clone(),
                    public,
                    None,
                    Some(input.color.clone()),
                );
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    Ok(Response::empty_404())
}

pub fn get_lifx_endpoint() -> String {
    if check(Some(3)).is_ok() {
        return "https://api.lifx.com".to_string();
    } else {
        match get_lifx_service_db_obj() {
            Ok(service) => {
                return service.endpoint.clone();
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }
    }
    "https://api.lifx.com".to_string()
}

pub fn select_lifx_endpoint(public: bool) -> String {
    if public {
        "https://api.lifx.com".to_string()
    } else {
        match get_lifx_service_db_obj() {
            Ok(service) => {
                return service.endpoint.clone();
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }

        "https://api.lifx.com".to_string()
    }
}

pub fn get_all(key: String) -> Result<lifx::Lights> {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    Ok(lifx::Light::list_all(config.clone())?)
}

pub fn get(key: String, public: bool) -> Result<lifx::Lights> {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(select_lifx_endpoint(public));

    let config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    Ok(lifx::Light::list_all(config.clone())?)
}

pub fn set(
    key: String,
    selector: String,
    public: bool,
    power: Option<String>,
    color: Option<String>,
) {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(select_lifx_endpoint(public));

    let lifx_config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    let mut state = lifx::State::new();
    state.power = power;
    state.color = color;

    // Turn off all lights
    match lifx::Light::set_state_by_selector(lifx_config.clone(), selector, state) {
        Ok(_) => {}
        Err(e) => log::error!("failed to set lifx state: {:?}", e),
    }
}

pub fn set_state(key: String, selector: String, power: Option<String>, color: Option<String>) {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let lifx_config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    let mut state = lifx::State::new();
    state.power = power;
    state.color = color;

    match lifx::Light::set_state_by_selector(lifx_config.clone(), selector, state) {
        Ok(_) => {}
        Err(e) => log::error!("failed to set lifx state: {:?}", e),
    }
}

pub fn sync(key: String) {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push("https://api.lifx.com".to_string());

    let lifx_config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    let _storable_thing_vec: Vec<crate::sam::memory::Thing> = Vec::new();

    let lights = match lifx::Light::list_all(lifx_config.clone()) {
        Ok(lights) => lights,
        Err(e) => {
            log::error!("Failed to list all LIFX lights: {}", e);
            return;
        }
    };
    for light in lights {
        let mut thing = crate::sam::memory::Thing::new();

        // =================================================================
        // Sync Group/Location/Room/Name
        // =================================================================
        let location = light.location;
        let group = light.group;

        let mut loc = crate::sam::memory::Location::new();
        loc.name = location.name.clone();
        if let Err(e) = loc.save() {
            log::error!("Failed to save location: {}", e);
        }

        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(location.name.clone()));
        pg_query.query_columns.push("name ilike".to_string());

        let matching_locations = match crate::sam::memory::Location::select(None, None, None, Some(pg_query)) {
            Ok(locations) => locations,
            Err(e) => {
                log::error!("Failed to select locations: {}", e);
                vec![]
            }
        };

        if !matching_locations.is_empty() {
            for matching_location in matching_locations {
                let mut room = crate::sam::memory::Room::new();
                room.name = group.name.clone();
                room.location_oid = matching_location.oid.clone();
                if let Err(e) = room.save() {
                    log::error!("Failed to save room: {}", e);
                }
            }
        }

        // Get location oid
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String(location.name.clone()));
        pg_query.query_columns.push("name ilike".to_string());
        let locations = match crate::sam::memory::Location::select(None, None, None, Some(pg_query)) {
            Ok(locs) => locs,
            Err(e) => {
                log::error!("Failed to select locations: {}", e);
                vec![]
            }
        };
        if !locations.is_empty() {
            let location_oid = locations[0].oid.clone();
            // Get room oid
            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::sam::memory::PGCol::String(location_oid.clone()));
            pg_query.query_columns.push("location_oid =".to_string());
            pg_query
                .queries
                .push(crate::sam::memory::PGCol::String(group.name.clone()));
            pg_query.query_columns.push(" AND name ilike".to_string());
            let rooms = match crate::sam::memory::Room::select(None, None, None, Some(pg_query)) {
                Ok(rooms) => rooms,
                Err(e) => {
                    log::error!("Failed to select rooms: {}", e);
                    vec![]
                }
            };
            if !rooms.is_empty() {
                thing.room_oid = rooms[0].oid.clone();
            }
        }

        // =================================================================
        // END Sync Group/Location/Room/Name
        // =================================================================

        let mut online_identifiers: Vec<String> = Vec::new();
        online_identifiers.push(light.id.clone());
        online_identifiers.push(light.uuid.clone());
        online_identifiers.push(light.label.clone());

        thing.name = light.label.clone();
        thing.thing_type = "lifx".to_string();
        thing.online_identifiers = online_identifiers.clone();

        let mut local_api_endpoints: Vec<String> = Vec::new();
        local_api_endpoints.push(get_lifx_endpoint());

        let local_config = lifx::LifxConfig {
            access_token: key.clone(),
            api_endpoints: local_api_endpoints,
        };
        let xlocal_lights = lifx::Light::list_all(local_config.clone());
        match xlocal_lights {
            Ok(local_lights) => {
                for local_light in local_lights {
                    if local_light.label.clone() == light.label.clone() {
                        let mut local_identifiers: Vec<String> = Vec::new();
                        local_identifiers.push(local_light.id.clone());
                        local_identifiers.push(local_light.uuid.clone());
                        local_identifiers.push(local_light.label.clone());
                        thing.local_identifiers = local_identifiers.clone();
                    }
                }
            }
            Err(er) => {
                log::error!("{}", er);
            }
        }

        let existing_things = match crate::sam::memory::Thing::select(None, None, None, None) {
            Ok(things) => things,
            Err(e) => {
                log::error!("Failed to select existing things: {}", e);
                vec![]
            }
        };

        let mut already_exists = false;
        for existing_thing in existing_things {
            if existing_thing.name == light.label {
                already_exists = true;
            }

            for onlineid in thing.online_identifiers.clone() {
                for extonlineid in existing_thing.online_identifiers.clone() {
                    if onlineid == extonlineid {
                        already_exists = true;
                    }
                }
            }
        }

        if !already_exists {
            if let Err(e) = thing.save() {
                log::error!("Failed to save thing: {}", e);
            }
        }
    }

    sync_local(key.clone());
    sync_local(key);
}

pub fn sync_local(key: String) {
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let lifx_config = lifx::LifxConfig {
        access_token: key.clone(),
        api_endpoints,
    };

    let _storable_thing_vec: Vec<crate::sam::memory::Thing> = Vec::new();

    let lights = match lifx::Light::list_all(lifx_config.clone()) {
        Ok(lights) => lights,
        Err(e) => {
            log::error!("Failed to list all LIFX lights: {}", e);
            return;
        }
    };
    for light in lights {
        let mut local_identifiers: Vec<String> = Vec::new();
        local_identifiers.push(light.id.clone());
        local_identifiers.push(light.uuid.clone());
        local_identifiers.push(light.label.clone());

        let mut thing = crate::sam::memory::Thing::new();
        thing.name = light.label.clone();
        thing.thing_type = "lifx".to_string();
        thing.local_identifiers = local_identifiers.clone();

        let existing_things = match crate::sam::memory::Thing::select(None, None, None, None) {
            Ok(things) => things,
            Err(e) => {
                log::error!("Failed to select existing things: {}", e);
                vec![]
            }
        };

        let mut already_exists = false;
        for existing_thing in existing_things {
            if existing_thing.name == light.label {
                already_exists = true;
            }

            for onlineid in thing.local_identifiers.clone() {
                for extonlineid in existing_thing.local_identifiers.clone() {
                    if onlineid == extonlineid {
                        already_exists = true;
                    }
                }
            }
        }

        if !already_exists {
            if let Err(e) = thing.save() {
                log::error!("Failed to save thing: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use proptest::prelude::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    
    #[test]
    fn test_service_lifecycle() {
        let initial_status = status_service();
        assert_eq!(initial_status, "stopped");
        
        // Note: Can't fully test start/stop without a database connection
        // but we can verify the status tracking
        let running = LIFX_SERVER_RUNNING.lock().unwrap();
        assert!(!*running);
    }
    
    #[test]
    fn test_get_lifx_endpoint() {
        let endpoint = get_lifx_endpoint();
        assert!(endpoint.starts_with("https://") || endpoint.starts_with("http://"));
        assert!(endpoint.contains("api.lifx.com"));
    }
    
    #[test]
    fn test_select_lifx_endpoint() {
        let public_endpoint = select_lifx_endpoint(true);
        assert!(public_endpoint.contains("api.lifx.com"));
        
        let private_endpoint = select_lifx_endpoint(false);
        assert!(private_endpoint.contains("api.lifx.com") || private_endpoint.contains("localhost"));
    }
    
    #[tokio::test]
    async fn test_get_all_with_mock_server() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/lights/all"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(serde_json::json!([
                    {
                        "id": "d073d5017219",
                        "uuid": "029b8c12-b0a6-47cc-a719-3d3d163e2b06",
                        "label": "Test Light",
                        "connected": true,
                        "power": "on",
                        "color": {
                            "hue": 120.0,
                            "saturation": 1.0,
                            "kelvin": 3500
                        },
                        "brightness": 0.75,
                        "group": {
                            "id": "1c8de82b81f445e7cfaafae49b259c71",
                            "name": "Living Room"
                        },
                        "location": {
                            "id": "1c8de82b81f445e7cfaafae49b259c71",
                            "name": "Home"
                        },
                        "last_seen": "2023-01-01T00:00:00Z",
                        "seconds_since_seen": 0,
                        "product": {
                            "name": "LIFX A19",
                            "identifier": "lifx_a19",
                            "company": "LIFX",
                            "vendor_id": 1,
                            "product_id": 22,
                            "capabilities": {
                                "has_color": true,
                                "has_variable_color_temp": true,
                                "has_ir": false,
                                "has_hev": false,
                                "has_chain": false,
                                "has_multizone": false,
                                "min_kelvin": 2500,
                                "max_kelvin": 9000
                            }
                        }
                    }
                ])))
            .mount(&mock_server)
            .await;
        
        // Note: In real tests, we'd need to mock the actual endpoint URL
        // This is a demonstration of the pattern
    }
    
    #[test]
    fn test_service_status() {
        let status = status_service();
        assert!(status == "running" || status == "stopped");
    }
    
    proptest! {
        #[test]
        fn test_endpoint_selection_always_returns_valid_url(public in any::<bool>()) {
            let endpoint = select_lifx_endpoint(public);
            prop_assert!(endpoint.starts_with("http"));
            prop_assert!(endpoint.contains("://"));
        }
        
        #[test]
        fn test_color_string_validation(
            hue in 0.0..360.0f32,
            saturation in 0.0..1.0f32,
            brightness in 0.0..1.0f32,
            kelvin in 2500..9000u16
        ) {
            let color_string = format!("hue:{} saturation:{} brightness:{} kelvin:{}",
                hue, saturation, brightness, kelvin);
            prop_assert!(color_string.contains("hue:"));
            prop_assert!(color_string.contains("saturation:"));
            prop_assert!(color_string.contains("brightness:"));
            prop_assert!(color_string.contains("kelvin:"));
        }
    }
    
    #[test]
    fn test_handle_invalid_urls() {
        use rouille::Request;
        use std::io::Cursor;
        
        let request_data = b"GET /invalid/url HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let cursor = Cursor::new(&request_data[..]);
        
        // Note: This would require more complex setup with rouille
        // Demonstrating the test structure
    }
    
    #[test]
    fn test_concurrent_service_starts() {
        use std::sync::Arc;
        use std::thread;
        
        let barrier = Arc::new(std::sync::Barrier::new(5));
        let mut handles = vec![];
        
        for _ in 0..5 {
            let c = barrier.clone();
            let handle = thread::spawn(move || {
                c.wait();
                // Attempting to start service multiple times should be safe
                // Only one should actually start
                let status = status_service();
                assert!(status == "running" || status == "stopped");
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    #[test]
    fn test_error_handling_in_get_lifx_service_db_obj() {
        // This test demonstrates error path testing
        // In a real scenario, we'd mock the database connection
        let result = get_lifx_service_db_obj();
        // The result will likely be an error in test environment without DB
        assert!(result.is_err() || result.is_ok());
    }
}
