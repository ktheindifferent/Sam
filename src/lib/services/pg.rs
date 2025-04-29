use log::{error, info};
use std::io;
use std::path::Path;
use std::process::Command;

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
        // On Windows, use the 'psql' command to create user and database.
        // Assumes PostgreSQL is installed and 'psql' is in PATH.
        let create_user = Command::new("psql")
            .args(&[
                "-U", "postgres",
                "-c",
                "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'sam') THEN CREATE USER sam WITH PASSWORD 'sam'; END IF; END $$;"
            ])
            .status()?;
        if !create_user.success() {
            log::info!("Warning: Could not create user 'sam' or user already exists.");
        }
        let create_db = Command::new("psql")
            .args(&[
                "-U", "postgres",
                "-c",
                "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_database WHERE datname = 'sam') THEN CREATE DATABASE sam OWNER sam; END IF; END $$;"
            ])
            .status()?;
        if !create_db.success() {
            log::info!("Warning: Could not create database 'sam' or database already exists.");
        }
        log::info!("User and database 'sam' created or already exist.");
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

/// Clone and build PostgreSQL server version 17 from source.
/// This function requires `git`, `make`, `gcc`, and other build tools to be installed.
/// On success, the built binaries will be in the `postgres` directory.
pub fn build_postgres_from_source() -> io::Result<()> {
    // 1. Clone the PostgreSQL 17 source if not already present
    let repo_url = "https://github.com/postgres/postgres.git";
    let dir = "postgres";
    if !Path::new(dir).exists() {
        let status = Command::new("git")
            .args([
                "clone",
                "--branch",
                "REL_17_STABLE",
                "--depth",
                "1",
                repo_url,
                dir,
            ])
            .status()?;
        if !status.success() {
            error!("Failed to clone PostgreSQL source.");
            return Err(io::Error::other("git clone failed"));
        }
        info!("Cloned PostgreSQL 17 source.");
    } else {
        info!("PostgreSQL source directory already exists, skipping clone.");
    }

    // 2. Run ./configure
    let configure_path = format!("{dir}/configure");
    if !Path::new(&configure_path).exists() {
        error!("configure script not found in postgres directory.");
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "configure script missing",
        ));
    }
    let status = Command::new("sh")
        .current_dir(dir)
        .arg("configure")
        .status()?;
    if !status.success() {
        error!("Failed to run configure.");
        return Err(io::Error::other("configure failed"));
    }
    info!("Ran configure.");

    // 3. Run make
    let status = Command::new("make").current_dir(dir).status()?;
    if !status.success() {
        error!("Failed to build PostgreSQL.");
        return Err(io::Error::other("make failed"));
    }
    info!("PostgreSQL built successfully.");

    // 4. Optionally, run make install (requires sudo/root)
    // let status = Command::new("sudo")
    //     .current_dir(dir)
    //     .arg("make")
    //     .arg("install")
    //     .status()?;
    // if !status.success() {
    //     error!("Failed to install PostgreSQL.");
    //     return Err(io::Error::new(io::ErrorKind::Other, "make install failed"));
    // }
    // info!("PostgreSQL installed successfully.");

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install_postgres(_user: &str) -> io::Result<()> {
    // 1. Fetch the EnterpriseDB binaries page
    let url = "https://www.enterprisedb.com/download-postgresql-binaries";
    let resp = get(url).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let body = resp
        .text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 2. Parse the HTML to find the latest Windows x86-64 installer link
    let document = Html::parse_document(&body);
    let selector = Selector::parse("a").unwrap();
    let mut latest_url = None;
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if href.contains("windows-x64.exe") && href.contains("postgresql-") {
                latest_url = Some(href.to_string());
            }
        }
    }
    let latest_url = match latest_url {
        Some(url) => {
            if url.starts_with("http") {
                url
            } else {
                format!("https://www.enterprisedb.com{}", url)
            }
        }
        None => {
            log::error!("Could not find latest PostgreSQL Windows installer link.");
            return Ok(());
        }
    };

    // 3. Download the installer
    let temp_dir = env::temp_dir();
    let installer_path = temp_dir.join("postgres_installer.exe");
    let mut resp = get(&latest_url).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut out = fs::File::create(&installer_path)?;
    resp.copy_to(&mut out)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 4. Run the installer in silent mode
    let install_dir = r"C:\\Program Files\\PostgreSQL\\latest";
    let data_dir = r"C:\\Program Files\\PostgreSQL\\latest\\data";
    let password = "sam_password";
    let install_cmd = format!(
        "\"{}\" --mode unattended --unattendedmodeui minimal --superpassword {} --servicename postgresql-x64-latest --serviceaccount postgres --servicepassword {} --prefix \"{}\" --datadir \"{}\" --serverport 5432",
        installer_path.display(), password, password, install_dir, data_dir
    );
    let status = Command::new("cmd")
        .args(["/C", &install_cmd])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        log::error!("Failed to run PostgreSQL installer.");
        return Ok(());
    }
    log::info!("PostgreSQL installed successfully.");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn install_postgres(_user: &str) -> io::Result<()> {
    // Try apt-get (Debian/Ubuntu)
    let status = Command::new("sudo").arg("apt-get").arg("update").status()?;
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
        // Query SC to check if the service is running
        let output = Command::new("sc")
            .args(&["query", "postgresql-x64-17"]) // Adjust service name as needed
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("RUNNING") {
            println!("PostgreSQL service is already running.");
            return Ok(());
        } else if stdout.contains("STOPPED") {
            // Start the PostgreSQL service
            let status = Command::new("net")
                .args(&["start", "postgresql-x64-17"]) // Adjust service name as needed
                .status()?;
            if !status.success() {
                println!("Warning: Could not start PostgreSQL service on Windows.");
            }
        } else if stdout.contains("NOT FOUND") {
            println!("Warning: Could not find PostgreSQL service on Windows.");
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}

pub async fn is_postgres_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("sc")
            .args(&["query", "postgresql-x64-17"]) // Adjust service name as needed
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("RUNNING");
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(&["is-active", "postgresql"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.trim() == "active";
        }
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("brew").args(["services", "list"]).output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("postgresql") && line.contains("started") {
                    return true;
                }
            }
        }
    }
    false
}

/// Install PostgreSQL using system package manager or Docker (if not installed)
pub async fn install() {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("winget")
            .arg("install")
            .arg("PostgreSQL.PostgreSQL.17")
            .status();
        if let Ok(status) = status {
            if status.success() {
                info!("PostgreSQL installed successfully.");
                return;
            }
        }
        error!("Failed to install PostgreSQL. Please install manually.");
        return;
    }
    #[cfg(target_os = "linux")]
    {
        // Try apt-get (Debian/Ubuntu)
        let status = Command::new("sudo").arg("apt-get").arg("update").status();
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
            .args(&["start", "postgresql-x64-17"]) // Adjust service name as needed
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
        let output = Command::new("brew").args(["services", "list"]).output();
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
