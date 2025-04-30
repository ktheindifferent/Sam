use std::env;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{self, AsyncWriteExt};
use tokio::process::Command;

// TODO - Automatically apply security settings and config
// /etc/snapserver.conf
pub async fn configure() {
    let cfg = "[server]
    threads = -1
    pidfile = /var/run/snapserver/pid

    [http]
    enabled = true
    bind_to_address = 0.0.0.0
    port = 1780
    doc_root = /usr/share/snapserver/snapweb

    [tcp]
    enabled = true
    bind_to_address = 0.0.0.0
    port = 1705

    [stream]
    bind_to_address = 0.0.0.0
    port = 1704
    source = librespot:///bin/librespot?name=Sam&username=calebsmithdev&password=Nofear1234&devicename=Sam&bitrate=320&nomalize=true
    source = pipe:///tmp/snapfifo?name=samfifo
    [logging]".to_string();
    log::info!("cfg: {:?}", cfg);
    tokio::fs::write("/etc/snapserver.conf", &cfg)
        .await
        .expect("Unable to write file");
}

// Only one install() definition per compilation
pub async fn install() -> io::Result<()> {
    // Determine the user: from SAM_USER env var or from /opt/sam/whoismyhuman file
    let user = if let Ok(val) = env::var("SAM_USER") {
        val
    } else if let Ok(contents) = tokio::fs::read_to_string("/opt/sam/whoismyhuman").await {
        contents.trim().to_string()
    } else {
        "unknown".to_string()
    };

    #[cfg(target_os = "macos")]
    {
        // Check if Homebrew is installed
        let brew_status = Command::new("which").arg("brew").status().await?;

        if brew_status.success() {
            log::info!("Attempting to install Snapcast via Homebrew...");
            let brew_status = Command::new("sudo")
                .arg("-u")
                .arg(user)
                .arg("brew")
                .arg("install")
                .arg("snapcast")
                .status()
                .await;

            if let Ok(status) = &brew_status {
                if status.success() {
                    log::info!("Snapcast installed via Homebrew.");
                    return Ok(());
                } else {
                    log::warn!("Homebrew install failed, falling back to source build.");
                }
            } else {
                log::warn!(
                    "Homebrew not available or failed to run, falling back to source build."
                );
            }
        } else {
            log::warn!("Homebrew is not installed. Will attempt to build snapcast from source. Please install homebrew if this installer fails to build snapcast from source.");
            return Err(io::Error::other("Homebrew not installed"));
        }
    }

    // Fallback: build from source
    log::info!("Starting Snapcast install from source...");
    let home_dir = env::var("HOME").unwrap_or("/tmp".to_string());
    let src_dir = format!("{home_dir}/snapcast-src");

    // Clone or pull latest Snapcast
    if Path::new(&src_dir).exists() {
        log::info!("Snapcast source exists, pulling latest...");
        let _ = Command::new("git")
            .arg("-C")
            .arg(&src_dir)
            .arg("pull")
            .status()
            .await;
    } else {
        log::info!("Cloning Snapcast source...");
        let _ = Command::new("git")
            .arg("clone")
            .arg("https://github.com/badaix/snapcast.git")
            .arg(&src_dir)
            .status()
            .await;
    }

    // Create build directory
    let build_dir = format!("{src_dir}/build");
    if !Path::new(&build_dir).exists() {
        let _ = tokio::fs::create_dir_all(&build_dir).await;
    }

    // Run CMake
    log::info!("Configuring Snapcast with CMake...");
    let mut cmake_cmd = Command::new("cmake");
    cmake_cmd.current_dir(&build_dir).arg("..");
    // #[cfg(target_os = "windows")]
    // {
        // Include boost include directory for Windows
        // Include vcpkg 
        // CMakeLists.txt does not respect CMAKE_TOOLCHAIN_FILE, so we need to set CMAKE_PREFIX_PATH
        // cmake_cmd.arg("-DCMAKE_PREFIX_PATH=C:/vcpkg/installed/x64-windows");
        // cmake_cmd.arg("-DCMAKE_TOOLCHAIN_FILE=C:/local/vcpkg/scripts/buildsystems/vcpkg.cmake");
    // }
    #[cfg(target_os = "windows")]
    {
        let vcpkg_prefix = "C:/vcpkg/installed/x64-windows";
        cmake_cmd.env("BOOST_INCLUDEDIR", "C:/local/boost_1_87_0");
        cmake_cmd.env("CMAKE_PREFIX_PATH", vcpkg_prefix);
        cmake_cmd.env("CMAKE_INCLUDE_PATH", format!("{}/include", vcpkg_prefix));
        cmake_cmd.env("CMAKE_LIBRARY_PATH", format!("{}/lib", vcpkg_prefix));
        cmake_cmd.env("CMAKE_TOOLCHAIN", "C:/vcpkg/scripts/buildsystems/vcpkg.cmake");
        cmake_cmd.arg("-G").arg("MinGW Makefiles");

        // Patch wasapi_player.cpp for MinGW before building Snapcast
        #[cfg(target_os = "windows")]
        log::info!("Patching wasapi_player.cpp for MinGW...");
        let _ = patch_wasapi_player_for_mingw();
    }

    let cmake_status = cmake_cmd.status().await;
    if !cmake_status.map(|s| s.success()).unwrap_or(false) {
        log::error!("CMake configuration failed");
        return Err(io::Error::other("CMake failed"));
    }

    // Check for Makefile before running make
    let makefile_path = format!("{}/Makefile", build_dir);
    if !Path::new(&makefile_path).exists() {
        log::error!("Makefile not found after CMake. Aborting build.");
        return Err(io::Error::other("Makefile not found after CMake"));
    }

    // Run Make
    log::info!("Building Snapcast...");
    let make_status = Command::new("make").current_dir(&build_dir).status().await;
    if !make_status.map(|s| s.success()).unwrap_or(false) {
        log::error!("Make build failed");
        return Err(io::Error::other("Make failed"));
    }

    // Install (may require sudo)
    log::info!("Installing Snapcast...");
    #[cfg(target_os = "windows")]
    let install_status = Command::new("make")
        .arg("install")
        .current_dir(&build_dir)
        .status()
        .await;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let install_status = Command::new("sudo")
        .arg("make")
        .arg("install")
        .current_dir(&build_dir)
        .status()
        .await;
    if !install_status.map(|s| s.success()).unwrap_or(false) {
        log::error!("Make install failed");
        return Err(io::Error::other("Make install failed"));
    }

    log::info!("Snapcast installed successfully.");

    // Copy binaries to /opt/sam/bin
    let bin_dir = "/opt/sam/bin";
    if !Path::new(bin_dir).exists() {
        let _ = tokio::fs::create_dir_all(bin_dir).await;
    }
    let snapserver_src = format!("{build_dir}/snapserver");
    let snapclient_src = format!("{build_dir}/snapclient");
    let snapserver_dst = format!("{bin_dir}/snapserver");
    let snapclient_dst = format!("{bin_dir}/snapclient");
    if Path::new(&snapserver_src).exists() {
        let _ = tokio::fs::copy(&snapserver_src, &snapserver_dst).await;
        log::info!("snapserver copied to /opt/sam/bin");
    }
    if Path::new(&snapclient_src).exists() {
        let _ = tokio::fs::copy(&snapclient_src, &snapclient_dst).await;
        log::info!("snapclient copied to /opt/sam/bin");
    }
    Ok(())
}

pub async fn install_snapcast_server(data: &[u8]) -> io::Result<()> {
    let mut buffer = File::create("/opt/sam/tmp/snapserver.deb").await?;
    buffer.write_all(data).await?;

    let _ = crate::cmd_async("dpkg --force-all -i /opt/sam/tmp/snapserver.deb").await?;
    let _ = crate::cmd_async("service snapserver start").await?;
    Ok(())
}


#[cfg(target_os = "windows")]
pub fn patch_wasapi_player_for_mingw() -> std::io::Result<()> {
    use std::fs;
    use std::path::Path;
    let file = "C:/tmp/snapcast-src/client/player/wasapi_player.cpp";
    let content = fs::read_to_string(file)?;
    let patched = content.replace(
        "const IID IID_IAudioEndpointVolume = _uuidof(IAudioEndpointVolume);",
        "const IID IID_IAudioEndpointVolume = {0x5CDF2C82,0x841E,0x4546,{0x97,0x22,0x0C,0xF7,0x08,0xE1,0x0F,0xA9}}; // patched for MinGW"
    );
    fs::write(file, patched)?;
    Ok(())
}
