// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// TODO:
// 1. Finish implementing SMS support.
// 2. Extend CLI
//  - move http to services
//  - add more commands
//  - add more options
//  - add more help
//  - add more error handling
//  - add more tests
// 3. Add support for other notification services (e.g., email, push notifications).
// 4. Implement a notification history feature.
// 5. Add a user interface for managing notification settings.
// 6. Finish revising database structure
// 7. Make cache databases redis/postgres hybrid
// 8. Add support for different database backends (e.g., SQLite, MySQL).
// 9. Create an oid for SAM on server startup if one does not exist....make sure only root can access it.
// 10. Add support for different storage backends (e.g., S3, Google Cloud Storage).
// 11. Implement a backup and restore feature for the database.
// 12. Whisper.cpp support
// 13. Bootcamp service that uses list of common prompts, collected prompts + data to train new models.
// 14. Revise default rivescript with bootcamp prompts.
// 15. Extend thing support to include more devices and platforms.
// 16. GUI+API overhaul!!!
// 17. Mobile app
// 18. Data goblin apps (recipie, shopping list, calendar, cat identification, etc.)
pub mod sam;

// External crates
// extern crate hound;
// extern crate postgres;
// extern crate threadpool;
// extern crate wikipedia;
// #[macro_use]
// extern crate lazy_static;
// #[macro_use]
// extern crate log;
// use tui_logger;

use std::env;

// Store application version as a const, set at compile time
// const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Main entry point for the SAM application.
/// Initializes logging, environment variables, configuration, and all core services.
fn main() {
    let num_workers = num_cpus::get().max(4) + 2; // Use at least 4 threads, or your CPU count
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_workers)
        .thread_name("sam")
        .thread_stack_size(4 * 1024 * 1024) // 4MB stack size
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    runtime.block_on(async {
      
        std::panic::set_hook(Box::new(|_info| {
            // Optionally log to a file or TUI logger instead
            // e.g., log::error!("Panic: {:?}", info);
            // Do nothing to suppress terminal output
            // TODO: Clear redis cache on panic
            // TODO: Shutdown services
            // crate::sam::services::redis::clear_cache();
        }));

        // Ensure CARGO_MANIFEST_DIR is set; if not, set it to the current directory
        if std::env::var("CARGO_MANIFEST_DIR").is_err() {
            if let Ok(current_dir) = std::env::current_dir() {
                if let Some(dir_str) = current_dir.to_str() {
                    std::env::set_var("CARGO_MANIFEST_DIR", dir_str);
                }
            }
        }

        // Attempt to read username from /opt/sam/whoismyhuman if it exists
        let user = crate::sam::tools::get_user_from_whois("human").unwrap_or_else(|_| {
            log::error!("Failed to read whoismyhuman file. Defaulting to 'human'.");
            "human".to_string()
        });

        libsam::print_banner(user.clone());

        // Ensure required environment variables are available for sudo context
        // dependent on OS
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            sudo::with_env(&[
                "LIBTORCH",
                "LD_LIBRARY_PATH",
                "PG_DBNAME",
                "PG_USER",
                "PG_PASS",
                "PG_ADDRESS",
                "SAM_USER",
            ])
            .unwrap();
        }

        

        // Optionally escalate privileges if needed
        // sudo::escalate_if_needed().unwrap();

        // // Initialize configuration and memory
        if crate::sam::memory::Config::check_postgres_installed() {
            println!("Postgres is already installed.");

            libsam::services::pg::start_postgres(user.as_str()).unwrap();

            crate::sam::memory::Config::create_user_and_database(user.as_str()).unwrap();
        } else {
            println!("Installing Postgres...");
            libsam::services::pg::install().await;

            // Start Postgres server
            println!("Starting Postgres...");
            libsam::services::pg::start_postgres(user.as_str()).unwrap();
            // Check if Postgres is running
            if libsam::services::pg::is_postgres_running().await {
                println!("Postgres is running.");
            } else {
                println!("Postgres failed to start.");
            }
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
                        new_path = format!("{bin}:{new_path}");
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

        // crate::sam::services::docker::install(); // This returns a Future, must be awaited or handled
        // let _ = crate::sam::services::docker::install(); // If install() returns Result or Future, handle appropriately

        // crate::sam::services::tts::init();

        // crate::sam::services::crawler::start_service_async().await;

        // if let Err(e) = crate::sam::services::mdns::MDns::init().await {
        //     println!("[mDNS] Failed to initialize: {}", e);
        // }

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

        loop {
            // Check for user input or other events
            // You can add your own logic here to handle commands or events
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
