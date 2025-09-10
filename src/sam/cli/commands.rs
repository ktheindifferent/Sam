pub mod cd;
pub mod crawler;
pub mod crawler_stats;
pub mod darknet;
pub mod docker;
pub mod help;
pub mod lifx;
pub mod llama;
pub mod matter;
pub mod mdns;
pub mod migrate;
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

#[cfg(unix)]
pub async fn handle_ssh(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
    tui_takeover: impl FnOnce(Box<dyn FnMut(&[u8]) + Send>, Box<dyn FnMut() -> Option<Vec<u8>> + Send>),
) {
    use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
    use std::io::Read;
    
    
    

    let ssh_args = cmd.trim_start_matches("ssh ").trim();
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd_builder = CommandBuilder::new("ssh");
    for arg in ssh_args.split_whitespace() {
        cmd_builder.arg(arg);
    }
    let child = pair.slave.spawn_command(cmd_builder).unwrap();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();

    // Use tui_takeover to forward input/output
    tui_takeover(
        Box::new(move |input: &[u8]| {
            let _ = writer.write_all(input);
            let _ = writer.flush();
        }),
        Box::new(move || {
            let mut buf = [0u8; 1024];
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => Some(buf[..n].to_vec()),
                _ => None,
            }
        }),
    );

    let mut lines = output_lines.lock().await;
    lines.push(format!("[ssh] Session ended: {cmd}"));
}

pub struct CommandContext<'a> {
    pub output_lines: &'a Arc<Mutex<Vec<String>>>,
    pub current_dir: &'a mut PathBuf,
    pub human_name: &'a str,
    pub output_height: usize,
    pub scroll_offset: &'a mut u16,
}

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

    execute_command(cmd, &mut ctx).await;
    adjust_scroll_offset(&mut ctx).await;
}

async fn execute_command(cmd: &str, ctx: &mut CommandContext<'_>) {
    match cmd {
        "help" => help::handle_help(ctx.output_lines).await,
        "clear" => misc::handle_clear(ctx.output_lines).await,
        "setup" => misc::handle_setup().await,
        "ls" => misc::handle_ls(ctx.output_lines, ctx.current_dir).await,
        "version" => misc::handle_version(ctx.output_lines).await,
        "status" => status::handle_status(ctx.output_lines, ctx.current_dir, ctx.human_name).await,
        _ => handle_service_commands(cmd, ctx).await,
    }
}

async fn handle_service_commands(cmd: &str, ctx: &mut CommandContext<'_>) {
    if is_crawler_command(cmd) {
        crawler::handle_crawler(cmd, ctx.output_lines).await;
    } else if is_redis_command(cmd) {
        redis::handle_redis(cmd, ctx.output_lines).await;
    } else if is_pg_command(cmd) {
        pg::handle_pg(cmd, ctx.output_lines).await;
    } else if is_migrate_command(cmd) {
        let args = cmd.trim_start_matches("migrate").split_whitespace()
            .map(String::from).collect();
        migrate::handle_migrate(args, ctx.output_lines).await;
    } else if is_docker_command(cmd) {
        docker::handle_docker(cmd, ctx.output_lines).await;
    } else if is_spotify_command(cmd) {
        spotify::handle_spotify(cmd, ctx.output_lines).await;
    } else if is_lifx_command(cmd) {
        lifx::handle_lifx(cmd, ctx.output_lines).await;
    } else if is_sms_command(cmd) {
        sms::handle_sms(cmd, ctx.output_lines).await;
    } else {
        handle_prefix_commands(cmd, ctx).await;
    }
}

async fn handle_prefix_commands(cmd: &str, ctx: &mut CommandContext<'_>) {
    if cmd.starts_with("p2p ") {
        p2p::handle_p2p(cmd, ctx.output_lines).await;
    } else if cmd.starts_with("cd ") {
        cd::handle_cd(cmd, ctx.output_lines, ctx.current_dir).await;
    } else if cmd.starts_with("darknet ") {
        darknet::handle_darknet(cmd, ctx.output_lines).await;
    } else if cmd.starts_with("tts ") {
        tts::handle_tts(cmd, ctx.output_lines).await;
    } else if cmd.starts_with("llama") {
        llama::handle_llama(cmd, ctx.output_lines).await;
    } else if cmd.starts_with("matter ") {
        crate::sam::cli::commands::matter::handle_matter(cmd, ctx.output_lines).await;
    } else if cmd.starts_with("crawl search ") {
        crawler::handle_crawl_search(cmd, ctx.output_lines)
            .await
            .unwrap();
    } else if cmd.starts_with("mdns ") {
        mdns::handle_mdns(cmd, ctx.output_lines.clone()).await;
    } else if cmd.starts_with("ssh ") {
        handle_ssh_command(cmd, ctx).await;
    } else {
        misc::handle_default(cmd, ctx.output_lines).await;
    }
}

async fn handle_ssh_command(cmd: &str, ctx: &CommandContext<'_>) {
    #[cfg(unix)]
    {
        use crate::sam::cli::tui::tui_takeover_ssh_session;
        handle_ssh(cmd, ctx.output_lines, tui_takeover_ssh_session).await;
    }
    #[cfg(not(unix))]
    {
        let mut lines = ctx.output_lines.lock().await;
        lines.push("[ssh] SSH interactive shell is only supported on Unix systems.".to_string());
    }
}

async fn adjust_scroll_offset(ctx: &mut CommandContext<'_>) {
    let lines = ctx.output_lines.lock().await;
    *ctx.scroll_offset = 0;
    if lines.len() > ctx.output_height {
        *ctx.scroll_offset = lines.len() as u16 - ctx.output_height as u16 + 2;
    }
}

// Helper functions to check command types
fn is_crawler_command(cmd: &str) -> bool {
    matches!(cmd, "crawler start" | "crawler stop" | "crawler status")
}

fn is_redis_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "redis install" | "redis start" | "redis stop" | "redis status"
    )
}

fn is_pg_command(cmd: &str) -> bool {
    matches!(cmd, "pg install" | "pg start" | "pg stop" | "pg status")
}

fn is_migrate_command(cmd: &str) -> bool {
    cmd.starts_with("migrate")
}

fn is_docker_command(cmd: &str) -> bool {
    matches!(cmd, "docker start" | "docker stop" | "docker status")
}

fn is_spotify_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "spotify start"
            | "spotify stop"
            | "spotify status"
            | "spotify play"
            | "spotify pause"
            | "spotify shuffle"
    )
}

fn is_lifx_command(cmd: &str) -> bool {
    matches!(cmd, "lifx start" | "lifx stop" | "lifx status")
}

fn is_sms_command(cmd: &str) -> bool {
    matches!(cmd, "sms start" | "sms stop" | "sms status")
}
