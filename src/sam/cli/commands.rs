pub mod cd;
pub mod crawler;
pub mod darknet;
pub mod docker;
pub mod help;
pub mod lifx;
pub mod llama;
pub mod misc;
pub mod p2p;
pub mod pg;
pub mod redis;
pub mod sms;
pub mod spotify;
pub mod status;
pub mod tts;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_command(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &mut PathBuf,
    human_name: &str,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    match cmd {
        "help" => help::handle_help(output_lines).await,
        "clear" => misc::handle_clear(output_lines).await,
        "setup" => misc::handle_setup().await,
        "ls" => misc::handle_ls(output_lines, current_dir).await,
        "version" => misc::handle_version(output_lines).await,
        "status" => status::handle_status(output_lines, current_dir, human_name).await,
        "crawler start" | "crawler stop" | "crawler status" => {
            crawler::handle_crawler(cmd, output_lines).await
        }
        "redis install" | "redis start" | "redis stop" | "redis status" => {
            redis::handle_redis(cmd, output_lines).await
        }
        "pg install" | "pg start" | "pg stop" | "pg status" => {
            pg::handle_pg(cmd, output_lines).await
        }
        "docker start" | "docker stop" | "docker status" => {
            docker::handle_docker(cmd, output_lines).await
        }
        "spotify start" | "spotify stop" | "spotify status" | "spotify play" | "spotify pause"
        | "spotify shuffle" => spotify::handle_spotify(cmd, output_lines).await,
        "lifx start" | "lifx stop" | "lifx status" => lifx::handle_lifx(cmd, output_lines).await,
        "sms start" | "sms stop" | "sms status" => sms::handle_sms(cmd, output_lines).await,
        _ if cmd.starts_with("p2p ") => p2p::handle_p2p(cmd, output_lines).await,
        _ if cmd.starts_with("cd ") => cd::handle_cd(cmd, output_lines, current_dir).await,
        _ if cmd.starts_with("darknet ") => darknet::handle_darknet(cmd, output_lines).await,
        _ if cmd.starts_with("tts ") => tts::handle_tts(cmd, output_lines).await,
        _ if cmd.starts_with("llama") => llama::handle_llama(cmd, output_lines).await,
        _ if cmd.starts_with("crawl search ") => crawler::handle_crawl_search(cmd, output_lines)
            .await
            .unwrap(),
        _ => misc::handle_default(cmd, output_lines).await,
    }
    // Scroll to bottom if needed
    let output_window_height = output_height;
    let lines = output_lines.lock().await;
    *scroll_offset = 0;
    if lines.len() > output_window_height {
        *scroll_offset = lines.len() as u16 - output_window_height as u16 + 2;
    }
}
