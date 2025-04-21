use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct LlamaService;

impl LlamaService {
    pub fn ensure_llama_binary() -> io::Result<()> {
        // Use absolute path to scripts/llama based on project root
        let llama_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/llama");
        let llama_bin = Path::new("/opt/sam/bin/llama");
        let build_dir = llama_src.join("build");
        // Ensure the build directory exists
        fs::create_dir_all(&build_dir)?;

        if llama_bin.exists() {
            return Ok(());
        }

        // // Run cmake -B build
        // let cmake_config = Command::new("cmake")
        //     .current_dir(llama_src)
        //     .arg("-B")
        //     .arg("build")
        //     .status()?;

        // if (!cmake_config.success()) {
        //     return Err(io::Error::new(
        //         io::ErrorKind::Other,
        //         "Failed to configure llama.cpp with cmake",
        //     ));
        // }

        // Run cmake --build build --config Release
        let cmake_build = Command::new("cmake")
            .current_dir(llama_src)
            .arg("--build")
            .arg("build")
            .arg("--config")
            .arg("Release")
            .status()?;

        if (!cmake_build.success()) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to build llama.cpp with cmake",
            ));
        }

        // Find the built binary (usually in build/bin/llama or build/llama)
        let built_bin = build_dir.join("bin/llama");
        let built_bin_alt = build_dir.join("llama");
        let built_bin = if built_bin.exists() {
            built_bin
        } else if built_bin_alt.exists() {
            built_bin_alt
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Built llama binary not found after cmake build",
            ));
        };

        // Ensure /opt/sam/bin exists
        fs::create_dir_all("/opt/sam/bin")?;

        // Copy to /opt/sam/bin/llama
        fs::copy(&built_bin, llama_bin)?;

        // Make sure it's executable
        let _ = Command::new("chmod")
            .arg("+x")
            .arg(llama_bin)
            .status();

        Ok(())
    }
    fn download_model(model_url: &str, model_filename: &str) -> io::Result<()> {
        let models_dir = Path::new("/opt/sam/models/");
        let model_path = models_dir.join(model_filename);

        // Create models directory if it doesn't exist
        fs::create_dir_all(models_dir)?;

        // Skip download if model already exists
        if model_path.exists() {
            return Ok(());
        }

        // Download the model file
        let mut resp = reqwest::blocking::get(model_url)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Download failed: {e}")))?;

        if (!resp.status().is_success()) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to download model: HTTP {}", resp.status()),
            ));
        }

        let mut out = fs::File::create(&model_path)?;
        io::copy(&mut resp, &mut out)?;

        Ok(())
    }

    pub fn download_v3_model() -> io::Result<()> {
        Self::download_model(
            "https://huggingface.co/meta-llama/Llama-3.1-8B-GGUF/resolve/main/llama-3.1-8b.Q4_K_M.gguf",
            "llama-3.1-8b.Q4_K_M.gguf",
        )
    }

    pub fn download_v2_model() -> io::Result<()> {
        Self::download_model(
            "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf",
            "llama-2-7b.Q4_K_M.gguf",
        )
    }

    pub fn install_blocking() -> io::Result<String> {
        Self::ensure_llama_binary()?;
        Self::download_v2_model()?;
        Self::download_v3_model()?;
        Ok("Llama binary and models installed.".to_string())
    }

    pub fn query(model_path: &Path, prompt: &str) -> io::Result<String> {
        Self::ensure_llama_binary()?;

        if (!model_path.exists()) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Model file not found.",
            ));
        }

        let llama_bin = "/opt/sam/bin/llama";
        let mut child = Command::new(llama_bin)
            .arg("--model")
            .arg(model_path)
            .arg("--prompt")
            .arg(prompt)
            .stdout(Stdio::piped())
            .spawn()?;

        let mut output = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            stdout.read_to_string(&mut output)?;
        }
        let _ = child.wait();
        Ok(output)
    }
}

/// Async install wrapper for CLI
pub async fn install() -> io::Result<String> {
    tokio::task::spawn_blocking(|| LlamaService::install_blocking()).await?
}

// Example usage (not part of the service):
// let model_path = Path::new("./models/llama-7b.bin");
// let response = LlamaService::query(model_path, "What is Rust?")?;