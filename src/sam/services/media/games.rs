use std::fs;
use std::fs::File;
use std::io::{Write};
use titlecase::titlecase;
use serde::{Serialize, Deserialize};
use rouille::Request;
use rouille::Response;


pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/games") {
        return Ok(Response::json(&games()?)); // Simplified unwrap
    }
    Ok(Response::empty_404())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub name: String,
    pub launch: String,
    pub icon: String,
}

pub fn games() -> Result<Vec<Game>, crate::sam::services::Error> {
    let mut games: Vec<Game> = Vec::new();
    let paths = fs::read_dir("/opt/sam/games/")?;
    for path in paths {

        let pth = path.unwrap().path().display().to_string();


        if !pth.contains(".zip") {
            let game = Game{
                name: titlecase(&pth.clone().to_string().replace("/opt/sam/games/", "").replace("_", " ")),
                launch: format!("{}/index.html", pth.clone().replace("/opt/sam", "")),
                icon: format!("{}/icon.png", pth.clone().replace("/opt/sam", "")),
            };
            games.push(game);
        }
        
    }
    Ok(games)
}

pub fn install() -> Result<(), crate::sam::services::Error> {
    let data = include_bytes!("../../../../packages/games/Flappy_Kitty.zip");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/games/Flappy_Kitty.zip")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    crate::sam::tools::extract_zip("/opt/sam/games/Flappy_Kitty.zip", "/opt/sam/games/");


    Ok(())
}