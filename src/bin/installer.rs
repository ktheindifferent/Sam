use std::env;
use std::process::{Command, Stdio};
use std::fs;
use std::path::Path;
use std::io::{self, Write};

#[cfg(target_os = "windows")]
const OS: &str = "windows";
#[cfg(target_os = "macos")]
const OS: &str = "macos";
#[cfg(target_os = "linux")]
const OS: &str = "linux";

fn main() {
    let home = dirs::home_dir().expect("Could not get home directory");
    let home_str = home.to_str().unwrap();
    let cmake_version = "3.31.7";
    let build_dir = format!("{}/libtorch_build", home_str);
    let cmake_dir = format!("{}/cmake-{}", home_str, cmake_version);

    // Platform-specific CMake URLs
    let (cmake_url, cmake_archive, cmake_extract_dir) = match OS {
        "macos" => (
            format!("https://github.com/Kitware/CMake/releases/download/v{}/cmake-{}-macos-universal.tar.gz", cmake_version, cmake_version),
            format!("cmake-{}-macos-universal.tar.gz", cmake_version),
            format!("{}/cmake-{}-macos-universal", home_str, cmake_version),
        ),
        "linux" => (
            format!("https://github.com/Kitware/CMake/releases/download/v{}/cmake-{}-linux-x86_64.tar.gz", cmake_version, cmake_version),
            format!("cmake-{}-linux-x86_64.tar.gz", cmake_version),
            format!("{}/cmake-{}-linux-x86_64", home_str, cmake_version),
        ),
        "windows" => (
            format!("https://github.com/Kitware/CMake/releases/download/v{}/cmake-{}-windows-x86_64.zip", cmake_version, cmake_version),
            format!("cmake-{}-windows-x86_64.zip", cmake_version),
            format!("{}/cmake-{}-windows-x86_64", home_str, cmake_version),
        ),
        _ => panic!("Unsupported OS"),
    };

    // Download and extract CMake if not present
    if !Path::new(&cmake_dir).exists() {
        let archive_path = format!("{}/{}", home_str, cmake_archive);
        if !Path::new(&archive_path).exists() {
            println!("Downloading CMake {} for {}...", cmake_version, OS);
            let status = Command::new("curl")
                .args(["-L", &cmake_url, "-o", &archive_path])
                .status().expect("Failed to run curl");
            assert!(status.success(), "curl failed");
        }
        println!("Extracting CMake {}...", cmake_version);
        match OS {
            "windows" => {
                let status = Command::new("powershell")
                    .args(["-Command", &format!("Expand-Archive -Path '{}' -DestinationPath '{}'", archive_path, home_str)])
                    .status().expect("Failed to extract zip");
                assert!(status.success(), "Expand-Archive failed");
                fs::rename(&cmake_extract_dir, &cmake_dir).expect("Failed to move extracted CMake");
            },
            _ => {
                let status = Command::new("tar")
                    .args(["-xzf", &archive_path, "-C", home_str])
                    .status().expect("Failed to run tar");
                assert!(status.success(), "tar failed");
                fs::rename(&cmake_extract_dir, &cmake_dir).expect("Failed to move extracted CMake");
            }
        }
    }
    let cmake_bin = match OS {
        "windows" => format!("{}/bin", cmake_dir),
        _ => format!("{}/CMake.app/Contents/bin", cmake_dir),
    };
    println!("Using CMake at: {}", cmake_bin);

    // Set up environment for subprocesses
    let mut env_path = env::var("PATH").unwrap_or_default();
    env_path = format!("{}:{}", cmake_bin, env_path);

    // Platform-specific dependency installation
    match OS {
        "macos" | "linux" => {
            let brew = Command::new("which").arg("brew").output().expect("Failed to check for brew");
            if !brew.status.success() {
                eprintln!("Homebrew not found. Please install Homebrew first.");
                std::process::exit(1);
            }
            Command::new("brew").args(["update"]).status().ok();
            Command::new("brew").args(["install", "ninja", "python", "git"]).status().ok();
        },
        "windows" => {
            println!("Please ensure Python 3, Git, and Ninja are installed and in your PATH on Windows.");
        },
        _ => {}
    }

    // Ensure python3 is available
    let python3 = Command::new("which").arg("python3").output().expect("Failed to find python3");
    if !python3.status.success() {
        eprintln!("python3 not found. Please install Python 3.");
        std::process::exit(1);
    }
    let python3_path = String::from_utf8_lossy(&python3.stdout).trim().to_string();

    // Install Python deps
    Command::new(&python3_path).args(["-m", "pip", "install", "--upgrade", "setuptools", "wheel"]).status().ok();
    Command::new(&python3_path).args(["-m", "pip", "install", "typing_extensions", "pyyaml", "numpy", "six"]).status().ok();

    // Clone PyTorch if not present
    if !Path::new(&build_dir).exists() {
        let status = Command::new("git")
            .args(["clone", "--recursive", "https://github.com/pytorch/pytorch.git", &build_dir])
            .status().expect("Failed to clone pytorch");
        assert!(status.success(), "git clone failed");
    }

    // --- PATCH AND BUILD STEPS (from tch.sh) ---
    // Enter build_dir
    let _ = env::set_current_dir(&build_dir);

    // git fetch, checkout, submodule sync/update
    let _ = Command::new("git").args(["fetch", "--all"]).status();
    let _ = Command::new("git").args(["checkout", "v2.6.0"]).status();
    let _ = Command::new("git").args(["submodule", "sync"]).status();
    let _ = Command::new("git").args(["submodule", "update", "--init", "--recursive"]).status();

    // Patch CMakeLists.txt to set CMAKE_POLICY_VERSION_MINIMUM 3.5
    let cmakelists = Path::new("CMakeLists.txt");
    if cmakelists.exists() {
        let contents = fs::read_to_string(cmakelists).unwrap();
        if !contents.contains("CMAKE_POLICY_VERSION_MINIMUM") {
            let mut new_contents = String::from("set(CMAKE_POLICY_VERSION_MINIMUM 3.5)\n");
            new_contents.push_str(&contents);
            fs::write(cmakelists, new_contents).unwrap();
            println!("Patched CMakeLists.txt with set(CMAKE_POLICY_VERSION_MINIMUM 3.5)");
        }
    }

    // Clean build directory and CMake cache
    let _ = fs::remove_dir_all("build");
    let _ = fs::remove_file("CMakeCache.txt");
    let _ = fs::remove_dir_all("CMakeFiles");

    // Remove -Werror from all CMakeLists.txt
    let find = Command::new("find")
        .args([".", "-name", "CMakeLists.txt"])
        .output().expect("Failed to run find");
    let files = String::from_utf8_lossy(&find.stdout);
    for file in files.lines() {
        let orig = fs::read_to_string(file).unwrap();
        let patched = orig.replace("-Werror", "");
        if patched != orig {
            fs::write(file, patched).unwrap();
        }
    }

    // Set CXXFLAGS
    env::set_var("CXXFLAGS", "-Wno-vla-cxx-extension");

    // Build LibTorch
    let _ = Command::new(&python3_path).args(["setup.py", "clean"]).status();
    let _ = Command::new(&python3_path).args(["setup.py", "install"]).status();

    // Find and copy built libtorch to install dir
    let install_dir = format!("{}/libtorch", home_str);
    let mut libtorch_build_path = None;
    let find_build = Command::new("find").args(["build", "-type", "d", "-name", "libtorch"]).output().ok();
    if let Some(out) = find_build {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if !line.trim().is_empty() {
                libtorch_build_path = Some(line.trim().to_string());
                break;
            }
        }
    }
    if libtorch_build_path.is_none() {
        let find_dist = Command::new("find").args(["dist", "-type", "d", "-name", "libtorch"]).output().ok();
        if let Some(out) = find_dist {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if !line.trim().is_empty() {
                    libtorch_build_path = Some(line.trim().to_string());
                    break;
                }
            }
        }
    }
    if let Some(libtorch_path) = libtorch_build_path {
        let _ = fs::remove_dir_all(&install_dir);
        let _ = Command::new("cp").args(["-r", &libtorch_path, &install_dir]).status();
        println!("Copied built libtorch to {}", install_dir);
    } else {
        eprintln!("Error: Could not find built libtorch directory after build.");
        std::process::exit(1);
    }

    // Print environment variable instructions
    println!("\nLibTorch is built and set up for torch-sys!\nTo use these variables in your shell, add the following to your ~/.zshrc or ~/.bash_profile:\n");
    println!("  export LIBTORCH=\"{}\"", install_dir);
    println!("  export LIBTORCH_INCLUDE=\"{}/include\"", install_dir);
    println!("  export LIBTORCH_LIB=\"{}/lib\"", install_dir);
    println!("  export DYLD_LIBRARY_PATH=\"{}/lib:$DYLD_LIBRARY_PATH\"", install_dir);
    println!("  export CXX=\"clang++\"\n");

    println!("\n[INFO] CMake and Python dependencies are set up for {}.\nNext steps: patch CMakeLists.txt, set CXXFLAGS, and build PyTorch as in tch.sh.\n", OS);
    println!("You can now run the build steps in Rust or shell as needed.");
}