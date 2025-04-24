use std::fs;
use std::path::Path;

// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod sam;



// External crates
extern crate wikipedia;
extern crate hound;
extern crate postgres;
extern crate threadpool;
#[macro_use]
extern crate lazy_static;

use std::env;
use std::os::unix::fs::PermissionsExt;

// Store application version as a const, set at compile time
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Main entry point for the SAM application.
/// Initializes logging, environment variables, configuration, and all core services.
#[tokio::main]
async fn main() {

    std::panic::set_hook(Box::new(|info| {
        // Optionally log to a file or TUI logger instead
        // e.g., log::error!("Panic: {:?}", info);
        // Do nothing to suppress terminal output
    }));

    // Ensure CARGO_MANIFEST_DIR is set; if not, set it to the current directory
    if std::env::var("CARGO_MANIFEST_DIR").is_err() {
        if let Ok(current_dir) = std::env::current_dir() {
            if let Some(dir_str) = current_dir.to_str() {
                std::env::set_var("CARGO_MANIFEST_DIR", dir_str);
            }
        }
    }



    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let opt_sam_path = Path::new("/opt/sam");
        if !opt_sam_path.exists() {
            if let Err(e) = fs::create_dir_all(opt_sam_path) {
                log::error!("Failed to create /opt/sam: {}", e);
            } else if let Err(e) = fs::set_permissions(opt_sam_path, fs::Permissions::from_mode(0o755)) {
                log::error!("Failed to set permissions on /opt/sam: {}", e);
            }
        }
        crate::sam::tools::uinx_cmd("chmod -R 777 /opt/sam");
        crate::sam::tools::uinx_cmd("chown 1000 -R /opt/sam");
    }


    // Store the current username in the SAM_USER environment variable
    let mut user = whoami::username();
    let opt_sam_path = Path::new("/opt/sam/");
    if user != "root" {
        let file_path = opt_sam_path.join("whoismyhuman");
        if !file_path.exists() {
            if let Err(e) = fs::write(&file_path, &user) {
                log::error!("Failed to create whoismyhuman: {}", e);
            }
        }
        if opt_sam_path.exists() && opt_sam_path.is_dir() {
            let file_path = opt_sam_path.join("whoismyhuman");
            if let Err(e) = fs::write(&file_path, &user) {
                log::error!("Failed to write whoismyhuman: {}", e);
            }
        }
    }

    // Attempt to read username from /opt/sam/whoismyhuman if it exists
    let whois_path = Path::new("/opt/sam/whoismyhuman");
    user = if whois_path.exists() {
        match fs::read_to_string(whois_path) {
            Ok(contents) => contents.trim().to_string(),
            Err(_) => user,
        }
    } else {
        user
    };
   

    // Print ASCII art banner and version
    println!("███████     █████     ███    ███    ");
    println!("██         ██   ██    ████  ████    ");
    println!("███████    ███████    ██ ████ ██    ");
    println!("     ██    ██   ██    ██  ██  ██    ");
    println!("███████ ██ ██   ██ ██ ██      ██ ██ ");
    println!("Smart Artificial Mind");
    println!("VERSION: {:?}", VERSION);
    println!("Copyright 2021-2026 The Open Sam Foundation (OSF)");
    println!("Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)");
    println!("Licensed under GPLv3....see LICENSE file.");
    println!("================================================");
    println!("Hello {}....SAM is starting up...", user);
    println!("================================================");

    // Initialize logger with color, warning level, and timestamps
    // simple_logger::SimpleLogger::new()
    //     .with_colors(true)
    //     .with_level(log::LevelFilter::Info)
    //     .with_timestamps(true)
    //     .init()
    //     .unwrap();



    // Optionally set environment variables for libraries (uncomment if needed)
    // env::set_var("LIBTORCH", "/app/libtorch/libtorch");
    // env::set_var("LD_LIBRARY_PATH", "${LIBTORCH}/lib:$LD_LIBRARY_PATH");

    // Ensure required environment variables are available for sudo context
    sudo::with_env(&[
        "LIBTORCH",
        "LD_LIBRARY_PATH",
        "PG_DBNAME",
        "PG_USER",
        "PG_PASS",
        "PG_ADDRESS",
    ])
    .unwrap();


    // Check for missing Postgres credentials and prompt user if missing
    // cli::check_postgres_env();




    // // Optionally escalate privileges if needed
    // // sudo::escalate_if_needed().unwrap();

    // // Run setup/install if required (e.g., on first run or specific path exists)
    // // if Path::new("/opt/sam").exists() {
    // crate::sam::setup::install().await;
    // // }

    // // Initialize configuration and memory
    
  
    if crate::sam::memory::Config::check_postgres_installed() {
        println!("Postgres is already installed.");

        crate::sam::services::pg::start_postgres(user.as_str()).unwrap();

        crate::sam::memory::Config::create_user_and_database(user.as_str()).unwrap();
    } else {
        println!("Installing Postgres...");
        crate::sam::services::pg::install().await;

        // Start Postgres server
        println!("Starting Postgres...");
        crate::sam::services::pg::start_postgres(user.as_str()).unwrap();

        // --- Add Homebrew Postgres bin to PATH if on macOS ---
        #[cfg(target_os = "macos")]
        {

            // Try common Homebrew Postgres bin locations
            let brew_bins = [
                "/usr/local/opt/postgresql@14/bin",
                "/usr/local/opt/postgresql@15/bin",
                "/usr/local/opt/postgresql@16/bin",
                "/usr/local/opt/postgresql/bin",
                "/opt/homebrew/opt/postgresql@14/bin",
                "/opt/homebrew/opt/postgresql@15/bin",
                "/opt/homebrew/opt/postgresql@16/bin",
                "/opt/homebrew/opt/postgresql/bin",
            ];
            let mut new_path = env::var("PATH").unwrap_or_default();
            for bin in brew_bins.iter() {
                if std::path::Path::new(bin).exists() && !new_path.contains(bin) {
                    new_path = format!("{}:{}", bin, new_path);
                }
            }
            env::set_var("PATH", &new_path);
        }
        // -----------------------------------------------------

        crate::sam::memory::Config::create_user_and_database(user.as_str()).unwrap();
        println!("Postgres installation complete.");
    }

    std::env::set_var("PG_DBNAME", "sam");
    std::env::set_var("PG_USER", "sam");
    std::env::set_var("PG_PASS", "sam");
    std::env::set_var("PG_ADDRESS", "localhost");

    // Move blocking code BEFORE any .await
    // Check if /opt/sam/bin/darknet exists before installing
    let darknet_path = Path::new("/opt/sam/bin/darknet");
    if !darknet_path.exists() {
        println!("Darknet binary not found at /opt/sam/bin/darknet. Installing...");
        crate::sam::services::darknet::install().await.unwrap();
    } else {
        println!("Darknet binary found at /opt/sam/bin/darknet. Skipping install.");
    }

     crate::sam::services::docker::install();

     crate::sam::services::tts::init();


    crate::sam::services::crawler::start_service_async().await;

  
    let config = crate::sam::memory::Config::new();
    config.init().await;
    
    // crate::sam::services::crawler::page::CrawledPage::write_most_common_tokens_async(500).await;

   

    // Move blocking code BEFORE any .await
    // crate::sam::services::darknet::install().unwrap();
    // crate::sam::services::docker::install();

    // // // --- Service Initializations ---

    // // // Start WebSocket server for real-time communication
    // // crate::sam::services::socket::init();

    // // // Start RTSP service for streaming
    // // crate::sam::services::rtsp::init();

    // // // Start Speech-to-Text (STT) service
    // // crate::sam::services::stt::init();

    // // // Start sound service for audio output/input
    // // crate::sam::services::sound::init();



    // // // Initialize and sync database with Lifx API (smart lighting)
    // crate::sam::services::lifx::init();

    // // // Start Snapcast server for multi-room audio
    // // crate::sam::services::media::snapcast::init();

    // // // Start storage service for persistent data
    // // crate::sam::services::storage::init();

    // // Experimental: Clean up Dropbox directories (uncomment if needed)
    // // crate::sam::services::dropbox::destroy_empty_directories();
    

    // // Start interactive CLI prompt instead of empty loop
    // // println!("SAM initialized and ready. Starting command prompt...");
    crate::sam::cli::start_prompt().await;

    loop{
        // Check for user input or other events
        // You can add your own logic here to handle commands or events
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // loop {
    //     // Check for user input or other events
    //     // You can add your own logic here to handle commands or events
    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // }
}
