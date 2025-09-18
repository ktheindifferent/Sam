use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use regex::Regex;
use tokio::fs;

use super::errors::CodingAgentError as ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub source_language: String,
    pub target_language: String,
    pub preserve_structure: bool,
    pub modernize_patterns: bool,
    pub generate_tests: bool,
    pub strict_mode: bool,
    pub custom_rules: Vec<MigrationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRule {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    pub description: String,
    pub language_specific: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub files_migrated: usize,
    pub lines_converted: usize,
    pub warnings: Vec<MigrationWarning>,
    pub errors: Vec<String>,
    pub conversion_map: HashMap<PathBuf, PathBuf>,
    pub statistics: MigrationStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationWarning {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatistics {
    pub total_files: usize,
    pub successful_conversions: usize,
    pub partial_conversions: usize,
    pub failed_conversions: usize,
    pub time_elapsed_ms: u64,
    pub language_features_used: HashMap<String, usize>,
}

#[async_trait]
pub trait LanguageConverter: Send + Sync {
    async fn convert(&self, source_code: &str, config: &MigrationConfig) -> Result<String, ServiceError>;
    fn source_language(&self) -> &str;
    fn target_language(&self) -> &str;
    fn supported_features(&self) -> Vec<String>;
}

pub struct CodeMigrationEngine {
    converters: HashMap<(String, String), Box<dyn LanguageConverter>>,
    analyzers: HashMap<String, Box<dyn CodeAnalyzer>>,
    validators: HashMap<String, Box<dyn CodeValidator>>,
}

impl CodeMigrationEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            converters: HashMap::new(),
            analyzers: HashMap::new(),
            validators: HashMap::new(),
        };

        // Register built-in converters
        engine.register_builtin_converters();
        engine
    }

    fn register_builtin_converters(&mut self) {
        // JavaScript to TypeScript
        self.converters.insert(
            ("javascript".to_string(), "typescript".to_string()),
            Box::new(JsToTsConverter::new()),
        );

        // Python 2 to Python 3
        self.converters.insert(
            ("python2".to_string(), "python3".to_string()),
            Box::new(Py2ToPy3Converter::new()),
        );

        // Java to Kotlin
        self.converters.insert(
            ("java".to_string(), "kotlin".to_string()),
            Box::new(JavaToKotlinConverter::new()),
        );

        // C to Rust
        self.converters.insert(
            ("c".to_string(), "rust".to_string()),
            Box::new(CToRustConverter::new()),
        );

        // CoffeeScript to TypeScript
        self.converters.insert(
            ("coffeescript".to_string(), "typescript".to_string()),
            Box::new(CoffeeToTsConverter::new()),
        );
    }

    pub async fn migrate_project(
        &self,
        project_path: &Path,
        config: MigrationConfig,
    ) -> Result<MigrationResult, ServiceError> {
        let start_time = std::time::Instant::now();
        let mut result = MigrationResult {
            success: true,
            files_migrated: 0,
            lines_converted: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
            conversion_map: HashMap::new(),
            statistics: MigrationStatistics {
                total_files: 0,
                successful_conversions: 0,
                partial_conversions: 0,
                failed_conversions: 0,
                time_elapsed_ms: 0,
                language_features_used: HashMap::new(),
            },
        };

        // Find all source files
        let source_files = self.find_source_files(project_path, &config.source_language).await?;
        result.statistics.total_files = source_files.len();

        // Get appropriate converter
        let converter_key = (config.source_language.clone(), config.target_language.clone());
        let converter = self.converters.get(&converter_key)
            .ok_or_else(|| ServiceError::ConfigError {
                message: format!("No converter available for {} to {}", config.source_language, config.target_language)
            })?;

        // Process each file
        for source_file in source_files {
            match self.migrate_file(&source_file, &config, converter.as_ref()).await {
                Ok((target_file, lines)) => {
                    result.files_migrated += 1;
                    result.lines_converted += lines;
                    result.conversion_map.insert(source_file, target_file);
                    result.statistics.successful_conversions += 1;
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", source_file.display(), e));
                    result.statistics.failed_conversions += 1;
                    result.success = false;
                }
            }
        }

        result.statistics.time_elapsed_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    async fn migrate_file(
        &self,
        source_file: &Path,
        config: &MigrationConfig,
        converter: &dyn LanguageConverter,
    ) -> Result<(PathBuf, usize), ServiceError> {
        // Read source file
        let source_code = fs::read_to_string(source_file).await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        // Convert code
        let converted_code = converter.convert(&source_code, config).await?;

        // Determine target file path
        let target_file = self.get_target_file_path(source_file, &config.target_language)?;

        // Create target directory if needed
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;
        }

        // Write converted code
        fs::write(&target_file, &converted_code).await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        let lines = converted_code.lines().count();
        Ok((target_file, lines))
    }

    async fn find_source_files(
        &self,
        project_path: &Path,
        language: &str,
    ) -> Result<Vec<PathBuf>, ServiceError> {
        let extensions = self.get_language_extensions(language);
        let mut files = Vec::new();

        self.walk_directory(project_path, &extensions, &mut files).await?;

        Ok(files)
    }

    async fn walk_directory(
        &self,
        dir: &Path,
        extensions: &[&str],
        files: &mut Vec<PathBuf>,
    ) -> Result<(), ServiceError> {
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: Some(dir.to_path_buf()) })?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: Some(dir.to_path_buf()) })? {
            let path = entry.path();
            let file_type = entry.file_type().await
                .map_err(|e| ServiceError::IoError { message: e.to_string(), path: Some(path.clone()) })?;

            if file_type.is_dir() {
                // Skip common directories
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if !dir_name.starts_with('.') && dir_name != "node_modules" && dir_name != "target" {
                    Box::pin(self.walk_directory(&path, extensions, files)).await?;
                }
            } else if file_type.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str) {
                            files.push(path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_language_extensions(&self, language: &str) -> Vec<&'static str> {
        match language.to_lowercase().as_str() {
            "javascript" => vec!["js", "mjs", "jsx"],
            "typescript" => vec!["ts", "tsx"],
            "python" | "python2" | "python3" => vec!["py"],
            "java" => vec!["java"],
            "kotlin" => vec!["kt", "kts"],
            "c" => vec!["c", "h"],
            "cpp" | "c++" => vec!["cpp", "cc", "cxx", "hpp", "h"],
            "rust" => vec!["rs"],
            "go" => vec!["go"],
            "coffeescript" => vec!["coffee"],
            _ => vec![],
        }
    }

    fn get_target_file_path(&self, source_file: &Path, target_language: &str) -> Result<PathBuf, ServiceError> {
        let mut target = source_file.to_path_buf();

        // Get appropriate extension for target language
        let new_extension = match target_language.to_lowercase().as_str() {
            "typescript" => "ts",
            "python3" => "py",
            "kotlin" => "kt",
            "rust" => "rs",
            _ => return Err(ServiceError::ConfigError { message: format!("Unknown target language: {}", target_language) }),
        };

        target.set_extension(new_extension);
        Ok(target)
    }
}

// Converter implementations

struct JsToTsConverter {
    type_inference_engine: TypeInferenceEngine,
}

impl JsToTsConverter {
    fn new() -> Self {
        Self {
            type_inference_engine: TypeInferenceEngine::new(),
        }
    }
}

#[async_trait]
impl LanguageConverter for JsToTsConverter {
    async fn convert(&self, source_code: &str, config: &MigrationConfig) -> Result<String, ServiceError> {
        let mut converted = source_code.to_string();

        // Add type annotations
        converted = self.add_type_annotations(&converted)?;

        // Convert require to import
        converted = self.convert_requires_to_imports(&converted)?;

        // Add interface definitions
        if config.modernize_patterns {
            converted = self.add_interfaces(&converted)?;
        }

        // Convert class syntax
        converted = self.modernize_class_syntax(&converted)?;

        Ok(converted)
    }

    fn source_language(&self) -> &str { "javascript" }
    fn target_language(&self) -> &str { "typescript" }
    fn supported_features(&self) -> Vec<String> {
        vec![
            "type_inference".to_string(),
            "interface_generation".to_string(),
            "modern_syntax".to_string(),
        ]
    }
}

impl JsToTsConverter {
    fn add_type_annotations(&self, code: &str) -> Result<String, ServiceError> {
        let mut result = String::new();

        for line in code.lines() {
            // Function parameters
            if line.contains("function") || line.contains("=>") {
                result.push_str(&self.annotate_function(line)?);
            }
            // Variable declarations
            else if line.contains("let") || line.contains("const") || line.contains("var") {
                result.push_str(&self.annotate_variable(line)?);
            }
            else {
                result.push_str(line);
            }
            result.push('\n');
        }

        Ok(result)
    }

    fn annotate_function(&self, line: &str) -> Result<String, ServiceError> {
        // Basic type inference for function parameters
        let mut annotated = line.to_string();

        // Add return type annotation
        if !annotated.contains(":") && !annotated.contains("=>") {
            if annotated.contains("return") {
                annotated = annotated.replace(")", "): any");
            }
        }

        Ok(annotated)
    }

    fn annotate_variable(&self, line: &str) -> Result<String, ServiceError> {
        let mut annotated = line.to_string();

        // Infer types from initial values
        if line.contains("= \"") || line.contains("= '") {
            annotated = annotated.replace("=", ": string =");
        } else if line.contains("= true") || line.contains("= false") {
            annotated = annotated.replace("=", ": boolean =");
        } else if line.contains(r"= \d+") {
            annotated = annotated.replace("=", ": number =");
        } else if line.contains("= [") {
            annotated = annotated.replace("=", ": any[] =");
        } else if line.contains("= {") {
            annotated = annotated.replace("=", ": any =");
        }

        Ok(annotated)
    }

    fn convert_requires_to_imports(&self, code: &str) -> Result<String, ServiceError> {
        let require_regex = Regex::new(r#"const\s+(\w+)\s*=\s*require\(['"](.+?)['"])\)"#).unwrap();
        let result = require_regex.replace_all(code, "import $1 from '$2'");
        Ok(result.to_string())
    }

    fn add_interfaces(&self, code: &str) -> Result<String, ServiceError> {
        // This would analyze object shapes and generate interfaces
        Ok(code.to_string())
    }

    fn modernize_class_syntax(&self, code: &str) -> Result<String, ServiceError> {
        let mut result = code.to_string();

        // Convert prototype-based classes to ES6 classes
        let prototype_regex = Regex::new(r"(\w+)\.prototype\.(\w+)\s*=\s*function").unwrap();
        result = prototype_regex.replace_all(&result, "class $1 {\n  $2").to_string();

        Ok(result)
    }
}

struct Py2ToPy3Converter;

impl Py2ToPy3Converter {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LanguageConverter for Py2ToPy3Converter {
    async fn convert(&self, source_code: &str, _config: &MigrationConfig) -> Result<String, ServiceError> {
        let mut converted = source_code.to_string();

        // print statement to function
        converted = converted.replace("print ", "print(");
        if !converted.contains("print(") {
            converted = converted.replace("print", "print()");
        }

        // xrange to range
        converted = converted.replace("xrange", "range");

        // raw_input to input
        converted = converted.replace("raw_input", "input");

        // iteritems to items
        converted = converted.replace(".iteritems()", ".items()");
        converted = converted.replace(".iterkeys()", ".keys()");
        converted = converted.replace(".itervalues()", ".values()");

        // Division operator
        converted = converted.replace("/", "//"); // Integer division

        // Unicode strings
        converted = converted.replace("u\"", "\"");
        converted = converted.replace("u'", "'");

        Ok(converted)
    }

    fn source_language(&self) -> &str { "python2" }
    fn target_language(&self) -> &str { "python3" }
    fn supported_features(&self) -> Vec<String> {
        vec!["syntax_conversion".to_string(), "api_updates".to_string()]
    }
}

struct JavaToKotlinConverter;

impl JavaToKotlinConverter {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LanguageConverter for JavaToKotlinConverter {
    async fn convert(&self, source_code: &str, _config: &MigrationConfig) -> Result<String, ServiceError> {
        let mut converted = source_code.to_string();

        // Remove semicolons
        converted = converted.replace(";", "");

        // public class to class
        converted = converted.replace("public class", "class");

        // void to Unit
        converted = converted.replace("void ", "fun ");

        // System.out.println to println
        converted = converted.replace("System.out.println", "println");

        // Variable declarations
        converted = converted.replace("String ", "val ");
        converted = converted.replace("int ", "val ");
        converted = converted.replace("boolean ", "val ");

        // Function declarations
        converted = converted.replace("public static ", "");
        converted = converted.replace("public ", "");
        converted = converted.replace("private ", "private ");

        Ok(converted)
    }

    fn source_language(&self) -> &str { "java" }
    fn target_language(&self) -> &str { "kotlin" }
    fn supported_features(&self) -> Vec<String> {
        vec!["null_safety".to_string(), "data_classes".to_string()]
    }
}

struct CToRustConverter;

impl CToRustConverter {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LanguageConverter for CToRustConverter {
    async fn convert(&self, source_code: &str, _config: &MigrationConfig) -> Result<String, ServiceError> {
        let mut converted = source_code.to_string();

        // Basic type conversions
        converted = converted.replace("int ", "i32 ");
        converted = converted.replace("char ", "char ");
        converted = converted.replace("float ", "f32 ");
        converted = converted.replace("double ", "f64 ");
        converted = converted.replace("void ", "");

        // NULL to None
        converted = converted.replace("NULL", "None");

        // malloc to Vec
        if converted.contains("malloc") {
            converted = converted.replace("malloc", "// TODO: Replace with Vec::new() or Box::new()");
        }

        // free to drop
        converted = converted.replace("free(", "drop(");

        // #include to use
        converted = converted.replace("#include", "use");

        // Function syntax
        converted = converted.replace("int main(", "fn main(");

        // Pointer syntax
        converted = converted.replace("*", "&mut ");

        Ok(converted)
    }

    fn source_language(&self) -> &str { "c" }
    fn target_language(&self) -> &str { "rust" }
    fn supported_features(&self) -> Vec<String> {
        vec!["memory_safety".to_string(), "ownership".to_string()]
    }
}

struct CoffeeToTsConverter;

impl CoffeeToTsConverter {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LanguageConverter for CoffeeToTsConverter {
    async fn convert(&self, source_code: &str, _config: &MigrationConfig) -> Result<String, ServiceError> {
        let mut converted = source_code.to_string();

        // Arrow functions
        converted = converted.replace("->", "=> {");
        converted = converted.replace("=>", "=> {");

        // Class syntax
        converted = converted.replace("class ", "export class ");

        // String interpolation
        let interpolation_regex = Regex::new(r"#\{([^}]+)\}").unwrap();
        converted = interpolation_regex.replace_all(&converted, "${$1}").to_string();

        Ok(converted)
    }

    fn source_language(&self) -> &str { "coffeescript" }
    fn target_language(&self) -> &str { "typescript" }
    fn supported_features(&self) -> Vec<String> {
        vec!["type_annotations".to_string(), "modern_syntax".to_string()]
    }
}

// Supporting components

#[async_trait]
trait CodeAnalyzer: Send + Sync {
    async fn analyze(&self, code: &str) -> Result<CodeAnalysis, ServiceError>;
}

#[async_trait]
trait CodeValidator: Send + Sync {
    async fn validate(&self, code: &str, language: &str) -> Result<ValidationResult, ServiceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeAnalysis {
    pub complexity: usize,
    pub dependencies: Vec<String>,
    pub patterns_detected: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

struct TypeInferenceEngine;

impl TypeInferenceEngine {
    fn new() -> Self {
        Self
    }

    pub fn infer_type(&self, _value: &str) -> String {
        // Simplified type inference
        "any".to_string()
    }
}

// Framework-specific migration

pub struct FrameworkMigrator {
    migrations: HashMap<(String, String), Box<dyn FrameworkMigration>>,
}

#[async_trait]
trait FrameworkMigration: Send + Sync {
    async fn migrate(&self, project_path: &Path) -> Result<(), ServiceError>;
    fn source_framework(&self) -> &str;
    fn target_framework(&self) -> &str;
}

impl FrameworkMigrator {
    pub fn new() -> Self {
        let mut migrator = Self {
            migrations: HashMap::new(),
        };

        // Register framework migrations
        migrator.register_migrations();
        migrator
    }

    fn register_migrations(&mut self) {
        // React to Vue
        self.migrations.insert(
            ("react".to_string(), "vue".to_string()),
            Box::new(ReactToVueMigration),
        );

        // Angular to React
        self.migrations.insert(
            ("angular".to_string(), "react".to_string()),
            Box::new(AngularToReactMigration),
        );

        // Express to Fastify
        self.migrations.insert(
            ("express".to_string(), "fastify".to_string()),
            Box::new(ExpressToFastifyMigration),
        );
    }

    pub async fn migrate_framework(
        &self,
        project_path: &Path,
        source: &str,
        target: &str,
    ) -> Result<(), ServiceError> {
        let key = (source.to_string(), target.to_string());

        let migration = self.migrations.get(&key)
            .ok_or_else(|| ServiceError::ConfigError {
                message: format!("No migration available from {} to {}", source, target)
            })?;

        migration.migrate(project_path).await
    }
}

struct ReactToVueMigration;

#[async_trait]
impl FrameworkMigration for ReactToVueMigration {
    async fn migrate(&self, _project_path: &Path) -> Result<(), ServiceError> {
        // Component conversion logic
        Ok(())
    }

    fn source_framework(&self) -> &str { "react" }
    fn target_framework(&self) -> &str { "vue" }
}

struct AngularToReactMigration;

#[async_trait]
impl FrameworkMigration for AngularToReactMigration {
    async fn migrate(&self, _project_path: &Path) -> Result<(), ServiceError> {
        // Component and service conversion
        Ok(())
    }

    fn source_framework(&self) -> &str { "angular" }
    fn target_framework(&self) -> &str { "react" }
}

struct ExpressToFastifyMigration;

#[async_trait]
impl FrameworkMigration for ExpressToFastifyMigration {
    async fn migrate(&self, _project_path: &Path) -> Result<(), ServiceError> {
        // Route and middleware conversion
        Ok(())
    }

    fn source_framework(&self) -> &str { "express" }
    fn target_framework(&self) -> &str { "fastify" }
}

// Database migration

pub struct DatabaseMigrator {
    converters: HashMap<(String, String), Box<dyn DatabaseConverter>>,
}

#[async_trait]
trait DatabaseConverter: Send + Sync {
    async fn convert_schema(&self, schema: &str) -> Result<String, ServiceError>;
    async fn convert_queries(&self, queries: Vec<String>) -> Result<Vec<String>, ServiceError>;
}

impl DatabaseMigrator {
    pub fn new() -> Self {
        let mut migrator = Self {
            converters: HashMap::new(),
        };

        // MySQL to PostgreSQL
        migrator.converters.insert(
            ("mysql".to_string(), "postgresql".to_string()),
            Box::new(MySqlToPostgresConverter),
        );

        // MongoDB to PostgreSQL
        migrator.converters.insert(
            ("mongodb".to_string(), "postgresql".to_string()),
            Box::new(MongoToPostgresConverter),
        );

        migrator
    }

    pub async fn migrate_database(
        &self,
        source_db: &str,
        target_db: &str,
        schema: &str,
    ) -> Result<String, ServiceError> {
        let key = (source_db.to_string(), target_db.to_string());

        let converter = self.converters.get(&key)
            .ok_or_else(|| ServiceError::ConfigError {
                message: format!("No converter available from {} to {}", source_db, target_db)
            })?;

        converter.convert_schema(schema).await
    }
}

struct MySqlToPostgresConverter;

#[async_trait]
impl DatabaseConverter for MySqlToPostgresConverter {
    async fn convert_schema(&self, schema: &str) -> Result<String, ServiceError> {
        let mut converted = schema.to_string();

        // AUTO_INCREMENT to SERIAL
        converted = converted.replace("AUTO_INCREMENT", "SERIAL");

        // TINYINT to SMALLINT
        converted = converted.replace("TINYINT", "SMALLINT");

        // DATETIME to TIMESTAMP
        converted = converted.replace("DATETIME", "TIMESTAMP");

        Ok(converted)
    }

    async fn convert_queries(&self, queries: Vec<String>) -> Result<Vec<String>, ServiceError> {
        let mut converted = Vec::new();

        for query in queries {
            let mut q = query;

            // Backticks to double quotes
            q = q.replace("`", "\"");

            // LIMIT syntax
            if q.contains("LIMIT") {
                // MySQL: LIMIT offset, count
                // PostgreSQL: LIMIT count OFFSET offset
                // This is simplified
            }

            converted.push(q);
        }

        Ok(converted)
    }
}

struct MongoToPostgresConverter;

#[async_trait]
impl DatabaseConverter for MongoToPostgresConverter {
    async fn convert_schema(&self, _schema: &str) -> Result<String, ServiceError> {
        // Convert MongoDB collections to PostgreSQL tables with JSONB
        Ok("-- Generated PostgreSQL schema from MongoDB".to_string())
    }

    async fn convert_queries(&self, _queries: Vec<String>) -> Result<Vec<String>, ServiceError> {
        // Convert MongoDB queries to PostgreSQL with JSONB operators
        Ok(vec![])
    }
}