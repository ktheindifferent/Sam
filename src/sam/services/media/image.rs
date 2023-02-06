pub mod nst;
pub mod srgan;


use std::fs;
use std::fs::File;
use std::io::{Write};

use rouille::post_input;
use rouille::Request;
use rouille::Response;



pub fn install() -> std::io::Result<()> {
    match nst::install(){
        Ok(_) => {
            log::info!("NST installed successfully");
        },
        Err(e) => {
            log::error!("Failed to install NST: {}", e);
        }
    }

    Ok(())
}

pub fn handle(current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/nst"){
        return nst::handle(current_session, request);
    }
    return Ok(Response::empty_404());
}
