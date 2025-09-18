use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::types::{ProjectStructure, ProjectType, BuildSystem};

/// Workspace context for better AI responses
#[derive(Debug, Clone, Default)]
pub struct WorkspaceContext {
    pub current_workspace: Option<PathBuf>,
    pub open_files: Vec<String>,
    pub recent_commands: Vec<String>,
    pub active_git_branch: Option<String>,
    pub running_services: Vec<String>,
    pub environment_variables: HashMap<String, String>,
}

/// Context manager for workspace awareness
pub struct ContextManager {
    workspace_context: WorkspaceContext,
    project_cache: Option<ProjectStructure>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            workspace_context: WorkspaceContext::default(),
            project_cache: None,
        }
    }

    /// Update workspace context with SAM ecosystem integration
    pub async fn update_workspace_context(&mut self, current_dir: &PathBuf) -> Result<()> {
        self.workspace_context.current_workspace = Some(current_dir.clone());

        // Get Git branch information
        self.workspace_context.active_git_branch = self.get_git_branch(current_dir).await;

        // Get environment variables
        self.workspace_context.environment_variables = self.get_relevant_env_vars();

        // Get running services status
        self.workspace_context.running_services = self.get_sam_service_status().await;

        Ok(())
    }

    /// Get enhanced context for better AI responses
    pub async fn get_enhanced_context(&self, current_dir: &PathBuf, session_lines: &[String]) -> Result<String> {
        let mut context_parts = Vec::new();

        // Basic system information
        context_parts.push(format!("Working Directory: {}", current_dir.display()));
        context_parts.push(format!("System: {}", std::env::consts::OS));

        // Git context if available
        if let Some(branch) = &self.workspace_context.active_git_branch {
            context_parts.push(format!("Git Branch: {}", branch));
        }

        // Project context if available
        if let Some(project) = &self.project_cache {
            context_parts.push(format!("Project Type: {:?}", project.project_type));
            context_parts.push(format!("Build System: {:?}", project.build_system));
            if !project.dependencies.is_empty() {
                context_parts.push(format!("Dependencies: {}", project.dependencies.join(", ")));
            }
        }

        // Recent session context
        if !session_lines.is_empty() {
            context_parts.push(format!("Recent Session:\n{}", session_lines.join("\n")));
        }

        // Running services
        if !self.workspace_context.running_services.is_empty() {
            context_parts.push(format!("Running Services: {}", self.workspace_context.running_services.join(", ")));
        }

        Ok(context_parts.join("\n\n"))
    }

    /// Get Git branch information
    async fn get_git_branch(&self, current_dir: &PathBuf) -> Option<String> {
        // Try to get the current git branch
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["branch", "--show-current"])
            .current_dir(current_dir)
            .output()
            .await
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    return Some(branch);
                }
            }
        }
        None
    }

    /// Get relevant environment variables
    fn get_relevant_env_vars(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        
        // Common development environment variables
        let relevant_vars = [
            "PATH", "HOME", "USER", "SHELL", "EDITOR", "TERM",
            "RUST_LOG", "CARGO_HOME", "RUSTUP_HOME",
            "NODE_ENV", "NPM_CONFIG_PREFIX", "PYTHON_PATH",
            "JAVA_HOME", "GOPATH", "GOROOT",
        ];

        for var in &relevant_vars {
            if let Ok(value) = std::env::var(var) {
                env_vars.insert(var.to_string(), value);
            }
        }

        env_vars
    }

    /// Get SAM service status
    async fn get_sam_service_status(&self) -> Vec<String> {
        let mut running_services = Vec::new();

        // Check common development services
        let services_to_check = [
            ("docker", "docker --version"),
            ("ollama", "ollama --version"),
            ("git", "git --version"),
            ("cargo", "cargo --version"),
            ("node", "node --version"),
            ("python", "python --version"),
        ];

        for (service_name, check_command) in &services_to_check {
            let parts: Vec<&str> = check_command.split_whitespace().collect();
            if !parts.is_empty() {
                if let Ok(output) = tokio::process::Command::new(parts[0])
                    .args(&parts[1..])
                    .output()
                    .await
                {
                    if output.status.success() {
                        running_services.push(service_name.to_string());
                    }
                }
            }
        }

        running_services
    }

    /// Analyze project structure and cache results
    pub async fn analyze_project_structure(&mut self, current_dir: &PathBuf) -> Result<ProjectStructure> {
        // Check if we need to refresh the cache
        if let Some(cached_project) = &self.project_cache {
            if cached_project.root_directory == current_dir.to_string_lossy() {
                return Ok(cached_project.clone());
            }
        }

        // Analyze the project structure
        let project_structure = self.analyze_project_structure_impl(current_dir).await?;
        self.project_cache = Some(project_structure.clone());
        Ok(project_structure)
    }

    async fn analyze_project_structure_impl(&self, current_dir: &PathBuf) -> Result<ProjectStructure> {
        let (project_type, build_system) = self.detect_project_type(current_dir).await;
        let (source_files, config_files, test_files) = self.scan_project_files(current_dir, &project_type).await?;
        let dependencies = self.detect_dependencies(current_dir, &build_system).await?;
        let git_repository = current_dir.join(".git").exists();

        Ok(ProjectStructure {
            project_type,
            root_directory: current_dir.to_string_lossy().to_string(),
            source_files,
            config_files,
            test_files,
            dependencies,
            git_repository,
            build_system,
        })
    }

    async fn detect_project_type(&self, current_dir: &PathBuf) -> (ProjectType, BuildSystem) {
        // Check for Rust project
        if current_dir.join("Cargo.toml").exists() {
            return (ProjectType::Rust, BuildSystem::Cargo);
        }

        // Check for Node.js projects
        if current_dir.join("package.json").exists() {
            if current_dir.join("yarn.lock").exists() {
                return (ProjectType::JavaScript, BuildSystem::Yarn);
            } else {
                return (ProjectType::JavaScript, BuildSystem::Npm);
            }
        }

        // Check for TypeScript
        if current_dir.join("tsconfig.json").exists() {
            return (ProjectType::TypeScript, BuildSystem::Npm);
        }

        // Check for Python projects
        if current_dir.join("pyproject.toml").exists() {
            return (ProjectType::Python, BuildSystem::Poetry);
        }

        if current_dir.join("requirements.txt").exists() || current_dir.join("setup.py").exists() {
            return (ProjectType::Python, BuildSystem::Unknown);
        }

        // Check for Go projects
        if current_dir.join("go.mod").exists() {
            return (ProjectType::Go, BuildSystem::Unknown);
        }

        // Check for Java projects
        if current_dir.join("pom.xml").exists() {
            return (ProjectType::Java, BuildSystem::Maven);
        }

        if current_dir.join("build.gradle").exists() || current_dir.join("build.gradle.kts").exists() {
            return (ProjectType::Java, BuildSystem::Gradle);
        }

        // Check for Makefile
        if current_dir.join("Makefile").exists() {
            return (ProjectType::Unknown, BuildSystem::Make);
        }

        (ProjectType::Unknown, BuildSystem::Unknown)
    }

    async fn scan_project_files(&self, current_dir: &PathBuf, project_type: &ProjectType) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let mut source_files = Vec::new();
        let mut config_files = Vec::new();
        let mut test_files = Vec::new();

        let (source_extensions, config_patterns, test_patterns) = match project_type {
            ProjectType::Rust => {
                (
                    vec!["rs"],
                    vec!["Cargo.toml", "Cargo.lock", "build.rs", ".rustfmt.toml", "clippy.toml"],
                    vec!["tests/", "test_", "_test.rs"],
                )
            }
            ProjectType::JavaScript | ProjectType::TypeScript => {
                (
                    vec!["js", "ts", "jsx", "tsx"],
                    vec!["package.json", "package-lock.json", "yarn.lock", "tsconfig.json", ".eslintrc", ".babelrc", "webpack.config.js"],
                    vec!["test/", "tests/", "__tests__/", ".test.", ".spec."],
                )
            }
            ProjectType::Python => {
                (
                    vec!["py", "pyx", "pyi"],
                    vec!["setup.py", "pyproject.toml", "requirements.txt", "setup.cfg", "tox.ini", ".flake8", "mypy.ini"],
                    vec!["test_", "_test.py", "tests/"],
                )
            }
            ProjectType::Go => {
                (
                    vec!["go"],
                    vec!["go.mod", "go.sum"],
                    vec!["_test.go"],
                )
            }
            ProjectType::Java => {
                (
                    vec!["java", "kt", "scala"],
                    vec!["pom.xml", "build.gradle", "build.gradle.kts", "settings.gradle"],
                    vec!["test/", "Test.java", "Tests.java"],
                )
            }
            ProjectType::Unknown => {
                (
                    vec!["txt", "md", "yml", "yaml", "json", "toml"],
                    vec!["Makefile", "CMakeLists.txt", "configure", "config.yml", "config.yaml"],
                    vec![],
                )
            }
        };

        // Scan for source files
        for extension in source_extensions {
            self.scan_directory_recursive(current_dir, current_dir, &mut source_files, &[extension]).await?;
        }

        // Collect config files
        for pattern in config_patterns {
            if current_dir.join(pattern).exists() {
                config_files.push(pattern.to_string());
            }
        }

        // Collect test files (this is a simplified implementation)
        for test_pattern in test_patterns {
            if test_pattern.ends_with('/') {
                let test_dir = current_dir.join(test_pattern.trim_end_matches('/'));
                if test_dir.exists() && test_dir.is_dir() {
                    test_files.push(test_pattern.to_string());
                }
            }
        }

        Ok((source_files, config_files, test_files))
    }

    async fn scan_directory_recursive(&self, base_dir: &PathBuf, dir: &PathBuf, files: &mut Vec<String>, extensions: &[&str]) -> Result<()> {
        // Use a stack-based approach to avoid recursion
        let mut dirs_to_scan = vec![dir.clone()];
        
        while let Some(current_dir) = dirs_to_scan.pop() {
            if let Ok(mut entries) = tokio::fs::read_dir(&current_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_file() {
                            let relative_path = path.strip_prefix(base_dir)
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .to_string();
                            
                            if let Some(file_ext) = path.extension() {
                                let ext_str = file_ext.to_string_lossy();
                                if extensions.iter().any(|&ext| ext == ext_str) {
                                    files.push(relative_path);
                                }
                            }
                        } else if metadata.is_dir() {
                            dirs_to_scan.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn detect_dependencies(&self, current_dir: &PathBuf, build_system: &BuildSystem) -> Result<Vec<String>> {
        let mut dependencies = Vec::new();

        match build_system {
            BuildSystem::Cargo => {
                let cargo_toml = current_dir.join("Cargo.toml");
                if cargo_toml.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(cargo_toml).await {
                        // Simple TOML parsing for dependencies
                        let mut in_dependencies = false;
                        for line in content.lines() {
                            let line = line.trim();
                            if line == "[dependencies]" {
                                in_dependencies = true;
                                continue;
                            }
                            if line.starts_with('[') && line != "[dependencies]" {
                                in_dependencies = false;
                                continue;
                            }
                            if in_dependencies && !line.is_empty() && !line.starts_with('#') {
                                if let Some(dep_name) = line.split('=').next() {
                                    dependencies.push(dep_name.trim().trim_matches('"').to_string());
                                }
                            }
                        }
                    }
                }
            }
            BuildSystem::Npm | BuildSystem::Yarn => {
                let package_json = current_dir.join("package.json");
                if package_json.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(package_json).await {
                        // Simple JSON parsing for dependencies
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                                dependencies.extend(deps.keys().cloned());
                            }
                            if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
                                dependencies.extend(dev_deps.keys().cloned());
                            }
                        }
                    }
                }
            }
            _ => {
                // For other build systems, we'd implement specific dependency detection
            }
        }

        Ok(dependencies)
    }

    /// Get project-aware context for better code suggestions
    pub async fn get_project_context(&self, current_dir: &PathBuf) -> String {
        let mut context_parts = Vec::new();

        if let Some(project) = &self.project_cache {
            context_parts.push(format!("Project Type: {:?}", project.project_type));
            context_parts.push(format!("Build System: {:?}", project.build_system));
            
            if !project.source_files.is_empty() {
                context_parts.push(format!("Source Files ({}): {}", 
                    project.source_files.len(),
                    project.source_files.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ));
            }

            if !project.dependencies.is_empty() {
                context_parts.push(format!("Dependencies: {}", 
                    project.dependencies.iter().take(10).cloned().collect::<Vec<_>>().join(", ")
                ));
            }

            if project.git_repository {
                context_parts.push("Git repository: Yes".to_string());
            }
        }

        context_parts.join("\n")
    }

    /// Clear project cache when directory changes
    pub fn clear_project_cache(&mut self) {
        self.project_cache = None;
    }

    /// Get Git repository status and context
    pub async fn get_git_context(&self, current_dir: &PathBuf) -> String {
        let mut git_info = Vec::new();

        // Get current branch
        if let Some(branch) = self.get_git_branch(current_dir).await {
            git_info.push(format!("Branch: {}", branch));
        }

        // Get status
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(current_dir)
            .output()
            .await
        {
            if output.status.success() {
                let status_output = String::from_utf8_lossy(&output.stdout);
                let status_lines: Vec<&str> = status_output
                    .lines()
                    .collect();
                
                if !status_lines.is_empty() {
                    git_info.push(format!("Modified files: {}", status_lines.len()));
                } else {
                    git_info.push("Working directory clean".to_string());
                }
            }
        }

        // Get recent commits
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["log", "--oneline", "-3"])
            .current_dir(current_dir)
            .output()
            .await
        {
            if output.status.success() {
                let commits_output = String::from_utf8_lossy(&output.stdout);
                let commits = commits_output.trim();
                if !commits.is_empty() {
                    git_info.push(format!("Recent commits:\n{}", commits));
                }
            }
        }

        if git_info.is_empty() {
            "Not a git repository".to_string()
        } else {
            git_info.join("\n")
        }
    }

    /// Get workspace context
    pub fn get_workspace_context(&self) -> &WorkspaceContext {
        &self.workspace_context
    }

    /// Get cached project structure
    pub fn get_project_structure(&self) -> Option<&ProjectStructure> {
        self.project_cache.as_ref()
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}