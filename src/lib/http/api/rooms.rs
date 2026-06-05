// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;
use serde::{Deserialize, Serialize};

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url().contains("/api/rooms") && request.url().contains("/things") {
        let url = request.url().clone();
        let split = url.split("/");
        let vec = split.collect::<Vec<&str>>();
        let Some(room_oid) = vec.get(3).filter(|oid| !oid.is_empty()) else {
            return Ok(Response::empty_404());
        };

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

        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String(room_oid.to_string()));
        pg_query.query_columns.push("room_oid =".to_string());
        let objects = crate::memory::Thing::select(None, None, None, Some(pg_query))?;

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

    if request.url() == "/api/rooms" {
        let objects = crate::memory::Room::select(None, None, None, None)?;
        return Ok(Response::json(&objects));
    }

    Ok(Response::empty_404())
}
