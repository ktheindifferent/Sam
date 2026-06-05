//! Data models and types for the coding agent
//!
//! This module consolidates all data structures that were previously
//! scattered across service.rs and other files.

pub mod analysis;
pub mod conversation;
pub mod debugging;
pub mod metrics;
pub mod review;
pub mod security;

pub use analysis::*;
pub use conversation::*;
pub use debugging::*;
pub use metrics::*;
pub use review::*;
pub use security::*;

use serde::{Deserialize, Serialize};

/// Documentation type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentationType {
    Api,
    Tutorial,
    Reference,
    Guide,
    Readme,
    Changelog,
    Architecture,
}

/// Applied fix result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFix {
    pub file_path: String,
    pub line_number: usize,
    pub original_code: String,
    pub fixed_code: String,
    pub fix_description: String,
}

/// Code fix suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFix {
    pub fix_type: String,
    pub description: String,
    pub diff: String,
}
