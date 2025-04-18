// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

//! # Smart Artificial Mind (SAM) Main Entry Point
//!
//! This is the main executable for the SAM project. It initializes logging, environment variables,
//! configuration, and all core services (websocket, RTSP, STT, sound, Lifx, Snapcast, storage).
//! 
//! ## Modules
//! - `sam`: Core SAM logic and services.

pub mod sam;
pub mod cli; // New CLI module


// External crates
extern crate wikipedia;
extern crate hound;
extern crate postgres;
extern crate threadpool;

// Store application version as a const, set at compile time
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Main entry point for the SAM application.
/// Initializes logging, environment variables, configuration, and all core services.
#[tokio::main]
async fn main() {
    // Print ASCII art banner and version
    println!("███████     █████     ███    ███    ");
    println!("██         ██   ██    ████  ████    ");
    println!("███████    ███████    ██ ████ ██    ");
    println!("     ██    ██   ██    ██  ██  ██    ");
    println!("███████ ██ ██   ██ ██ ██      ██ ██ ");
    println!("Smart Artificial Mind");
    println!("VERSION: {:?}", VERSION);

    // Initialize logger with color, warning level, and timestamps
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Warn)
        .with_timestamps(true)
        .init()
        .unwrap();

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
    cli::check_postgres_env();


    // Optionally escalate privileges if needed
    // sudo::escalate_if_needed().unwrap();

    // Run setup/install if required (e.g., on first run or specific path exists)
    // if Path::new("/opt/sam").exists() {
    crate::sam::setup::install().await;
    // }

    // Initialize configuration and memory
    let config = crate::sam::memory::Config::new();
    config.init().await;

    // --- Service Initializations ---

    // Start WebSocket server for real-time communication
    crate::sam::services::socket::init();

    // Start RTSP service for streaming
    crate::sam::services::rtsp::init();

    // Start Speech-to-Text (STT) service
    crate::sam::services::stt::init();

    // Start sound service for audio output/input
    crate::sam::services::sound::init();

    // Initialize and sync database with Lifx API (smart lighting)
    crate::sam::services::lifx::init();

    // Start Snapcast server for multi-room audio
    crate::sam::services::media::snapcast::init();

    // Start storage service for persistent data
    crate::sam::services::storage::init();

    // Experimental: Clean up Dropbox directories (uncomment if needed)
    // crate::sam::services::dropbox::destroy_empty_directories();

    // Start interactive CLI prompt instead of empty loop
    println!("SAM initialized and ready. Starting command prompt...");
    cli::start_prompt().await;
}
