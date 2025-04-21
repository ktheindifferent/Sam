use std::process::Command;
use std::io;

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
        println!("Please use pgAdmin or psql to create user/database 'sam'.");
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
            println!("Warning: Could not create user 'sam' or user already exists.");
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
            println!("Warning: Could not create database 'sam' or database already exists.");
        }
        println!("User and database 'sam' created or already exist.");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install_postgres(_user: &str) -> io::Result<()> {
    println!("Please download and run the PostgreSQL installer from https://www.postgresql.org/download/windows/");
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
            println!("PostgreSQL installed successfully.");
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
            println!("PostgreSQL installed successfully.");
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
        println!("PostgreSQL installed successfully.");
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
            println!("Warning: Could not start PostgreSQL service on macOS.");
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
            println!("Warning: Could not start PostgreSQL service on Linux.");
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
            println!("Warning: Could not start PostgreSQL service on Windows.");
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}
