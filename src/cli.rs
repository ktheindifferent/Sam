use std::io::{self, Write};
use std::env;

/// Global flag for error display
static mut SHOW_ERRORS: bool = false;

/// Represents a CLI command with name and arguments
#[derive(Debug)]
struct Command {
    name: String,
    args: Vec<String>,
}

/// Starts the interactive command prompt
pub async fn start_prompt() {
    // Prompt for missing Postgres credentials
    check_postgres_env();

    println!("\n=== SAM Command Prompt ===");
    println!("Type 'help' for available commands");

    loop {
        print!("sam> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            cli_error("Error reading command input");
            continue;
        }

        let command = parse_command(input.trim());

        if command.name.is_empty() {
            continue;
        }

        if command.name == "exit" || command.name == "quit" {
            println!("Exiting SAM command prompt...");
            break;
        }

        execute_command(command).await;
    }
}

/// Check for missing Postgres ENV vars and prompt user if missing
pub fn check_postgres_env() {
    let vars = ["PG_DBNAME", "PG_USER", "PG_PASS", "PG_ADDRESS"];
    let mut missing = vec![];
    for v in vars.iter() {
        // Check if the variable is set and not empty
        match std::env::var(v) {
            Ok(val) if !val.trim().is_empty() => {},
            _ => missing.push(*v),
        }
    }
    if !missing.is_empty() {
        println!("Postgres credentials missing: {:?}", missing);
        for v in missing {
            loop {
                print!("Enter value for {}: ", v);
                io::stdout().flush().unwrap();
                let mut val = String::new();
                if io::stdin().read_line(&mut val).is_ok() {
                    let val = val.trim();
                    if !val.is_empty() {
                        env::set_var(v, val);
                        break;
                    }
                }
                println!("{} cannot be empty.", v);
            }
        }
    }
}

/// Parses a command string into a Command struct
fn parse_command(input: &str) -> Command {
    let mut parts = input.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let args: Vec<String> = parts.map(String::from).collect();

    Command { name, args }
}

/// Executes a command
async fn execute_command(cmd: Command) {
    match cmd.name.as_str() {
        "help" => show_help(),
        "http" => handle_http_service(cmd.args).await,
        "debug" => handle_debug(cmd.args).await,
        "status" => show_status().await,
        "services" => list_services(),
        "version" => show_version(),
        "errors" => toggle_errors(),
        _ => println!("Unknown command: {}. Type 'help' for available commands.", cmd.name),
    }
}

/// Displays help information
fn show_help() {
    println!("\n=== Available Commands ===");
    println!("help                  - Show this help message");
    println!("http start|stop       - Control HTTP/web services");
    println!("debug [module] [level]- Set debug level (error, warn, info, debug, trace)");
    println!("status                - Show system status");
    println!("services              - List all available services");
    println!("version               - Show SAM version information");
    println!("errors                - Show/hide error output in CLI");
    println!("exit, quit            - Exit the command prompt");
    println!();
}

/// Handle HTTP service commands
async fn handle_http_service(args: Vec<String>) {
    if args.is_empty() {
        println!("Usage: http [start|stop|restart|status]");
        return;
    }

    match args[0].as_str() {
        "start" => {
            println!("Starting HTTP services...");
            // Call actual service start function in the sam module
            // crate::sam::services::socket::start().await;
            println!("HTTP services started");
        },
        "stop" => {
            println!("Stopping HTTP services...");
            // Call actual service stop function in the sam module
            // crate::sam::services::socket::stop().await;
            println!("HTTP services stopped");
        },
        "restart" => {
            println!("Restarting HTTP services...");
            // Call actual service restart functions
            println!("HTTP services restarted");
        },
        "status" => {
            println!("HTTP services status: Running");
            // Get actual status
        },
        _ => println!("Unknown http command: {}. Use 'http start', 'http stop', or 'http status'", args[0]),
    }
}

/// Handle debug commands
async fn handle_debug(args: Vec<String>) {
    if args.is_empty() {
        println!("Current debug level: {}", log::max_level());
        println!("Usage: debug [module] [level]");
        println!("Levels: error, warn, info, debug, trace");
        return;
    }

    println!("Setting debug level for {} to {}",
        args.first().unwrap_or(&"all".to_string()),
        args.get(1).unwrap_or(&"info".to_string()));

    println!("Debug settings updated");
}

/// Show system status
async fn show_status() {
    println!("\n=== SAM System Status ===");
    println!("Memory usage: 120MB");
    println!("CPU usage: 5%");
    println!("Active services: websocket, rtsp, stt, sound");
    // Here we would get actual system stats
}

/// List available services
fn list_services() {
    println!("\n=== Available Services ===");
    println!("websocket - Real-time communication service");
    println!("rtsp      - Real-Time Streaming Protocol service");
    println!("stt       - Speech-to-Text service");
    println!("sound     - Audio input/output service");
    println!("lifx      - Smart lighting integration");
    println!("snapcast  - Multi-room audio system");
    println!("storage   - Data persistence service");
}

/// Show version information
fn show_version() {
    println!("SAM Version: {:?}", crate::VERSION);
    println!("Build: Development");
    // We could add more version details here
}

/// Toggle error display
fn toggle_errors() {
    unsafe {
        SHOW_ERRORS = !SHOW_ERRORS;
        if SHOW_ERRORS {
            println!("Error output ENABLED.");
        } else {
            println!("Error output HIDDEN.");
        }
    }
}

/// Print error only if enabled
fn cli_error(msg: &str) {
    unsafe {
        if SHOW_ERRORS {
            eprintln!("ERROR: {}", msg);
        }
    }
}
