pub mod nst;
pub mod srgan;


use std::fs;
use std::fs::File;
use std::io::{Write};

use rouille::post_input;
use rouille::Request;
use rouille::Response;



pub fn install() -> std::io::Result<()> {
    // let data = include_bytes!("../../../packages/tch/vgg16.ot");

    // let mut pos = 0;
    // let mut buffer = File::create("/opt/sam/models/vgg16.ot")?;

    // while pos < data.len() {
    //     let bytes_written = buffer.write(&data[pos..])?;
    //     pos += bytes_written;
    // }

    Ok(())
}

pub fn handle(current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/nst"){
        return nst::handle(current_session, request);
    }
    return Ok(Response::empty_404());
}
