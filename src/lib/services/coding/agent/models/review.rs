//! Code review models

use serde::{Deserialize, Serialize};

/// Code review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReview {
    pub overall_score: f32,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<ReviewSuggestion>,
    pub positive_aspects: Vec<String>,
    pub summary: String,
    pub recommendation: ReviewRecommendation,
}

/// Review issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: ReviewSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub suggestion: Option<String>,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Blocker,
    Critical,
    Major,
    Minor,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    Bug,
    Security,
    Performance,
    Maintainability,
    Style,
    Documentation,
    Testing,
    Accessibility,
}

/// Review suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub example: Option<String>,
    pub benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    Refactoring,
    Testing,
    Documentation,
    Performance,
    Security,
    BestPractice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewRecommendation {
    Approve,
    ApproveWithSuggestions,
    RequestChanges,
    Reject,
}

/// Pull request review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestReview {
    pub pr_number: u32,
    pub title: String,
    pub description: String,
    pub files_changed: Vec<FileChange>,
    pub review: CodeReview,
    pub auto_fixable_issues: Vec<AutoFixableIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFixableIssue {
    pub issue: ReviewIssue,
    pub fix: String,
    pub confidence: f32,
}

/// Review metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMetrics {
    pub lines_reviewed: usize,
    pub issues_found: usize,
    pub critical_issues: usize,
    pub coverage: f32,
    pub complexity_score: f32,
    pub maintainability_score: f32,
    pub test_coverage: Option<f32>,
}