use std::process::Command;
use std::io;
use log::{info, error};

/*
This Rust code provides functions to install and configure PostgreSQL on Windows, Linux, and macOS.
It uses the `std::process::Command` API to run system commands. 
You may need to run your program with administrator/root privileges.

Note: This is a simplified example. For production, use proper error handling and security practices.
*/
pub fn create_sam_user_and_db() -> io::Result<()> {
    // Create user 'sam' and database 'sam'
    // This assumes you have sufficient privileges (e.g., running as postgres or with sudo)
    #[cfg(target_os = "windows")]
    {
        log::info!("Please use pgAdmin or psql to create user/database 'sam'.");
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Create user 'sam'
        let status = Command::new("sudo")
            .arg("-u")
            .arg("postgres")
            .arg("psql")
            .arg("-c")
            .arg("DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'sam') THEN CREATE USER sam WITH PASSWORD 'sam'; END IF; END $$;")
            .status()?;
        if !status.success() {
            log::info!("Warning: Could not create user 'sam' or user already exists.");
        }
        // Create database 'sam' owned by 'sam'
        let status = Command::new("sudo")
            .arg("-u")
            .arg("postgres")
            .arg("psql")
            .arg("-c")
            .arg("DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_database WHERE datname = 'sam') THEN CREATE DATABASE sam OWNER sam; END IF; END $$;")
            .status()?;
        if !status.success() {
            log::info!("Warning: Could not create database 'sam' or database already exists.");
        }
        log::info!("User and database 'sam' created or already exist.");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install_postgres(_user: &str) -> io::Result<()> {
    log::info!("Please download and run the PostgreSQL installer from https://www.postgresql.org/download/windows/");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn install_postgres(_user: &str) -> io::Result<()> {
    // Try apt-get (Debian/Ubuntu)
    let status = Command::new("sudo")
        .arg("apt-get")
        .arg("update")
        .status()?;
    if status.success() {
        let status = Command::new("sudo")
            .arg("apt-get")
            .arg("install")
            .arg("-y")
            .arg("postgresql")
            .status()?;
        if status.success() {
            log::info!("PostgreSQL installed successfully.");
        }
    } else {
        // Try yum (Fedora/CentOS)
        let status = Command::new("sudo")
            .arg("yum")
            .arg("install")
            .arg("-y")
            .arg("postgresql-server")
            .status()?;
        if status.success() {
            log::info!("PostgreSQL installed successfully.");
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn install_postgres(user: &str) -> io::Result<()> {
    // Use Homebrew
    let status = Command::new("sudo")
        .arg("-u")
        .arg(user)
        .arg("brew")
        .arg("install")
        .arg("postgresql")
        .status()?;
    if status.success() {
        log::info!("PostgreSQL installed successfully.");
    }
    Ok(())
}

pub fn start_postgres(user: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Try to start the Postgres server if it's not running (macOS Homebrew typical path)
        let status = Command::new("sudo")
            .args(["-u", user, "brew", "services", "start", "postgresql"])
            .status()?;
        if !status.success() {
            log::info!("Warning: Could not start PostgreSQL service on macOS.");
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    #[cfg(target_os = "linux")]
    {
        // Try to start the Postgres server using systemctl (common on Linux)
        let status = Command::new("sudo")
            .args(&["systemctl", "start", "postgresql"])
            .status()?;
        if !status.success() {
            log::info!("Warning: Could not start PostgreSQL service on Linux.");
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    #[cfg(target_os = "windows")]
    {
        // Try to start the Postgres server using Windows Service Control
        let status = Command::new("net")
            .args(&["start", "postgresql-x64-15"]) // Adjust service name as needed
            .status()?;
        if !status.success() {
            log::info!("Warning: Could not start PostgreSQL service on Windows.");
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}

/// Install PostgreSQL using system package manager or Docker (if not installed)
pub async fn install() {
    #[cfg(target_os = "windows")]
    {
        info!("Please download and run the PostgreSQL installer from https://www.postgresql.org/download/windows/");
        return;
    }
    #[cfg(target_os = "linux")]
    {
        // Try apt-get (Debian/Ubuntu)
        let status = Command::new("sudo")
            .arg("apt-get")
            .arg("update")
            .status();
        if let Ok(status) = status {
            if status.success() {
                let status = Command::new("sudo")
                    .arg("apt-get")
                    .arg("install")
                    .arg("-y")
                    .arg("postgresql")
                    .status();
                if let Ok(status) = status {
                    if status.success() {
                        info!("PostgreSQL installed successfully.");
                        return;
                    }
                }
            }
        }
        // Try yum (Fedora/CentOS)
        let status = Command::new("sudo")
            .arg("yum")
            .arg("install")
            .arg("-y")
            .arg("postgresql-server")
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL installed successfully.");
                return;
            }
        }
        error!("Failed to install PostgreSQL. Please install manually.");
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("brew")
            .arg("install")
            .arg("postgresql")
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL installed successfully.");
                return;
            }
        }
        error!("Failed to install PostgreSQL. Please install manually.");
    }
}

/// Start the PostgreSQL service/daemon
pub async fn start() {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("net")
            .args(&["start", "postgresql-x64-15"]) // Adjust service name as needed
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service started.");
                return;
            }
        }
        error!("Could not start PostgreSQL service on Windows.");
    }
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("sudo")
            .args(&["systemctl", "start", "postgresql"])
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service started.");
                return;
            }
        }
        error!("Could not start PostgreSQL service on Linux.");
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("brew")
            .args(["services", "start", "postgresql"])
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service started.");
                return;
            }
        }
        error!("Could not start PostgreSQL service on macOS.");
    }
}

/// Stop the PostgreSQL service/daemon
pub async fn stop() {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("net")
            .args(&["stop", "postgresql-x64-15"]) // Adjust service name as needed
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service stopped.");
                return;
            }
        }
        error!("Could not stop PostgreSQL service on Windows.");
    }
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("sudo")
            .args(&["systemctl", "stop", "postgresql"])
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service stopped.");
                return;
            }
        }
        error!("Could not stop PostgreSQL service on Linux.");
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("brew")
            .args(["services", "stop", "postgresql"])
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL service stopped.");
                return;
            }
        }
        error!("Could not stop PostgreSQL service on macOS.");
    }
}

/// Return the status of the PostgreSQL service: "running", "stopped", or "not installed"
pub fn status() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("sc")
            .args(&["query", "postgresql-x64-15"]) // Adjust service name as needed
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("RUNNING") {
                return "running";
            } else if stdout.contains("STOPPED") {
                return "stopped";
            } else {
                return "not installed";
            }
        }
        return "not installed";
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(&["is-active", "postgresql"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "active" {
                return "running";
            } else if stdout.trim() == "inactive" || stdout.trim() == "failed" {
                return "stopped";
            } else {
                return "not installed";
            }
        }
        return "not installed";
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("brew")
            .args(["services", "list"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("postgresql") {
                    if line.contains("started") {
                        return "running";
                    } else if line.contains("stopped") {
                        return "stopped";
                    } else {
                        return "not installed";
                    }
                }
            }
            return "not installed";
        }
        return "not installed";
    }
    #[allow(unreachable_code)]
    "not installed"
}
