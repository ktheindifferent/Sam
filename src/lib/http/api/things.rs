// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::post_input;
use rouille::Request;
use rouille::Response;
use serde::{Deserialize, Serialize};

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/things" && request.method() == "GET" {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct WebThing {
            pub id: i32,
            pub oid: String,
            pub name: String,
            pub room: Option<crate::memory::Room>,
            pub thing_type: String, // lifx, etc
            pub online_identifiers: Vec<String>,
            pub local_identifiers: Vec<String>,
            pub created_at: i64,
            pub updated_at: i64,
        }

        let mut webthings: Vec<WebThing> = Vec::new();

        let objects = crate::memory::Thing::select(None, None, None, None)?;

        for object in objects {
            let mut room: Option<crate::memory::Room> = None;

            let mut pg_query = crate::memory::PostgresQueries::default();
            pg_query
                .queries
                .push(crate::memory::PGCol::String(object.room_oid.clone()));
            pg_query.query_columns.push("oid =".to_string());
            let rooms = crate::memory::Room::select(None, None, None, Some(pg_query));
            match rooms {
                Ok(r) => {
                    if let Some(found_room) = r.first() {
                        room = Some(found_room.clone());
                    }
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }

            let web_thing = WebThing {
                id: object.id,
                oid: object.oid,
                name: object.name,
                room,
                thing_type: object.thing_type,
                online_identifiers: object.online_identifiers,
                local_identifiers: object.local_identifiers,
                created_at: object.created_at,
                updated_at: object.updated_at,
            };
            webthings.push(web_thing);
        }

        return Ok(Response::json(&webthings));
    }

    if request.url() == "/api/things" && request.method() == "POST" {
        let input = post_input!(request, {
            new_thing_name: String,
            new_thing_ip: String,
            new_thing_username: String,
            new_thing_password: String,
            new_thing_type: String
        })?;

        let mut thing = crate::memory::Thing::new();
        thing.name = input.new_thing_name;
        thing.ip_address = input.new_thing_ip;
        thing.username = input.new_thing_username;
        thing.password = input.new_thing_password;
        thing.thing_type = input.new_thing_type;
        thing.save()?;

        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String(thing.oid.clone()));
        pg_query.query_columns.push("oid =".to_string());

        let objects = crate::memory::Thing::select(None, None, None, Some(pg_query))?;
        if let Some(thing) = objects.first() {
            if request.url().contains(".json") {
                return Ok(Response::json(thing));
            } else {
                let response = Response::redirect_302("/things.html");
                return Ok(response);
            }
        } else {
            return Ok(Response::empty_404());
        }
    }

    if request.url().contains("/api/things/matter") {
        return Ok(crate::services::matter::handle(request));
    }

    Ok(Response::empty_404())
}
