// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2023 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;
use serde::{Serialize, Deserialize};
use rouille::post_input;

pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    

    
    if request.url() == "/api/things" && request.method() == "GET"{
        

        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct WebThing {
            pub id: i32,
            pub oid: String,
            pub name: String,
            pub room: Option<crate::sam::memory::Room>,
            pub thing_type: String, // lifx, etc
            pub online_identifiers: Vec<String>,
            pub local_identifiers: Vec<String>,
            pub created_at: i64,
            pub updated_at: i64
        }

        let mut webthings: Vec<WebThing> = Vec::new();

        let objects = crate::sam::memory::Thing::select(None, None, None, None)?;
        
        for object in objects{

            let mut room: Option<crate::sam::memory::Room> = None;

            let mut pg_query = crate::sam::memory::PostgresQueries::default();
            pg_query.queries.push(crate::sam::memory::PGCol::String(object.room_oid.clone()));
            pg_query.query_coulmns.push(format!("oid ="));
            let rooms = crate::sam::memory::Room::select(None, None, None, Some(pg_query));
            match rooms{
                Ok(r) => {
                    if r.len() > 0 {
                        room = Some(r[0].clone());
                    }
                },
                Err(e) => {
                    log::error!("{}", e);
                }
            }


            let web_thing = WebThing{
                id: object.id,
                oid: object.oid,
                name: object.name,
                room: room,
                thing_type: object.thing_type,
                online_identifiers: object.online_identifiers,
                local_identifiers: object.local_identifiers,
                created_at: object.created_at,
                updated_at: object.updated_at
            };
            webthings.push(web_thing);
        }
        
        return Ok(Response::json(&webthings));
    }

    if request.url() == "/api/things" && request.method() == "POST"{
        let input = post_input!(request, {
            new_thing_name: String,
            new_thing_ip: String,
            new_thing_username: String,
            new_thing_password: String,
            new_thing_type: String
        })?;

        let mut thing = crate::sam::memory::Thing::new();
        thing.name = input.new_thing_name;
        thing.ip_address = input.new_thing_ip;
        thing.username = input.new_thing_username;
        thing.password = input.new_thing_password;
        thing.thing_type = input.new_thing_type;
        thing.save().unwrap();

        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(thing.oid.clone()));
        pg_query.query_coulmns.push(format!("oid ="));

        let objects = crate::sam::memory::Thing::select(None, None, None, Some(pg_query))?;
        if objects.len() > 0 {
            if request.url().contains(".json"){
                return Ok(Response::json(&objects[0]));
            } else {
                let response = Response::redirect_302("/things.html");
                return Ok(response);
            }
            
        } else {
            return Ok(Response::empty_404());
        }
    }

    return Ok(Response::empty_404());
}