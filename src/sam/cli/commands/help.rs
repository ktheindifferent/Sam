use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_help(output_lines: &Arc<Mutex<Vec<String>>>) {
    let lines = get_help_lines();
    let mut out = output_lines.lock().await;
    out.extend(lines);
}

fn get_help_lines() -> Vec<String> {
    vec![
        "help                  - Show this help message".to_string(),
        "http start|stop       - Control HTTP/web services".to_string(),
        "debug [module] [level]- Set debug level (error, warn, info, debug, trace)".to_string(),
        "status                - Show system status".to_string(),
        "services              - List all available services".to_string(),
        "version               - Show SAM version information".to_string(),
        "errors                - Show/hide error output in CLI".to_string(),
        "clear                 - Clear the terminal screen".to_string(),
        "exit, quit            - Exit the command prompt".to_string(),
        "ls                    - List files in current directory".to_string(),
        "cd <dir>              - Change current directory".to_string(),
        "tts <text>            - Convert text to speech and play it".to_string(),
        "llama install         - Install or update Llama.cpp models".to_string(),
        "llama <model_path> <prompt> - Query a Llama.cpp model".to_string(),
        "llama v2 <prompt>     - Query a Llama v2 model".to_string(),
        "lifx start            - Start the LIFX service".to_string(),
        "lifx stop             - Stop the LIFX service".to_string(),
        "lifx status           - Show LIFX service status".to_string(),
        "crawler start           - Start the background web crawler".to_string(),
        "crawler stop            - Stop the background web crawler".to_string(),
        "crawler status          - Show crawler service status".to_string(),
        "crawl search <query>   - Search crawled pages for a keyword".to_string(),
        "redis install           - Install Redis using Docker".to_string(),
        "redis start             - Start the Redis Docker container".to_string(),
        "redis stop              - Stop the Redis Docker container".to_string(),
        "redis status            - Show Redis Docker container status".to_string(),
        "docker start             - Start the Docker daemon/service".to_string(),
        "docker stop              - Stop the Docker daemon/service".to_string(),
        "docker status            - Show Docker daemon/service status".to_string(),
        "spotify start             - Start Spotify playback service".to_string(),
        "spotify stop              - Stop Spotify playback service".to_string(),
        "spotify status            - Show Spotify playback status".to_string(),
        "spotify play              - Resume Spotify playback".to_string(),
        "spotify pause             - Pause Spotify playback".to_string(),
        "spotify shuffle           - Toggle Spotify shuffle mode".to_string(),
    ]
}
