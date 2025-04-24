pub mod nst;
pub mod srgan;



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

pub fn handle(current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/nst"){
        return nst::handle(current_session, request);
    }
    Ok(Response::empty_404())
}
