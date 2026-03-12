use std::sync::Arc;
use tokio::sync::Mutex;

/// Run diagnostics and output a checklist
pub async fn run_doctor() {
    println!("SAM Doctor - System Diagnostics");
    println!("================================\n");

    // Show database engine
    let user_config = crate::services::config::SamUserConfig::load();
    let db_engine = user_config.database_engine();
    println!("  Database Engine ...... {}", db_engine);
    println!();

    // Check PostgreSQL (skip if using SQLite)
    if db_engine == "postgres" {
        print!("  PostgreSQL ........... ");
        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            crate::services::pg::health_check(),
        ).await {
            Ok(Ok(_)) => println!("✅ Connected"),
            Ok(Err(e)) => println!("❌ {}", e),
            Err(_) => println!("⏰ Timeout"),
        }
    } else {
        println!("  PostgreSQL ........... ⏭  Skipped (engine={})", db_engine);
    }

    // Check Redis
    print!("  Redis ................ ");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        crate::services::redis::status(),
    ).await {
        Ok(status) => {
            if status == "running" || status == "connected" {
                println!("✅ {}", status);
            } else {
                println!("❌ {}", status);
            }
        }
        Err(_) => println!("⏰ Timeout"),
    }

    // Check Docker
    print!("  Docker ............... ");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        crate::services::docker::is_running_async(),
    ).await {
        Ok(Ok(true)) => println!("✅ Running"),
        Ok(Ok(false)) => println!("❌ Stopped"),
        Ok(Err(e)) => println!("❌ {}", e),
        Err(_) => println!("⏰ Timeout"),
    }

    // Check Ollama
    print!("  Ollama ............... ");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        async {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            if service.is_installed().await {
                if service.is_running().await { "running" } else { "stopped" }
            } else {
                "not installed"
            }
        },
    ).await {
        Ok("running") => println!("✅ Running"),
        Ok("stopped") => println!("⚠️  Stopped (installed)"),
        Ok("not installed") => println!("❌ Not installed"),
        Ok(s) => println!("❌ {}", s),
        Err(_) => println!("⏰ Timeout"),
    }

    // Check ports
    println!("\n  Port Availability:");
    for (port, name) in [(8000, "HTTP"), (8080, "WebSocket"), (2222, "SSH Server")] {
        print!("    {} ({}) ...... ", name, port);
        match std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(_) => println!("✅ Available"),
            Err(_) => println!("⚠️  In use (may be SAM)"),
        }
    }

    // Check environment variables
    println!("\n  Environment Variables:");
    for var in ["PG_DBNAME", "PG_USER", "PG_ADDRESS", "REDIS_URL"] {
        print!("    {} ...... ", var);
        match std::env::var(var) {
            Ok(val) => {
                if var.contains("PASS") {
                    println!("✅ Set [REDACTED]");
                } else {
                    println!("✅ {}", val);
                }
            }
            Err(_) => println!("⚠️  Not set"),
        }
    }

    // Check disk space
    println!("\n  Disk Space:");
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let total = disk.total_space();
        let available = disk.available_space();
        if total > 0 {
            let used_pct = ((total - available) as f64 / total as f64) * 100.0;
            let avail_gb = available as f64 / (1024.0 * 1024.0 * 1024.0);
            let mount = disk.mount_point().display();
            if used_pct > 90.0 {
                println!("    {} ...... ❌ {:.1}% used ({:.1} GB free)", mount, used_pct, avail_gb);
            } else if used_pct > 80.0 {
                println!("    {} ...... ⚠️  {:.1}% used ({:.1} GB free)", mount, used_pct, avail_gb);
            } else {
                println!("    {} ...... ✅ {:.1}% used ({:.1} GB free)", mount, used_pct, avail_gb);
            }
        }
    }

    println!("\n================================");
    println!("Diagnostics complete.");
}
