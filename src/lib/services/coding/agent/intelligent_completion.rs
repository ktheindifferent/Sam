use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::services::coding::agent::{
    code_intelligence::{Symbol, SymbolKind},
    code_review::CodeLocation,
    errors::{CodingAgentError, CodingAgentResult},
    gpu_offload::GpuOffloadManager,
};

use super::traits::provider::LLMProvider;

/// Intelligent code completion engine with deep context awareness
pub struct IntelligentCompletionEngine {
    llm_provider: Box<dyn LLMProvider>,
    gpu_manager: Option<GpuOffloadManager>,
    context_analyzer: ContextAnalyzer,
    symbol_resolver: SymbolResolver,
    type_inference: TypeInferenceEngine,
    pattern_matcher: PatternMatcher,
    cache: CompletionCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub file_path: PathBuf,
    pub position: Position,
    pub trigger_character: Option<char>,
    pub context: CodeContext,
    pub max_suggestions: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    pub current_line: String,
    pub preceding_lines: Vec<String>,
    pub following_lines: Vec<String>,
    pub file_content: String,
    pub language: String,
    pub project_context: ProjectContext,
    pub semantic_context: SemanticContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: ProjectType,
    pub dependencies: Vec<Dependency>,
    pub imported_modules: Vec<String>,
    pub available_symbols: Vec<Symbol>,
    pub recent_edits: Vec<Edit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    RustCargo,
    NodeNpm,
    PythonPip,
    GoMod,
    JavaMaven,
    JavaGradle,
    DotNetNuget,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub exports: Vec<Export>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportKind {
    Function,
    Class,
    Interface,
    Type,
    Constant,
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    pub timestamp: DateTime<Utc>,
    pub location: CodeLocation,
    pub edit_type: EditType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditType {
    Insert,
    Delete,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub current_scope: Scope,
    pub parent_scopes: Vec<Scope>,
    pub local_variables: Vec<Variable>,
    pub available_types: Vec<TypeInfo>,
    pub control_flow: ControlFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub scope_type: ScopeType,
    pub name: String,
    pub start: Position,
    pub end: Position,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScopeType {
    Function,
    Method,
    Class,
    Module,
    Block,
    Loop,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub var_type: Option<TypeInfo>,
    pub mutable: bool,
    pub initialized: bool,
    pub used: bool,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub generics: Vec<TypeInfo>,
    pub nullable: bool,
    pub array_dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Primitive,
    Class,
    Interface,
    Enum,
    Generic,
    Function,
    Union,
    Intersection,
    Tuple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlow {
    pub in_loop: bool,
    pub in_conditional: bool,
    pub in_try_catch: bool,
    pub return_type: Option<TypeInfo>,
    pub can_break: bool,
    pub can_continue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub suggestions: Vec<CompletionSuggestion>,
    pub metadata: CompletionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSuggestion {
    pub text: String,
    pub insert_text: String,
    pub display_text: String,
    pub kind: CompletionKind,
    pub priority: f32,
    pub confidence: f32,
    pub documentation: Option<String>,
    pub signature: Option<String>,
    pub snippet: Option<String>,
    pub edit_range: Range,
    pub additional_edits: Vec<TextEdit>,
    pub metadata: SuggestionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionKind {
    Variable,
    Function,
    Method,
    Class,
    Interface,
    Module,
    Property,
    Keyword,
    Snippet,
    Type,
    Constant,
    Operator,
    Parameter,
    Template,
    AIGenerated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    pub source: CompletionSource,
    pub requires_import: bool,
    pub import_statement: Option<String>,
    pub is_deprecated: bool,
    pub performance_impact: PerformanceImpact,
    pub usage_frequency: f32,
    pub learned_from_patterns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionSource {
    LocalScope,
    FileScope,
    ProjectScope,
    Library,
    AIModel,
    HistoricalPatterns,
    TeamPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceImpact {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionMetadata {
    pub total_suggestions: usize,
    pub filtered_count: usize,
    pub inference_time_ms: u64,
    pub cache_hit: bool,
    pub gpu_accelerated: bool,
}

/// Context analyzer for understanding code context
pub struct ContextAnalyzer {
    ast_cache: HashMap<PathBuf, ASTCache>,
}

#[derive(Clone)]
pub struct ASTCache {
    pub ast: serde_json::Value,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportStatement>,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    pub module: String,
    pub symbols: Vec<String>,
    pub alias: Option<String>,
}

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self {
            ast_cache: HashMap::new(),
        }
    }

    pub async fn analyze_context(
        &mut self,
        request: &CompletionRequest,
    ) -> CodingAgentResult<AnalyzedContext> {
        // Parse AST if not cached
        let ast = self.get_or_parse_ast(&request.file_path).await?;

        // Extract relevant context
        let current_scope = self.find_current_scope(&ast, &request.position)?;
        let available_symbols = self.extract_available_symbols(&ast, &current_scope)?;
        let type_context = self.extract_type_context(&ast, &request.position)?;

        // Analyze patterns
        let patterns = self.analyze_patterns(&request.context)?;

        Ok(AnalyzedContext {
            ast,
            current_scope,
            available_symbols,
            type_context,
            patterns,
        })
    }

    async fn get_or_parse_ast(&mut self, path: &PathBuf) -> CodingAgentResult<serde_json::Value> {
        // Check cache
        if let Some(cached) = self.ast_cache.get(path) {
            return Ok(cached.ast.clone());
        }

        // Parse file
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| CodingAgentError::IoError {
                    message: e.to_string(),
                    path: Some(path.to_path_buf()),
                })?;

        // This would use language-specific parser
        let ast = self.parse_to_ast(&content, path)?;

        // Cache result
        self.ast_cache.insert(
            path.clone(),
            ASTCache {
                ast: ast.clone(),
                symbols: Vec::new(),
                imports: Vec::new(),
                last_modified: Utc::now(),
            },
        );

        Ok(ast)
    }

    fn parse_to_ast(
        &self,
        _content: &str,
        _path: &PathBuf,
    ) -> CodingAgentResult<serde_json::Value> {
        // Simplified - would use tree-sitter or language-specific parser
        Ok(serde_json::json!({}))
    }

    fn find_current_scope(
        &self,
        _ast: &serde_json::Value,
        _position: &Position,
    ) -> CodingAgentResult<Scope> {
        Ok(Scope {
            scope_type: ScopeType::Function,
            name: "current_function".to_string(),
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 100,
                column: 0,
            },
            symbols: Vec::new(),
        })
    }

    fn extract_available_symbols(
        &self,
        _ast: &serde_json::Value,
        _scope: &Scope,
    ) -> CodingAgentResult<Vec<Symbol>> {
        Ok(Vec::new())
    }

    fn extract_type_context(
        &self,
        _ast: &serde_json::Value,
        _position: &Position,
    ) -> CodingAgentResult<TypeContext> {
        Ok(TypeContext {
            expected_type: None,
            available_types: Vec::new(),
            type_constraints: Vec::new(),
        })
    }

    fn analyze_patterns(&self, _context: &CodeContext) -> CodingAgentResult<Vec<Pattern>> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct AnalyzedContext {
    pub ast: serde_json::Value,
    pub current_scope: Scope,
    pub available_symbols: Vec<Symbol>,
    pub type_context: TypeContext,
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Clone)]
pub struct TypeContext {
    pub expected_type: Option<TypeInfo>,
    pub available_types: Vec<TypeInfo>,
    pub type_constraints: Vec<TypeConstraint>,
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub constraint_type: ConstraintType,
    pub target_type: TypeInfo,
}

#[derive(Debug, Clone)]
pub enum ConstraintType {
    MustExtend,
    MustImplement,
    MustBe,
    MustNotBe,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub pattern_type: PatternType,
    pub frequency: f32,
    pub context: String,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    MethodCall,
    VariableDeclaration,
    ControlFlow,
    ErrorHandling,
    DataStructure,
}

/// Symbol resolver for finding and resolving symbols
pub struct SymbolResolver {
    symbol_index: BTreeMap<String, Vec<ResolvedSymbol>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub name: String,
    pub fully_qualified_name: String,
    pub symbol_type: SymbolType,
    pub source: SymbolSource,
    pub signature: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Variable,
    Function,
    Class,
    Interface,
    Type,
    Constant,
    Module,
}

#[derive(Debug, Clone)]
pub enum SymbolSource {
    Local,
    Project,
    Library(String),
    BuiltIn,
}

impl SymbolResolver {
    pub fn new() -> Self {
        Self {
            symbol_index: BTreeMap::new(),
        }
    }

    pub async fn resolve(
        &self,
        name: &str,
        context: &AnalyzedContext,
    ) -> CodingAgentResult<Vec<ResolvedSymbol>> {
        let mut results = Vec::new();

        // Check local scope
        if let Some(local_symbols) = self.symbol_index.get(name) {
            results.extend(local_symbols.clone());
        }

        // Filter by context
        results.retain(|s| self.is_accessible(s, context));

        Ok(results)
    }

    fn is_accessible(&self, _symbol: &ResolvedSymbol, _context: &AnalyzedContext) -> bool {
        // Check if symbol is accessible in current context
        true
    }
}

/// Type inference engine
pub struct TypeInferenceEngine {
    type_rules: HashMap<String, TypeRule>,
}

#[derive(Debug, Clone)]
pub struct TypeRule {
    pub pattern: String,
    pub inferred_type: TypeInfo,
    pub confidence: f32,
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        Self {
            type_rules: HashMap::new(),
        }
    }

    pub async fn infer_type(
        &self,
        expression: &str,
        context: &AnalyzedContext,
    ) -> CodingAgentResult<Option<TypeInfo>> {
        // Use type rules and context to infer type
        Ok(None)
    }
}

/// Pattern matcher for code patterns
pub struct PatternMatcher {
    patterns: Vec<CodePattern>,
}

#[derive(Debug, Clone)]
pub struct CodePattern {
    pub name: String,
    pub pattern: String,
    pub suggestion_template: String,
    pub frequency: f32,
}

impl PatternMatcher {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub async fn match_patterns(&self, context: &CodeContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            if self.matches_pattern(&context.current_line, &pattern.pattern) {
                matches.push(PatternMatch {
                    pattern: pattern.clone(),
                    confidence: pattern.frequency,
                });
            }
        }

        matches
    }

    fn matches_pattern(&self, _text: &str, _pattern: &str) -> bool {
        // Implement pattern matching
        false
    }
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern: CodePattern,
    pub confidence: f32,
}

/// Completion cache for fast lookups
pub struct CompletionCache {
    cache: HashMap<String, CachedCompletion>,
    max_size: usize,
}

#[derive(Debug, Clone)]
pub struct CachedCompletion {
    pub request_hash: String,
    pub suggestions: Vec<CompletionSuggestion>,
    pub timestamp: DateTime<Utc>,
}

impl CompletionCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&self, request: &CompletionRequest) -> Option<Vec<CompletionSuggestion>> {
        let hash = self.hash_request(request);
        self.cache.get(&hash).map(|c| c.suggestions.clone())
    }

    pub fn put(&mut self, request: &CompletionRequest, suggestions: Vec<CompletionSuggestion>) {
        if self.cache.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest_key) = self.find_oldest_key() {
                self.cache.remove(&oldest_key);
            }
        }

        let hash = self.hash_request(request);
        self.cache.insert(
            hash.clone(),
            CachedCompletion {
                request_hash: hash,
                suggestions,
                timestamp: Utc::now(),
            },
        );
    }

    fn hash_request(&self, request: &CompletionRequest) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", request).as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn find_oldest_key(&self) -> Option<String> {
        self.cache
            .iter()
            .min_by_key(|(_, v)| v.timestamp)
            .map(|(k, _)| k.clone())
    }
}

impl IntelligentCompletionEngine {
    pub fn new(llm_provider: Box<dyn LLMProvider>, gpu_manager: Option<GpuOffloadManager>) -> Self {
        Self {
            llm_provider,
            gpu_manager,
            context_analyzer: ContextAnalyzer::new(),
            symbol_resolver: SymbolResolver::new(),
            type_inference: TypeInferenceEngine::new(),
            pattern_matcher: PatternMatcher::new(),
            cache: CompletionCache::new(1000),
        }
    }

    pub async fn get_completions(
        &mut self,
        request: CompletionRequest,
    ) -> CodingAgentResult<CompletionResponse> {
        let start_time = std::time::Instant::now();

        // Check cache
        if let Some(cached) = self.cache.get(&request) {
            let total_suggestions = cached.len();
            return Ok(CompletionResponse {
                suggestions: cached,
                metadata: CompletionMetadata {
                    total_suggestions,
                    filtered_count: 0,
                    inference_time_ms: 0,
                    cache_hit: true,
                    gpu_accelerated: false,
                },
            });
        }

        // Analyze context
        let analyzed_context = self.context_analyzer.analyze_context(&request).await?;

        let mut suggestions = Vec::new();

        // Get symbol-based completions
        let symbol_completions = self
            .get_symbol_completions(&request, &analyzed_context)
            .await?;
        suggestions.extend(symbol_completions);

        // Get pattern-based completions
        let pattern_completions = self
            .get_pattern_completions(&request, &analyzed_context)
            .await?;
        suggestions.extend(pattern_completions);

        // Get AI-generated completions
        if let Some(gpu_manager) = &self.gpu_manager {
            let ai_completions = self
                .get_ai_completions(&request, &analyzed_context, gpu_manager)
                .await?;
            suggestions.extend(ai_completions);
        }

        // Rank and filter suggestions
        suggestions = self.rank_suggestions(suggestions, &analyzed_context);
        suggestions.truncate(request.max_suggestions);

        // Cache results
        self.cache.put(&request, suggestions.clone());

        let inference_time = start_time.elapsed().as_millis() as u64;
        let total_suggestions = suggestions.len();

        Ok(CompletionResponse {
            suggestions,
            metadata: CompletionMetadata {
                total_suggestions,
                filtered_count: 0,
                inference_time_ms: inference_time,
                cache_hit: false,
                gpu_accelerated: self.gpu_manager.is_some(),
            },
        })
    }

    async fn get_symbol_completions(
        &self,
        request: &CompletionRequest,
        context: &AnalyzedContext,
    ) -> CodingAgentResult<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();

        // Get prefix for completion
        let prefix = self.extract_prefix(&request.context.current_line, request.position.column);

        // Resolve symbols matching prefix
        for symbol in &context.available_symbols {
            if symbol.name.starts_with(&prefix) {
                suggestions.push(self.symbol_to_suggestion(symbol, &prefix)?);
            }
        }

        Ok(suggestions)
    }

    async fn get_pattern_completions(
        &self,
        request: &CompletionRequest,
        context: &AnalyzedContext,
    ) -> CodingAgentResult<Vec<CompletionSuggestion>> {
        let matches = self.pattern_matcher.match_patterns(&request.context).await;
        let mut suggestions = Vec::new();

        for pattern_match in matches {
            suggestions.push(self.pattern_to_suggestion(pattern_match)?);
        }

        Ok(suggestions)
    }

    async fn get_ai_completions(
        &self,
        request: &CompletionRequest,
        context: &AnalyzedContext,
        gpu_manager: &GpuOffloadManager,
    ) -> CodingAgentResult<Vec<CompletionSuggestion>> {
        // Build prompt for AI completion
        let prompt = self.build_ai_prompt(request, context)?;

        // Use GPU-accelerated inference if available
        let session_id = "completion";
        let completion_text = if let Ok(_) = gpu_manager.start_gpu_instance(session_id).await {
            match gpu_manager
                .generate_code(session_id, &prompt, Some("deepseek-coder:33b".to_string()))
                .await
            {
                Ok(code) => code,
                Err(_) => self
                    .llm_provider
                    .generate_response(&prompt, "gpt-4")
                    .await
                    .unwrap_or_default(),
            }
        } else {
            self.llm_provider
                .generate_response(&prompt, "gpt-4")
                .await?
        };

        // Parse AI response into suggestions
        let suggestions = self.parse_ai_completions(&completion_text)?;

        Ok(suggestions)
    }

    fn build_ai_prompt(
        &self,
        request: &CompletionRequest,
        _context: &AnalyzedContext,
    ) -> CodingAgentResult<String> {
        Ok(format!(
            "Complete the following {} code:\n\n{}\n[CURSOR]\n{}\n\nProvide the completion:",
            request.context.language,
            request.context.preceding_lines.join("\n"),
            request.context.current_line
        ))
    }

    fn parse_ai_completions(&self, text: &str) -> CodingAgentResult<Vec<CompletionSuggestion>> {
        // Parse AI-generated text into completion suggestions
        Ok(vec![CompletionSuggestion {
            text: text.to_string(),
            insert_text: text.to_string(),
            display_text: text.lines().next().unwrap_or("").to_string(),
            kind: CompletionKind::AIGenerated,
            priority: 0.9,
            confidence: 0.8,
            documentation: Some("AI-generated completion".to_string()),
            signature: None,
            snippet: None,
            edit_range: Range {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
            additional_edits: Vec::new(),
            metadata: SuggestionMetadata {
                source: CompletionSource::AIModel,
                requires_import: false,
                import_statement: None,
                is_deprecated: false,
                performance_impact: PerformanceImpact::None,
                usage_frequency: 0.0,
                learned_from_patterns: true,
            },
        }])
    }

    fn extract_prefix(&self, line: &str, column: usize) -> String {
        let prefix_start = line[..column.min(line.len())]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        line[prefix_start..column.min(line.len())].to_string()
    }

    fn symbol_to_suggestion(
        &self,
        symbol: &Symbol,
        prefix: &str,
    ) -> CodingAgentResult<CompletionSuggestion> {
        Ok(CompletionSuggestion {
            text: symbol.name.clone(),
            insert_text: symbol.name[prefix.len()..].to_string(),
            display_text: symbol.name.clone(),
            kind: self.symbol_kind_to_completion_kind(&symbol.kind),
            priority: 0.8,
            confidence: 0.9,
            documentation: symbol.documentation.clone(),
            signature: symbol.signature.clone(),
            snippet: None,
            edit_range: Range {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
            additional_edits: Vec::new(),
            metadata: SuggestionMetadata {
                source: CompletionSource::LocalScope,
                requires_import: false,
                import_statement: None,
                is_deprecated: false,
                performance_impact: PerformanceImpact::None,
                usage_frequency: 0.5,
                learned_from_patterns: false,
            },
        })
    }

    fn symbol_kind_to_completion_kind(&self, kind: &SymbolKind) -> CompletionKind {
        match kind {
            SymbolKind::Function => CompletionKind::Function,
            SymbolKind::Struct => CompletionKind::Class,
            SymbolKind::Enum => CompletionKind::Type,
            SymbolKind::Interface => CompletionKind::Interface,
            SymbolKind::Module => CompletionKind::Module,
            SymbolKind::Constant => CompletionKind::Constant,
            SymbolKind::Variable => CompletionKind::Variable,
            SymbolKind::Class => CompletionKind::Class,
            SymbolKind::Method => CompletionKind::Function,
            SymbolKind::Property => CompletionKind::Property,
            SymbolKind::Parameter => CompletionKind::Variable,
        }
    }

    fn pattern_to_suggestion(
        &self,
        pattern_match: PatternMatch,
    ) -> CodingAgentResult<CompletionSuggestion> {
        Ok(CompletionSuggestion {
            text: pattern_match.pattern.suggestion_template.clone(),
            insert_text: pattern_match.pattern.suggestion_template.clone(),
            display_text: pattern_match.pattern.name.clone(),
            kind: CompletionKind::Snippet,
            priority: 0.7,
            confidence: pattern_match.confidence,
            documentation: Some(format!("Pattern: {}", pattern_match.pattern.name)),
            signature: None,
            snippet: Some(pattern_match.pattern.suggestion_template.clone()),
            edit_range: Range {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
            additional_edits: Vec::new(),
            metadata: SuggestionMetadata {
                source: CompletionSource::HistoricalPatterns,
                requires_import: false,
                import_statement: None,
                is_deprecated: false,
                performance_impact: PerformanceImpact::None,
                usage_frequency: pattern_match.pattern.frequency,
                learned_from_patterns: true,
            },
        })
    }

    fn rank_suggestions(
        &self,
        mut suggestions: Vec<CompletionSuggestion>,
        context: &AnalyzedContext,
    ) -> Vec<CompletionSuggestion> {
        // Sort by priority and confidence
        suggestions.sort_by(|a, b| {
            let a_score = a.priority * a.confidence;
            let b_score = b.priority * b.confidence;
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        suggestions
    }
}
