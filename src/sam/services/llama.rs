use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct LlamaService;

impl LlamaService {
    pub fn ensure_llama_binary() -> io::Result<()> {
        let llama_src = Path::new("./scripts/llama");
        let llama_bin = Path::new("/opt/sam/bin/llama");

        if llama_bin.exists() {
            return Ok(());
        }

        // Build llama.cpp if not built
        let build_result = Command::new("make")
            .current_dir(llama_src)
            .arg("llama")
            .status()?;

        if !build_result.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to build llama.cpp",
            ));
        }

        // Find the built binary (usually main or llama)
        let built_bin = llama_src.join("llama");
        if !built_bin.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Built llama binary not found after make",
            ));
        }

        // Ensure /opt/sam/bin exists
        fs::create_dir_all("/opt/sam/bin")?;

        // Copy to /opt/sam/bin/llama
        fs::copy(&built_bin, &llama_bin)?;

        // Make sure it's executable
        let _ = Command::new("chmod")
            .arg("+x")
            .arg(&llama_bin)
            .status();

        Ok(())
    }

    pub fn query(model_path: &Path, prompt: &str) -> io::Result<String> {
        Self::ensure_llama_binary()?;

        if !model_path.exists() {
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

// Example usage (not part of the service):
// let model_path = Path::new("./models/llama-7b.bin");
// let response = LlamaService::query(model_path, "What is Rust?")?;