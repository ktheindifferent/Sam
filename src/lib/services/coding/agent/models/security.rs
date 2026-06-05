//! Security related models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security scan report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanReport {
    pub scan_date: chrono::DateTime<chrono::Utc>,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub risk_score: f32,
    pub compliance_status: ComplianceStatus,
    pub recommendations: Vec<SecurityRecommendation>,
}

/// Security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    pub id: String,
    pub severity: VulnerabilitySeverity,
    pub vulnerability_type: VulnerabilityType,
    pub description: String,
    pub affected_file: String,
    pub line_number: Option<usize>,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilityType {
    SqlInjection,
    CrossSiteScripting,
    CommandInjection,
    PathTraversal,
    InsecureDeserialization,
    HardcodedCredentials,
    WeakCryptography,
    InsecureRandomness,
    UnvalidatedInput,
    InformationDisclosure,
    DenialOfService,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    Unknown,
}

/// Security recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub priority: SecurityPriority,
    pub title: String,
    pub description: String,
    pub implementation_guide: Vec<String>,
    pub effort: EffortEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPriority {
    Immediate,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortEstimate {
    Hours(u32),
    Days(u32),
    Weeks(u32),
}

/// Dependency vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyVulnerability {
    pub package_name: String,
    pub current_version: String,
    pub safe_version: Option<String>,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub update_available: bool,
}

/// Security audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAudit {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub audit_type: AuditType,
    pub findings: Vec<AuditFinding>,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub compliance_frameworks: Vec<ComplianceFramework>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditType {
    Full,
    Partial,
    Dependency,
    Configuration,
    CodeQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub check_id: String,
    pub check_name: String,
    pub result: CheckResult,
    pub details: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckResult {
    Pass,
    Fail,
    Warning,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFramework {
    pub name: String, // e.g., "OWASP Top 10", "PCI DSS", "GDPR"
    pub version: String,
    pub compliance_level: f32, // 0.0 to 100.0
    pub requirements: HashMap<String, RequirementStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequirementStatus {
    Met,
    NotMet,
    PartiallyMet,
    NotApplicable,
}
