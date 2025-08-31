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
    let runtime = build_tokio_runtime();
    runtime.block_on(async {
        initialize_application().await;
    });
}

/// Builds and configures the Tokio runtime
fn build_tokio_runtime() -> tokio::runtime::Runtime {
    let num_workers = num_cpus::get().max(4) + 2;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_workers)
        .thread_name("sam")
        .thread_stack_size(4 * 1024 * 1024)
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime")
}

/// Main application initialization logic
async fn initialize_application() {
    setup_panic_handler();
    ensure_manifest_dir();

    let user = get_application_user();
    libsam::print_banner(user.clone());

    setup_environment_variables();
    setup_postgres(&user).await;
    configure_database_connection();

    let config = crate::sam::memory::Config::new();
    config.init().await;

    crate::sam::cli::start_prompt().await;
    run_event_loop().await;
}

/// Sets up the panic handler for the application
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        // Log panic information
        log::error!("Application panic occurred: {}", info);
        
        // Clear Redis cache on panic
        if let Ok(runtime) = tokio::runtime::Runtime::new() {
            runtime.block_on(async {
                // Clear Redis cache
                if let Err(e) = clear_redis_cache_on_panic().await {
                    log::error!("Failed to clear Redis cache on panic: {}", e);
                }
                
                // Shutdown all services gracefully
                if let Err(e) = shutdown_services_on_panic().await {
                    log::error!("Failed to shutdown services on panic: {}", e);
                }
            });
        }
        
        // Optionally log to a file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/var/log/sam_panic.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] Panic: {}", chrono::Utc::now(), info);
        }
    }));
}

/// Clear Redis cache on panic
async fn clear_redis_cache_on_panic() -> Result<(), Box<dyn std::error::Error>> {
    use deadpool_redis::{Config, Runtime};
    use redis::AsyncCommands;
    
    // Try to connect to Redis
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    let cfg = Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    
    if let Ok(mut conn) = pool.get().await {
        // Clear all keys with a pattern or flush the database
        // Using FLUSHDB to clear the current database
        let _: Result<String, _> = conn.flushdb(false).await;
        log::info!("Redis cache cleared on panic");
    }
    
    Ok(())
}

/// Shutdown services gracefully on panic
async fn shutdown_services_on_panic() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Shutting down services due to panic...");
    
    // Shutdown crawler database pool
    libsam::services::crawler::shutdown_db_pool().await;
    
    // Stop crawler service if running
    if let Err(e) = libsam::services::crawler::stop_service().await {
        log::error!("Failed to stop crawler service: {}", e);
    }
    
    // Stop Redis if it was started by us
    libsam::services::redis::stop().await;
    
    // Stop PostgreSQL if needed
    if libsam::services::pg::is_postgres_running().await {
        if let Err(e) = libsam::services::pg::stop_postgres() {
            log::error!("Failed to stop PostgreSQL: {}", e);
        }
    }
    
    // Add any other service shutdowns here
    log::info!("All services shut down");
    
    Ok(())
}

/// Ensures CARGO_MANIFEST_DIR environment variable is set
fn ensure_manifest_dir() {
    if std::env::var("CARGO_MANIFEST_DIR").is_err() {
        if let Ok(current_dir) = std::env::current_dir() {
            if let Some(dir_str) = current_dir.to_str() {
                std::env::set_var("CARGO_MANIFEST_DIR", dir_str);
            }
        }
    }
}

/// Gets the application user from the whois file or defaults to "human"
fn get_application_user() -> String {
    crate::sam::tools::get_user_from_whois("human").unwrap_or_else(|_| {
        log::error!("Failed to read whoismyhuman file. Defaulting to 'human'.");
        "human".to_string()
    })
}

/// Sets up required environment variables for sudo context
fn setup_environment_variables() {
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
        .expect("Failed to set up sudo environment variables");
    }
}

/// Sets up and configures PostgreSQL database
async fn setup_postgres(user: &str) {
    if crate::sam::memory::Config::check_postgres_installed() {
        println!("Postgres is already installed.");
        libsam::services::pg::start_postgres(user).expect("Failed to start PostgreSQL service");
        crate::sam::memory::Config::create_user_and_database(user).expect("Failed to create PostgreSQL user and database");
    } else {
        install_and_configure_postgres(user).await;
    }
}

/// Installs and configures PostgreSQL for first-time setup
async fn install_and_configure_postgres(user: &str) {
    println!("Installing Postgres...");
    libsam::services::pg::install().await;

    println!("Starting Postgres...");
    libsam::services::pg::start_postgres(user).expect("Failed to start PostgreSQL service during initial setup");

    if libsam::services::pg::is_postgres_running().await {
        println!("Postgres is running.");
    } else {
        println!("Postgres failed to start.");
    }

    add_postgres_to_path_if_macos();
    crate::sam::memory::Config::create_user_and_database(user).expect("Failed to create PostgreSQL user and database during setup");
    println!("Postgres installation complete.");
}

/// Adds Homebrew PostgreSQL binary paths to PATH on macOS
#[cfg(target_os = "macos")]
fn add_postgres_to_path_if_macos() {
    const BREW_POSTGRES_PATHS: &[&str] = &[
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
    for bin_path in BREW_POSTGRES_PATHS {
        if std::path::Path::new(bin_path).exists() && !new_path.contains(bin_path) {
            new_path = format!("{}:{}", bin_path, new_path);
        }
    }
    env::set_var("PATH", &new_path);
}

#[cfg(not(target_os = "macos"))]
fn add_postgres_to_path_if_macos() {
    // No-op on non-macOS platforms
}

/// Configures database connection environment variables
fn configure_database_connection() {
    std::env::set_var("PG_DBNAME", "sam");
    std::env::set_var("PG_USER", "sam");
    std::env::set_var("PG_PASS", "sam");
    std::env::set_var("PG_ADDRESS", "localhost");
}

/// Runs the main event loop
async fn run_event_loop() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
