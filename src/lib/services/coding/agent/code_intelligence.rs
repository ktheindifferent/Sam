use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Code intelligence engine for advanced analysis and refactoring
pub struct CodeIntelligence {
    analyzers: HashMap<String, Box<dyn CodeAnalyzer>>,
    refactoring_engine: RefactoringEngine,
    symbol_index: SymbolIndex,
    dependency_graph: DependencyGraph,
}

/// Trait for language-specific code analyzers
#[async_trait::async_trait]
pub trait CodeAnalyzer: Send + Sync {
    async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis>;
    async fn find_symbols(&self, path: &Path) -> Result<Vec<Symbol>>;
    async fn get_dependencies(&self, path: &Path) -> Result<Vec<Dependency>>;
    async fn suggest_improvements(&self, analysis: &FileAnalysis) -> Vec<Improvement>;
    fn supported_extensions(&self) -> Vec<&str>;
}

/// File analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub path: PathBuf,
    pub language: Language,
    pub metrics: CodeMetrics,
    pub issues: Vec<CodeIssue>,
    pub symbols: Vec<Symbol>,
    pub dependencies: Vec<Dependency>,
    pub complexity_hotspots: Vec<ComplexityHotspot>,
}

/// Programming language enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    Ruby,
    Swift,
    Kotlin,
    Unknown,
}

/// Code metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub lines_of_code: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub maintainability_index: f32,
    pub test_coverage: Option<f32>,
    pub duplicate_lines: usize,
    pub technical_debt_minutes: u32,
}

/// Code issue/smell detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub suggestion: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueCategory {
    Performance,
    Security,
    Maintainability,
    Reliability,
    Duplication,
    Complexity,
    Style,
    BestPractice,
}

/// Symbol information for navigation and refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub references: Vec<SymbolReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Variable,
    Constant,
    Property,
    Parameter,
}

/// Symbol reference for find-all-references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub is_write: bool,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub kind: DependencyKind,
    pub is_dev: bool,
    pub vulnerabilities: Vec<Vulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyKind {
    Library,
    Framework,
    Tool,
    Runtime,
}

/// Security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String, // CVE ID
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub fixed_version: Option<String>,
    pub published_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Complexity hotspot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityHotspot {
    pub function_name: String,
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub complexity: u32,
    pub recommendation: String,
}

/// Code improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub title: String,
    pub description: String,
    pub impact: ImpactLevel,
    pub effort: EffortLevel,
    pub category: IssueCategory,
    pub examples: Vec<CodeExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffortLevel {
    Trivial,
    Easy,
    Medium,
    Hard,
}

/// Code example for improvements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub before: String,
    pub after: String,
    pub explanation: String,
}

/// Refactoring engine
pub struct RefactoringEngine {
    refactorings: HashMap<String, Box<dyn Refactoring>>,
}

/// Trait for refactoring operations
#[async_trait::async_trait]
pub trait Refactoring: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn is_applicable(&self, context: &RefactoringContext) -> bool;
    async fn preview(&self, context: &RefactoringContext) -> Result<RefactoringPreview>;
    async fn apply(&self, context: &RefactoringContext) -> Result<RefactoringResult>;
}

/// Refactoring context
#[derive(Debug, Clone)]
pub struct RefactoringContext {
    pub file: PathBuf,
    pub selection_start: Position,
    pub selection_end: Position,
    pub symbol: Option<Symbol>,
    pub scope: RefactoringScope,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefactoringScope {
    File,
    Function,
    Class,
    Module,
    Project,
}

/// Refactoring preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringPreview {
    pub changes: Vec<FileChange>,
    pub affected_files: Vec<PathBuf>,
    pub estimated_impact: String,
}

/// File change for refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file: PathBuf,
    pub hunks: Vec<ChangeHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeHunk {
    pub start_line: usize,
    pub end_line: usize,
    pub old_text: String,
    pub new_text: String,
}

/// Refactoring result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringResult {
    pub success: bool,
    pub files_modified: Vec<PathBuf>,
    pub symbols_renamed: Vec<String>,
    pub message: String,
}

/// Symbol index for fast lookups
pub struct SymbolIndex {
    symbols: HashMap<String, Vec<Symbol>>,
    file_symbols: HashMap<PathBuf, Vec<Symbol>>,
    references: HashMap<String, Vec<SymbolReference>>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            file_symbols: HashMap::new(),
            references: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(symbol.clone());

        self.file_symbols
            .entry(symbol.file.clone())
            .or_insert_with(Vec::new)
            .push(symbol);
    }

    pub fn find_symbol(&self, name: &str) -> Option<&Vec<Symbol>> {
        self.symbols.get(name)
    }

    pub fn find_references(&self, symbol_name: &str) -> Option<&Vec<SymbolReference>> {
        self.references.get(symbol_name)
    }

    pub fn get_file_symbols(&self, file: &Path) -> Option<&Vec<Symbol>> {
        self.file_symbols.get(file)
    }
}

/// Dependency graph for understanding project structure
pub struct DependencyGraph {
    nodes: HashMap<String, DependencyNode>,
    edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub name: String,
    pub kind: NodeKind,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    File,
    Module,
    Package,
    Function,
}

#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Imports,
    Calls,
    Extends,
    Implements,
    Uses,
}

impl CodeIntelligence {
    pub fn new() -> Self {
        Self {
            analyzers: HashMap::new(),
            refactoring_engine: RefactoringEngine::new(),
            symbol_index: SymbolIndex::new(),
            dependency_graph: DependencyGraph::new(),
        }
    }

    /// Register a language analyzer
    pub fn register_analyzer(&mut self, language: String, analyzer: Box<dyn CodeAnalyzer>) {
        self.analyzers.insert(language, analyzer);
    }

    /// Analyze a file
    pub async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        for (_, analyzer) in &self.analyzers {
            if analyzer.supported_extensions().contains(&extension) {
                return analyzer.analyze_file(path).await;
            }
        }

        Err(anyhow::anyhow!(
            "No analyzer found for file type: {}",
            extension
        ))
    }

    /// Find all references to a symbol
    pub async fn find_all_references(&self, symbol_name: &str) -> Vec<SymbolReference> {
        self.symbol_index
            .find_references(symbol_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get refactoring suggestions for a context
    pub async fn get_refactorings(&self, context: &RefactoringContext) -> Vec<String> {
        let mut available = Vec::new();

        for (name, refactoring) in &self.refactoring_engine.refactorings {
            if refactoring.is_applicable(context).await {
                available.push(name.clone());
            }
        }

        available
    }

    /// Preview a refactoring
    pub async fn preview_refactoring(
        &self,
        refactoring_name: &str,
        context: &RefactoringContext,
    ) -> Result<RefactoringPreview> {
        self.refactoring_engine
            .preview(refactoring_name, context)
            .await
    }

    /// Apply a refactoring
    pub async fn apply_refactoring(
        &self,
        refactoring_name: &str,
        context: &RefactoringContext,
    ) -> Result<RefactoringResult> {
        self.refactoring_engine
            .apply(refactoring_name, context)
            .await
    }

    /// Get code complexity report for a project
    pub async fn get_complexity_report(&self, project_path: &Path) -> Result<ComplexityReport> {
        let mut report = ComplexityReport::default();

        // Walk through all source files
        let files = self.find_source_files(project_path).await?;

        for file in files {
            if let Ok(analysis) = self.analyze_file(&file).await {
                report.total_files += 1;
                report.total_lines += analysis.metrics.lines_of_code;
                report.average_complexity += analysis.metrics.cyclomatic_complexity as f32;

                for hotspot in analysis.complexity_hotspots {
                    if hotspot.complexity > 10 {
                        report.high_complexity_functions.push(hotspot);
                    }
                }

                report
                    .issues_by_severity
                    .entry(IssueSeverity::Error)
                    .or_insert(0);
                report
                    .issues_by_severity
                    .entry(IssueSeverity::Warning)
                    .or_insert(0);

                for issue in analysis.issues {
                    *report.issues_by_severity.entry(issue.severity).or_insert(0) += 1;
                }
            }
        }

        if report.total_files > 0 {
            report.average_complexity /= report.total_files as f32;
        }

        Ok(report)
    }

    /// Find all source files in a project
    async fn find_source_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![path.to_path_buf()];

        while let Some(current) = stack.pop() {
            if current.is_dir() {
                let mut entries = fs::read_dir(&current).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.is_dir()
                        && !path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .starts_with('.')
                    {
                        stack.push(path);
                    } else if self.is_source_file(&path) {
                        files.push(path);
                    }
                }
            }
        }

        Ok(files)
    }

    fn is_source_file(&self, path: &Path) -> bool {
        let extensions = [
            "rs", "py", "js", "ts", "go", "java", "cs", "cpp", "rb", "swift", "kt",
        ];
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.contains(&e))
            .unwrap_or(false)
    }
}

/// Complexity report for a project
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplexityReport {
    pub total_files: usize,
    pub total_lines: usize,
    pub average_complexity: f32,
    pub high_complexity_functions: Vec<ComplexityHotspot>,
    pub issues_by_severity: HashMap<IssueSeverity, usize>,
}

impl RefactoringEngine {
    pub fn new() -> Self {
        Self {
            refactorings: HashMap::new(),
        }
    }

    pub fn register(&mut self, refactoring: Box<dyn Refactoring>) {
        self.refactorings
            .insert(refactoring.name().to_string(), refactoring);
    }

    pub async fn preview(
        &self,
        name: &str,
        context: &RefactoringContext,
    ) -> Result<RefactoringPreview> {
        self.refactorings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Refactoring '{}' not found", name))?
            .preview(context)
            .await
    }

    pub async fn apply(
        &self,
        name: &str,
        context: &RefactoringContext,
    ) -> Result<RefactoringResult> {
        self.refactorings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Refactoring '{}' not found", name))?
            .apply(context)
            .await
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: DependencyNode) {
        self.nodes.insert(node.name.clone(), node);
    }

    pub fn add_edge(&mut self, edge: DependencyEdge) {
        self.edges.push(edge);
    }

    pub fn find_dependencies(&self, node_name: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.from == node_name)
            .map(|e| e.to.clone())
            .collect()
    }

    pub fn find_dependents(&self, node_name: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.to == node_name)
            .map(|e| e.from.clone())
            .collect()
    }

    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                self.dfs_cycle_detection(
                    node,
                    &mut visited,
                    &mut stack,
                    &mut Vec::new(),
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_cycle_detection(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        path.push(node.to_string());

        for dep in self.find_dependencies(node) {
            if !visited.contains(&dep) {
                self.dfs_cycle_detection(&dep, visited, stack, path, cycles);
            } else if stack.contains(&dep) {
                // Found a cycle
                let cycle_start = path.iter().position(|n| n == &dep).unwrap();
                cycles.push(path[cycle_start..].to_vec());
            }
        }

        stack.remove(node);
        path.pop();
    }
}
