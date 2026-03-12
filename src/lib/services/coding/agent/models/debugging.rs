//! Debugging related models

use serde::{Deserialize, Serialize};

/// Debugging help information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggingHelp {
    pub error_analysis: ErrorAnalysis,
    pub stack_trace_analysis: Option<StackTraceAnalysis>,
    pub suggested_fixes: Vec<SuggestedFix>,
    pub debugging_steps: Vec<String>,
    pub related_documentation: Vec<String>,
}

/// Error analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    pub error_type: String,
    pub error_message: String,
    pub probable_causes: Vec<String>,
    pub affected_components: Vec<String>,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Stack trace analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTraceAnalysis {
    pub frames: Vec<StackFrame>,
    pub root_cause: Option<String>,
    pub error_origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function: String,
    pub file: String,
    pub line: usize,
    pub is_user_code: bool,
    pub variables: Option<Vec<Variable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: String,
}

/// Suggested fix for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedFix {
    pub description: String,
    pub code_change: Option<String>,
    pub explanation: String,
    pub confidence: f32,
}

/// Breakpoint suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointSuggestion {
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub reason: String,
}

/// Debug session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub session_id: String,
    pub target: String,
    pub state: DebugState,
    pub breakpoints: Vec<Breakpoint>,
    pub watch_expressions: Vec<String>,
    pub call_stack: Vec<StackFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugState {
    Running,
    Paused,
    Stopped,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub hit_count: usize,
    pub enabled: bool,
}