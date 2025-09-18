use std::collections::{HashMap, HashSet, BinaryHeap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::cmp::Ordering;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::fs;
use regex::Regex;

use super::errors::CodingAgentError as ServiceError;
use super::providers::LLMProvider;

// AI-Powered Code Search and Navigation System

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query_text: String,
    pub query_type: QueryType,
    pub filters: SearchFilters,
    pub semantic_search: bool,
    pub fuzzy_matching: bool,
    pub max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    NaturalLanguage,
    Regex,
    Ast,
    Symbol,
    Reference,
    Definition,
    Implementation,
    Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub file_types: Vec<String>,
    pub directories: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub date_range: Option<(std::time::SystemTime, std::time::SystemTime)>,
    pub author: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub relevance_score: f64,
    pub location: CodeLocation,
    pub snippet: CodeSnippet,
    pub context: SearchContext,
    pub metadata: ResultMetadata,
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.relevance_score.partial_cmp(&other.relevance_score)
            .unwrap_or(Ordering::Equal)
            .reverse()
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.relevance_score == other.relevance_score
    }
}

impl Eq for SearchResult {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub code: String,
    pub language: String,
    pub highlighted_regions: Vec<HighlightRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRegion {
    pub start: usize,
    pub end: usize,
    pub highlight_type: HighlightType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HighlightType {
    Match,
    Context,
    Reference,
    Definition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub surrounding_lines: Vec<String>,
    pub function_context: Option<FunctionContext>,
    pub class_context: Option<ClassContext>,
    pub module_context: Option<ModuleContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionContext {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassContext {
    pub name: String,
    pub base_classes: Vec<String>,
    pub methods: Vec<String>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleContext {
    pub name: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMetadata {
    pub file_size: usize,
    pub last_modified: std::time::SystemTime,
    pub git_info: Option<GitInfo>,
    pub complexity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub last_commit: String,
    pub author: String,
    pub branch: String,
}

// Code Navigation

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationRequest {
    pub from_location: CodeLocation,
    pub navigation_type: NavigationType,
    pub include_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NavigationType {
    GoToDefinition,
    GoToImplementation,
    GoToTypeDefinition,
    FindReferences,
    FindUsages,
    GoToSymbol,
    GoToNextError,
    GoToPreviousError,
    GoToParent,
    GoToChild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationResult {
    pub destinations: Vec<NavigationDestination>,
    pub navigation_path: Vec<CodeLocation>,
    pub suggestions: Vec<NavigationSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationDestination {
    pub location: CodeLocation,
    pub destination_type: DestinationType,
    pub preview: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DestinationType {
    Definition,
    Implementation,
    Reference,
    Usage,
    Test,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationSuggestion {
    pub suggestion: String,
    pub location: CodeLocation,
    pub relevance: f64,
}

// Semantic Code Index

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndex {
    pub symbols: HashMap<String, SymbolInfo>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub call_graph: CallGraph,
    pub type_hierarchy: TypeHierarchy,
    pub embeddings: HashMap<String, Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: SymbolType,
    pub location: CodeLocation,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub references: Vec<CodeLocation>,
    pub implementations: Vec<CodeLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Type,
    Namespace,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallNode>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    pub function_name: String,
    pub location: CodeLocation,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub from: String,
    pub to: String,
    pub call_count: usize,
    pub call_sites: Vec<CodeLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHierarchy {
    pub types: HashMap<String, TypeInfo>,
    pub inheritance_tree: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub type_kind: TypeKind,
    pub base_types: Vec<String>,
    pub derived_types: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Class,
    Interface,
    Struct,
    Enum,
    Trait,
    Abstract,
}

// AI Code Search Engine

pub struct AiCodeSearchEngine {
    indexer: Arc<CodeIndexer>,
    searcher: Arc<SemanticSearcher>,
    navigator: Arc<CodeNavigator>,
    llm_provider: Arc<dyn LLMProvider>,
    cache: Arc<tokio::sync::RwLock<SearchCache>>,
}

impl AiCodeSearchEngine {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            indexer: Arc::new(CodeIndexer::new()),
            searcher: Arc::new(SemanticSearcher::new(llm_provider.clone())),
            navigator: Arc::new(CodeNavigator::new()),
            llm_provider,
            cache: Arc::new(tokio::sync::RwLock::new(SearchCache::new())),
        }
    }

    pub async fn search(
        &self,
        query: SearchQuery,
        project_path: &Path,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        // Check cache
        if let Some(cached) = self.cache.read().await.get(&query) {
            return Ok(cached);
        }

        // Build or update index
        let index = self.indexer.index_project(project_path).await?;

        // Perform search based on query type
        let results = match query.query_type {
            QueryType::NaturalLanguage => {
                self.natural_language_search(&query, &index).await?
            }
            QueryType::Regex => {
                self.regex_search(&query, project_path).await?
            }
            QueryType::Symbol => {
                self.symbol_search(&query, &index).await?
            }
            QueryType::Reference => {
                self.reference_search(&query, &index).await?
            }
            _ => {
                self.generic_search(&query, &index).await?
            }
        };

        // Apply filters
        let filtered = self.apply_filters(results, &query.filters);

        // Rank results
        let ranked = self.rank_results(filtered, &query).await?;

        // Limit results
        let limited: Vec<SearchResult> = ranked.into_iter()
            .take(query.max_results)
            .collect();

        // Cache results
        self.cache.write().await.put(query, limited.clone());

        Ok(limited)
    }

    async fn natural_language_search(
        &self,
        query: &SearchQuery,
        index: &CodeIndex,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        // Convert natural language to search intent
        let intent = self.analyze_search_intent(&query.query_text).await?;

        // Generate embeddings for semantic search
        let query_embedding = self.generate_embedding(&query.query_text).await?;

        // Search using embeddings
        let mut results = Vec::new();

        for (symbol, embedding) in &index.embeddings {
            let similarity = self.cosine_similarity(&query_embedding, embedding);

            if similarity > 0.7 {
                if let Some(symbol_info) = index.symbols.get(symbol) {
                    results.push(SearchResult {
                        relevance_score: similarity,
                        location: symbol_info.location.clone(),
                        snippet: self.extract_snippet(&symbol_info.location).await?,
                        context: self.build_context(&symbol_info.location, index).await?,
                        metadata: self.get_metadata(&symbol_info.location).await?,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn analyze_search_intent(&self, query: &str) -> Result<SearchIntent, ServiceError> {
        let prompt = format!(
            "Analyze this code search query and identify the intent:\n\
            Query: {}\n\n\
            Identify:\n\
            1. What type of code element is being searched\n\
            2. The action or relationship being queried\n\
            3. Any specific constraints or filters",
            query
        );

        let response = self.llm_provider.generate_response(&prompt, "gpt-4").await?;

        // Parse response into intent
        Ok(SearchIntent {
            target_type: "function".to_string(),
            action: "find".to_string(),
            constraints: Vec::new(),
        })
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, ServiceError> {
        // Generate embedding using LLM
        // This is a placeholder - in production, use a proper embedding model
        let hash = md5::compute(text.as_bytes());
        let embedding: Vec<f32> = hash.iter()
            .map(|&b| b as f32 / 255.0)
            .collect();

        Ok(embedding)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter())
            .map(|(x, y)| x * y)
            .sum();

        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot_product / (norm_a * norm_b)) as f64
    }

    async fn regex_search(
        &self,
        query: &SearchQuery,
        project_path: &Path,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        let regex = Regex::new(&query.query_text)
            .map_err(|e| ServiceError::ValidationError {
                field: "regex".to_string(),
                message: e.to_string(),
            })?;

        let mut results = Vec::new();
        self.search_directory(&regex, project_path, &mut results).await?;

        Ok(results)
    }

    async fn search_directory(
        &self,
        regex: &Regex,
        dir: &Path,
        results: &mut Vec<SearchResult>,
    ) -> Result<(), ServiceError> {
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(dir.to_path_buf()),
            })?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(dir.to_path_buf()),
            })? {
            let path = entry.path();

            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path).await {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            results.push(SearchResult {
                                relevance_score: 1.0,
                                location: CodeLocation {
                                    file: path.clone(),
                                    line_start: line_num + 1,
                                    line_end: line_num + 1,
                                    column_start: 0,
                                    column_end: line.len(),
                                },
                                snippet: CodeSnippet {
                                    code: line.to_string(),
                                    language: self.detect_language(&path),
                                    highlighted_regions: vec![],
                                },
                                context: SearchContext {
                                    surrounding_lines: vec![],
                                    function_context: None,
                                    class_context: None,
                                    module_context: None,
                                },
                                metadata: ResultMetadata {
                                    file_size: content.len(),
                                    last_modified: std::time::SystemTime::now(),
                                    git_info: None,
                                    complexity_score: 0.0,
                                },
                            });
                        }
                    }
                }
            } else if path.is_dir() {
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if !dir_name.starts_with('.') && dir_name != "node_modules" && dir_name != "target" {
                    Box::pin(self.search_directory(regex, &path, results)).await?;
                }
            }
        }

        Ok(())
    }

    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("py") => "python".to_string(),
            Some("go") => "go".to_string(),
            Some("java") => "java".to_string(),
            _ => "text".to_string(),
        }
    }

    async fn symbol_search(
        &self,
        query: &SearchQuery,
        index: &CodeIndex,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        let mut results = Vec::new();

        for (name, symbol) in &index.symbols {
            if self.matches_symbol(name, &query.query_text, query.fuzzy_matching) {
                results.push(SearchResult {
                    relevance_score: self.calculate_relevance(name, &query.query_text),
                    location: symbol.location.clone(),
                    snippet: self.extract_snippet(&symbol.location).await?,
                    context: self.build_context(&symbol.location, index).await?,
                    metadata: self.get_metadata(&symbol.location).await?,
                });
            }
        }

        Ok(results)
    }

    fn matches_symbol(&self, symbol: &str, query: &str, fuzzy: bool) -> bool {
        if fuzzy {
            self.fuzzy_match(symbol, query)
        } else {
            symbol.contains(query)
        }
    }

    fn fuzzy_match(&self, text: &str, pattern: &str) -> bool {
        let text_lower = text.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        let mut pattern_chars = pattern_lower.chars();
        let mut current_char = pattern_chars.next();

        for text_char in text_lower.chars() {
            if let Some(pc) = current_char {
                if text_char == pc {
                    current_char = pattern_chars.next();
                }
            } else {
                return true;
            }
        }

        current_char.is_none()
    }

    fn calculate_relevance(&self, text: &str, query: &str) -> f64 {
        if text == query {
            1.0
        } else if text.starts_with(query) {
            0.9
        } else if text.contains(query) {
            0.7
        } else {
            0.5
        }
    }

    async fn reference_search(
        &self,
        query: &SearchQuery,
        index: &CodeIndex,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        let mut results = Vec::new();

        if let Some(symbol) = index.symbols.get(&query.query_text) {
            for reference in &symbol.references {
                results.push(SearchResult {
                    relevance_score: 1.0,
                    location: reference.clone(),
                    snippet: self.extract_snippet(reference).await?,
                    context: self.build_context(reference, index).await?,
                    metadata: self.get_metadata(reference).await?,
                });
            }
        }

        Ok(results)
    }

    async fn generic_search(
        &self,
        query: &SearchQuery,
        index: &CodeIndex,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        // Fallback to text search
        self.regex_search(query, &PathBuf::from(".")).await
    }

    async fn extract_snippet(&self, location: &CodeLocation) -> Result<CodeSnippet, ServiceError> {
        let content = fs::read_to_string(&location.file).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(location.file.clone()),
            })?;

        let lines: Vec<&str> = content.lines().collect();
        let snippet_lines = &lines[location.line_start.saturating_sub(1)..location.line_end.min(lines.len())];

        Ok(CodeSnippet {
            code: snippet_lines.join("\n"),
            language: self.detect_language(&location.file),
            highlighted_regions: vec![],
        })
    }

    async fn build_context(
        &self,
        location: &CodeLocation,
        _index: &CodeIndex,
    ) -> Result<SearchContext, ServiceError> {
        Ok(SearchContext {
            surrounding_lines: vec![],
            function_context: None,
            class_context: None,
            module_context: None,
        })
    }

    async fn get_metadata(&self, location: &CodeLocation) -> Result<ResultMetadata, ServiceError> {
        let metadata = fs::metadata(&location.file).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(location.file.clone()),
            })?;

        Ok(ResultMetadata {
            file_size: metadata.len() as usize,
            last_modified: metadata.modified().unwrap_or(std::time::SystemTime::now()),
            git_info: None,
            complexity_score: 0.0,
        })
    }

    fn apply_filters(&self, results: Vec<SearchResult>, filters: &SearchFilters) -> Vec<SearchResult> {
        results.into_iter()
            .filter(|r| {
                // File type filter
                if !filters.file_types.is_empty() {
                    let ext = r.location.file.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !filters.file_types.contains(&ext.to_string()) {
                        return false;
                    }
                }

                // Directory filter
                if !filters.directories.is_empty() {
                    let mut in_directory = false;
                    for dir in &filters.directories {
                        if r.location.file.starts_with(dir) {
                            in_directory = true;
                            break;
                        }
                    }
                    if !in_directory {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    async fn rank_results(
        &self,
        mut results: Vec<SearchResult>,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        // Additional ranking based on AI analysis
        if query.semantic_search {
            for result in &mut results {
                let ai_score = self.calculate_ai_relevance(result, query).await?;
                result.relevance_score = (result.relevance_score + ai_score) / 2.0;
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(results)
    }

    async fn calculate_ai_relevance(
        &self,
        result: &SearchResult,
        query: &SearchQuery,
    ) -> Result<f64, ServiceError> {
        let prompt = format!(
            "Rate the relevance of this code snippet to the search query on a scale of 0 to 1:\n\
            Query: {}\n\
            Code: {}\n\
            Return only a number between 0 and 1.",
            query.query_text,
            result.snippet.code
        );

        let response = self.llm_provider.generate_response(&prompt, "gpt-4").await?;

        response.trim().parse::<f64>()
            .map_err(|_| ServiceError::ValidationError {
                field: "relevance".to_string(),
                message: "Invalid relevance score".to_string(),
            })
    }

    pub async fn navigate(
        &self,
        request: NavigationRequest,
        project_path: &Path,
    ) -> Result<NavigationResult, ServiceError> {
        let index = self.indexer.index_project(project_path).await?;

        let destinations = match request.navigation_type {
            NavigationType::GoToDefinition => {
                self.find_definition(&request.from_location, &index).await?
            }
            NavigationType::FindReferences => {
                self.find_references(&request.from_location, &index).await?
            }
            _ => Vec::new(),
        };

        Ok(NavigationResult {
            destinations,
            navigation_path: vec![request.from_location],
            suggestions: vec![],
        })
    }

    async fn find_definition(
        &self,
        from: &CodeLocation,
        index: &CodeIndex,
    ) -> Result<Vec<NavigationDestination>, ServiceError> {
        // Find symbol at location
        let symbol = self.get_symbol_at_location(from, index).await?;

        if let Some(symbol_info) = index.symbols.get(&symbol) {
            Ok(vec![NavigationDestination {
                location: symbol_info.location.clone(),
                destination_type: DestinationType::Definition,
                preview: self.extract_preview(&symbol_info.location).await?,
                confidence: 0.95,
            }])
        } else {
            Ok(vec![])
        }
    }

    async fn find_references(
        &self,
        from: &CodeLocation,
        index: &CodeIndex,
    ) -> Result<Vec<NavigationDestination>, ServiceError> {
        let symbol = self.get_symbol_at_location(from, index).await?;

        if let Some(symbol_info) = index.symbols.get(&symbol) {
            let destinations = symbol_info.references.iter()
                .map(|loc| NavigationDestination {
                    location: loc.clone(),
                    destination_type: DestinationType::Reference,
                    preview: String::new(),
                    confidence: 0.9,
                })
                .collect();

            Ok(destinations)
        } else {
            Ok(vec![])
        }
    }

    async fn get_symbol_at_location(
        &self,
        location: &CodeLocation,
        _index: &CodeIndex,
    ) -> Result<String, ServiceError> {
        let content = fs::read_to_string(&location.file).await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(location.file.clone()),
            })?;

        let lines: Vec<&str> = content.lines().collect();
        if location.line_start > 0 && location.line_start <= lines.len() {
            let line = lines[location.line_start - 1];
            // Simple word extraction - in production would be more sophisticated
            Ok(line.split_whitespace().next().unwrap_or("").to_string())
        } else {
            Ok(String::new())
        }
    }

    async fn extract_preview(&self, location: &CodeLocation) -> Result<String, ServiceError> {
        let snippet = self.extract_snippet(location).await?;
        Ok(snippet.code)
    }
}

#[derive(Debug, Clone)]
struct SearchIntent {
    target_type: String,
    action: String,
    constraints: Vec<String>,
}

// Code Indexer
struct CodeIndexer;

impl CodeIndexer {
    fn new() -> Self {
        Self
    }

    async fn index_project(&self, _project_path: &Path) -> Result<CodeIndex, ServiceError> {
        Ok(CodeIndex {
            symbols: HashMap::new(),
            dependencies: HashMap::new(),
            call_graph: CallGraph {
                nodes: HashMap::new(),
                edges: vec![],
            },
            type_hierarchy: TypeHierarchy {
                types: HashMap::new(),
                inheritance_tree: HashMap::new(),
            },
            embeddings: HashMap::new(),
        })
    }
}

// Semantic Searcher
struct SemanticSearcher {
    llm_provider: Arc<dyn LLMProvider>,
}

impl SemanticSearcher {
    fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }
}

// Code Navigator
struct CodeNavigator;

impl CodeNavigator {
    fn new() -> Self {
        Self
    }
}

// Search Cache
struct SearchCache {
    cache: HashMap<String, Vec<SearchResult>>,
    max_size: usize,
}

impl SearchCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 100,
        }
    }

    fn get(&self, query: &SearchQuery) -> Option<Vec<SearchResult>> {
        let key = format!("{:?}", query);
        self.cache.get(&key).cloned()
    }

    fn put(&mut self, query: SearchQuery, results: Vec<SearchResult>) {
        if self.cache.len() >= self.max_size {
            // Remove oldest entry (simple FIFO)
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }

        let key = format!("{:?}", query);
        self.cache.insert(key, results);
    }
}