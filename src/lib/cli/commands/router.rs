use crate::cli::commands::*;
use super::CommandContext;

pub async fn route_command(cmd: &str, ctx: &mut CommandContext<'_>) {
    // Check for pipe operations first
    if cmd.contains(" | ") {
        handle_pipe_command(cmd, ctx).await;
        return;
    }
    
    match cmd {
        // Direct commands
        "help" => help::handle_help(ctx.output_lines).await,
        "clear" => misc::handle_clear(ctx.output_lines).await,
        "setup" => misc::handle_setup().await,
        "ls" => misc::handle_ls(ctx.output_lines, ctx.current_dir).await,
        "version" => misc::handle_version(ctx.output_lines).await,
        "status" => status::handle_status(ctx.output_lines, ctx.current_dir, ctx.human_name).await,
        
        // Service commands (exact matches)
        _ if is_crawler_command(cmd) => crawler::handle_crawler(cmd, ctx.output_lines).await,
        _ if is_redis_command(cmd) => redis::handle_redis(cmd, ctx.output_lines).await,
        _ if is_pg_command(cmd) => pg::handle_pg(cmd, ctx.output_lines).await,
        _ if is_docker_command(cmd) => docker::handle_docker(cmd, ctx.output_lines).await,
        _ if is_spotify_command(cmd) => spotify::handle_spotify(cmd, ctx.output_lines).await,
        _ if is_lifx_command(cmd) => lifx::handle_lifx(cmd, ctx.output_lines).await,
        _ if is_sms_command(cmd) => sms::handle_sms(cmd, ctx.output_lines).await,
        
        // Migrate command (special handling)
        _ if is_migrate_command(cmd) => {
            let args = cmd.trim_start_matches("migrate").split_whitespace()
                .map(String::from).collect();
            migrate::handle_migrate(args, ctx.output_lines).await;
        },
        
        // Prefix commands
        _ if cmd.starts_with("p2p ") => p2p::handle_p2p(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("cd ") => cd::handle_cd(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("cat ") => misc::handle_cat(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("less ") => misc::handle_less(cmd, ctx.output_lines, ctx.current_dir, ctx.output_height, ctx.scroll_offset).await,
        _ if cmd.starts_with("grep ") => misc::handle_grep(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("darknet ") => darknet::handle_darknet(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("tts ") => tts::handle_tts(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("llama") => llama::handle_llama(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("matter ") => matter::handle_matter(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("crawl search ") => {
            crawler::handle_crawl_search(cmd, ctx.output_lines)
                .await
                .unwrap_or_else(|e| {
                    let mut lines = ctx.output_lines.blocking_lock();
                    lines.push(format!("crawl search error: {}", e));
                });
        },
        _ if cmd.starts_with("mdns ") => mdns::handle_mdns(cmd, ctx.output_lines.clone()).await,
        _ if cmd.starts_with("ssh ") => ssh::handle_ssh_command(cmd, ctx.output_lines).await,
        
        // Default fallback
        _ => misc::handle_default(cmd, ctx.output_lines).await,
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

// Handle piped commands
async fn handle_pipe_command(cmd: &str, ctx: &mut CommandContext<'_>) {
    let pipe_parts: Vec<&str> = cmd.split(" | ").collect();
    
    if pipe_parts.len() < 2 {
        let mut out = ctx.output_lines.lock().await;
        out.push("Invalid pipe syntax".to_string());
        return;
    }
    
    // Create a temporary buffer to store intermediate results
    let intermediate_output = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    
    // Execute the first command
    let first_cmd = pipe_parts[0].trim();
    
    // Create a new context for the first command
    let mut first_ctx = CommandContext {
        output_lines: &intermediate_output,
        current_dir: ctx.current_dir,
        human_name: ctx.human_name,
        output_height: ctx.output_height,
        scroll_offset: ctx.scroll_offset,
    };
    
    // Execute first command (but route it back to avoid infinite recursion)
    execute_single_command(first_cmd, &mut first_ctx).await;
    
    // Get the output from the first command
    let first_output = {
        let output = intermediate_output.lock().await;
        output.clone()
    };
    
    // Process through the pipe chain
    let mut current_input = first_output;
    
    for (i, pipe_cmd) in pipe_parts.iter().skip(1).enumerate() {
        let pipe_cmd = pipe_cmd.trim();
        
        // Create intermediate output for this stage
        let stage_output = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        
        // Execute the piped command with the input from previous stage
        execute_piped_command(pipe_cmd, &current_input, &stage_output, ctx.current_dir).await;
        
        // Get the output for next stage or final result
        current_input = {
            let output = stage_output.lock().await;
            output.clone()
        };
        
        // If this is the last command, put results in main output
        if i == pipe_parts.len() - 2 {
            let mut out = ctx.output_lines.lock().await;
            out.extend(current_input.clone());
        }
    }
}

// Execute a single command without pipe processing
async fn execute_single_command(cmd: &str, ctx: &mut CommandContext<'_>) {
    match cmd {
        // Direct commands
        "help" => help::handle_help(ctx.output_lines).await,
        "clear" => misc::handle_clear(ctx.output_lines).await,
        "setup" => misc::handle_setup().await,
        "ls" => misc::handle_ls(ctx.output_lines, ctx.current_dir).await,
        "version" => misc::handle_version(ctx.output_lines).await,
        "status" => status::handle_status(ctx.output_lines, ctx.current_dir, ctx.human_name).await,
        
        // Service commands (exact matches)
        _ if is_crawler_command(cmd) => crawler::handle_crawler(cmd, ctx.output_lines).await,
        _ if is_redis_command(cmd) => redis::handle_redis(cmd, ctx.output_lines).await,
        _ if is_pg_command(cmd) => pg::handle_pg(cmd, ctx.output_lines).await,
        _ if is_docker_command(cmd) => docker::handle_docker(cmd, ctx.output_lines).await,
        _ if is_spotify_command(cmd) => spotify::handle_spotify(cmd, ctx.output_lines).await,
        _ if is_lifx_command(cmd) => lifx::handle_lifx(cmd, ctx.output_lines).await,
        _ if is_sms_command(cmd) => sms::handle_sms(cmd, ctx.output_lines).await,
        
        // Migrate command (special handling)
        _ if is_migrate_command(cmd) => {
            let args = cmd.trim_start_matches("migrate").split_whitespace()
                .map(String::from).collect();
            migrate::handle_migrate(args, ctx.output_lines).await;
        },
        
        // Prefix commands
        _ if cmd.starts_with("p2p ") => p2p::handle_p2p(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("cd ") => cd::handle_cd(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("cat ") => misc::handle_cat(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("less ") => misc::handle_less(cmd, ctx.output_lines, ctx.current_dir, ctx.output_height, ctx.scroll_offset).await,
        _ if cmd.starts_with("grep ") => misc::handle_grep(cmd, ctx.output_lines, ctx.current_dir).await,
        _ if cmd.starts_with("darknet ") => darknet::handle_darknet(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("tts ") => tts::handle_tts(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("llama") => llama::handle_llama(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("matter ") => matter::handle_matter(cmd, ctx.output_lines).await,
        _ if cmd.starts_with("crawl search ") => {
            crawler::handle_crawl_search(cmd, ctx.output_lines)
                .await
                .unwrap_or_else(|e| {
                    let mut lines = ctx.output_lines.blocking_lock();
                    lines.push(format!("crawl search error: {}", e));
                });
        },
        _ if cmd.starts_with("mdns ") => mdns::handle_mdns(cmd, ctx.output_lines.clone()).await,
        _ if cmd.starts_with("ssh ") => ssh::handle_ssh_command(cmd, ctx.output_lines).await,
        
        // Default fallback
        _ => misc::handle_default(cmd, ctx.output_lines).await,
    }
}

// Execute a command in a pipe context (with stdin input)
async fn execute_piped_command(
    cmd: &str, 
    input_lines: &[String], 
    output_lines: &std::sync::Arc<tokio::sync::Mutex<Vec<String>>>, 
    current_dir: &std::path::PathBuf
) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    if args.is_empty() {
        return;
    }
    
    match args[0] {
        "grep" => {
            // Handle grep with stdin input
            misc::handle_grep_with_input(cmd, input_lines, output_lines, current_dir).await;
        },
        "cat" => {
            // Cat in pipe context just passes through the input
            let mut out = output_lines.lock().await;
            out.extend_from_slice(input_lines);
        },
        "head" => {
            // Implement head command for pipes
            let n = if args.len() > 1 && args[1] == "-n" && args.len() > 2 {
                args[2].parse().unwrap_or(10)
            } else {
                10
            };
            let mut out = output_lines.lock().await;
            out.extend(input_lines.iter().take(n).cloned());
        },
        "tail" => {
            // Implement tail command for pipes
            let n = if args.len() > 1 && args[1] == "-n" && args.len() > 2 {
                args[2].parse().unwrap_or(10)
            } else {
                10
            };
            let mut out = output_lines.lock().await;
            let start_index = input_lines.len().saturating_sub(n);
            out.extend(input_lines.iter().skip(start_index).cloned());
        },
        "wc" => {
            // Word count command for pipes
            let mut line_count = 0;
            let mut word_count = 0;
            let mut char_count = 0;
            
            for line in input_lines {
                line_count += 1;
                word_count += line.split_whitespace().count();
                char_count += line.chars().count() + 1; // +1 for newline
            }
            
            let mut out = output_lines.lock().await;
            out.push(format!("{:8} {:8} {:8}", line_count, word_count, char_count));
        },
        "sort" => {
            // Sort command for pipes
            let mut sorted_lines = input_lines.to_vec();
            sorted_lines.sort();
            let mut out = output_lines.lock().await;
            out.extend(sorted_lines);
        },
        "uniq" => {
            // Unique command for pipes
            let mut out = output_lines.lock().await;
            let mut last_line = "";
            for line in input_lines {
                if line != last_line {
                    out.push(line.clone());
                    last_line = line;
                }
            }
        },
        _ => {
            // Unknown command in pipe context
            let mut out = output_lines.lock().await;
            out.push(format!("pipe: unknown command: {}", args[0]));
        }
    }
}
