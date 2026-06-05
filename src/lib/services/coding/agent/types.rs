use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk level assessment for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,     // No risk operations (ls, cat, etc.)
    Low,      // Minor modifications (formatting, simple edits)
    Medium,   // Significant changes (dependency additions, file creation)
    High,     // Potentially destructive (file deletion, major refactoring)
    Critical, // System-level operations (chmod, rm -rf, etc.)
}

/// Code execution request with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionRequest {
    pub command: String,
    pub require_confirmation: bool,
    pub explanation: String,
    pub risk_level: RiskLevel,
    pub estimated_duration: Option<u32>, // seconds
    pub prerequisites: Vec<String>,
    pub expected_outputs: Vec<String>,
}

/// Coding agent response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentResponse {
    pub response_text: String,
    pub suggested_commands: Vec<CodeExecutionRequest>,
    pub model_used: String,
    pub context_used: usize,
}

/// Command history entry for tracking executed commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub command: String,
    pub timestamp: u64,
    pub success: bool,
    pub output: String,
    pub working_directory: String,
}

/// Project type detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Unknown,
}

/// Build system detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Yarn,
    Make,
    Gradle,
    Maven,
    Poetry,
    Unknown,
}

/// Project structure analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub project_type: ProjectType,
    pub root_directory: String,
    pub source_files: Vec<String>,
    pub config_files: Vec<String>,
    pub test_files: Vec<String>,
    pub dependencies: Vec<String>,
    pub git_repository: bool,
    pub build_system: BuildSystem,
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
    pub variable_type: VariableType,
}

/// Variable type for template substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Integer,
    Boolean,
    List,
    Enum(Vec<String>),
}

/// Code template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTemplate {
    pub name: String,
    pub description: String,
    pub language: ProjectType,
    pub template_content: String,
    pub variables: Vec<TemplateVariable>,
    pub dependencies: Vec<String>,
    pub use_cases: Vec<String>,
}

/// Refactoring suggestion types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractFunction,
    ExtractVariable,
    RenameSymbol,
    MoveFunction,
    SplitFile,
    MergeFiles,
    AddError,
    OptimizeImports,
    UpdateDependencies,
    AddDocumentation,
}

/// Impact assessment for refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub lines_affected: usize,
    pub files_affected: usize,
    pub breaking_changes: bool,
    pub test_updates_required: bool,
    pub documentation_updates_required: bool,
}

/// Refactoring suggestion with impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub suggestion_type: RefactoringType,
    pub description: String,
    pub affected_files: Vec<String>,
    pub commands: Vec<CodeExecutionRequest>,
    pub confidence_score: f32, // 0.0 to 1.0
    pub impact_assessment: ImpactAssessment,
}

/// Performance metrics for the coding agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_commands_executed: u64,
    pub average_execution_time: f64,
    pub success_rate: f64,
    pub most_used_commands: HashMap<String, u32>,
    pub error_patterns: HashMap<String, u32>,
}

/// Learning metrics for adaptive behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub command_success_patterns: HashMap<String, f32>, // command -> success rate
    pub user_preferences: HashMap<String, String>,      // preference -> value
    pub task_completion_times: HashMap<String, Vec<u64>>, // task_type -> [completion_times]
    pub error_resolution_patterns: HashMap<String, Vec<String>>, // error_pattern -> [successful_resolutions]
}
