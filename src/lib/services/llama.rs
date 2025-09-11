use std::fs;
use std::io::{self};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct LlamaService;

impl LlamaService {
    fn ensure_repository_cloned() -> io::Result<()> {
        let repositories_dir = Path::new("/opt/sam/repositories");
        let llama_repo_dir = repositories_dir.join("llama.cpp");
        
        if llama_repo_dir.exists() {
            return Ok(());
        }
        
        // Create repositories directory if it doesn't exist
        fs::create_dir_all(repositories_dir)?;
        
        // Clone the repository
        let output = Command::new("git")
            .arg("clone")
            .arg("https://github.com/ggml-org/llama.cpp.git")
            .arg(&llama_repo_dir)
            .output()?;
        
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "Failed to clone llama.cpp repository: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        Ok(())
    }

    pub fn ensure_llama_binary_with_output() -> io::Result<String> {
        let llama_src = Path::new("/opt/sam/repositories/llama.cpp");
        let llama_bin = Path::new("/opt/sam/bin/llama-cli");
        
        // Ensure repository is cloned first
        Self::ensure_repository_cloned()?;

        if llama_bin.exists() {
            return Ok("llama binary already exists.".to_string());
        }

        let mut output_log = String::new();

        // Run cmake with configuration (matching CLI version)
        let cmake_config = Command::new("cmake")
            .current_dir(llama_src)
            .arg("-DLLAMA_CURL=OFF")
            .arg("-DGGML_CCACHE=OFF")
            .arg(".")
            .output()?;
        output_log.push_str("--- cmake configure ---\n");
        output_log.push_str(&String::from_utf8_lossy(&cmake_config.stdout));
        output_log.push_str(&String::from_utf8_lossy(&cmake_config.stderr));
        if !cmake_config.status.success() {
            return Err(io::Error::other(format!(
                "Failed to configure llama.cpp with cmake\n{output_log}"
            )));
        }

        // Run cmake --build . (matching CLI version)
        let cmake_build = Command::new("cmake")
            .current_dir(llama_src)
            .arg("--build")
            .arg(".")
            .output()?;
        output_log.push_str("--- cmake build ---\n");
        output_log.push_str(&String::from_utf8_lossy(&cmake_build.stdout));
        output_log.push_str(&String::from_utf8_lossy(&cmake_build.stderr));
        if !cmake_build.status.success() {
            return Err(io::Error::other(format!(
                "Failed to build llama.cpp with cmake\n{output_log}"
            )));
        }

        // Copy binaries from repository root (matching CLI version)
        let mut found_any = false;
        fs::create_dir_all("/opt/sam/bin")?;
        
        // Read directory contents to find built binaries
        let entries = fs::read_dir(llama_src)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(fname) = path.file_name() {
                    let fname_str = fname.to_string_lossy();
                    if fname_str.starts_with("llama") || fname_str == "main" {
                        let target_bin = Path::new("/opt/sam/bin").join(&fname_str.as_ref());
                        fs::copy(&path, &target_bin)?;
                        let _ = Command::new("chmod").arg("+x").arg(&target_bin).output();
                        found_any = true;
                        output_log.push_str(&format!("Installed binary: {}\n", target_bin.display()));
                    }
                }
            }
        }

        if !found_any {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("None of the expected llama binaries were found after cmake build\n{output_log}"),
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
            .map_err(|e| io::Error::other(format!("Download failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(io::Error::other(format!(
                "Failed to download model: HTTP {}",
                resp.status()
            )));
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

        if !model_path.exists() {
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

// Example usage (not part of the service):
// let model_path = Path::new("./models/llama-7b.bin");
// let response = LlamaService::query(model_path, "What is Rust?")?;
