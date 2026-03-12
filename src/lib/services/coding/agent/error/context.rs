//! Error context for better debugging and recovery

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Rich error context with debugging information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Operation that was being performed
    pub operation: Option<String>,

    /// File or resource being processed
    pub file_path: Option<PathBuf>,

    /// Line and column information
    pub location: Option<Location>,

    /// Additional key-value metadata
    pub metadata: HashMap<String, String>,

    /// Stack of operations leading to error
    pub call_stack: Vec<String>,

    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,

    /// User-friendly help text
    pub help_text: Option<String>,

    /// Suggested fixes
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            ..Default::default()
        }
    }

    /// Builder pattern for operation
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Builder pattern for file path
    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Builder pattern for location
    pub fn with_location(mut self, file: String, line: usize, column: usize) -> Self {
        self.location = Some(Location { file, line, column });
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add to call stack
    pub fn push_call(mut self, call: impl Into<String>) -> Self {
        self.call_stack.push(call.into());
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help_text = Some(help.into());
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Merge with another context
    pub fn merge(mut self, other: ErrorContext) -> Self {
        if self.operation.is_none() {
            self.operation = other.operation;
        }
        if self.file_path.is_none() {
            self.file_path = other.file_path;
        }
        if self.location.is_none() {
            self.location = other.location;
        }
        self.metadata.extend(other.metadata);
        self.call_stack.extend(other.call_stack);
        self.suggestions.extend(other.suggestions);
        self
    }

    /// Format as detailed error message
    pub fn format_detailed(&self) -> String {
        let mut output = Vec::new();

        if let Some(ref op) = self.operation {
            output.push(format!("Operation: {}", op));
        }

        if let Some(ref path) = self.file_path {
            output.push(format!("File: {}", path.display()));
        }

        if let Some(ref loc) = self.location {
            output.push(format!("Location: {}:{}:{}", loc.file, loc.line, loc.column));
        }

        if !self.metadata.is_empty() {
            output.push("Metadata:".to_string());
            for (k, v) in &self.metadata {
                output.push(format!("  {}: {}", k, v));
            }
        }

        if !self.call_stack.is_empty() {
            output.push("Call Stack:".to_string());
            for (i, call) in self.call_stack.iter().enumerate() {
                output.push(format!("  {}: {}", i, call));
            }
        }

        if let Some(ref help) = self.help_text {
            output.push(format!("\nHelp: {}", help));
        }

        if !self.suggestions.is_empty() {
            output.push("\nSuggested fixes:".to_string());
            for suggestion in &self.suggestions {
                output.push(format!("  • {}", suggestion));
            }
        }

        output.push(format!("Time: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));

        if let Some(ref id) = self.correlation_id {
            output.push(format!("Correlation ID: {}", id));
        }

        output.join("\n")
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref op) = self.operation {
            write!(f, "[{}]", op)?;
        }
        if let Some(ref path) = self.file_path {
            write!(f, " {}", path.display())?;
        }
        if let Some(ref loc) = self.location {
            write!(f, " at {}:{}:{}", loc.file, loc.line, loc.column)?;
        }
        Ok(())
    }
}

/// Builder for error context
pub struct ErrorContextBuilder {
    context: ErrorContext,
}

impl ErrorContextBuilder {
    pub fn new() -> Self {
        Self {
            context: ErrorContext::new(),
        }
    }

    pub fn operation(mut self, op: impl Into<String>) -> Self {
        self.context.operation = Some(op.into());
        self
    }

    pub fn file(mut self, path: PathBuf) -> Self {
        self.context.file_path = Some(path);
        self
    }

    pub fn location(mut self, file: String, line: usize, column: usize) -> Self {
        self.context.location = Some(Location { file, line, column });
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.metadata.insert(key.into(), value.into());
        self
    }

    pub fn call(mut self, call: impl Into<String>) -> Self {
        self.context.call_stack.push(call.into());
        self
    }

    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.context.correlation_id = Some(id.into());
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.context.help_text = Some(text.into());
        self
    }

    pub fn suggestion(mut self, text: impl Into<String>) -> Self {
        self.context.suggestions.push(text.into());
        self
    }

    pub fn build(self) -> ErrorContext {
        self.context
    }
}

/// Macro for creating error context
#[macro_export]
macro_rules! error_context {
    () => {
        $crate::error::context::ErrorContext::new()
    };
    ($op:expr) => {
        $crate::error::context::ErrorContext::new().with_operation($op)
    };
    ($op:expr, $($key:expr => $value:expr),+) => {{
        let mut ctx = $crate::error::context::ErrorContext::new().with_operation($op);
        $(
            ctx = ctx.with_metadata($key, $value);
        )+
        ctx
    }};
}