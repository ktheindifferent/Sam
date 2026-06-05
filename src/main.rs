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

use std::env;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

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
        .thread_stack_size(8 * 1024 * 1024) // Increased from 4MB to 8MB to prevent stack overflow
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime")
}

/// Setup dual logging to console and file
fn setup_dual_logger(log_file: &std::path::Path, is_serve_mode: bool) {
    use env_logger::{Builder, Target};

    // Custom writer that writes to both console and file
    struct DualWriter {
        file: Arc<Mutex<std::fs::File>>,
    }

    impl std::io::Write for DualWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            // Write to console
            std::io::stderr().write_all(buf)?;

            // Write to file
            if let Ok(mut file) = self.file.lock() {
                file.write_all(buf)?;
                file.flush()?;
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            std::io::stderr().flush()?;
            if let Ok(mut file) = self.file.lock() {
                file.flush()?;
            }
            Ok(())
        }
    }

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        Ok(f) => Arc::new(Mutex::new(f)),
        Err(e) => {
            eprintln!(
                "Warning: Failed to open log file: {}. Logging to stderr only.",
                e
            );
            // Fall back to stderr-only logging
            if is_serve_mode || env::var("RUST_LOG").is_ok() {
                Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                    .target(Target::Stderr)
                    .init();
            }
            return;
        }
    };

    // Only setup env_logger in serve mode or when explicitly requested
    // In TUI mode, we use tui_logger instead
    if is_serve_mode || env::var("RUST_LOG").is_ok() {
        Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .target(Target::Pipe(Box::new(DualWriter { file })))
            .init();
    }
}

/// Main application initialization logic
async fn initialize_application() {
    // Initialize Sentry for error tracking and monitoring
    let _sentry_guard = sam::monitoring::init_sentry();

    // Initialize logging first
    // Setup logging to both console and file
    let args: Vec<String> = env::args().collect();
    let is_serve_mode = args.len() > 1 && args[1] == "serve";
    let is_doctor_mode = args.len() > 1 && args[1] == "doctor";

    // Doctor mode: run diagnostics and exit immediately
    if is_doctor_mode {
        libsam::cli::commands::doctor::run_doctor().await;
        return;
    }

    // Create .sam directory if it doesn't exist
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sam_dir = std::path::PathBuf::from(home).join(".sam");
    let _ = std::fs::create_dir_all(&sam_dir);

    // Ensure default config exists on first run
    libsam::services::config::SamUserConfig::write_defaults_if_missing();

    // Setup logging to file
    let log_file = sam_dir.join("output.log");

    // Clear the log file at startup
    let _ = std::fs::write(
        &log_file,
        format!("=== SAM Log Started at {} ===\n", chrono::Local::now()),
    );

    // Initialize dual logger (console + file) for all modes
    setup_dual_logger(&log_file, is_serve_mode);

    setup_panic_handler();
    ensure_manifest_dir();

    let user = get_application_user();
    libsam::print_banner(user.clone());

    log::debug!("After banner, before setup_environment_variables");
    setup_environment_variables();
    log::debug!("After setup_environment_variables");

    // Check if we're running in serve mode or CapRover environment (already checked above)
    let is_caprover = env::var("CAPROVER").unwrap_or_default().to_lowercase() == "true";
    let user_config = libsam::services::config::SamUserConfig::load();
    let database_engine = user_config.database_engine();

    log::debug!(
        "Serve mode: {}, CapRover: {}, Database engine: {}",
        is_serve_mode,
        is_caprover,
        database_engine
    );

    if is_serve_mode || is_caprover {
        log::info!(
            "Running in {} mode with database engine: {}",
            if is_caprover { "CapRover" } else { "serve" },
            database_engine
        );
    }

    // Handle database setup based on engine type and mode
    log::debug!("Starting database setup");
    if database_engine == "sqlite" {
        // For SQLite, we don't need PostgreSQL setup
        // Set dummy values for PostgreSQL config to prevent panics
        env::set_var("PG_DBNAME", "dummy");
        env::set_var("PG_USER", "dummy");
        env::set_var("PG_PASS", "dummy");
        env::set_var("PG_ADDRESS", "dummy");
        log::info!("Using SQLite database engine");
    } else if is_serve_mode || is_caprover {
        // In serve/CapRover mode with PostgreSQL, assume external database is configured
        // Don't try to start local PostgreSQL
        log::info!(
            "Using external PostgreSQL database in {}",
            if is_caprover {
                "CapRover mode"
            } else {
                "serve mode"
            }
        );
        // Ensure the environment variables are already set
        if env::var("PG_DBNAME").is_err()
            || env::var("PG_USER").is_err()
            || env::var("PG_PASS").is_err()
            || env::var("PG_ADDRESS").is_err()
        {
            panic!("PostgreSQL environment variables must be set in {}: PG_DBNAME, PG_USER, PG_PASS, PG_ADDRESS",
                   if is_caprover { "CapRover mode" } else { "serve mode" });
        }
        log::debug!("PostgreSQL env vars verified");
    } else {
        // For local development with PostgreSQL, do the full setup
        setup_postgres(&user).await;
        configure_database_connection();
        log::info!("Using local PostgreSQL database");
    }

    log::debug!("Creating Config");
    let config = libsam::memory::Config::new();
    log::debug!("Calling config.init()");
    config.init().await;
    log::debug!("config.init() completed");

    // Start plugin loader if enabled (feature-gated)
    #[cfg(feature = "plugins")]
    {
        if let Some(ref plugin_config) = user_config.plugins {
            if plugin_config.enabled {
                log::info!("Starting plugin loader");
                let loader_config = libsam::services::plugins::loader::PluginLoaderConfig {
                    plugins_dir: plugin_config
                        .plugins_dir
                        .as_ref()
                        .map(|d| std::path::PathBuf::from(d))
                        .unwrap_or_else(|| {
                            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
                            std::path::PathBuf::from(home).join(".sam").join("plugins")
                        }),
                    max_memory_per_plugin: plugin_config.max_memory_per_plugin_mb.unwrap_or(64)
                        * 1024
                        * 1024,
                    fuel_limit: 1_000_000_000,
                    hot_reload: plugin_config.hot_reload.unwrap_or(true),
                };
                let registry = std::sync::Arc::new(tokio::sync::RwLock::new(
                    libsam::services::plugins::PluginRegistry::new(),
                ));
                let loader =
                    libsam::services::plugins::loader::PluginLoader::new(loader_config, registry);
                let _watcher_handle = loader.spawn_watcher();
            }
        }
    }

    // Start notification service if enabled in user config
    if let Some(ref notif_config) = user_config.notifications {
        if notif_config.enabled.unwrap_or(false) {
            log::info!("Starting notification service");
            let _notif_handle =
                libsam::services::notifications::NotificationService::spawn(notif_config.clone());
        }
    }

    if is_serve_mode || is_caprover {
        // In serve/CapRover mode, just run the event loop (HTTP server is started by config.init())
        let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
        log::info!("HTTP server started on port {}", port);
        run_event_loop().await;
    } else {
        // In interactive mode, start the TUI
        libsam::cli::start_prompt().await;
        run_event_loop().await;
    }
}

/// Sets up the panic handler for the application
fn setup_panic_handler() {
    // Use atomic flag to prevent recursive panic handling
    use std::sync::atomic::{AtomicBool, Ordering};
    static PANIC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

    // First set up Sentry's panic handler
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Check if we're already handling a panic to prevent infinite recursion
        if PANIC_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            eprintln!("Recursive panic detected, aborting immediately");
            std::process::abort();
        }

        // Log panic information to stderr immediately
        eprintln!("Application panic occurred: {}", info);

        // Try to log to file first (most reliable)
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/var/log/sam_panic.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] Panic: {}", chrono::Utc::now(), info);
        }

        // Report to Sentry (may fail in stack overflow)
        sentry::capture_event(sentry::protocol::Event {
            message: Some(info.to_string()),
            level: sentry::Level::Fatal,
            ..Default::default()
        });

        // Call the default panic handler (which includes Sentry's own handling)
        default_panic(info);

        // Skip cleanup if this looks like a stack overflow to prevent further issues
        let panic_msg = info.to_string();
        if panic_msg.contains("stack overflow") || panic_msg.contains("overflowed its stack") {
            eprintln!("Stack overflow detected, skipping cleanup to prevent further issues");
            PANIC_IN_PROGRESS.store(false, Ordering::SeqCst);
            return;
        }

        // Try to perform cleanup with timeout
        std::thread::spawn(move || {
            // Set a timeout for cleanup operations
            let cleanup_start = std::time::Instant::now();
            const CLEANUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

            // Use Handle::try_current() to check if we're in a runtime context
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // We're in a runtime context, spawn a task for cleanup
                    handle.spawn(async move {
                        if cleanup_start.elapsed() < CLEANUP_TIMEOUT {
                            // Clear Redis cache
                            if let Err(e) = clear_redis_cache_on_panic().await {
                                eprintln!("Failed to clear Redis cache on panic: {}", e);
                            }
                        }

                        if cleanup_start.elapsed() < CLEANUP_TIMEOUT {
                            // Shutdown all services gracefully
                            if let Err(e) = shutdown_services_on_panic().await {
                                eprintln!("Failed to shutdown services on panic: {}", e);
                            }
                        }

                        PANIC_IN_PROGRESS.store(false, Ordering::SeqCst);
                    });
                }
                Err(_) => {
                    // We're not in a runtime context, try to create a minimal one
                    if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                        .thread_stack_size(1024 * 1024) // Small stack for cleanup
                        .enable_all()
                        .build()
                    {
                        runtime.block_on(async {
                            if cleanup_start.elapsed() < CLEANUP_TIMEOUT {
                                // Clear Redis cache
                                if let Err(e) = clear_redis_cache_on_panic().await {
                                    eprintln!("Failed to clear Redis cache on panic: {}", e);
                                }
                            }

                            if cleanup_start.elapsed() < CLEANUP_TIMEOUT {
                                // Shutdown all services gracefully
                                if let Err(e) = shutdown_services_on_panic().await {
                                    eprintln!("Failed to shutdown services on panic: {}", e);
                                }
                            }
                        });
                    }
                    PANIC_IN_PROGRESS.store(false, Ordering::SeqCst);
                }
            }
        });
    }));
}

/// Clear Redis cache on panic
async fn clear_redis_cache_on_panic() -> Result<(), Box<dyn std::error::Error>> {
    use deadpool_redis::{Config, Runtime};
    // use deadpool_redis::redis::AsyncCommands;

    // Try to connect to Redis
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let cfg = Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    if let Ok(mut conn) = pool.get().await {
        // Clear all keys with a pattern or flush the database
        // Using FLUSHDB to clear the current database
        let _: Result<(), _> = deadpool_redis::redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await;
        log::info!("Redis cache cleared on panic");
    }

    Ok(())
}

/// Shutdown services gracefully on panic
async fn shutdown_services_on_panic() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Shutting down services due to panic...");

    // Shutdown crawler database pool
    sam::services::crawler::shutdown_db_pool().await;

    // Stop crawler service if running
    sam::services::crawler::stop_service();

    // Stop Redis if it was started by us
    sam::services::redis::stop().await;

    // Stop PostgreSQL if needed
    if libsam::cli::commands::pg::is_postgres_running().await {
        libsam::cli::commands::pg::stop().await;
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
    libsam::tools::get_user_from_whois("human").unwrap_or_else(|_| {
        log::error!("Failed to read whoismyhuman file. Defaulting to 'human'.");
        "human".to_string()
    })
}

/// Sets up required environment variables for sudo context
fn setup_environment_variables() {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Skip sudo setup in Docker containers or when running as root
        if env::var("DOCKER_CONTAINER").is_ok() || env::var("CAPROVER").is_ok() {
            log::info!("Running in container, skipping sudo environment setup");
            return;
        }

        // Check if we're running under sudo
        let is_sudo = unsafe { libc::geteuid() } == 0 && env::var("SUDO_USER").is_ok();

        if is_sudo {
            log::info!("Running under sudo, ensuring required environment variables are preserved");

            // Preserve important environment variables that sudo might strip
            // These are typically preserved automatically, but we ensure they're available
            let important_vars = [
                ("LIBTORCH", "Library path for PyTorch"),
                ("LD_LIBRARY_PATH", "Dynamic library search path"),
                ("PG_DBNAME", "PostgreSQL database name"),
                ("PG_USER", "PostgreSQL user"),
                ("PG_PASS", "PostgreSQL password"),
                ("PG_ADDRESS", "PostgreSQL address"),
                ("SAM_USER", "SAM application user"),
                ("HOME", "User home directory"),
                ("USER", "Current user"),
                ("PATH", "System PATH"),
            ];

            for (var_name, description) in important_vars.iter() {
                if let Ok(value) = env::var(var_name) {
                    log::debug!(
                        "Environment variable {} ({}) is set: {}",
                        var_name,
                        description,
                        if var_name.contains("PASS") {
                            "[REDACTED]"
                        } else {
                            &value
                        }
                    );
                } else if var_name != &"LIBTORCH" && var_name != &"SAM_USER" {
                    // These are optional, so only warn for critical ones
                    if ["PG_DBNAME", "PG_USER", "PG_PASS", "PG_ADDRESS"].contains(var_name) {
                        log::warn!(
                            "Critical environment variable {} ({}) not found",
                            var_name,
                            description
                        );
                    }
                }
            }

            // Set default values for missing PostgreSQL variables if needed
            if env::var("PG_DBNAME").is_err() {
                env::set_var("PG_DBNAME", "sam");
                log::debug!("Set default PG_DBNAME=sam");
            }
            if env::var("PG_USER").is_err() {
                env::set_var("PG_USER", "sam");
                log::debug!("Set default PG_USER=sam");
            }
            if env::var("PG_PASS").is_err() {
                env::set_var("PG_PASS", "sam");
                log::debug!("Set default PG_PASS=[REDACTED]");
            }
            if env::var("PG_ADDRESS").is_err() {
                env::set_var("PG_ADDRESS", "localhost");
                log::debug!("Set default PG_ADDRESS=localhost");
            }

            log::info!("Sudo environment setup completed successfully");
        } else {
            log::debug!(
                "Not running under sudo, environment variables should be available normally"
            );
        }
    }
}

/// Sets up and configures PostgreSQL database
async fn setup_postgres(user: &str) {
    if libsam::memory::Config::check_postgres_installed() {
        log::info!("Postgres is already installed.");
        if let Err(e) = libsam::cli::commands::pg::start_postgres(user) {
            log::error!(
                "Failed to start PostgreSQL service: {}. Continuing without PostgreSQL.",
                e
            );
            return;
        }
        if let Err(e) = libsam::memory::Config::create_user_and_database(user) {
            log::error!(
                "Failed to create PostgreSQL user and database: {}. Continuing anyway.",
                e
            );
        }
    } else {
        install_and_configure_postgres(user).await;
    }
}

/// Installs and configures PostgreSQL for first-time setup
async fn install_and_configure_postgres(user: &str) {
    log::info!("Installing Postgres...");
    libsam::cli::commands::pg::install().await;

    log::info!("Starting Postgres...");
    if let Err(e) = libsam::cli::commands::pg::start_postgres(user) {
        log::error!(
            "Failed to start PostgreSQL service during initial setup: {}. Continuing.",
            e
        );
        return;
    }

    if libsam::cli::commands::pg::is_postgres_running().await {
        log::info!("Postgres is running.");
    } else {
        log::warn!("Postgres failed to start.");
    }

    add_postgres_to_path_if_macos();
    if let Err(e) = libsam::memory::Config::create_user_and_database(user) {
        log::error!(
            "Failed to create PostgreSQL user and database during setup: {}. Continuing.",
            e
        );
    }
    log::info!("Postgres installation complete.");
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
