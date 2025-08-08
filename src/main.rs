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
    std::panic::set_hook(Box::new(|_info| {
        // Optionally log to a file or TUI logger instead
        // TODO: Clear redis cache on panic
        // TODO: Shutdown services
    }));
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
        .unwrap();
    }
}

/// Sets up and configures PostgreSQL database
async fn setup_postgres(user: &str) {
    if crate::sam::memory::Config::check_postgres_installed() {
        println!("Postgres is already installed.");
        libsam::services::pg::start_postgres(user).unwrap();
        crate::sam::memory::Config::create_user_and_database(user).unwrap();
    } else {
        install_and_configure_postgres(user).await;
    }
}

/// Installs and configures PostgreSQL for first-time setup
async fn install_and_configure_postgres(user: &str) {
    println!("Installing Postgres...");
    libsam::services::pg::install().await;

    println!("Starting Postgres...");
    libsam::services::pg::start_postgres(user).unwrap();

    if libsam::services::pg::is_postgres_running().await {
        println!("Postgres is running.");
    } else {
        println!("Postgres failed to start.");
    }

    add_postgres_to_path_if_macos();
    crate::sam::memory::Config::create_user_and_database(user).unwrap();
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
