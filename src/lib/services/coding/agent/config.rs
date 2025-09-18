use serde::{Deserialize, Serialize};
use super::providers::ModelProvider;

/// Model configuration for different LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub model_name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Main configuration for the coding agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentConfig {
    pub default_model: String,
    pub max_context_lines: usize,
    pub safe_commands: Vec<String>,
    pub require_confirmation: bool,
    pub system_prompt_template: String,
    pub available_models: Vec<ModelConfig>,
    pub fallback_models: Vec<String>,
    pub enable_model_switching: bool,
    pub workspace_integration: bool,
    pub ollama_timeout_seconds: u64,
}

impl Default for CodingAgentConfig {
    fn default() -> Self {
        Self {
            default_model: "gpt-oss".to_string(),
            max_context_lines: 100,
            safe_commands: vec![
                "ls".to_string(), "cat".to_string(), "pwd".to_string(),
                "echo".to_string(), "mkdir".to_string(), "touch".to_string(),
                "cp".to_string(), "mv".to_string(), "rm".to_string(),
                "grep".to_string(), "cd".to_string(), "find".to_string(),
                "head".to_string(), "tail".to_string(), "test".to_string(),
                "wc".to_string(), "sort".to_string(), "uniq".to_string(),
                "cargo".to_string(), "git".to_string(), "tree".to_string(),
                "file".to_string(), "stat".to_string(), "which".to_string(),
                "basename".to_string(), "dirname".to_string(), "clear".to_string(),
                "date".to_string(), "whoami".to_string(), "id".to_string(),
                "uname".to_string(), "rustc".to_string(), "rustfmt".to_string(),
                "sed".to_string(), "awk".to_string(), "curl".to_string(),
                "wget".to_string(), "python".to_string(), "python3".to_string(),
                "npm".to_string(), "node".to_string(), "yarn".to_string(),
                "pip".to_string(), "pip3".to_string(), "make".to_string(),
                "cmake".to_string(), "gcc".to_string(), "clang".to_string(),
                "java".to_string(), "javac".to_string(), "go".to_string(),
                "docker".to_string(), "kubectl".to_string(), "ssh".to_string(),
            ],
            require_confirmation: false,
            system_prompt_template: r#"You are SAM (System Assistant Manager), an expert coding assistant with advanced understanding of software development, system administration, and project management.

CURRENT CONTEXT:
- Working Directory: {current_dir}
- System: {system_info}
- Available Code Models: {available_models}

SESSION HISTORY (last {context_lines} lines):
{session_context}

SAFE COMMANDS: {safe_commands}

CORE CAPABILITIES:
1. **File Operations**: Read, edit, create, and manipulate files
2. **Code Generation**: Write, modify, and improve code in any language
3. **Project Management**: Create projects, manage dependencies, run builds
4. **Development Workflow**: Git operations, testing, debugging, formatting

COMMAND EXECUTION RULES:
1. ALWAYS start with `pwd` and `ls -la` to understand current context
2. Check if the project/file already exists before trying to create or navigate
3. For file modifications, use appropriate tools:
   - `cat > filename << 'EOF'` for creating/replacing files
   - `sed` for simple text replacements (when available)
   - Direct file operations in current directory when possible
4. After EVERY change, verify with `cat` or `cargo run`
5. If something fails, read the error and adjust accordingly

FILE MODIFICATION STRATEGIES:
- **Find First**: For existing projects, use `find . -name "projectname" -type d` or `ls -R` to locate them
- **Read First**: `cat filename` to understand current content
- **Simple Changes**: `sed -i 's/old/new/g' filename` for replacements
- **Add Content**: `echo "new content" >> filename` for appending
- **Create Files**: `cat > filename << 'EOF'` for new content
- **Verify**: `cat filename` to confirm changes

RUST-SPECIFIC GUIDANCE:
- Use `cargo new` for new projects
- Use `cargo build` and `cargo run` for compilation and execution
- Use `rustfmt` for code formatting
- Use `cargo clippy` for linting
- Understand main.rs structure and Rust syntax

EXAMPLES:
- User: "add a random number after hello world" →
  Response: "I'll read the current file first: `cat src/main.rs`
  Then modify it to add random number generation: `sed -i 's/println!("Hello, world!");/println!("Hello, world!"); println!("Random number: {}", rand::random::<u32>());/' src/main.rs`
  And add the rand dependency: `cargo add rand`"

- User: "create a function to calculate fibonacci" →
  Response: "I'll add a fibonacci function to your Rust file:
  ```rust
  fn fibonacci(n: u32) -> u32 {
      if n <= 1 {
          n
      } else {
          fibonacci(n - 1) + fibonacci(n - 2)
      }
  }
  ```
  Let me add this to your main.rs: `cat src/main.rs` first to see the current content."

Always provide working, executable commands and explain the reasoning behind each step."#.to_string(),
            available_models: vec![
                ModelConfig {
                    provider: ModelProvider::Ollama,
                    model_name: "codellama:latest".to_string(),
                    api_key: None,
                    base_url: Some("http://localhost:11434".to_string()),
                    temperature: Some(0.0),
                    max_tokens: Some(4096),
                },
                ModelConfig {
                    provider: ModelProvider::Ollama,
                    model_name: "llama3.1:latest".to_string(),
                    api_key: None,
                    base_url: Some("http://localhost:11434".to_string()),
                    temperature: Some(0.1),
                    max_tokens: Some(4096),
                },
                ModelConfig {
                    provider: ModelProvider::Ollama,
                    model_name: "mistrallite:latest".to_string(),
                    api_key: None,
                    base_url: Some("http://localhost:11434".to_string()),
                    temperature: Some(0.1),
                    max_tokens: Some(4096),
                },
            ],
            fallback_models: vec![
                "llama3.1:latest".to_string(),
                "mistrallite:latest".to_string(),
            ],
            enable_model_switching: true,
            workspace_integration: true,
            ollama_timeout_seconds: 300,  // 5 minutes default timeout for LLM operations
        }
    }
}

impl CodingAgentConfig {
    /// Get the default model configuration
    pub fn get_default_model_config(&self) -> Option<&ModelConfig> {
        self.available_models.iter()
            .find(|config| config.model_name == self.default_model)
    }

    /// Get all models for a specific provider
    pub fn get_models_for_provider(&self, provider: &ModelProvider) -> Vec<&ModelConfig> {
        self.available_models.iter()
            .filter(|config| &config.provider == provider)
            .collect()
    }

    /// Check if a command is in the safe list
    pub fn is_safe_command(&self, command: &str) -> bool {
        let base_cmd = command.split_whitespace().next().unwrap_or("");
        self.safe_commands.iter().any(|safe_cmd| safe_cmd == base_cmd)
    }

    /// Add a new safe command
    pub fn add_safe_command(&mut self, command: String) {
        if !self.safe_commands.contains(&command) {
            self.safe_commands.push(command);
        }
    }

    /// Remove a safe command
    pub fn remove_safe_command(&mut self, command: &str) {
        self.safe_commands.retain(|cmd| cmd != command);
    }

    /// Update the system prompt template
    pub fn update_system_prompt(&mut self, new_prompt: String) {
        self.system_prompt_template = new_prompt;
    }

    /// Add a new model configuration
    pub fn add_model_config(&mut self, config: ModelConfig) {
        self.available_models.push(config);
    }

    /// Remove a model configuration
    pub fn remove_model_config(&mut self, model_name: &str) {
        self.available_models.retain(|config| config.model_name != model_name);
    }

    /// Set the default model
    pub fn set_default_model(&mut self, model_name: String) -> Result<(), String> {
        if self.available_models.iter().any(|config| config.model_name == model_name) {
            self.default_model = model_name;
            Ok(())
        } else {
            Err(format!("Model '{}' not found in available models", model_name))
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check if default model exists in available models
        if !self.available_models.iter().any(|config| config.model_name == self.default_model) {
            errors.push(format!("Default model '{}' not found in available models", self.default_model));
        }

        // Check if fallback models exist
        for fallback in &self.fallback_models {
            if !self.available_models.iter().any(|config| config.model_name == *fallback) {
                errors.push(format!("Fallback model '{}' not found in available models", fallback));
            }
        }

        // Check for reasonable context limits
        if self.max_context_lines == 0 {
            errors.push("max_context_lines must be greater than 0".to_string());
        }

        if self.max_context_lines > 10000 {
            errors.push("max_context_lines should be reasonable (recommended: 100-1000)".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}