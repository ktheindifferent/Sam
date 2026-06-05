//! Code analysis related models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete code analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisReport {
    pub structure: CodeStructure,
    pub metrics: CodeMetrics,
    pub suggestions: Vec<RefactoringSuggestion>,
    pub issues: Vec<CodeIssue>,
    pub summary: String,
}

/// Code structure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub modules: Vec<ModuleInfo>,
    pub dependencies: Vec<DependencyInfo>,
    pub entry_points: Vec<String>,
}

/// Function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub complexity: usize,
    pub is_async: bool,
    pub is_public: bool,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
}

/// Class information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub inheritance: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version: Option<String>,
    pub dependency_type: DependencyType,
    pub is_dev: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Library,
    Framework,
    Tool,
    Runtime,
}

/// Code metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub lines_of_code: usize,
    pub cyclomatic_complexity: usize,
    pub maintainability_index: f32,
    pub test_coverage: Option<f32>,
    pub duplicate_percentage: f32,
    pub technical_debt_hours: f32,
}

/// Refactoring suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub suggestion_type: RefactoringType,
    pub description: String,
    pub affected_lines: Vec<usize>,
    pub priority: Priority,
    pub estimated_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractMethod,
    ExtractVariable,
    InlineMethod,
    RenameSymbol,
    MoveMethod,
    SimplifyConditional,
    RemoveDuplication,
    IntroduceParameter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Code issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub issue_type: IssueType,
    pub severity: IssueSeverity,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    SyntaxError,
    TypeMismatch,
    UnusedVariable,
    UnhandledError,
    PerformanceIssue,
    SecurityVulnerability,
    CodeSmell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Complexity visualization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityVisualization {
    pub hotspots: Vec<ComplexityHotspot>,
    pub average_complexity: f32,
    pub max_complexity: usize,
    pub distribution: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityHotspot {
    pub location: String,
    pub complexity: usize,
    pub function_name: String,
    pub suggestions: Vec<String>,
}

/// Function complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub lines: usize,
    pub parameters: usize,
}
