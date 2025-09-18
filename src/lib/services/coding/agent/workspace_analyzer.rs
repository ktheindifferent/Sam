use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::fs;
use log::info;

use super::types::{ProjectType, BuildSystem, ProjectStructure};

/// Workspace analyzer for comprehensive project understanding
pub struct WorkspaceAnalyzer {
    root_path: PathBuf,
    ignore_patterns: Vec<String>,
    max_depth: usize,
    file_cache: HashMap<PathBuf, FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub file_type: FileType,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileType {
    Source,
    Test,
    Config,
    Documentation,
    Asset,
    Build,
    Unknown,
}

/// Comprehensive workspace analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAnalysis {
    pub project_structure: ProjectStructure,
    pub statistics: WorkspaceStatistics,
    pub dependencies: DependencyAnalysis,
    pub code_health: CodeHealthMetrics,
    pub architecture: ArchitectureInfo,
    pub security_issues: Vec<SecurityIssue>,
    pub suggestions: Vec<WorkspaceSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatistics {
    pub total_files: usize,
    pub total_lines: usize,
    pub language_distribution: HashMap<String, LanguageStats>,
    pub file_types: HashMap<FileType, usize>,
    pub largest_files: Vec<(PathBuf, u64)>,
    pub recently_modified: Vec<(PathBuf, std::time::SystemTime)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    pub files: usize,
    pub lines: usize,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    pub direct_dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub peer_dependencies: Vec<Dependency>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub outdated: Vec<OutdatedDependency>,
    pub vulnerabilities: Vec<DependencyVulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub source: String, // npm, crates.io, pypi, etc.
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedDependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub update_type: UpdateType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyVulnerability {
    pub dependency: String,
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub fix_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeHealthMetrics {
    pub test_coverage: Option<f32>,
    pub documentation_coverage: f32,
    pub code_duplication: f32,
    pub technical_debt_hours: f32,
    pub maintainability_index: f32,
    pub complexity_hotspots: Vec<ComplexityHotspot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityHotspot {
    pub file: PathBuf,
    pub function: String,
    pub complexity: u32,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureInfo {
    pub layers: Vec<ArchitectureLayer>,
    pub modules: Vec<ModuleInfo>,
    pub coupling_score: f32, // 0.0 (loose) to 1.0 (tight)
    pub cohesion_score: f32, // 0.0 (low) to 1.0 (high)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureLayer {
    pub name: String,
    pub directories: Vec<PathBuf>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub public_api: Vec<String>,
    pub internal_dependencies: Vec<String>,
    pub external_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: VulnerabilitySeverity,
    pub category: SecurityCategory,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityCategory {
    HardcodedSecret,
    InsecureFunction,
    SQLInjection,
    PathTraversal,
    WeakCrypto,
    UnvalidatedInput,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSuggestion {
    pub priority: SuggestionPriority,
    pub category: SuggestionCategory,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub estimated_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionCategory {
    Performance,
    Security,
    Maintainability,
    Testing,
    Documentation,
    Dependencies,
    Architecture,
}

impl WorkspaceAnalyzer {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".venv".to_string(),
                "__pycache__".to_string(),
            ],
            max_depth: 10,
            file_cache: HashMap::new(),
        }
    }

    /// Perform comprehensive workspace analysis
    pub async fn analyze(&mut self) -> Result<WorkspaceAnalysis> {
        info!("Starting workspace analysis for: {:?}", self.root_path);

        // Detect project type and structure
        let project_structure = self.detect_project_structure().await?;

        // Gather statistics
        let statistics = self.gather_statistics().await?;

        // Analyze dependencies
        let dependencies = self.analyze_dependencies(&project_structure).await?;

        // Calculate code health metrics
        let code_health = self.calculate_code_health(&statistics).await?;

        // Analyze architecture
        let architecture = self.analyze_architecture(&project_structure).await?;

        // Scan for security issues
        let security_issues = self.scan_security_issues().await?;

        // Generate suggestions
        let suggestions = self.generate_suggestions(
            &project_structure,
            &statistics,
            &dependencies,
            &code_health,
            &security_issues,
        );

        Ok(WorkspaceAnalysis {
            project_structure,
            statistics,
            dependencies,
            code_health,
            architecture,
            security_issues,
            suggestions,
        })
    }

    /// Detect project type and basic structure
    async fn detect_project_structure(&self) -> Result<ProjectStructure> {
        let mut project_type = ProjectType::Unknown;
        let mut build_system = BuildSystem::Unknown;
        let mut config_files = Vec::new();
        let mut dependencies = Vec::new();

        // Check for common project files
        let checks = vec![
            ("Cargo.toml", ProjectType::Rust, BuildSystem::Cargo),
            ("package.json", ProjectType::JavaScript, BuildSystem::Npm),
            ("pyproject.toml", ProjectType::Python, BuildSystem::Poetry),
            ("go.mod", ProjectType::Go, BuildSystem::Unknown),
            ("pom.xml", ProjectType::Java, BuildSystem::Maven),
            ("build.gradle", ProjectType::Java, BuildSystem::Gradle),
        ];

        for (file, p_type, b_system) in checks {
            let path = self.root_path.join(file);
            if path.exists() {
                project_type = p_type;
                build_system = b_system;
                config_files.push(file.to_string());

                // Extract dependencies from config file
                if let Ok(content) = fs::read_to_string(&path).await {
                    dependencies.extend(self.extract_dependencies_from_content(&content, &project_type));
                }
                break;
            }
        }

        // Find source files
        let source_files = self.find_source_files(&project_type).await?;

        // Find test files
        let test_files = self.find_test_files(&project_type).await?;

        // Check for git repository
        let git_repository = self.root_path.join(".git").exists();

        Ok(ProjectStructure {
            project_type,
            root_directory: self.root_path.to_string_lossy().to_string(),
            source_files,
            config_files,
            test_files,
            dependencies,
            git_repository,
            build_system,
        })
    }

    /// Find source files based on project type
    async fn find_source_files(&self, project_type: &ProjectType) -> Result<Vec<String>> {
        let extensions = match project_type {
            ProjectType::Rust => vec!["rs"],
            ProjectType::Python => vec!["py"],
            ProjectType::JavaScript | ProjectType::TypeScript => vec!["js", "jsx", "ts", "tsx"],
            ProjectType::Go => vec!["go"],
            ProjectType::Java => vec!["java"],
            _ => vec![],
        };

        let mut files = Vec::new();
        self.walk_directory(&self.root_path, &mut |path| {
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if extensions.contains(&ext.to_str().unwrap_or("")) {
                        files.push(path.strip_prefix(&self.root_path)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string());
                    }
                }
            }
        }).await?;

        Ok(files)
    }

    /// Walk directory recursively
    async fn walk_directory<F>(&self, dir: &Path, callback: &mut F) -> Result<()>
    where
        F: FnMut(&Path),
    {
        if self.should_ignore(dir) {
            return Ok(());
        }

        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() && !self.should_ignore(&path) {
                Box::pin(self.walk_directory(&path, callback)).await?;
            } else {
                callback(&path);
            }
        }

        Ok(())
    }

    /// Find test files
    async fn find_test_files(&self, project_type: &ProjectType) -> Result<Vec<String>> {
        let test_patterns = match project_type {
            ProjectType::Rust => vec!["test", "tests"],
            ProjectType::Python => vec!["test_", "_test", "tests"],
            ProjectType::JavaScript | ProjectType::TypeScript => vec!["test", "spec", "__tests__"],
            ProjectType::Go => vec!["_test"],
            ProjectType::Java => vec!["Test", "Tests"],
            _ => vec![],
        };

        let mut test_files = Vec::new();
        self.walk_directory(&self.root_path, &mut |path| {
            if path.is_file() {
                let path_str = path.to_string_lossy();
                if test_patterns.iter().any(|pattern| path_str.contains(pattern)) {
                    test_files.push(path.strip_prefix(&self.root_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string());
                }
            }
        }).await?;

        Ok(test_files)
    }

    /// Extract dependencies from config file content
    fn extract_dependencies_from_content(&self, content: &str, project_type: &ProjectType) -> Vec<String> {
        let mut deps = Vec::new();

        match project_type {
            ProjectType::Rust => {
                // Parse Cargo.toml dependencies
                for line in content.lines() {
                    if line.contains("=") && !line.starts_with("#") && !line.starts_with("[") {
                        if let Some(dep_name) = line.split("=").next() {
                            deps.push(dep_name.trim().to_string());
                        }
                    }
                }
            }
            ProjectType::JavaScript | ProjectType::TypeScript => {
                // Parse package.json dependencies
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                    if let Some(dependencies) = json.get("dependencies") {
                        if let Some(obj) = dependencies.as_object() {
                            deps.extend(obj.keys().cloned());
                        }
                    }
                }
            }
            ProjectType::Python => {
                // Parse pyproject.toml or requirements.txt
                for line in content.lines() {
                    if !line.starts_with("#") && !line.is_empty() {
                        if let Some(dep_name) = line.split(&['=', '>', '<', '~'][..]).next() {
                            deps.push(dep_name.trim().to_string());
                        }
                    }
                }
            }
            _ => {}
        }

        deps
    }

    /// Gather workspace statistics
    async fn gather_statistics(&mut self) -> Result<WorkspaceStatistics> {
        let mut total_files = 0;
        let mut total_lines = 0;
        let mut language_distribution = HashMap::new();
        let mut file_types = HashMap::new();
        let mut file_sizes = Vec::new();
        let mut modified_times = Vec::new();

        let mut files_to_analyze = Vec::new();
        self.walk_directory(&self.root_path, &mut |path| {
            if path.is_file() {
                files_to_analyze.push(path.to_path_buf());
            }
        }).await?;

        for path in files_to_analyze {
            if path.is_file() {
                total_files += 1;

                if let Ok(metadata) = fs::metadata(&path).await {
                    let size = metadata.len();
                    file_sizes.push((path.clone(), size));

                    if let Ok(modified) = metadata.modified() {
                        modified_times.push((path.clone(), modified));
                    }

                    // Count lines if it's a text file
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
                        if self.is_text_file(ext_str) {
                            if let Ok(content) = fs::read_to_string(&path).await {
                                let lines = content.lines().count();
                                total_lines += lines;

                                let lang = self.detect_language_from_extension(ext_str);
                                let entry = language_distribution.entry(lang.clone())
                                    .or_insert(LanguageStats { files: 0, lines: 0, percentage: 0.0 });
                                entry.files += 1;
                                entry.lines += lines;
                            }
                        }
                    }

                    // Categorize file type
                    let file_type = self.categorize_file(&path);
                    *file_types.entry(file_type).or_insert(0) += 1;
                }
            }
        }

        // Calculate language percentages
        for stats in language_distribution.values_mut() {
            stats.percentage = (stats.lines as f32 / total_lines as f32) * 100.0;
        }

        // Sort and get largest files
        file_sizes.sort_by_key(|&(_, size)| std::cmp::Reverse(size));
        let largest_files = file_sizes.into_iter().take(10).collect();

        // Sort and get recently modified
        modified_times.sort_by_key(|&(_, time)| std::cmp::Reverse(time));
        let recently_modified = modified_times.into_iter().take(10).collect();

        Ok(WorkspaceStatistics {
            total_files,
            total_lines,
            language_distribution,
            file_types,
            largest_files,
            recently_modified,
        })
    }

    /// Analyze project dependencies
    async fn analyze_dependencies(&self, project_structure: &ProjectStructure) -> Result<DependencyAnalysis> {
        let mut direct_dependencies = Vec::new();
        let mut dev_dependencies = Vec::new();
        let mut outdated = Vec::new();
        let mut vulnerabilities = Vec::new();

        // Parse dependency files based on project type
        match project_structure.project_type {
            ProjectType::Rust => {
                if let Ok(content) = fs::read_to_string(self.root_path.join("Cargo.toml")).await {
                    // Simple TOML parsing for dependencies
                    let mut in_dependencies = false;
                    let mut in_dev_dependencies = false;

                    for line in content.lines() {
                        if line.trim() == "[dependencies]" {
                            in_dependencies = true;
                            in_dev_dependencies = false;
                        } else if line.trim() == "[dev-dependencies]" {
                            in_dependencies = false;
                            in_dev_dependencies = true;
                        } else if line.starts_with('[') {
                            in_dependencies = false;
                            in_dev_dependencies = false;
                        } else if (in_dependencies || in_dev_dependencies) && line.contains('=') {
                            if let Some(eq_pos) = line.find('=') {
                                let name = line[..eq_pos].trim().to_string();
                                let version_part = line[eq_pos + 1..].trim();

                                // Extract version from various formats
                                let version = if version_part.starts_with('"') {
                                    version_part.trim_matches('"').to_string()
                                } else if version_part.starts_with('{') {
                                    // Handle inline table format
                                    if let Some(v_start) = version_part.find("version") {
                                        let v_part = &version_part[v_start..];
                                        if let Some(q_start) = v_part.find('"') {
                                            let v_rest = &v_part[q_start + 1..];
                                            if let Some(q_end) = v_rest.find('"') {
                                                v_rest[..q_end].to_string()
                                            } else {
                                                "*".to_string()
                                            }
                                        } else {
                                            "*".to_string()
                                        }
                                    } else {
                                        "*".to_string()
                                    }
                                } else {
                                    version_part.to_string()
                                };

                                let dep = Dependency {
                                    name,
                                    version,
                                    source: "crates.io".to_string(),
                                    license: None,
                                };

                                if in_dependencies {
                                    direct_dependencies.push(dep);
                                } else if in_dev_dependencies {
                                    dev_dependencies.push(dep);
                                }
                            }
                        }
                    }
                }
            }
            ProjectType::JavaScript | ProjectType::TypeScript => {
                if let Ok(content) = fs::read_to_string(self.root_path.join("package.json")).await {
                    if let Ok(package) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(deps) = package.get("dependencies") {
                            if let Some(obj) = deps.as_object() {
                                for (name, version) in obj {
                                    direct_dependencies.push(Dependency {
                                        name: name.clone(),
                                        version: version.as_str().unwrap_or("*").to_string(),
                                        source: "npm".to_string(),
                                        license: None,
                                    });
                                }
                            }
                        }

                        if let Some(deps) = package.get("devDependencies") {
                            if let Some(obj) = deps.as_object() {
                                for (name, version) in obj {
                                    dev_dependencies.push(Dependency {
                                        name: name.clone(),
                                        version: version.as_str().unwrap_or("*").to_string(),
                                        source: "npm".to_string(),
                                        license: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Create a simple dependency graph (this would need more sophisticated parsing in production)
        let mut dependency_graph = HashMap::new();
        for dep in &direct_dependencies {
            dependency_graph.insert(dep.name.clone(), Vec::new());
        }

        Ok(DependencyAnalysis {
            direct_dependencies,
            dev_dependencies,
            peer_dependencies: Vec::new(),
            dependency_graph,
            outdated,
            vulnerabilities,
        })
    }

    /// Calculate code health metrics
    async fn calculate_code_health(&self, statistics: &WorkspaceStatistics) -> Result<CodeHealthMetrics> {
        let total_lines = statistics.total_lines as f32;

        // Simple heuristic calculations (would need proper analysis tools in production)
        let test_coverage = if statistics.file_types.get(&FileType::Test).unwrap_or(&0) > &0 {
            Some(35.0) // Placeholder
        } else {
            None
        };

        let documentation_coverage = if statistics.file_types.get(&FileType::Documentation).unwrap_or(&0) > &0 {
            25.0 // Placeholder
        } else {
            0.0
        };

        let code_duplication = 5.0; // Placeholder
        let technical_debt_hours = (total_lines / 100.0) * 0.5; // Simple estimate
        let maintainability_index = 75.0; // Placeholder

        Ok(CodeHealthMetrics {
            test_coverage,
            documentation_coverage,
            code_duplication,
            technical_debt_hours,
            maintainability_index,
            complexity_hotspots: Vec::new(),
        })
    }

    /// Analyze project architecture
    async fn analyze_architecture(&self, project_structure: &ProjectStructure) -> Result<ArchitectureInfo> {
        let mut layers = Vec::new();
        let mut modules = Vec::new();

        // Detect common architectural patterns
        let common_layers = vec![
            ("presentation", vec!["ui", "views", "controllers", "handlers"]),
            ("business", vec!["services", "domain", "core", "lib"]),
            ("data", vec!["models", "repositories", "database", "persistence"]),
        ];

        for (layer_name, patterns) in common_layers {
            let mut layer_dirs = Vec::new();
            for pattern in patterns {
                let path = self.root_path.join(pattern);
                if path.exists() {
                    layer_dirs.push(path);
                }
            }

            if !layer_dirs.is_empty() {
                layers.push(ArchitectureLayer {
                    name: layer_name.to_string(),
                    directories: layer_dirs,
                    depends_on: Vec::new(),
                });
            }
        }

        // Calculate coupling and cohesion scores (simplified)
        let coupling_score = 0.3; // Low coupling is good
        let cohesion_score = 0.7; // High cohesion is good

        Ok(ArchitectureInfo {
            layers,
            modules,
            coupling_score,
            cohesion_score,
        })
    }

    /// Scan for security issues
    async fn scan_security_issues(&self) -> Result<Vec<SecurityIssue>> {
        let mut issues = Vec::new();

        let mut files_to_scan = Vec::new();
        self.walk_directory(&self.root_path, &mut |path| {
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if self.is_text_file(ext.to_str().unwrap_or("")) {
                        files_to_scan.push(path.to_path_buf());
                    }
                }
            }
        }).await?;

        for path in files_to_scan {
            if let Ok(content) = fs::read_to_string(&path).await {
                // Check for hardcoded secrets
                if content.contains("password =") || content.contains("api_key =") ||
                   content.contains("secret =") || content.contains("token =") {
                    issues.push(SecurityIssue {
                        severity: VulnerabilitySeverity::High,
                        category: SecurityCategory::HardcodedSecret,
                        file: path.clone(),
                        line: None,
                        description: "Potential hardcoded secret detected".to_string(),
                        recommendation: "Use environment variables or secure key management".to_string(),
                    });
                }

                // Check for SQL injection risks
                if content.contains("SELECT * FROM") && content.contains("format!") {
                    issues.push(SecurityIssue {
                        severity: VulnerabilitySeverity::Critical,
                        category: SecurityCategory::SQLInjection,
                        file: path.clone(),
                        line: None,
                        description: "Potential SQL injection vulnerability".to_string(),
                        recommendation: "Use parameterized queries instead of string formatting".to_string(),
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Generate workspace improvement suggestions
    fn generate_suggestions(
        &self,
        project_structure: &ProjectStructure,
        statistics: &WorkspaceStatistics,
        dependencies: &DependencyAnalysis,
        code_health: &CodeHealthMetrics,
        security_issues: &[SecurityIssue],
    ) -> Vec<WorkspaceSuggestion> {
        let mut suggestions = Vec::new();

        // Security suggestions
        if !security_issues.is_empty() {
            suggestions.push(WorkspaceSuggestion {
                priority: SuggestionPriority::Critical,
                category: SuggestionCategory::Security,
                title: "Address security vulnerabilities".to_string(),
                description: format!("Found {} security issues that need immediate attention", security_issues.len()),
                action_items: vec![
                    "Review and fix all hardcoded secrets".to_string(),
                    "Implement secure coding practices".to_string(),
                    "Use security scanning tools regularly".to_string(),
                ],
                estimated_effort: "2-4 hours".to_string(),
            });
        }

        // Testing suggestions
        if code_health.test_coverage.unwrap_or(0.0) < 50.0 {
            suggestions.push(WorkspaceSuggestion {
                priority: SuggestionPriority::High,
                category: SuggestionCategory::Testing,
                title: "Improve test coverage".to_string(),
                description: "Test coverage is below recommended levels".to_string(),
                action_items: vec![
                    "Write unit tests for critical functions".to_string(),
                    "Add integration tests".to_string(),
                    "Set up continuous testing".to_string(),
                ],
                estimated_effort: "1-2 days".to_string(),
            });
        }

        // Documentation suggestions
        if code_health.documentation_coverage < 30.0 {
            suggestions.push(WorkspaceSuggestion {
                priority: SuggestionPriority::Medium,
                category: SuggestionCategory::Documentation,
                title: "Enhance documentation".to_string(),
                description: "Documentation coverage is low".to_string(),
                action_items: vec![
                    "Add README with setup instructions".to_string(),
                    "Document public APIs".to_string(),
                    "Create architecture documentation".to_string(),
                ],
                estimated_effort: "4-8 hours".to_string(),
            });
        }

        // Dependency suggestions
        if !dependencies.outdated.is_empty() {
            suggestions.push(WorkspaceSuggestion {
                priority: SuggestionPriority::Medium,
                category: SuggestionCategory::Dependencies,
                title: "Update outdated dependencies".to_string(),
                description: format!("{} dependencies have newer versions available", dependencies.outdated.len()),
                action_items: vec![
                    "Review changelog for breaking changes".to_string(),
                    "Update dependencies incrementally".to_string(),
                    "Run tests after updates".to_string(),
                ],
                estimated_effort: "2-4 hours".to_string(),
            });
        }

        suggestions
    }

    /// Check if path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        for pattern in &self.ignore_patterns {
            if path.to_string_lossy().contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Check if file is a text file
    fn is_text_file(&self, extension: &str) -> bool {
        matches!(extension,
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "cpp" | "c" | "h" |
            "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "yml" |
            "yaml" | "json" | "xml" | "toml" | "ini" | "cfg" | "conf" | "txt" | "md" |
            "markdown" | "rst" | "html" | "css" | "scss" | "sass" | "sql"
        )
    }

    /// Detect language from file extension
    fn detect_language_from_extension(&self, extension: &str) -> String {
        match extension {
            "rs" => "Rust",
            "py" => "Python",
            "js" | "jsx" => "JavaScript",
            "ts" | "tsx" => "TypeScript",
            "go" => "Go",
            "java" => "Java",
            "cpp" | "cc" | "cxx" => "C++",
            "c" | "h" => "C",
            "cs" => "C#",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" | "kts" => "Kotlin",
            "scala" => "Scala",
            "sh" | "bash" => "Shell",
            "sql" => "SQL",
            _ => "Other",
        }.to_string()
    }

    /// Categorize file type
    fn categorize_file(&self, path: &Path) -> FileType {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains("test") || path_str.contains("spec") {
            FileType::Test
        } else if path_str.contains("config") || path_str.ends_with(".toml") ||
                  path_str.ends_with(".json") || path_str.ends_with(".yml") {
            FileType::Config
        } else if path_str.ends_with(".md") || path_str.ends_with(".rst") ||
                  path_str.contains("readme") || path_str.contains("license") {
            FileType::Documentation
        } else if path_str.contains("build") || path_str.contains("dist") {
            FileType::Build
        } else if self.is_text_file(path.extension().and_then(|e| e.to_str()).unwrap_or("")) {
            FileType::Source
        } else {
            FileType::Asset
        }
    }
}