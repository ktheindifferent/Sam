// Coding Agent Module
// This module provides a comprehensive coding agent system with AI assistance,
// command execution, and incremental task management.

pub mod providers;
pub mod types;
pub mod errors;
pub mod resource_limits;
pub mod gpu_offload;
pub mod remote_ollama;
pub mod ollama_config_manager;
pub mod ollama_auto_config;
pub mod code_intelligence;
pub mod debugging;
pub mod testing;
pub mod config;
pub mod context;
pub mod execution_context;
pub mod templates;
pub mod metrics;
pub mod service;
pub mod execution_state;
pub mod step_parser;
pub mod command_executor;
pub mod executor;
pub mod interactive_executor;
pub mod workspace_analyzer;
pub mod collaboration;
pub mod completion;
pub mod refactoring;
pub mod git_integration;
pub mod migration;
pub mod benchmarking;
pub mod pair_programming;
pub mod scaffolding;
pub mod error_recovery;
pub mod advanced_testing;
pub mod documentation_generator;
pub mod security_analyzer;
pub mod performance_profiler;
pub mod distributed_collaboration;
pub mod ai_code_search;
pub mod model_training;
pub mod code_explanation;
pub mod automated_debugging;
pub mod code_metrics_dashboard;
pub mod paradigm_translator;
pub mod api_client_generator;
pub mod code_review;
pub mod automated_refactoring;
pub mod dependency_analyzer;
pub mod continuous_learning;
pub mod multi_language_search;
pub mod test_generation;
pub mod realtime_collaboration;
pub mod intelligent_completion;
pub mod code_flow_visualizer;
pub mod performance_optimizer;
pub mod bug_predictor;

// Re-export the main service and executor types for easy access
pub use service::CodingAgentService;
pub use executor::{CodingAgentExecutor, UserMessage, EnhancedContext};
pub use interactive_executor::{InteractiveExecutor, ExecutionContext as InteractiveContext};

// Re-export commonly used types
pub use types::{
    CodeExecutionRequest,
    CodingAgentResponse,
    CommandHistoryEntry,
    ProjectStructure,
    RiskLevel,
};

// Re-export execution state types
pub use execution_state::{
    ExecutionState,
    ExecutionStep,
    IncrementalExecution,
};

// Re-export configuration
pub use config::CodingAgentConfig;

// Re-export provider management
pub use providers::{
    LLMProvider,
    ProviderManager,
    ModelProvider,
    ModelConfig,
    ModelPerformanceMetrics
};

// Re-export execution components for advanced usage
pub use step_parser::StepParser;
pub use command_executor::CommandExecutor;

// Re-export utility types
pub use context::ContextManager;
pub use templates::TemplateManager;
pub use metrics::MetricsManager;

// Re-export error types
pub use errors::{CodingAgentError, CodingAgentResult, ErrorSeverity};

// Re-export resource management
pub use resource_limits::{ResourceLimits, ResourceMonitor, ResourceUsageStats};

// Re-export execution context
pub use execution_context::{
    ExecutionContext, ExecutionContextManager, ShellType, GuiMode,
    ContextCommand, ProcessInfo, ProcessStatus
};

// Re-export GPU offloading
pub use gpu_offload::{
    GpuOffloadManager, GpuOffloadConfig, GpuProvider, GpuInstance,
    GpuInstanceSpec, CostTracker, SaladClient, OllamaClient
};

// Re-export remote Ollama
pub use remote_ollama::{
    RemoteOllamaProvider, RemoteOllamaConfig, RemoteOllamaSessionManager
};

// Re-export code intelligence
pub use code_intelligence::{
    CodeIntelligence, FileAnalysis, CodeMetrics, CodeIssue,
    Symbol, SymbolKind, Improvement, ComplexityReport,
    RefactoringEngine, RefactoringContext, RefactoringPreview
};

// Re-export debugging
pub use debugging::{
    DebuggingEngine, DebugConfig, DebugSession, DebugState,
    Breakpoint, StackFrame, Variable, Value, DebugEvent
};

// Re-export testing
pub use testing::{
    TestingEngine, TestContext, GeneratedTest, TestResult,
    TestSuiteResult, CoverageReport, TestFramework, TestStrategy
};

// Re-export git integration
pub use git_integration::{
    GitIntegration, GitConfig, RepositoryStatus, FileStatus,
    Commit, Branch, Tag, Remote, DiffInfo, ConflictInfo
};

// Re-export migration
pub use migration::{
    CodeMigrationEngine, MigrationConfig, MigrationResult,
    MigrationWarning, WarningSeverity, MigrationStatistics,
    FrameworkMigrator, DatabaseMigrator
};

// Re-export benchmarking
pub use benchmarking::{
    BenchmarkingEngine, BenchmarkConfig, BenchmarkResult,
    OptimizationSuggestion, OptimizationImpact, OptimizationCategory,
    HotspotAnalysis, PerformanceComparator
};

// Re-export pair programming
pub use pair_programming::{
    PairProgrammingEngine, PairProgrammingSession, AiPersona,
    InteractionStyle, AiResponse, CollaborativeSession
};

// Re-export scaffolding
pub use scaffolding::{
    ScaffoldingEngine, ProjectTemplate, ProjectCategory,
    ScaffoldResult, GeneratorContext, Field
};

// Re-export error recovery
pub use error_recovery::{
    ErrorRecoveryEngine, ErrorContext, ErrorType,
    RecoveryStrategy, RecoveryResult, ErrorStatistics,
    ErrorPreventionEngine, PotentialIssue
};

// Re-export advanced testing
pub use advanced_testing::{
    AdvancedTestingEngine, TestSuite, TestCase, TestType,
    MutationTestingReport, PropertyTestingReport, FuzzingReport,
    CoverageData, TestResults
};

// Re-export documentation generator
pub use documentation_generator::{
    DocumentationGenerator, Documentation, DocumentationConfig,
    OutputFormat, GeneratedDocumentation, ApiReference,
    Tutorial, CodeExample
};

// Re-export security analyzer
pub use security_analyzer::{
    SecurityAnalyzer, SecurityAnalysisReport, SecurityConfig,
    Vulnerability, VulnerabilityType, Severity,
    DependencyAnalysis, ComplianceReport, SecurityRecommendation
};

// Re-export performance profiler
pub use performance_profiler::{
    PerformanceProfiler, PerformanceProfile, ProfileConfig,
    FlameGraph, Hotspot, PerformanceMetrics,
    MemoryProfile, CpuProfile, IoProfile
};

// Re-export distributed collaboration
pub use distributed_collaboration::{
    DistributedCollaborationEngine, DistributedSession,
    Participant, SharedWorkspace, CollaborationEvent,
    CodeReview, MergeRequest, ChatChannel
};

// Re-export AI code search
pub use ai_code_search::{
    AiCodeSearchEngine, SearchQuery, SearchResult,
    NavigationRequest, NavigationResult, CodeIndex,
    SymbolInfo, CallGraph, TypeHierarchy
};

// Re-export model training
pub use model_training::{
    ModelTrainingEngine, TrainingConfig, TrainingDataConfig,
    Hyperparameters, OptimizationConfig, EvaluationConfig,
    HardwareConfig, ModelType, TrainingStatus, TrainingSession,
    AugmentationConfig
};

// Re-export code explanation
pub use code_explanation::{
    CodeExplanationEngine, ExplanationRequest, Explanation,
    ExplanationType, DetailLevel, AlgorithmAnalysis, Visualization, Quiz,
    LearningResource, ExplanationSection
};

// Re-export automated debugging
pub use automated_debugging::{
    AutomatedDebuggingEngine, DebugSession as AutomatedDebugSession, DebugSessionType,
    DebugSessionStatus, DebugFinding, FindingType, DebugSeverity,
    CodeLocation as DebugCodeLocation, Evidence, SuggestedFix, FixType, CodeChange,
    TestResults as DebugTestResults, ExecutionTrace, StackFrame as DebugStackFrame, Breakpoint as DebugBreakpoint,
    ConditionalBreakpoint, Watchpoint, WatchType, TracePattern,
    PatternType, ImpactLevel, TestFramework as DebugTestFramework
};

// Re-export code metrics dashboard
pub use code_metrics_dashboard::{
    CodeMetricsDashboard, MetricsCollector, CollectionConfig, MetricType,
    CollectedMetrics, ProjectMetrics, FileMetrics, ModuleMetrics,
    TeamMetrics, QualityMetrics, QualityRating, QualityGateStatus,
    TrendAnalysis, TrendDirection, Anomaly, AnomalySeverity,
    ChartType, Chart, ChartData, ReportType, GeneratedReport,
    Recommendation, Priority, MonitoringConfig, Alert, AlertLevel
};

// Re-export paradigm translator
pub use paradigm_translator::{
    ParadigmTranslator, Paradigm, TranslationRequest, TranslationResult,
    ParadigmMapping, TranslationWarning, TranslationMetrics,
    ImprovementSuggestion, ParadigmAnalysis, OptimizationLevel,
    StylePreferences, NamingConvention, WarningSeverity as ParadigmWarningSeverity
};

// Re-export API client generator
pub use api_client_generator::{
    ApiClientGenerator, SpecFormat, Language, ParsedApiSpec,
    GenerationConfig, GeneratedCode, GeneratedFile, FileType,
    Endpoint, DataModel, AuthType, HttpMethod,
    GenerationFeatures, ErrorHandlingStrategy, BuildSystem,
    Documentation as ApiDocumentation, GenerationStatistics, Dependency
};

// Re-export code review system
pub use code_review::{
    CodeReviewSystem, ReviewRequest, ReviewResult, ReviewVerdict,
    ReviewComment, CodeSuggestion, ReviewMetrics, SecurityFinding,
    BestPracticeViolation, LearningPoint, ReviewType, ReviewDepth,
    CommentType, CommentSeverity, ReviewPolicy, ReviewFeedback
};

// Re-export automated refactoring
pub use automated_refactoring::{
    RefactoringEngine as AutomatedRefactoringEngine, RefactoringRequest, RefactoringResult,
    RefactoringType, RefactoringPlan, RefactoringOpportunity,
    PatternType as RefactoringPatternType, PatternMatch, CodeChange as RefactoringChange,
    ValidationResults, RiskLevel as RefactoringRiskLevel
};

// Re-export dependency analyzer
pub use dependency_analyzer::{
    DependencyAnalyzer, AnalysisRequest, AnalysisResult,
    DependencyManifest, DependencyTree, Dependency as AnalyzedDependency,
    UpdateInfo, SecurityAdvisory, VulnerabilitySeverity,
    HealthReport, HealthScore, UpgradePlan, Recommendation as DependencyRecommendation
};

// Re-export continuous learning
// TODO: Fix exports when module is complete
// pub use continuous_learning::{
//     ContinuousLearningSystem, LearningConfig, LearningSession,
//     Pattern, PatternType as LearningPatternType, PatternInstance, KnowledgeEntry,
//     KnowledgeCategory, Feedback, FeedbackType, FeedbackRating,
//     Adaptation, AdaptationType, LearningMetrics, LearningTrend
// };

// Re-export multi-language search
// TODO: Fix exports when module is complete
// pub use multi_language_search::{
//     MultiLanguageSearchEngine, SearchRequest, SearchResult as MultiLanguageSearchResult,
//     SearchMatch, MatchType, SearchContext, IndexStatus,
//     IndexStatistics, QueryType, SemanticQuery, VectorEmbedding,
//     CodeLocation as SearchCodeLocation, CodeSymbol, SymbolType as SearchSymbolType, Language as SearchLanguage
// };

// Re-export test generation
pub use test_generation::{
    TestGenerationEngine, TestGenerationRequest, GeneratedTestSuite,
    GeneratedTest as TestGenGeneratedTest, TestType as TestGenTestType, EdgeCase, EdgeCaseCategory, EdgeCaseSeverity,
    PropertyTest, FuzzTest, FuzzerType, Assertion, AssertionType,
    TestData, TestValue, ExpectedOutcome, OutcomeType, SideEffect,
    CoverageReport as TestGenCoverageReport, TestStatistics, TestSuggestion, OptimizationLevel as TestOptimizationLevel
};