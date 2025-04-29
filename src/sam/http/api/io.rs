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
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IOReply {
    pub text: String,
    pub timestamp: i64,
    pub response_type: String,
}


pub fn handle(_current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
   
    let input = request.get_param("input");

    match input{
        Some(iput) => {
            
            let rivescript_reply = crate::sam::tools::cmd(format!("python3 /opt/sam/scripts/rivescript/brain.py \"{iput}\"").as_str());

            // Match on the reply before responding
            match rivescript_reply {
                Ok(rs) => {
                    if rs.contains(":::::"){
                        // TODO - Parse Command
                    } 
                
                    let io = IOReply{
                        text: rs,
                        timestamp: 0,
                        response_type: "io".to_string()
                    };
                
                    Ok(Response::json(&io))
                },
                Err(_e) => {
                    let response = Response::text("RiveScript error").with_status_code(500);
                    Ok(response)
                }
            }
        
          
        },
        None => {
            let response = Response::text("IO input malformed").with_status_code(500);
            Ok(response)
        }
    }
   
}