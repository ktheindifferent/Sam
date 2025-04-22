use std::fs;
use std::io::{self, Read, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct LlamaService;

impl LlamaService {
    pub fn ensure_llama_binary_with_output() -> io::Result<String> {
        let llama_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/llama.cpp");
        let llama_bin = Path::new("/opt/sam/bin/llama");
        let build_dir = llama_src.join("build");
        fs::create_dir_all(&build_dir)?;

        if llama_bin.exists() {
            return Ok("llama binary already exists.".to_string());
        }

        let mut output_log = String::new();

        // Run cmake -B build
        let cmake_config = Command::new("cmake")
            .current_dir(llama_src.clone())
            .arg("-B")
            .arg("build")
            .output()?;
        output_log.push_str("--- cmake configure ---\n");
        output_log.push_str(&String::from_utf8_lossy(&cmake_config.stdout));
        output_log.push_str(&String::from_utf8_lossy(&cmake_config.stderr));
        if !cmake_config.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to configure llama.cpp with cmake\n{}", output_log),
            ));
        }

        // Run cmake --build build --config Release
        let cmake_build = Command::new("cmake")
            .current_dir(llama_src.clone())
            .arg("--build")
            .arg("build")
            .arg("--config")
            .arg("Release")
            .output()?;
        output_log.push_str("--- cmake build ---\n");
        output_log.push_str(&String::from_utf8_lossy(&cmake_build.stdout));
        output_log.push_str(&String::from_utf8_lossy(&cmake_build.stderr));
        if !cmake_build.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to build llama.cpp with cmake\n{}", output_log),
            ));
        }

        // Find the built binaries
        let binaries = ["llama-cli", "llama-simple", "llama-bench", "llama-run", "llama-server", "llama-perplexity"];
        let mut found_any = false;

        fs::create_dir_all("/opt/sam/bin")?;

        for bin_name in &binaries {
            let built_bin = build_dir.join("bin").join(bin_name);
            let built_bin_alt = build_dir.join(bin_name);
            let target_bin = Path::new("/opt/sam/bin").join(bin_name);

            let src_bin = if built_bin.exists() {
                built_bin
            } else if built_bin_alt.exists() {
                built_bin_alt
            } else {
                continue;
            };

            fs::copy(&src_bin, &target_bin)?;
            let _ = Command::new("chmod")
                .arg("+x")
                .arg(&target_bin)
                .output();
            found_any = true;
            output_log.push_str(&format!("Installed binary: {}\n", target_bin.display()));
        }

        if !found_any {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("None of the expected llama binaries were found after cmake build\n{}", output_log),
            ));
        }

        Ok(output_log)
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

    pub fn download_v2_tiny_model() -> io::Result<()> {
        Self::download_model(
            "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_0.gguf?download=true",
            "tinyllama-1.1b-chat-v1.0.Q4_0.gguf",
        )
    }

    pub fn download_v2_model() -> io::Result<()> {
        Self::download_model(
            "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf",
            "llama-2-7b.Q4_K_M.gguf",
        )
    }

    pub fn install_blocking() -> io::Result<String> {
        let mut log = String::new();
        log.push_str(&Self::ensure_llama_binary_with_output()?);
        Self::download_v2_model()?;
        Self::download_v2_tiny_model()?;
        Self::download_v3_model()?;
        log.push_str("Llama binary and models installed.\n");
        Ok(log)
    }

    pub fn query_v2(prompt: &str) -> io::Result<String> {
        let model_path = Path::new("/opt/sam/models/llama-2-7b.Q4_K_M.gguf");
        if !model_path.exists() {
            Self::download_v2_model()?;
        }
        Self::query(model_path, prompt)
    }

    pub fn query_v2_tiny(prompt: &str) -> io::Result<String> {
        let model_path = Path::new("/opt/sam/models/tinyllama-1.1b-chat-v1.0.Q4_0.gguf");
        if !model_path.exists() {
            Self::download_v2_tiny_model()?;
        }
        Self::query(model_path, prompt)
    }

    pub fn query(model_path: &Path, prompt: &str) -> io::Result<String> {
        Self::ensure_llama_binary_with_output()?;

        if (!model_path.exists()) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Model file not found.",
            ));
        }

        let llama_bin = "/opt/sam/bin/llama-cli";
        let output = Command::new(llama_bin)
            .arg("--model")
            .arg(model_path)
            .arg("--prompt")
            .arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let mut output_str = String::new();
        output_str.push_str(&String::from_utf8_lossy(&output.stdout));
        // output_str.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(output_str)
    }
}

/// Async install wrapper for CLI
pub async fn install() -> io::Result<String> {
    tokio::task::spawn_blocking(|| LlamaService::install_blocking()).await?
}

// Example usage (not part of the service):
// let model_path = Path::new("./models/llama-7b.bin");
// let response = LlamaService::query(model_path, "What is Rust?")?;