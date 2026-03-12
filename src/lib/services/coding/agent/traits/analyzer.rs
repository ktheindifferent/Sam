//! Analyzer traits

use async_trait::async_trait;
use anyhow::Result;
use std::path::Path;

use crate::services::coding::agent::models::*;

/// Trait for code analysis
#[async_trait]
pub trait CodeAnalyzer: Send + Sync {
    /// Analyze a single file
    async fn analyze_file(&self, path: &Path) -> Result<CodeAnalysisReport>;

    /// Analyze a directory
    async fn analyze_directory(&self, path: &Path) -> Result<Vec<CodeAnalysisReport>>;

    /// Get supported languages
    fn supported_languages(&self) -> Vec<String>;

    /// Check if language is supported
    fn supports_language(&self, language: &str) -> bool {
        self.supported_languages()
            .iter()
            .any(|l| l.eq_ignore_ascii_case(language))
    }
}

/// Trait for security analysis
#[async_trait]
pub trait SecurityAnalyzer: Send + Sync {
    /// Perform security scan
    async fn scan(&self, path: &Path) -> Result<SecurityScanReport>;

    /// Check dependencies for vulnerabilities
    async fn check_dependencies(&self, path: &Path) -> Result<Vec<DependencyVulnerability>>;

    /// Perform compliance audit
    async fn audit(&self, path: &Path, frameworks: Vec<String>) -> Result<SecurityAudit>;
}

/// Trait for performance analysis
#[async_trait]
pub trait PerformanceAnalyzer: Send + Sync {
    /// Analyze performance
    async fn analyze_performance(&self, path: &Path) -> Result<PerformanceMetrics>;

    /// Profile code execution
    async fn profile(&self, path: &Path, profile_type: ProfileType) -> Result<ProfilingResult>;

    /// Suggest optimizations
    async fn suggest_optimizations(&self, path: &Path) -> Result<PerformanceSuggestions>;
}

/// Trait for complexity analysis
#[async_trait]
pub trait ComplexityAnalyzer: Send + Sync {
    /// Calculate cyclomatic complexity
    async fn calculate_complexity(&self, path: &Path) -> Result<ComplexityVisualization>;

    /// Identify complexity hotspots
    async fn find_hotspots(&self, path: &Path, threshold: usize) -> Result<Vec<ComplexityHotspot>>;

    /// Suggest simplifications
    async fn suggest_simplifications(&self, path: &Path) -> Result<Vec<RefactoringSuggestion>>;
}

/// Trait for test analysis
#[async_trait]
pub trait TestAnalyzer: Send + Sync {
    /// Analyze test coverage
    async fn analyze_coverage(&self, path: &Path) -> Result<TestCoverage>;

    /// Find untested code
    async fn find_untested(&self, path: &Path) -> Result<Vec<UntestedCode>>;

    /// Suggest test cases
    async fn suggest_tests(&self, path: &Path) -> Result<Vec<TestSuggestion>>;
}

#[derive(Debug, Clone)]
pub struct TestCoverage {
    pub line_coverage: f32,
    pub branch_coverage: f32,
    pub function_coverage: f32,
    pub uncovered_lines: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct UntestedCode {
    pub file: String,
    pub function: String,
    pub lines: Vec<usize>,
    pub complexity: usize,
    pub priority: Priority,
}

#[derive(Debug, Clone)]
pub struct TestSuggestion {
    pub test_name: String,
    pub test_type: TestType,
    pub target_function: String,
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub description: String,
    pub input: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub assertions: Vec<String>,
}