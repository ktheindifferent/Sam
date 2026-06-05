//! Generator traits

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Trait for code generation
#[async_trait]
pub trait CodeGenerator: Send + Sync {
    /// Generate code from template
    async fn generate_from_template(
        &self,
        template: &str,
        variables: serde_json::Value,
    ) -> Result<String>;

    /// Generate code from specification
    async fn generate_from_spec(&self, spec: CodeSpec) -> Result<GeneratedCode>;

    /// Generate boilerplate
    async fn generate_boilerplate(&self, project_type: &str) -> Result<ProjectScaffold>;

    /// Get available templates
    async fn list_templates(&self) -> Result<Vec<Template>>;
}

#[derive(Debug, Clone)]
pub struct CodeSpec {
    pub language: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub examples: Vec<CodeExample>,
}

#[derive(Debug, Clone)]
pub struct CodeExample {
    pub input: String,
    pub output: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub code: String,
    pub language: String,
    pub files: Vec<GeneratedFile>,
    pub dependencies: Vec<String>,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub file_type: FileType,
}

#[derive(Debug, Clone)]
pub enum FileType {
    Source,
    Test,
    Config,
    Documentation,
    Asset,
}

#[derive(Debug, Clone)]
pub struct ProjectScaffold {
    pub name: String,
    pub structure: Vec<ScaffoldItem>,
    pub dependencies: serde_json::Value,
    pub scripts: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ScaffoldItem {
    pub path: String,
    pub item_type: ScaffoldItemType,
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ScaffoldItemType {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

/// Trait for documentation generation
#[async_trait]
pub trait DocumentationGenerator: Send + Sync {
    /// Generate documentation from code
    async fn generate_docs(&self, path: &Path) -> Result<Documentation>;

    /// Generate API documentation
    async fn generate_api_docs(&self, path: &Path) -> Result<ApiDocumentation>;

    /// Generate README
    async fn generate_readme(&self, path: &Path) -> Result<String>;

    /// Update existing documentation
    async fn update_docs(&self, path: &Path, docs: Documentation) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Documentation {
    pub modules: Vec<ModuleDoc>,
    pub functions: Vec<FunctionDoc>,
    pub types: Vec<TypeDoc>,
    pub examples: Vec<ExampleDoc>,
}

#[derive(Debug, Clone)]
pub struct ModuleDoc {
    pub name: String,
    pub description: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionDoc {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterDoc>,
    pub returns: Option<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParameterDoc {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct TypeDoc {
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDoc>,
    pub methods: Vec<FunctionDoc>,
}

#[derive(Debug, Clone)]
pub struct FieldDoc {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ExampleDoc {
    pub title: String,
    pub description: String,
    pub code: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiDocumentation {
    pub endpoints: Vec<ApiEndpoint>,
    pub models: Vec<ApiModel>,
    pub authentication: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub description: String,
    pub parameters: Vec<ApiParameter>,
    pub request_body: Option<serde_json::Value>,
    pub responses: Vec<ApiResponse>,
}

#[derive(Debug, Clone)]
pub struct ApiParameter {
    pub name: String,
    pub location: String, // "path", "query", "header"
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: String,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ApiModel {
    pub name: String,
    pub description: String,
    pub properties: serde_json::Value,
}

/// Trait for test generation
#[async_trait]
pub trait TestGenerator: Send + Sync {
    /// Generate unit tests
    async fn generate_unit_tests(&self, path: &Path) -> Result<Vec<GeneratedTest>>;

    /// Generate integration tests
    async fn generate_integration_tests(&self, path: &Path) -> Result<Vec<GeneratedTest>>;

    /// Generate test fixtures
    async fn generate_fixtures(&self, path: &Path) -> Result<Vec<TestFixture>>;

    /// Generate property-based tests
    async fn generate_property_tests(&self, path: &Path) -> Result<Vec<GeneratedTest>>;
}

#[derive(Debug, Clone)]
pub struct GeneratedTest {
    pub name: String,
    pub test_type: TestType,
    pub code: String,
    pub setup: Option<String>,
    pub teardown: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Property,
    Benchmark,
}

#[derive(Debug, Clone)]
pub struct TestFixture {
    pub name: String,
    pub data: serde_json::Value,
    pub setup_code: Option<String>,
}
