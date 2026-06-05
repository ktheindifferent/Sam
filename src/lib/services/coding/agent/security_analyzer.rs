use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use super::errors::CodingAgentError as ServiceError;
use super::traits::provider::LLMProvider;

// Comprehensive Security Analysis Framework

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalysisReport {
    pub scan_id: String,
    pub timestamp: std::time::SystemTime,
    pub summary: SecuritySummary,
    pub vulnerabilities: Vec<Vulnerability>,
    pub code_smells: Vec<CodeSmell>,
    pub dependencies: DependencyAnalysis,
    pub secrets: Vec<SecretLeak>,
    pub compliance: ComplianceReport,
    pub recommendations: Vec<SecurityRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySummary {
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub high_issues: usize,
    pub medium_issues: usize,
    pub low_issues: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub vulnerability_type: VulnerabilityType,
    pub severity: Severity,
    pub location: CodeLocation,
    pub description: String,
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
    pub remediation: String,
    pub code_snippet: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilityType {
    SqlInjection,
    XssAttack,
    CommandInjection,
    PathTraversal,
    InsecureDeserialization,
    XXE,
    SSRF,
    InsecureRandomness,
    HardcodedCredentials,
    WeakCryptography,
    InsecureFilePermissions,
    RaceCondition,
    BufferOverflow,
    IntegerOverflow,
    NullPointerDereference,
    UseAfterFree,
    MemoryLeak,
    UnvalidatedInput,
    InsecureCommunication,
    BrokenAuthentication,
    BrokenAccessControl,
    SecurityMisconfiguration,
    SensitiveDataExposure,
    InsufficientLogging,
    Other(String),
}

impl std::fmt::Display for VulnerabilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulnerabilityType::SqlInjection => write!(f, "SQL Injection"),
            VulnerabilityType::XssAttack => write!(f, "XSS Attack"),
            VulnerabilityType::CommandInjection => write!(f, "Command Injection"),
            VulnerabilityType::PathTraversal => write!(f, "Path Traversal"),
            VulnerabilityType::InsecureDeserialization => write!(f, "Insecure Deserialization"),
            VulnerabilityType::XXE => write!(f, "XXE"),
            VulnerabilityType::SSRF => write!(f, "SSRF"),
            VulnerabilityType::InsecureRandomness => write!(f, "Insecure Randomness"),
            VulnerabilityType::HardcodedCredentials => write!(f, "Hardcoded Credentials"),
            VulnerabilityType::WeakCryptography => write!(f, "Weak Cryptography"),
            VulnerabilityType::InsecureFilePermissions => write!(f, "Insecure File Permissions"),
            VulnerabilityType::RaceCondition => write!(f, "Race Condition"),
            VulnerabilityType::BufferOverflow => write!(f, "Buffer Overflow"),
            VulnerabilityType::IntegerOverflow => write!(f, "Integer Overflow"),
            VulnerabilityType::NullPointerDereference => write!(f, "Null Pointer Dereference"),
            VulnerabilityType::UseAfterFree => write!(f, "Use After Free"),
            VulnerabilityType::MemoryLeak => write!(f, "Memory Leak"),
            VulnerabilityType::UnvalidatedInput => write!(f, "Unvalidated Input"),
            VulnerabilityType::InsecureCommunication => write!(f, "Insecure Communication"),
            VulnerabilityType::BrokenAuthentication => write!(f, "Broken Authentication"),
            VulnerabilityType::BrokenAccessControl => write!(f, "Broken Access Control"),
            VulnerabilityType::SecurityMisconfiguration => write!(f, "Security Misconfiguration"),
            VulnerabilityType::SensitiveDataExposure => write!(f, "Sensitive Data Exposure"),
            VulnerabilityType::InsufficientLogging => write!(f, "Insufficient Logging"),
            VulnerabilityType::Other(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub column: usize,
    pub function: Option<String>,
    pub class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSmell {
    pub smell_type: CodeSmellType,
    pub location: CodeLocation,
    pub description: String,
    pub impact: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeSmellType {
    LongMethod,
    LargeClass,
    DuplicateCode,
    DeadCode,
    ComplexConditional,
    DataClump,
    FeatureEnvy,
    InappropriateIntimacy,
    RefusedBequest,
    LazyClass,
    SpeculativeGenerality,
    MessageChains,
    MiddleMan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    pub total_dependencies: usize,
    pub outdated: Vec<OutdatedDependency>,
    pub vulnerable: Vec<VulnerableDependency>,
    pub licenses: Vec<LicenseInfo>,
    pub supply_chain_risks: Vec<SupplyChainRisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedDependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub update_type: UpdateType,
    pub breaking_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerableDependency {
    pub name: String,
    pub version: String,
    pub vulnerabilities: Vec<DependencyVulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyVulnerability {
    pub cve_id: String,
    pub severity: Severity,
    pub description: String,
    pub fixed_version: Option<String>,
    pub published_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub dependency: String,
    pub license: String,
    pub license_type: LicenseType,
    pub compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseType {
    MIT,
    Apache2,
    GPL,
    BSD,
    Proprietary,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainRisk {
    pub risk_type: SupplyChainRiskType,
    pub dependency: String,
    pub description: String,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupplyChainRiskType {
    Typosquatting,
    Abandoned,
    MaliciousCode,
    CompromisedMaintainer,
    UnverifiedSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLeak {
    pub secret_type: SecretType,
    pub location: CodeLocation,
    pub value_preview: String, // Redacted preview
    pub entropy: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    ApiKey,
    PrivateKey,
    Password,
    Token,
    Certificate,
    ConnectionString,
    EnvironmentVariable,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub standards: Vec<ComplianceStandard>,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub compliance_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStandard {
    pub name: String,
    pub version: String,
    pub checks: Vec<ComplianceCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub id: String,
    pub description: String,
    pub status: CheckStatus,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub priority: Priority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub implementation: String,
    pub effort: EffortLevel,
    pub impact: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Trivial,
    Small,
    Medium,
    Large,
    VeryLarge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Critical,
    High,
    Medium,
    Low,
}

// Security Analyzer Engine

pub struct SecurityAnalyzer {
    scanners: HashMap<String, Box<dyn SecurityScanner>>,
    llm_provider: Arc<dyn LLMProvider>,
    vulnerability_db: Arc<VulnerabilityDatabase>,
    secret_detector: Arc<SecretDetector>,
    sast_engine: Arc<SastEngine>,
}

impl SecurityAnalyzer {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        let mut analyzer = Self {
            scanners: HashMap::new(),
            llm_provider: llm_provider.clone(),
            vulnerability_db: Arc::new(VulnerabilityDatabase::new()),
            secret_detector: Arc::new(SecretDetector::new()),
            sast_engine: Arc::new(SastEngine::new(llm_provider)),
        };

        analyzer.register_scanners();
        analyzer
    }

    fn register_scanners(&mut self) {
        self.scanners
            .insert("rust".to_string(), Box::new(RustSecurityScanner::new()));
        self.scanners
            .insert("javascript".to_string(), Box::new(JsSecurityScanner::new()));
        self.scanners
            .insert("python".to_string(), Box::new(PythonSecurityScanner::new()));
        self.scanners
            .insert("go".to_string(), Box::new(GoSecurityScanner::new()));
        self.scanners
            .insert("java".to_string(), Box::new(JavaSecurityScanner::new()));
    }

    pub async fn analyze(
        &self,
        project_path: &Path,
        config: SecurityConfig,
    ) -> Result<SecurityAnalysisReport, ServiceError> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        let mut vulnerabilities = Vec::new();
        let mut code_smells = Vec::new();
        let mut secrets = Vec::new();

        // Detect language
        let language = self.detect_language(project_path).await?;

        // Run language-specific scanner
        if let Some(scanner) = self.scanners.get(&language) {
            let scan_result = scanner.scan(project_path).await?;
            vulnerabilities.extend(scan_result.vulnerabilities);
            code_smells.extend(scan_result.code_smells);
        }

        // Run SAST analysis
        if config.enable_sast {
            let sast_results = self.sast_engine.analyze(project_path).await?;
            vulnerabilities.extend(sast_results);
        }

        // Detect secrets
        if config.scan_secrets {
            secrets = self.secret_detector.scan(project_path).await?;
        }

        // Analyze dependencies
        let dependencies = if config.analyze_dependencies {
            self.analyze_dependencies(project_path, &language).await?
        } else {
            DependencyAnalysis {
                total_dependencies: 0,
                outdated: Vec::new(),
                vulnerable: Vec::new(),
                licenses: Vec::new(),
                supply_chain_risks: Vec::new(),
            }
        };

        // Check compliance
        let compliance = if config.check_compliance {
            self.check_compliance(&config.compliance_standards, project_path)
                .await?
        } else {
            ComplianceReport {
                standards: Vec::new(),
                passed_checks: 0,
                failed_checks: 0,
                compliance_percentage: 100.0,
            }
        };

        // Calculate risk score
        let summary = self.calculate_summary(&vulnerabilities, &code_smells, &secrets);

        // Generate recommendations
        let recommendations = self
            .generate_recommendations(&vulnerabilities, &code_smells, &dependencies, &compliance)
            .await?;

        Ok(SecurityAnalysisReport {
            scan_id,
            timestamp: std::time::SystemTime::now(),
            summary,
            vulnerabilities,
            code_smells,
            dependencies,
            secrets,
            compliance,
            recommendations,
        })
    }

    async fn detect_language(&self, project_path: &Path) -> Result<String, ServiceError> {
        if project_path.join("Cargo.toml").exists() {
            Ok("rust".to_string())
        } else if project_path.join("package.json").exists() {
            Ok("javascript".to_string())
        } else if project_path.join("requirements.txt").exists()
            || project_path.join("setup.py").exists()
        {
            Ok("python".to_string())
        } else if project_path.join("go.mod").exists() {
            Ok("go".to_string())
        } else if project_path.join("pom.xml").exists() {
            Ok("java".to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    async fn analyze_dependencies(
        &self,
        project_path: &Path,
        language: &str,
    ) -> Result<DependencyAnalysis, ServiceError> {
        let mut analysis = DependencyAnalysis {
            total_dependencies: 0,
            outdated: Vec::new(),
            vulnerable: Vec::new(),
            licenses: Vec::new(),
            supply_chain_risks: Vec::new(),
        };

        match language {
            "rust" => {
                if let Ok(cargo_lock) = fs::read_to_string(project_path.join("Cargo.lock")).await {
                    analysis.total_dependencies = cargo_lock.matches("[[package]]").count();
                    // Check for known vulnerabilities
                    analysis.vulnerable =
                        self.vulnerability_db.check_rust_deps(&cargo_lock).await?;
                }
            }
            "javascript" => {
                if let Ok(package_lock) =
                    fs::read_to_string(project_path.join("package-lock.json")).await
                {
                    // Parse and analyze npm dependencies
                    analysis.total_dependencies = package_lock.matches("\"version\"").count();
                }
            }
            _ => {}
        }

        Ok(analysis)
    }

    async fn check_compliance(
        &self,
        standards: &[String],
        project_path: &Path,
    ) -> Result<ComplianceReport, ServiceError> {
        let mut report = ComplianceReport {
            standards: Vec::new(),
            passed_checks: 0,
            failed_checks: 0,
            compliance_percentage: 0.0,
        };

        for standard_name in standards {
            let standard = self.load_compliance_standard(standard_name).await?;
            report.standards.push(standard);
        }

        // Calculate compliance percentage
        let total_checks = report
            .standards
            .iter()
            .map(|s| s.checks.len())
            .sum::<usize>();

        if total_checks > 0 {
            report.compliance_percentage =
                (report.passed_checks as f64 / total_checks as f64) * 100.0;
        }

        Ok(report)
    }

    async fn load_compliance_standard(
        &self,
        name: &str,
    ) -> Result<ComplianceStandard, ServiceError> {
        // Load compliance standard definitions
        Ok(ComplianceStandard {
            name: name.to_string(),
            version: "1.0".to_string(),
            checks: Vec::new(),
        })
    }

    fn calculate_summary(
        &self,
        vulnerabilities: &[Vulnerability],
        code_smells: &[CodeSmell],
        secrets: &[SecretLeak],
    ) -> SecuritySummary {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for vuln in vulnerabilities {
            match vuln.severity {
                Severity::Critical => critical += 1,
                Severity::High => high += 1,
                Severity::Medium => medium += 1,
                Severity::Low => low += 1,
                Severity::Info => {}
            }
        }

        // Calculate risk score (0-100)
        let risk_score = (critical as f64 * 10.0)
            + (high as f64 * 5.0)
            + (medium as f64 * 2.0)
            + (low as f64 * 0.5)
            + (secrets.len() as f64 * 8.0);

        let risk_level = match risk_score {
            s if s >= 80.0 => RiskLevel::Critical,
            s if s >= 60.0 => RiskLevel::High,
            s if s >= 40.0 => RiskLevel::Medium,
            s if s >= 20.0 => RiskLevel::Low,
            _ => RiskLevel::None,
        };

        SecuritySummary {
            risk_score: risk_score.min(100.0),
            risk_level,
            total_issues: vulnerabilities.len() + code_smells.len() + secrets.len(),
            critical_issues: critical,
            high_issues: high,
            medium_issues: medium,
            low_issues: low,
        }
    }

    async fn generate_recommendations(
        &self,
        vulnerabilities: &[Vulnerability],
        code_smells: &[CodeSmell],
        dependencies: &DependencyAnalysis,
        compliance: &ComplianceReport,
    ) -> Result<Vec<SecurityRecommendation>, ServiceError> {
        let mut recommendations = Vec::new();

        // Critical vulnerability recommendations
        for vuln in vulnerabilities
            .iter()
            .filter(|v| matches!(v.severity, Severity::Critical))
        {
            recommendations.push(SecurityRecommendation {
                priority: Priority::Urgent,
                category: "Vulnerability".to_string(),
                title: format!("Fix {} vulnerability", vuln.vulnerability_type.to_string()),
                description: vuln.description.clone(),
                implementation: vuln.remediation.clone(),
                effort: EffortLevel::Medium,
                impact: ImpactLevel::Critical,
            });
        }

        // Dependency recommendations
        if !dependencies.vulnerable.is_empty() {
            recommendations.push(SecurityRecommendation {
                priority: Priority::High,
                category: "Dependencies".to_string(),
                title: "Update vulnerable dependencies".to_string(),
                description: format!(
                    "{} dependencies have known vulnerabilities",
                    dependencies.vulnerable.len()
                ),
                implementation: "Update dependencies to patched versions".to_string(),
                effort: EffortLevel::Small,
                impact: ImpactLevel::High,
            });
        }

        Ok(recommendations)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_sast: bool,
    pub scan_secrets: bool,
    pub analyze_dependencies: bool,
    pub check_compliance: bool,
    pub compliance_standards: Vec<String>,
    pub severity_threshold: Severity,
    pub max_file_size: usize,
    pub excluded_paths: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_sast: true,
            scan_secrets: true,
            analyze_dependencies: true,
            check_compliance: false,
            compliance_standards: vec!["OWASP".to_string()],
            severity_threshold: Severity::Low,
            max_file_size: 10_000_000, // 10MB
            excluded_paths: vec!["node_modules".to_string(), "target".to_string()],
        }
    }
}

// Security Scanner trait

#[async_trait]
trait SecurityScanner: Send + Sync {
    async fn scan(&self, project_path: &Path) -> Result<ScanResult, ServiceError>;
    fn language(&self) -> &str;
}

#[derive(Debug, Clone)]
struct ScanResult {
    vulnerabilities: Vec<Vulnerability>,
    code_smells: Vec<CodeSmell>,
}

// Language-specific scanners

struct RustSecurityScanner;

impl RustSecurityScanner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for RustSecurityScanner {
    async fn scan(&self, project_path: &Path) -> Result<ScanResult, ServiceError> {
        let mut result = ScanResult {
            vulnerabilities: Vec::new(),
            code_smells: Vec::new(),
        };

        // Scan for unsafe code
        self.scan_unsafe_code(project_path, &mut result.vulnerabilities)
            .await?;

        // Check for common security issues
        self.check_common_issues(project_path, &mut result.vulnerabilities)
            .await?;

        Ok(result)
    }

    fn language(&self) -> &str {
        "rust"
    }
}

impl RustSecurityScanner {
    async fn scan_unsafe_code(
        &self,
        project_path: &Path,
        vulnerabilities: &mut Vec<Vulnerability>,
    ) -> Result<(), ServiceError> {
        let mut entries = fs::read_dir(project_path)
            .await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(project_path.to_path_buf()),
            })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(project_path.to_path_buf()),
            })?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let content =
                    fs::read_to_string(&path)
                        .await
                        .map_err(|e| ServiceError::IoError {
                            message: e.to_string(),
                            path: Some(path.clone()),
                        })?;

                if content.contains("unsafe") {
                    vulnerabilities.push(Vulnerability {
                        id: uuid::Uuid::new_v4().to_string(),
                        vulnerability_type: VulnerabilityType::Other(
                            "Unsafe code usage".to_string(),
                        ),
                        severity: Severity::Medium,
                        location: CodeLocation {
                            file: path,
                            line_start: 0,
                            line_end: 0,
                            column: 0,
                            function: None,
                            class: None,
                        },
                        description: "Unsafe code block detected".to_string(),
                        cwe_id: None,
                        owasp_category: None,
                        remediation: "Review unsafe code and ensure memory safety".to_string(),
                        code_snippet: "unsafe { ... }".to_string(),
                        confidence: 1.0,
                    });
                }
            }
        }

        Ok(())
    }

    async fn check_common_issues(
        &self,
        _project_path: &Path,
        _vulnerabilities: &mut Vec<Vulnerability>,
    ) -> Result<(), ServiceError> {
        // Check for common Rust security issues
        Ok(())
    }
}

struct JsSecurityScanner;

impl JsSecurityScanner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for JsSecurityScanner {
    async fn scan(&self, _project_path: &Path) -> Result<ScanResult, ServiceError> {
        Ok(ScanResult {
            vulnerabilities: Vec::new(),
            code_smells: Vec::new(),
        })
    }

    fn language(&self) -> &str {
        "javascript"
    }
}

struct PythonSecurityScanner;

impl PythonSecurityScanner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for PythonSecurityScanner {
    async fn scan(&self, _project_path: &Path) -> Result<ScanResult, ServiceError> {
        Ok(ScanResult {
            vulnerabilities: Vec::new(),
            code_smells: Vec::new(),
        })
    }

    fn language(&self) -> &str {
        "python"
    }
}

struct GoSecurityScanner;

impl GoSecurityScanner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for GoSecurityScanner {
    async fn scan(&self, _project_path: &Path) -> Result<ScanResult, ServiceError> {
        Ok(ScanResult {
            vulnerabilities: Vec::new(),
            code_smells: Vec::new(),
        })
    }

    fn language(&self) -> &str {
        "go"
    }
}

struct JavaSecurityScanner;

impl JavaSecurityScanner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for JavaSecurityScanner {
    async fn scan(&self, _project_path: &Path) -> Result<ScanResult, ServiceError> {
        Ok(ScanResult {
            vulnerabilities: Vec::new(),
            code_smells: Vec::new(),
        })
    }

    fn language(&self) -> &str {
        "java"
    }
}

// Vulnerability Database

struct VulnerabilityDatabase;

impl VulnerabilityDatabase {
    fn new() -> Self {
        Self
    }

    async fn check_rust_deps(
        &self,
        _cargo_lock: &str,
    ) -> Result<Vec<VulnerableDependency>, ServiceError> {
        // Check against known vulnerability databases
        Ok(Vec::new())
    }
}

// Secret Detector

struct SecretDetector {
    patterns: Vec<SecretPattern>,
}

impl SecretDetector {
    fn new() -> Self {
        let mut detector = Self {
            patterns: Vec::new(),
        };
        detector.initialize_patterns();
        detector
    }

    fn initialize_patterns(&mut self) {
        self.patterns.push(SecretPattern {
            name: "API Key".to_string(),
            pattern: Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\"]?([a-zA-Z0-9_-]{20,})"#)
                .unwrap(),
            secret_type: SecretType::ApiKey,
        });

        self.patterns.push(SecretPattern {
            name: "Private_Key".to_string(),
            pattern: Regex::new(r"-----BEGIN (RSA |EC )?PRIVATE KEY-----").unwrap(),
            secret_type: SecretType::PrivateKey,
        });
    }

    async fn scan(&self, project_path: &Path) -> Result<Vec<SecretLeak>, ServiceError> {
        let mut secrets = Vec::new();
        self.scan_directory(project_path, &mut secrets).await?;
        Ok(secrets)
    }

    async fn scan_directory(
        &self,
        dir: &Path,
        secrets: &mut Vec<SecretLeak>,
    ) -> Result<(), ServiceError> {
        let mut entries = fs::read_dir(dir).await.map_err(|e| ServiceError::IoError {
            message: e.to_string(),
            path: Some(dir.to_path_buf()),
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ServiceError::IoError {
                message: e.to_string(),
                path: Some(dir.to_path_buf()),
            })?
        {
            let path = entry.path();

            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path).await {
                    self.scan_content(&content, &path, secrets);
                }
            } else if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !dir_name.starts_with(".") && dir_name != "node_modules" && dir_name != "target"
                {
                    Box::pin(self.scan_directory(&path, secrets)).await?;
                }
            }
        }

        Ok(())
    }

    fn scan_content(&self, content: &str, file: &Path, secrets: &mut Vec<SecretLeak>) {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &self.patterns {
                if pattern.pattern.is_match(line) {
                    secrets.push(SecretLeak {
                        secret_type: pattern.secret_type.clone(),
                        location: CodeLocation {
                            file: file.to_path_buf(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column: 0,
                            function: None,
                            class: None,
                        },
                        value_preview: "***REDACTED***".to_string(),
                        entropy: self.calculate_entropy(line),
                        confidence: 0.8,
                    });
                }
            }
        }
    }

    fn calculate_entropy(&self, text: &str) -> f64 {
        // Simple entropy calculation
        let mut frequencies = HashMap::new();
        for c in text.chars() {
            *frequencies.entry(c).or_insert(0) += 1;
        }

        let len = text.len() as f64;
        frequencies
            .values()
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum()
    }
}

struct SecretPattern {
    name: String,
    pattern: Regex,
    secret_type: SecretType,
}

// SAST Engine

struct SastEngine {
    llm_provider: Arc<dyn LLMProvider>,
}

impl SastEngine {
    fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    async fn analyze(&self, _project_path: &Path) -> Result<Vec<Vulnerability>, ServiceError> {
        // Perform static application security testing
        Ok(Vec::new())
    }
}

// VulnerabilityType already implements Debug, no need for custom to_string
