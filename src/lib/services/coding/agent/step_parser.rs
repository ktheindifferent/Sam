use super::execution_state::ExecutionStep;
use anyhow::Result;
use log;

/// Parser for converting AI responses into executable steps
#[derive(Debug)]
pub struct StepParser {
    safe_commands: Vec<String>,
}

impl StepParser {
    pub fn new(safe_commands: Vec<String>) -> Self {
        Self { safe_commands }
    }

    /// Parse AI response into executable steps
    pub async fn parse_execution_steps(&self, response: &str, task_description: &str) -> Vec<ExecutionStep> {
        let mut steps = Vec::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Try multiple parsing strategies with enhanced validation

        // Strategy 1: Look for markdown code blocks
        if let Ok(regex) = regex::Regex::new(r"```(?:bash|shell|sh)?\s*\n((?:[^\n]+\n?)+?)```") {
            for captures in regex.captures_iter(response) {
                if let Some(code_block) = captures.get(1) {
                    let block_text = code_block.as_str();
                    let lines: Vec<&str> = block_text.lines().collect();
                    let mut i = 0;

                    while i < lines.len() {
                        let line = lines[i].trim();

                        // Skip empty lines and comments
                        if line.is_empty() || line.starts_with('#') {
                            i += 1;
                            continue;
                        }

                        // Check for heredoc pattern first (before is_command_like check)
                        if line.contains("<<") && (line.contains("'EOF'") || line.contains("\"EOF\"") || line.contains("EOF")) {
                            // This is a heredoc command, collect all lines until EOF
                            let mut full_command = String::from(line);
                            i += 1;

                            // Look for the closing EOF
                            while i < lines.len() {
                                let content_line = lines[i];
                                full_command.push('\n');
                                full_command.push_str(content_line);

                                if content_line.trim() == "EOF" {
                                    break;
                                }
                                i += 1;
                            }

                            log::info!("Creating heredoc step with command: {}", full_command);
                            if let Some(step) = self.create_heredoc_step(&full_command, timestamp) {
                                steps.push(step);
                            } else {
                                log::warn!("Failed to create heredoc step");
                            }
                        } else if self.is_command_like(line) {
                            // Regular single-line command
                            if let Some(step) = self.create_execution_step(line, timestamp) {
                                steps.push(step);
                            }
                        }

                        i += 1;
                    }
                }
            }
        }

        // Strategy 2: Look for backtick-wrapped commands (single commands only)
        if steps.is_empty() {
            if let Ok(regex) = regex::Regex::new(r"`([^`\n]+)`") {
                for captures in regex.captures_iter(response) {
                    if let Some(command_match) = captures.get(1) {
                        let command = command_match.as_str().trim();
                        // Only accept short, command-like strings
                        if command.len() <= 100 && self.is_command_like(command) {
                            if command.contains("&&") || command.contains(";") {
                                let compound_steps = self.split_compound_command(command, timestamp);
                                steps.extend(compound_steps);
                            } else if let Some(step) = self.create_execution_step(command, timestamp) {
                                steps.push(step);
                            }
                        }
                    }
                }
            }
        }

        // Strategy 3: Line-by-line parsing for direct commands with enhanced validation
        if steps.is_empty() {
            for line in response.lines() {
                let line = line.trim();

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Skip lines that don't look like commands
                if !self.is_command_like(line) {
                    continue;
                }

                // Check if line starts with a known command
                if line.contains("&&") || line.contains(";") {
                    // Split compound commands
                    let compound_steps = self.split_compound_command(line, timestamp);
                    steps.extend(compound_steps);
                } else if let Some(step) = self.create_execution_step(line, timestamp) {
                    steps.push(step);
                }
            }
        }

        // Strategy 4: Manual fallback for common patterns
        if steps.is_empty() {
            steps = self.create_fallback_steps(task_description, timestamp);
        }

        // Validate all steps before returning
        let pre_validation_count = steps.len();
        steps.retain(|step| {
            let is_valid = self.is_valid_execution_step(step);
            if !is_valid {
                log::warn!("Removing invalid step: {} (command: {}...)",
                    step.description,
                    &step.command.chars().take(50).collect::<String>()
                );
            }
            is_valid
        });

        if pre_validation_count != steps.len() {
            log::warn!("Validation removed {} steps ({} -> {})",
                pre_validation_count - steps.len(),
                pre_validation_count,
                steps.len()
            );
        }

        log::info!("Parsed {} steps from AI response", steps.len());
        for (i, step) in steps.iter().enumerate() {
            log::info!("  Step {}: {} (command length: {} chars)", i + 1, step.description, step.command.len());
        }

        steps
    }

    /// Check if a line looks like a command
    fn is_command_like(&self, line: &str) -> bool {
        let line = line.trim();

        // Must be reasonable length
        if line.is_empty() || line.len() > 200 {
            return false;
        }

        // Skip explanatory text patterns
        let skip_patterns = [
            "need to", "you can", "additionally", "also", "furthermore", "however",
            "let me", "i will", "we need", "first", "next", "then", "finally",
            "to add", "to include", "to modify", "to create", "to install",
            "file before", "file to include", "code.", "dependency",
        ];

        let line_lower = line.to_lowercase();
        for pattern in &skip_patterns {
            if line_lower.contains(pattern) {
                return false;
            }
        }

        // Check if starts with a known command
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }

        let first_word = parts[0];

        // Check against safe commands list
        if self.safe_commands.contains(&first_word.to_string()) {
            return true;
        }

        // Additional patterns that look like commands
        let command_patterns = [
            r"^\w+\s+",           // word followed by space (basic command pattern)
            r"^[a-zA-Z_][a-zA-Z0-9_]*\s", // valid command name pattern
        ];

        for pattern in &command_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(line) && self.is_valid_command(first_word) {
                    return true;
                }
            }
        }

        false
    }

    /// Validate that an execution step is safe and meaningful
    fn is_valid_execution_step(&self, step: &ExecutionStep) -> bool {
        let command = &step.command;

        // Basic validation
        if command.trim().is_empty() {
            return false;
        }

        // Special handling for heredoc commands - they can be longer
        let is_heredoc = command.contains("<<") &&
            (command.contains("'EOF'") || command.contains("\"EOF\"") || command.contains("EOF"));

        // Check if command looks like explanatory text
        // Heredoc commands can be longer than 200 chars
        if !is_heredoc && command.len() > 200 {
            return false;
        }

        // For heredoc, check the first line for the command
        let first_line = if is_heredoc {
            command.lines().next().unwrap_or(command)
        } else {
            command
        };

        // Must start with a valid command
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }

        // Check if it's in our safe commands list
        self.is_valid_command(parts[0])
    }

    /// Create execution steps from a command, splitting compound commands if needed
    fn create_execution_step(&self, command: &str, timestamp: u64) -> Option<ExecutionStep> {
        // Split compound commands that contain && or ;
        if command.contains("&&") || command.contains(";") {
            // Return None to let compound commands be handled elsewhere
            return None;
        }

        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        if !cmd_parts.is_empty() && self.is_valid_command(&cmd_parts[0]) {
            Some(ExecutionStep {
                description: self.get_command_description(command),
                command: command.to_string(),
                output: None,
                success: false,
                timestamp,
            })
        } else {
            None
        }
    }

    /// Create execution step specifically for heredoc commands
    fn create_heredoc_step(&self, command: &str, timestamp: u64) -> Option<ExecutionStep> {
        // Extract the first line to get the command
        let lines: Vec<&str> = command.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];
        let cmd_parts: Vec<&str> = first_line.split_whitespace().collect();

        // Check if the first command (usually 'cat') is valid
        if !cmd_parts.is_empty() && self.is_valid_command(&cmd_parts[0]) {
            let description = self.get_heredoc_description(first_line);
            Some(ExecutionStep {
                description,
                command: command.to_string(),
                output: None,
                success: false,
                timestamp,
            })
        } else {
            None
        }
    }

    /// Split compound commands into individual steps
    fn split_compound_command(&self, command: &str, timestamp: u64) -> Vec<ExecutionStep> {
        let mut steps = Vec::new();

        // Split on && or ;
        let individual_commands: Vec<&str> = if command.contains("&&") {
            command.split("&&").collect()
        } else if command.contains(";") {
            command.split(";").collect()
        } else {
            vec![command]
        };

        for cmd in individual_commands {
            let trimmed_cmd = cmd.trim();
            if let Some(step) = self.create_execution_step(trimmed_cmd, timestamp) {
                steps.push(step);
            }
        }

        steps
    }

    /// Create fallback steps for common task patterns
    fn create_fallback_steps(&self, task_description: &str, timestamp: u64) -> Vec<ExecutionStep> {
        let lower_task = task_description.to_lowercase();
        let mut steps = Vec::new();

        // Pattern: "make a new directory called X and in it create a rust project called Y"
        if (lower_task.contains("directory") || lower_task.contains("folder")) &&
           (lower_task.contains("rust project") || lower_task.contains("rust")) {

            // Try multiple regex patterns to be more flexible
            let patterns = [
                r"directory called (\w+).*(?:rust project called|rust project named|project called|project named) (\w+)",
                r"folder called (\w+).*(?:rust project called|rust project named|project called|project named) (\w+)",
                r"directory (\w+).*(?:rust project|project) (\w+)",
                r"folder (\w+).*(?:rust project|project) (\w+)",
            ];

            for pattern in &patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if let Some(captures) = regex.captures(&lower_task) {
                        let dir_name = captures.get(1).map(|m| m.as_str()).unwrap_or("new_dir");
                        let project_name = captures.get(2).map(|m| m.as_str()).unwrap_or("new_project");

                        steps.push(ExecutionStep {
                            description: format!("Creating directory {}", dir_name),
                            command: format!("mkdir {}", dir_name),
                            output: None,
                            success: false,
                            timestamp,
                        });

                        steps.push(ExecutionStep {
                            description: format!("Changing to directory {}", dir_name),
                            command: format!("cd {}", dir_name),
                            output: None,
                            success: false,
                            timestamp,
                        });

                        steps.push(ExecutionStep {
                            description: format!("Creating Rust project {}", project_name),
                            command: format!("cargo new {}", project_name),
                            output: None,
                            success: false,
                            timestamp,
                        });

                        return steps;
                    }
                }
            }
        }

        // Pattern: "create a rust project called X"
        if lower_task.contains("rust project") {
            if let Ok(regex) = regex::Regex::new(r"rust project called (\w+)") {
                if let Some(captures) = regex.captures(&lower_task) {
                    let project_name = captures.get(1).map(|m| m.as_str()).unwrap_or("new_project");

                    steps.push(ExecutionStep {
                        description: format!("Creating Rust project {}", project_name),
                        command: format!("cargo new {}", project_name),
                        output: None,
                        success: false,
                        timestamp,
                    });

                    return steps;
                }
            }
        }

        // Pattern: "make directory X"
        if lower_task.contains("directory") {
            if let Ok(regex) = regex::Regex::new(r"directory (?:called )?(\w+)") {
                if let Some(captures) = regex.captures(&lower_task) {
                    let dir_name = captures.get(1).map(|m| m.as_str()).unwrap_or("new_dir");

                    steps.push(ExecutionStep {
                        description: format!("Creating directory {}", dir_name),
                        command: format!("mkdir {}", dir_name),
                        output: None,
                        success: false,
                        timestamp,
                    });

                    return steps;
                }
            }
        }

        // Pattern: file operations
        if lower_task.contains("create") && lower_task.contains("file") {
            if let Ok(regex) = regex::Regex::new(r"file (?:called |named )?(\w+\.?\w*)") {
                if let Some(captures) = regex.captures(&lower_task) {
                    let file_name = captures.get(1).map(|m| m.as_str()).unwrap_or("new_file.txt");

                    steps.push(ExecutionStep {
                        description: format!("Creating file {}", file_name),
                        command: format!("touch {}", file_name),
                        output: None,
                        success: false,
                        timestamp,
                    });

                    return steps;
                }
            }
        }

        // Pattern: build operations
        if lower_task.contains("build") || lower_task.contains("compile") {
            // Determine build command based on project context
            if lower_task.contains("rust") || lower_task.contains("cargo") {
                steps.push(ExecutionStep {
                    description: "Building Rust project".to_string(),
                    command: "cargo build".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            } else if lower_task.contains("npm") || lower_task.contains("node") {
                steps.push(ExecutionStep {
                    description: "Building Node.js project".to_string(),
                    command: "npm run build".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            } else {
                steps.push(ExecutionStep {
                    description: "Building project".to_string(),
                    command: "make".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            }
            return steps;
        }

        // Pattern: git operations
        if lower_task.contains("git") {
            if lower_task.contains("status") {
                steps.push(ExecutionStep {
                    description: "Checking git status".to_string(),
                    command: "git status".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            } else if lower_task.contains("commit") {
                steps.push(ExecutionStep {
                    description: "Adding files to git".to_string(),
                    command: "git add .".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
                steps.push(ExecutionStep {
                    description: "Committing changes".to_string(),
                    command: "git commit -m \"Update\"".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            } else if lower_task.contains("init") {
                steps.push(ExecutionStep {
                    description: "Initializing git repository".to_string(),
                    command: "git init".to_string(),
                    output: None,
                    success: false,
                    timestamp,
                });
            }
            return steps;
        }

        // Final fallback
        steps.push(ExecutionStep {
            description: format!("Execute task: {}", task_description),
            command: "echo 'Task needs manual implementation'".to_string(),
            output: None,
            success: false,
            timestamp,
        });

        steps
    }

    /// Get a human-readable description for a heredoc command
    fn get_heredoc_description(&self, first_line: &str) -> String {
        // Parse the heredoc command to extract target file
        if let Some(target_start) = first_line.find('>') {
            let after_redirect = &first_line[target_start + 1..];
            if let Some(heredoc_start) = after_redirect.find("<<") {
                let file_part = after_redirect[..heredoc_start].trim();
                return format!("Writing content to file {}", file_part);
            }
        }

        // Fallback description
        "Writing content using heredoc".to_string()
    }

    /// Get a human-readable description for a command
    fn get_command_description(&self, command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return command.to_string();
        }

        match parts[0] {
            "mkdir" => format!("Creating directory {}", parts.get(1).unwrap_or(&"<name>")),
            "cd" => format!("Changing to directory {}", parts.get(1).unwrap_or(&"<dir>")),
            "cargo" if parts.get(1) == Some(&"new") => {
                format!("Creating new Rust project {}", parts.get(2).unwrap_or(&"<name>"))
            },
            "cargo" if parts.get(1) == Some(&"build") => "Building Rust project".to_string(),
            "cargo" if parts.get(1) == Some(&"run") => "Running Rust project".to_string(),
            "cargo" if parts.get(1) == Some(&"test") => "Running Rust tests".to_string(),
            "touch" => format!("Creating file {}", parts.get(1).unwrap_or(&"<file>")),
            "echo" => "Writing content".to_string(),
            "cp" => "Copying files".to_string(),
            "mv" => "Moving/renaming files".to_string(),
            "ls" => "Listing directory contents".to_string(),
            "cat" => {
                // Check if this is a heredoc command
                if command.contains("<<") {
                    self.get_heredoc_description(command)
                } else {
                    "Displaying file contents".to_string()
                }
            },
            "git" => match parts.get(1) {
                Some(&"status") => "Checking git status".to_string(),
                Some(&"add") => "Adding files to git".to_string(),
                Some(&"commit") => "Committing changes".to_string(),
                Some(&"init") => "Initializing git repository".to_string(),
                Some(&"clone") => "Cloning repository".to_string(),
                Some(&"pull") => "Pulling latest changes".to_string(),
                Some(&"push") => "Pushing changes".to_string(),
                _ => format!("Git operation: {}", command),
            },
            "npm" => match parts.get(1) {
                Some(&"install") => "Installing npm dependencies".to_string(),
                Some(&"run") => format!("Running npm script: {}", parts.get(2).unwrap_or(&"<script>")),
                Some(&"build") => "Building npm project".to_string(),
                Some(&"test") => "Running npm tests".to_string(),
                _ => format!("NPM operation: {}", command),
            },
            "python" | "python3" => "Running Python script".to_string(),
            "node" => "Running Node.js script".to_string(),
            "make" => "Building with Make".to_string(),
            "docker" => "Docker operation".to_string(),
            _ => command.to_string(),
        }
    }

    /// Check if a command is valid/safe
    fn is_valid_command(&self, cmd: &str) -> bool {
        self.safe_commands.iter().any(|safe_cmd| safe_cmd == cmd)
    }

    /// Update safe commands list
    pub fn update_safe_commands(&mut self, safe_commands: Vec<String>) {
        self.safe_commands = safe_commands;
    }

    /// Add a safe command
    pub fn add_safe_command(&mut self, command: String) {
        if !self.safe_commands.contains(&command) {
            self.safe_commands.push(command);
        }
    }

    /// Check if a command is a compound command
    pub fn is_compound_command(&self, command: &str) -> bool {
        command.contains("&&") || command.contains(";")
    }

    /// Extract individual commands from a compound command
    pub fn extract_commands(&self, compound_command: &str) -> Vec<String> {
        let commands = if compound_command.contains("&&") {
            compound_command.split("&&").collect::<Vec<_>>()
        } else if compound_command.contains(";") {
            compound_command.split(";").collect::<Vec<_>>()
        } else {
            vec![compound_command]
        };

        commands.into_iter()
            .map(|cmd| cmd.trim().to_string())
            .filter(|cmd| !cmd.is_empty())
            .collect()
    }

    #[cfg(test)]
    pub async fn test_heredoc_parsing(&self) -> bool {
        let test_response = r#"```bash
cargo new randy
cargo add rand
cat > src/main.rs << 'EOF'
use rand::Rng;
fn main() {
    let mut rng = rand::thread_rng();
    let n: u32 = rng.gen_range(1..=100);
    println!("Random number: {}", n);
}
EOF
cargo build
cargo run
```"#;

        let steps = self.parse_execution_steps(&test_response, "test task").await;
        let step_commands: Vec<String> = steps.iter().map(|s| s.command.clone()).collect();

        log::info!("Test parsed {} steps:", steps.len());
        for (i, step) in steps.iter().enumerate() {
            log::info!("  Step {}: {}", i + 1, step.description);
        }

        // Should have 5 steps: cargo new, cargo add, heredoc, cargo build, cargo run
        steps.len() == 5 && step_commands.iter().any(|cmd| cmd.contains("<<") && cmd.contains("EOF"))
    }
}

impl Default for StepParser {
    fn default() -> Self {
        Self::new(vec![
            "ls".to_string(), "cat".to_string(), "pwd".to_string(),
            "echo".to_string(), "mkdir".to_string(), "touch".to_string(),
            "cp".to_string(), "mv".to_string(), "grep".to_string(),
            "find".to_string(), "head".to_string(), "tail".to_string(),
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
        ])
    }
}