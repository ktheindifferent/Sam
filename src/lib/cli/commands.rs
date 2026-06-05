// Command modules
pub mod cd;
pub mod crawler;
pub mod crawler_stats;
pub mod darknet;
pub mod docker;
pub mod doctor;
pub mod help;
pub mod lifx;
pub mod llama;
pub mod matter;
pub mod mdns;
pub mod migrate;
pub mod misc;
pub mod nano;
pub mod ollama;
pub mod p2p;
pub mod pg;
pub mod plugin;
pub mod redis;
pub mod router;
pub mod sms;
pub mod spotify;
pub mod ssh;
pub mod status;
pub mod tts;
pub mod utils;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Context struct to pass command execution state
pub struct CommandContext<'a> {
    pub output_lines: &'a Arc<Mutex<Vec<String>>>,
    pub current_dir: &'a mut PathBuf,
    pub human_name: &'a str,
    pub output_height: usize,
    pub scroll_offset: &'a mut u16,
}

/// Main command handler entry point
pub async fn handle_command(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &mut PathBuf,
    human_name: &str,
    output_height: usize,
    scroll_offset: &mut u16,
) {
    let mut ctx = CommandContext {
        output_lines,
        current_dir,
        human_name,
        output_height,
        scroll_offset,
    };

    // Route the command to the appropriate handler
    router::route_command(cmd, &mut ctx).await;

    // Adjust scroll offset after command execution
    utils::adjust_scroll_offset(&mut ctx).await;
}
