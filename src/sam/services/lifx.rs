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

use online::check;
use rouille::Request;
use rouille::Response;
use rouille::post_input;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use once_cell::sync::Lazy;
use crate::sam::services::Result;


// Add a static for the StopHandle
static LIFX_SERVER_STOP_HANDLE: Lazy<Arc<Mutex<Option<crate::sam::services::lifx::lifx_api_server::StopHandle>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static LIFX_SERVER_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static LIFX_SERVER_RUNNING: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

/// Start the LIFX service (server and sync)
pub fn start_service() {
    let mut running = LIFX_SERVER_RUNNING.lock().unwrap();
    if *running {
        log::info!("LIFX service already running");
        return;
    }
    *running = true;
    let handle = thread::spawn(move || {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String("lifx".to_string()));
        pg_query.query_columns.push("identifier =".to_string());
        let services = crate::sam::memory::config::Service::select(None, None, None, Some(pg_query));
        match services {
            Ok(services) => {
                crate::sam::services::lifx::init_server(services[0].secret.clone());
                crate::sam::services::lifx::sync(services[0].secret.clone());
            },
            Err(e) => {
                log::error!("{}", e);
            }
        }
        // Keep thread alive until stopped
        while *LIFX_SERVER_RUNNING.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        log::info!("LIFX service thread exiting");
    });
    let mut handle_slot = LIFX_SERVER_HANDLE.lock().unwrap();
    *handle_slot = Some(handle);
    log::info!("LIFX service started");
}

/// Stop the LIFX service
pub fn stop_service() {
    let mut running = LIFX_SERVER_RUNNING.lock().unwrap();
    if !*running {
        log::info!("LIFX service is not running");
        return;
    }
    *running = false;
    drop(running); // Release lock before joining thread to avoid deadlock

    // Stop the HTTP server via StopHandle
    let mut stop_handle_slot = LIFX_SERVER_STOP_HANDLE.lock().unwrap();
    if let Some(stop_handle) = stop_handle_slot.take() {
        stop_handle.stop(); // now consumes the handle and joins the thread
    }

    // Join the background thread
    let mut handle_slot = LIFX_SERVER_HANDLE.lock().unwrap();
    if let Some(handle) = handle_slot.take() {
        let _ = handle.join();
        log::info!("LIFX service stopped");
    }
}

/// Get the status of the LIFX service
pub fn status_service() -> &'static str {
    let running = LIFX_SERVER_RUNNING.lock().unwrap();
    if *running {
        "running"
    } else {
        "stopped"
    }
}

// Refactor init to use start_service
pub fn init() {
    start_service();
}

pub fn init_server(key: String) {
    let stop_handle_slot = LIFX_SERVER_STOP_HANDLE.clone();
    let lifx_thread = thread::Builder::new().name("lifx_api_server".to_string()).spawn(move || {
        let config = lifx_api_server::Config { 
            secret_key: key,
            port: 7084
        };

        // Start the lifx_api_server and store the StopHandle
        let server_stop_handle = lifx_api_server::start(config);

        // Store the StopHandle in the global static for later control (e.g., stop)
        {
            let mut slot = stop_handle_slot.lock().unwrap();
            *slot = Some(server_stop_handle);
        }

        // Keep thread alive until service is stopped
        while *LIFX_SERVER_RUNNING.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    match lifx_thread{
        Ok(handle) => {
            let mut handle_slot = LIFX_SERVER_HANDLE.lock().unwrap();
            *handle_slot = Some(handle);
            log::info!("lifx api server started successfully");
        },
        Err(e) => {
            log::error!("failed to initialize lifx api server: {}", e);
        }
    }
}



pub fn get_lifx_service_db_obj() -> Result<crate::sam::memory::config::Service>{
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String("lifx".to_string()));
    pg_query.query_columns.push("identifier =".to_string());
    let service = crate::sam::memory::config::Service::select(None, None, None, Some(pg_query))?;
    Ok(service[0].clone())
}

pub fn handle(_current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> std::result::Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/lifx/list_all" {

        match get_lifx_service_db_obj(){
            Ok(service) => {
                let objects = crate::sam::services::lifx::get_all(service.secret.clone()).unwrap();
                return Ok(Response::json(&objects));
            },
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/public/list" {

        match get_lifx_service_db_obj(){
            Ok(service) => {
                let objects = crate::sam::services::lifx::get(service.secret.clone(), true).unwrap();
                return Ok(Response::json(&objects));
            },
            Err(e) => {
                log::error!("{}", e);
            }
        }

        return Ok(Response::empty_404());
    }

    if request.url() == "/api/services/lifx/private/list" {

        match get_lifx_service_db_obj(){
            Ok(service) => {
                let objects = crate::sam::services::lifx::get(service.secret.clone(), false);
                match objects {
                    Ok(objects) => {
                        return Ok(Response::json(&objects));
                    },
                    Err(e) => {
                        log::error!("{}", e);
                    }
                }
            },
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

         



        match get_lifx_service_db_obj(){
            Ok(service) => {
          
                let mut public = false;
                if input.use_public == "true"{
                    public = true;
                }

                crate::sam::services::lifx::set(service.secret.clone(), input.selector.clone(), public, Some(input.power.clone()), None);


                
                return Ok(Response::json(&()));
            },
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


        match get_lifx_service_db_obj(){
            Ok(service) => {
                let mut public = false;
                if input.use_public == "true"{
                    public = true;
                }
    
                crate::sam::services::lifx::set(service.secret.clone(), input.selector.clone(), public, None, Some(input.color.clone()));    
            },
            Err(e) => {
                log::error!("{}", e);
            }
        }


        return Ok(Response::empty_404());
    }

    
    Ok(Response::empty_404())
}

pub fn get_lifx_endpoint() -> String {
    if check(Some(3)).is_ok(){
        return "https://api.lifx.com".to_string();
    } else {
        match get_lifx_service_db_obj(){
            Ok(service) => {
                return service.endpoint.clone();
            },
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

        match get_lifx_service_db_obj(){
            Ok(service) => {
                return service.endpoint.clone();
            },
            Err(e) => {
                log::error!("{}", e);
            }
        }

        "https://api.lifx.com".to_string()
    }
}

pub fn get_all(key: String) -> Result<lifx::Lights>{
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    Ok(lifx::Light::list_all(config.clone())?)
}

pub fn get(key: String, public: bool) -> Result<lifx::Lights>{
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(select_lifx_endpoint(public));

    let config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    Ok(lifx::Light::list_all(config.clone())?)
}


pub fn set(key: String, selector: String, public: bool, power: Option<String>, color: Option<String>){
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(select_lifx_endpoint(public));

    let lifx_config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    let mut state = lifx::State::new();
    state.power = power;
    state.color = color;

    // Turn off all lights
    match lifx::Light::set_state_by_selector(lifx_config.clone(), selector, state){
        Ok(_) => {},
        Err(e) => log::error!("failed to set lifx state: {:?}", e),
    }
}

pub fn set_state(key: String, selector: String, power: Option<String>, color: Option<String>){
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let lifx_config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    let mut state = lifx::State::new();
    state.power = power;
    state.color = color;

    match lifx::Light::set_state_by_selector(lifx_config.clone(), selector, state){
        Ok(_) => {},
        Err(e) => log::error!("failed to set lifx state: {:?}", e),
    }
}


pub fn sync(key: String){

    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push("https://api.lifx.com".to_string());

    let lifx_config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    let _storable_thing_vec: Vec<crate::sam::memory::Thing> = Vec::new();


    let lights = lifx::Light::list_all(lifx_config.clone()).unwrap();
    for light in lights{

        let mut thing = crate::sam::memory::Thing::new();

        // =================================================================
        // Sync Group/Location/Room/Name
        // =================================================================
        let location = light.location;
        let group = light.group;

        let mut loc = crate::sam::memory::Location::new();
        loc.name = location.name.clone();
        loc.save().unwrap();



        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(location.name.clone()));
        pg_query.query_columns.push("name ilike".to_string());

        let matching_locations = crate::sam::memory::Location::select(None, None, None, Some(pg_query)).unwrap();
        
        
        
        
        if !matching_locations.is_empty() {
            for matching_location in matching_locations{
                let mut room = crate::sam::memory::Room::new();
                room.name = group.name.clone();
                room.location_oid = matching_location.oid.clone();
                room.save().unwrap();
            }
        }

        // Get location oid
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(location.name.clone()));
        pg_query.query_columns.push("name ilike".to_string());
        let locations = crate::sam::memory::Location::select(None, None, None, Some(pg_query)).unwrap();
        if !locations.is_empty() {
            let location_oid = locations[0].oid.clone();
              // Get room oid
              let mut pg_query = crate::sam::memory::PostgresQueries::default();
              pg_query.queries.push(crate::sam::memory::PGCol::String(location_oid.clone()));
              pg_query.query_columns.push("location_oid =".to_string());
              pg_query.queries.push(crate::sam::memory::PGCol::String(group.name.clone()));
              pg_query.query_columns.push(" AND name ilike".to_string());
            let rooms = crate::sam::memory::Room::select(None, None, None, Some(pg_query)).unwrap();
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


        let local_config = lifx::LifxConfig{
            access_token: key.clone(),
            api_endpoints: local_api_endpoints
        };
        let xlocal_lights = lifx::Light::list_all(local_config.clone());
        match xlocal_lights{
            Ok(local_lights) => {
                for local_light in local_lights{
                    if local_light.label.clone() == light.label.clone() {
                        let mut local_identifiers: Vec<String> = Vec::new();
                        local_identifiers.push(local_light.id.clone());
                        local_identifiers.push(local_light.uuid.clone());
                        local_identifiers.push(local_light.label.clone());
                        thing.local_identifiers = local_identifiers.clone();
                    }
                }
            },
            Err(er) => {
                log::error!("{}", er);
            }
        }
        



        let existing_things = crate::sam::memory::Thing::select(None, None, None, None).unwrap();
        
        let mut already_exists = false;
        for existing_thing in existing_things{
            if existing_thing.name == light.label {
                already_exists = true;
            }

            for onlineid in thing.online_identifiers.clone(){
                for extonlineid in existing_thing.online_identifiers.clone(){
                    if onlineid == extonlineid{
                        already_exists = true;
                    }
                }
            }
        }

        if !already_exists{
            thing.save().unwrap();
        }

    }

    sync_local(key.clone());
    sync_local(key);
}



pub fn sync_local(key: String){
    let mut api_endpoints: Vec<String> = Vec::new();
    api_endpoints.push(get_lifx_endpoint());

    let lifx_config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints
    };

    let _storable_thing_vec: Vec<crate::sam::memory::Thing> = Vec::new();

    let lights = lifx::Light::list_all(lifx_config.clone()).unwrap();
    for light in lights{

        let mut local_identifiers: Vec<String> = Vec::new();
        local_identifiers.push(light.id.clone());
        local_identifiers.push(light.uuid.clone());
        local_identifiers.push(light.label.clone());

        let mut thing = crate::sam::memory::Thing::new();
        thing.name = light.label.clone();
        thing.thing_type = "lifx".to_string();
        thing.local_identifiers = local_identifiers.clone();
        


        let existing_things = crate::sam::memory::Thing::select(None, None, None, None).unwrap();
        
        let mut already_exists = false;
        for existing_thing in existing_things{
            if existing_thing.name == light.label {
                already_exists = true;
            }

            for onlineid in thing.local_identifiers.clone(){
                for extonlineid in existing_thing.local_identifiers.clone(){
                    if onlineid == extonlineid{
                        already_exists = true;
                    }
                }
            }
        }

        if !already_exists{
            thing.save().unwrap();
        }

    }
}