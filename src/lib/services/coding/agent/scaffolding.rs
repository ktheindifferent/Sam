use std::collections::HashMap;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::fs;
use regex::Regex;

use super::errors::CodingAgentError as ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub category: ProjectCategory,
    pub language: String,
    pub framework: Option<String>,
    pub structure: DirectoryStructure,
    pub files: Vec<FileTemplate>,
    pub dependencies: Vec<Dependency>,
    pub scripts: HashMap<String, String>,
    pub environment_variables: Vec<EnvVariable>,
    pub documentation: DocumentationTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectCategory {
    WebApplication,
    API,
    CLI,
    Library,
    MobileApp,
    Desktop,
    Microservice,
    DataScience,
    MachineLearning,
    GameDevelopment,
    Blockchain,
    IoT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryStructure {
    pub root_name: String,
    pub directories: Vec<Directory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    pub name: String,
    pub path: PathBuf,
    pub purpose: String,
    pub subdirectories: Vec<Directory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTemplate {
    pub path: PathBuf,
    pub content: String,
    pub file_type: FileType,
    pub is_required: bool,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Source,
    Config,
    Documentation,
    Test,
    Build,
    Asset,
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub validation_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dependency_type: DependencyType,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Test,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationTemplate {
    pub readme_template: String,
    pub contributing_guide: Option<String>,
    pub license: String,
    pub changelog_template: Option<String>,
}

pub struct ScaffoldingEngine {
    templates: HashMap<String, ProjectTemplate>,
    generators: HashMap<String, Box<dyn CodeGenerator>>,
    validators: HashMap<String, Box<dyn TemplateValidator>>,
}

impl ScaffoldingEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            templates: HashMap::new(),
            generators: HashMap::new(),
            validators: HashMap::new(),
        };

        engine.register_default_templates();
        engine.register_generators();
        engine
    }

    fn register_default_templates(&mut self) {
        // Rust CLI Application
        self.templates.insert(
            "rust-cli".to_string(),
            ProjectTemplate {
                name: "Rust CLI Application".to_string(),
                description: "A command-line application written in Rust".to_string(),
                category: ProjectCategory::CLI,
                language: "rust".to_string(),
                framework: Some("clap".to_string()),
                structure: DirectoryStructure {
                    root_name: "{{project_name}}".to_string(),
                    directories: vec![
                        Directory {
                            name: "src".to_string(),
                            path: PathBuf::from("src"),
                            purpose: "Source code".to_string(),
                            subdirectories: vec![
                                Directory {
                                    name: "commands".to_string(),
                                    path: PathBuf::from("src/commands"),
                                    purpose: "CLI commands".to_string(),
                                    subdirectories: vec![],
                                },
                            ],
                        },
                        Directory {
                            name: "tests".to_string(),
                            path: PathBuf::from("tests"),
                            purpose: "Integration tests".to_string(),
                            subdirectories: vec![],
                        },
                    ],
                },
                files: vec![
                    FileTemplate {
                        path: PathBuf::from("Cargo.toml"),
                        content: RUST_CLI_CARGO_TOML.to_string(),
                        file_type: FileType::Config,
                        is_required: true,
                        variables: vec![
                            TemplateVariable {
                                name: "project_name".to_string(),
                                description: "Name of the project".to_string(),
                                default_value: None,
                                validation_pattern: Some(r"^[a-z][a-z0-9_-]*$".to_string()),
                            },
                        ],
                    },
                    FileTemplate {
                        path: PathBuf::from("src/main.rs"),
                        content: RUST_CLI_MAIN.to_string(),
                        file_type: FileType::Source,
                        is_required: true,
                        variables: vec![],
                    },
                ],
                dependencies: vec![
                    Dependency {
                        name: "clap".to_string(),
                        version: "4.0".to_string(),
                        dependency_type: DependencyType::Runtime,
                        optional: false,
                    },
                    Dependency {
                        name: "tokio".to_string(),
                        version: "1.0".to_string(),
                        dependency_type: DependencyType::Runtime,
                        optional: false,
                    },
                ],
                scripts: HashMap::from([
                    ("build".to_string(), "cargo build --release".to_string()),
                    ("test".to_string(), "cargo test".to_string()),
                    ("run".to_string(), "cargo run".to_string()),
                ]),
                environment_variables: vec![],
                documentation: DocumentationTemplate {
                    readme_template: README_TEMPLATE.to_string(),
                    contributing_guide: Some(CONTRIBUTING_TEMPLATE.to_string()),
                    license: "MIT".to_string(),
                    changelog_template: Some(CHANGELOG_TEMPLATE.to_string()),
                },
            },
        );

        // React TypeScript Application
        self.templates.insert(
            "react-typescript".to_string(),
            ProjectTemplate {
                name: "React TypeScript Application".to_string(),
                description: "A modern React application with TypeScript".to_string(),
                category: ProjectCategory::WebApplication,
                language: "typescript".to_string(),
                framework: Some("react".to_string()),
                structure: DirectoryStructure {
                    root_name: "{{project_name}}".to_string(),
                    directories: vec![
                        Directory {
                            name: "src".to_string(),
                            path: PathBuf::from("src"),
                            purpose: "Source code".to_string(),
                            subdirectories: vec![
                                Directory {
                                    name: "components".to_string(),
                                    path: PathBuf::from("src/components"),
                                    purpose: "React components".to_string(),
                                    subdirectories: vec![],
                                },
                                Directory {
                                    name: "hooks".to_string(),
                                    path: PathBuf::from("src/hooks"),
                                    purpose: "Custom React hooks".to_string(),
                                    subdirectories: vec![],
                                },
                                Directory {
                                    name: "services".to_string(),
                                    path: PathBuf::from("src/services"),
                                    purpose: "API services".to_string(),
                                    subdirectories: vec![],
                                },
                                Directory {
                                    name: "utils".to_string(),
                                    path: PathBuf::from("src/utils"),
                                    purpose: "Utility functions".to_string(),
                                    subdirectories: vec![],
                                },
                            ],
                        },
                        Directory {
                            name: "public".to_string(),
                            path: PathBuf::from("public"),
                            purpose: "Static assets".to_string(),
                            subdirectories: vec![],
                        },
                    ],
                },
                files: vec![
                    FileTemplate {
                        path: PathBuf::from("package.json"),
                        content: REACT_PACKAGE_JSON.to_string(),
                        file_type: FileType::Config,
                        is_required: true,
                        variables: vec![
                            TemplateVariable {
                                name: "project_name".to_string(),
                                description: "Name of the project".to_string(),
                                default_value: None,
                                validation_pattern: Some(r"^[a-z][a-z0-9-]*$".to_string()),
                            },
                        ],
                    },
                    FileTemplate {
                        path: PathBuf::from("tsconfig.json"),
                        content: TSCONFIG_JSON.to_string(),
                        file_type: FileType::Config,
                        is_required: true,
                        variables: vec![],
                    },
                    FileTemplate {
                        path: PathBuf::from("src/App.tsx"),
                        content: REACT_APP_TSX.to_string(),
                        file_type: FileType::Source,
                        is_required: true,
                        variables: vec![],
                    },
                ],
                dependencies: vec![
                    Dependency {
                        name: "react".to_string(),
                        version: "^18.0.0".to_string(),
                        dependency_type: DependencyType::Runtime,
                        optional: false,
                    },
                    Dependency {
                        name: "typescript".to_string(),
                        version: "^5.0.0".to_string(),
                        dependency_type: DependencyType::Development,
                        optional: false,
                    },
                ],
                scripts: HashMap::from([
                    ("start".to_string(), "vite".to_string()),
                    ("build".to_string(), "tsc && vite build".to_string()),
                    ("test".to_string(), "vitest".to_string()),
                ]),
                environment_variables: vec![
                    EnvVariable {
                        name: "VITE_API_URL".to_string(),
                        description: "API endpoint URL".to_string(),
                        default_value: Some("http://localhost:3000".to_string()),
                        is_secret: false,
                    },
                ],
                documentation: DocumentationTemplate {
                    readme_template: README_TEMPLATE.to_string(),
                    contributing_guide: None,
                    license: "MIT".to_string(),
                    changelog_template: None,
                },
            },
        );

        // Python FastAPI Application
        self.templates.insert(
            "python-fastapi".to_string(),
            self.create_python_fastapi_template(),
        );

        // Go Microservice
        self.templates.insert(
            "go-microservice".to_string(),
            self.create_go_microservice_template(),
        );
    }

    fn create_python_fastapi_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "Python FastAPI Application".to_string(),
            description: "A modern REST API built with FastAPI".to_string(),
            category: ProjectCategory::API,
            language: "python".to_string(),
            framework: Some("fastapi".to_string()),
            structure: DirectoryStructure {
                root_name: "{{project_name}}".to_string(),
                directories: vec![
                    Directory {
                        name: "app".to_string(),
                        path: PathBuf::from("app"),
                        purpose: "Application code".to_string(),
                        subdirectories: vec![
                            Directory {
                                name: "api".to_string(),
                                path: PathBuf::from("app/api"),
                                purpose: "API endpoints".to_string(),
                                subdirectories: vec![],
                            },
                            Directory {
                                name: "models".to_string(),
                                path: PathBuf::from("app/models"),
                                purpose: "Data models".to_string(),
                                subdirectories: vec![],
                            },
                            Directory {
                                name: "services".to_string(),
                                path: PathBuf::from("app/services"),
                                purpose: "Business logic".to_string(),
                                subdirectories: vec![],
                            },
                        ],
                    },
                    Directory {
                        name: "tests".to_string(),
                        path: PathBuf::from("tests"),
                        purpose: "Test files".to_string(),
                        subdirectories: vec![],
                    },
                ],
            },
            files: vec![
                FileTemplate {
                    path: PathBuf::from("requirements.txt"),
                    content: PYTHON_REQUIREMENTS.to_string(),
                    file_type: FileType::Config,
                    is_required: true,
                    variables: vec![],
                },
                FileTemplate {
                    path: PathBuf::from("app/main.py"),
                    content: FASTAPI_MAIN.to_string(),
                    file_type: FileType::Source,
                    is_required: true,
                    variables: vec![],
                },
            ],
            dependencies: vec![
                Dependency {
                    name: "fastapi".to_string(),
                    version: "0.100.0".to_string(),
                    dependency_type: DependencyType::Runtime,
                    optional: false,
                },
                Dependency {
                    name: "uvicorn".to_string(),
                    version: "0.23.0".to_string(),
                    dependency_type: DependencyType::Runtime,
                    optional: false,
                },
            ],
            scripts: HashMap::from([
                ("dev".to_string(), "uvicorn app.main:app --reload".to_string()),
                ("test".to_string(), "pytest".to_string()),
            ]),
            environment_variables: vec![],
            documentation: DocumentationTemplate {
                readme_template: README_TEMPLATE.to_string(),
                contributing_guide: None,
                license: "MIT".to_string(),
                changelog_template: None,
            },
        }
    }

    fn create_go_microservice_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "Go Microservice".to_string(),
            description: "A microservice built with Go".to_string(),
            category: ProjectCategory::Microservice,
            language: "go".to_string(),
            framework: Some("gin".to_string()),
            structure: DirectoryStructure {
                root_name: "{{project_name}}".to_string(),
                directories: vec![
                    Directory {
                        name: "cmd".to_string(),
                        path: PathBuf::from("cmd"),
                        purpose: "Application entry points".to_string(),
                        subdirectories: vec![],
                    },
                    Directory {
                        name: "internal".to_string(),
                        path: PathBuf::from("internal"),
                        purpose: "Private application code".to_string(),
                        subdirectories: vec![],
                    },
                    Directory {
                        name: "pkg".to_string(),
                        path: PathBuf::from("pkg"),
                        purpose: "Public packages".to_string(),
                        subdirectories: vec![],
                    },
                ],
            },
            files: vec![
                FileTemplate {
                    path: PathBuf::from("go.mod"),
                    content: GO_MOD.to_string(),
                    file_type: FileType::Config,
                    is_required: true,
                    variables: vec![
                        TemplateVariable {
                            name: "module_name".to_string(),
                            description: "Go module name".to_string(),
                            default_value: None,
                            validation_pattern: None,
                        },
                    ],
                },
                FileTemplate {
                    path: PathBuf::from("cmd/server/main.go"),
                    content: GO_MAIN.to_string(),
                    file_type: FileType::Source,
                    is_required: true,
                    variables: vec![],
                },
            ],
            dependencies: vec![],
            scripts: HashMap::from([
                ("build".to_string(), "go build -o bin/server cmd/server/main.go".to_string()),
                ("run".to_string(), "go run cmd/server/main.go".to_string()),
                ("test".to_string(), "go test ./...".to_string()),
            ]),
            environment_variables: vec![],
            documentation: DocumentationTemplate {
                readme_template: README_TEMPLATE.to_string(),
                contributing_guide: None,
                license: "MIT".to_string(),
                changelog_template: None,
            },
        }
    }

    fn register_generators(&mut self) {
        self.generators.insert(
            "crud".to_string(),
            Box::new(CrudGenerator::new()),
        );
        self.generators.insert(
            "rest_api".to_string(),
            Box::new(RestApiGenerator::new()),
        );
        self.generators.insert(
            "graphql".to_string(),
            Box::new(GraphQLGenerator::new()),
        );
        self.generators.insert(
            "auth".to_string(),
            Box::new(AuthGenerator::new()),
        );
    }

    pub async fn scaffold_project(
        &self,
        template_name: &str,
        target_dir: &Path,
        variables: HashMap<String, String>,
    ) -> Result<ScaffoldResult, ServiceError> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| ServiceError::NotFound {
                resource: "template".to_string(),
                id: template_name.to_string(),
            })?;

        // Validate variables
        self.validate_variables(template, &variables)?;

        // Create directory structure
        self.create_directory_structure(target_dir, &template.structure, &variables).await?;

        // Generate files
        let mut files_created = Vec::new();
        for file_template in &template.files {
            let file_path = self.generate_file(target_dir, file_template, &variables).await?;
            files_created.push(file_path);
        }

        // Generate documentation
        self.generate_documentation(target_dir, &template.documentation, &variables).await?;

        // Initialize version control
        self.initialize_git(target_dir).await?;

        // Install dependencies
        let dependencies_installed = self.install_dependencies(target_dir, template).await?;

        Ok(ScaffoldResult {
            project_path: target_dir.to_path_buf(),
            files_created,
            directories_created: self.count_directories(&template.structure),
            dependencies_installed,
            next_steps: self.generate_next_steps(template),
        })
    }

    fn validate_variables(
        &self,
        template: &ProjectTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<(), ServiceError> {
        for file in &template.files {
            for var in &file.variables {
                if !variables.contains_key(&var.name) && var.default_value.is_none() {
                    return Err(ServiceError::ValidationError {
                        field: var.name.clone(),
                        message: format!("Required variable '{}' is missing", var.name),
                    });
                }

                if let Some(pattern) = &var.validation_pattern {
                    if let Some(value) = variables.get(&var.name) {
                        let re = Regex::new(pattern)
                            .map_err(|e| ServiceError::ConfigError {
                                message: format!("Invalid regex pattern: {}", e),
                            })?;

                        if !re.is_match(value) {
                            return Err(ServiceError::ValidationError {
                                field: var.name.clone(),
                                message: format!("Value '{}' doesn't match required pattern", value),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn create_directory_structure(
        &self,
        base_dir: &Path,
        structure: &DirectoryStructure,
        variables: &HashMap<String, String>,
    ) -> Result<(), ServiceError> {
        let root_name = self.replace_variables(&structure.root_name, variables);
        let root_path = base_dir.join(root_name);

        fs::create_dir_all(&root_path).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(root_path.clone()),
            })?;

        for dir in &structure.directories {
            self.create_directory_recursive(&root_path, dir).await?;
        }

        Ok(())
    }

    async fn create_directory_recursive(
        &self,
        base_dir: &Path,
        dir: &Directory,
    ) -> Result<(), ServiceError> {
        let dir_path = base_dir.join(&dir.path);

        fs::create_dir_all(&dir_path).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(dir_path.clone()),
            })?;

        for subdir in &dir.subdirectories {
            Box::pin(self.create_directory_recursive(base_dir, subdir)).await?;
        }

        Ok(())
    }

    async fn generate_file(
        &self,
        base_dir: &Path,
        template: &FileTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<PathBuf, ServiceError> {
        let content = self.replace_variables(&template.content, variables);
        let file_path = base_dir.join(&template.path);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| ServiceError::IoError {
                    message: e.to_string(),
                    path: Some(parent.to_path_buf()),
                })?;
        }

        fs::write(&file_path, content).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(file_path.clone()),
            })?;

        Ok(file_path)
    }

    fn replace_variables(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        for (key, value) in variables {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }

        result
    }

    async fn generate_documentation(
        &self,
        base_dir: &Path,
        doc_template: &DocumentationTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<(), ServiceError> {
        // Generate README
        let readme_content = self.replace_variables(&doc_template.readme_template, variables);
        let readme_path = base_dir.join("README.md");
        fs::write(&readme_path, readme_content).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(readme_path),
            })?;

        // Generate CONTRIBUTING guide if provided
        if let Some(ref contributing) = doc_template.contributing_guide {
            let content = self.replace_variables(contributing, variables);
            let path = base_dir.join("CONTRIBUTING.md");
            fs::write(&path, content).await
                .map_err(|e| ServiceError::IoError {
                    message: e.to_string(),
                    path: Some(path),
                })?;
        }

        // Generate LICENSE
        let license_path = base_dir.join("LICENSE");
        fs::write(&license_path, &doc_template.license).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(license_path),
            })?;

        Ok(())
    }

    async fn initialize_git(&self, project_dir: &Path) -> Result<(), ServiceError> {
        // Initialize git repository
        tokio::process::Command::new("git")
            .arg("init")
            .current_dir(project_dir)
            .output()
            .await
            .map_err(|e| ServiceError::ExecutionError(e.to_string()))?;

        // Create .gitignore
        let gitignore_content = self.generate_gitignore();
        let gitignore_path = project_dir.join(".gitignore");
        fs::write(&gitignore_path, gitignore_content).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(gitignore_path),
            })?;

        Ok(())
    }

    fn generate_gitignore(&self) -> String {
        "# Dependencies\nnode_modules/\ntarget/\nvenv/\n__pycache__/\n\n# Build outputs\ndist/\nbuild/\n*.egg-info/\n\n# IDE\n.vscode/\n.idea/\n*.swp\n*.swo\n\n# Environment\n.env\n.env.local\n\n# Logs\n*.log\nlogs/\n\n# OS\n.DS_Store\nThumbs.db".to_string()
    }

    async fn install_dependencies(
        &self,
        project_dir: &Path,
        template: &ProjectTemplate,
    ) -> Result<bool, ServiceError> {
        match template.language.as_str() {
            "rust" => {
                // Dependencies are in Cargo.toml, cargo build will fetch them
                Ok(true)
            }
            "javascript" | "typescript" => {
                // Run npm install
                let output = tokio::process::Command::new("npm")
                    .arg("install")
                    .current_dir(project_dir)
                    .output()
                    .await;

                match output {
                    Ok(o) => Ok(o.status.success()),
                    Err(_) => Ok(false),
                }
            }
            "python" => {
                // Run pip install
                let output = tokio::process::Command::new("pip")
                    .args(&["install", "-r", "requirements.txt"])
                    .current_dir(project_dir)
                    .output()
                    .await;

                match output {
                    Ok(o) => Ok(o.status.success()),
                    Err(_) => Ok(false),
                }
            }
            "go" => {
                // Run go mod tidy
                let output = tokio::process::Command::new("go")
                    .args(&["mod", "tidy"])
                    .current_dir(project_dir)
                    .output()
                    .await;

                match output {
                    Ok(o) => Ok(o.status.success()),
                    Err(_) => Ok(false),
                }
            }
            _ => Ok(false),
        }
    }

    fn count_directories(&self, structure: &DirectoryStructure) -> usize {
        let mut count = structure.directories.len();
        for dir in &structure.directories {
            count += self.count_subdirectories(dir);
        }
        count
    }

    fn count_subdirectories(&self, dir: &Directory) -> usize {
        let mut count = dir.subdirectories.len();
        for subdir in &dir.subdirectories {
            count += self.count_subdirectories(subdir);
        }
        count
    }

    fn generate_next_steps(&self, template: &ProjectTemplate) -> Vec<String> {
        let mut steps = Vec::new();

        steps.push(format!("cd {{{{project_name}}}}"));

        if !template.dependencies.is_empty() {
            match template.language.as_str() {
                "rust" => steps.push("cargo build".to_string()),
                "javascript" | "typescript" => steps.push("npm install".to_string()),
                "python" => steps.push("pip install -r requirements.txt".to_string()),
                "go" => steps.push("go mod tidy".to_string()),
                _ => {}
            }
        }

        if let Some(run_command) = template.scripts.get("run") {
            steps.push(run_command.clone());
        } else if let Some(dev_command) = template.scripts.get("dev") {
            steps.push(dev_command.clone());
        } else if let Some(start_command) = template.scripts.get("start") {
            steps.push(start_command.clone());
        }

        steps
    }

    pub async fn generate_code(
        &self,
        generator_name: &str,
        context: GeneratorContext,
    ) -> Result<Vec<GeneratedFile>, ServiceError> {
        let generator = self.generators.get(generator_name)
            .ok_or_else(|| ServiceError::NotFound {
                resource: "generator".to_string(),
                id: generator_name.to_string(),
            })?;

        generator.generate(&context).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldResult {
    pub project_path: PathBuf,
    pub files_created: Vec<PathBuf>,
    pub directories_created: usize,
    pub dependencies_installed: bool,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorContext {
    pub entity_name: String,
    pub fields: Vec<Field>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
}

#[async_trait]
trait CodeGenerator: Send + Sync {
    async fn generate(&self, context: &GeneratorContext) -> Result<Vec<GeneratedFile>, ServiceError>;
    fn supported_languages(&self) -> Vec<String>;
}

#[async_trait]
trait TemplateValidator: Send + Sync {
    async fn validate(&self, template: &ProjectTemplate) -> Result<(), ServiceError>;
}

// Generator implementations

struct CrudGenerator;

impl CrudGenerator {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CodeGenerator for CrudGenerator {
    async fn generate(&self, context: &GeneratorContext) -> Result<Vec<GeneratedFile>, ServiceError> {
        let mut files = Vec::new();

        // Generate model
        files.push(GeneratedFile {
            path: PathBuf::from(format!("models/{}.rs", context.entity_name.to_lowercase())),
            content: self.generate_model(context),
        });

        // Generate repository
        files.push(GeneratedFile {
            path: PathBuf::from(format!("repositories/{}_repository.rs", context.entity_name.to_lowercase())),
            content: self.generate_repository(context),
        });

        // Generate service
        files.push(GeneratedFile {
            path: PathBuf::from(format!("services/{}_service.rs", context.entity_name.to_lowercase())),
            content: self.generate_service(context),
        });

        // Generate controller
        files.push(GeneratedFile {
            path: PathBuf::from(format!("controllers/{}_controller.rs", context.entity_name.to_lowercase())),
            content: self.generate_controller(context),
        });

        Ok(files)
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["rust".to_string(), "typescript".to_string(), "python".to_string()]
    }
}

impl CrudGenerator {
    fn generate_model(&self, context: &GeneratorContext) -> String {
        let mut fields_str = String::new();
        for field in &context.fields {
            let field_type = self.map_field_type(&field.field_type);
            let required = if field.is_required { "" } else { "Option<" };
            let required_close = if field.is_required { "" } else { ">" };
            fields_str.push_str(&format!(
                "    pub {}: {}{}{},\n",
                field.name, required, field_type, required_close
            ));
        }

        format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n{}}}\n",
            context.entity_name, fields_str
        )
    }

    fn generate_repository(&self, context: &GeneratorContext) -> String {
        format!(
            "pub struct {}Repository {{\n    // Implementation\n}}\n",
            context.entity_name
        )
    }

    fn generate_service(&self, context: &GeneratorContext) -> String {
        format!(
            "pub struct {}Service {{\n    // Implementation\n}}\n",
            context.entity_name
        )
    }

    fn generate_controller(&self, context: &GeneratorContext) -> String {
        format!(
            "pub struct {}Controller {{\n    // Implementation\n}}\n",
            context.entity_name
        )
    }

    fn map_field_type(&self, field_type: &str) -> String {
        match field_type {
            "string" => "String".to_string(),
            "int" | "integer" => "i32".to_string(),
            "long" => "i64".to_string(),
            "float" => "f32".to_string(),
            "double" => "f64".to_string(),
            "bool" | "boolean" => "bool".to_string(),
            "date" | "datetime" => "chrono::DateTime<chrono::Utc>".to_string(),
            "uuid" => "uuid::Uuid".to_string(),
            _ => field_type.to_string(),
        }
    }
}

struct RestApiGenerator;

impl RestApiGenerator {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CodeGenerator for RestApiGenerator {
    async fn generate(&self, _context: &GeneratorContext) -> Result<Vec<GeneratedFile>, ServiceError> {
        Ok(vec![])
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["rust".to_string(), "go".to_string(), "python".to_string()]
    }
}

struct GraphQLGenerator;

impl GraphQLGenerator {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CodeGenerator for GraphQLGenerator {
    async fn generate(&self, _context: &GeneratorContext) -> Result<Vec<GeneratedFile>, ServiceError> {
        Ok(vec![])
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["typescript".to_string(), "javascript".to_string()]
    }
}

struct AuthGenerator;

impl AuthGenerator {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CodeGenerator for AuthGenerator {
    async fn generate(&self, _context: &GeneratorContext) -> Result<Vec<GeneratedFile>, ServiceError> {
        Ok(vec![])
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["rust".to_string(), "typescript".to_string(), "python".to_string()]
    }
}

// Template content constants

const RUST_CLI_CARGO_TOML: &str = r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;

const RUST_CLI_MAIN: &str = r#"use clap::Parser;
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("Hello, {}!", args.name);
    Ok(())
}
"#;

const REACT_PACKAGE_JSON: &str = r#"{
  "name": "{{project_name}}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^4.4.0"
  }
}
"#;

const TSCONFIG_JSON: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
"#;

const REACT_APP_TSX: &str = r#"import React from 'react';

function App() {
  return (
    <div className="App">
      <h1>Welcome to {{project_name}}</h1>
      <p>Get started by editing src/App.tsx</p>
    </div>
  );
}

export default App;
"#;

const PYTHON_REQUIREMENTS: &str = r#"fastapi==0.100.0
uvicorn[standard]==0.23.0
pydantic==2.0.0
python-dotenv==1.0.0
"#;

const FASTAPI_MAIN: &str = r#"from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

app = FastAPI(title="{{project_name}}")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.get("/")
def read_root():
    return {"message": "Welcome to {{project_name}}"}

@app.get("/health")
def health_check():
    return {"status": "healthy"}
"#;

const GO_MOD: &str = r#"module {{module_name}}

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
)
"#;

const GO_MAIN: &str = r#"package main

import (
    "net/http"
    "github.com/gin-gonic/gin"
)

func main() {
    router := gin.Default()

    router.GET("/", func(c *gin.Context) {
        c.JSON(http.StatusOK, gin.H{
            "message": "Welcome to {{project_name}}",
        })
    })

    router.GET("/health", func(c *gin.Context) {
        c.JSON(http.StatusOK, gin.H{
            "status": "healthy",
        })
    })

    router.Run(":8080")
}
"#;

const README_TEMPLATE: &str = r#"# {{project_name}}

## Description
{{description}}

## Installation
Follow these steps to get started with the project.

## Usage
Instructions on how to use the project.

## Contributing
Contributions are welcome! Please read CONTRIBUTING.md for details.

## License
This project is licensed under the MIT License.
"#;

const CONTRIBUTING_TEMPLATE: &str = r#"# Contributing to {{project_name}}

## Code of Conduct
Please read and follow our Code of Conduct.

## How to Contribute
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## Development Setup
Instructions for setting up the development environment.
"#;

const CHANGELOG_TEMPLATE: &str = r#"# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Initial project structure
- Basic functionality

### Changed

### Deprecated

### Removed

### Fixed

### Security
"#;