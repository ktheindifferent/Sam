use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use tokio::fs;

use super::errors::CodingAgentError as ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub language: String,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub timeout_seconds: u64,
    pub memory_profiling: bool,
    pub cpu_profiling: bool,
    pub detailed_metrics: bool,
    pub comparison_baseline: Option<String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            language: "rust".to_string(),
            iterations: 100,
            warmup_iterations: 10,
            timeout_seconds: 60,
            memory_profiling: true,
            cpu_profiling: true,
            detailed_metrics: true,
            comparison_baseline: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_mean: Duration,
    pub duration_median: Duration,
    pub duration_min: Duration,
    pub duration_max: Duration,
    pub duration_std_dev: Duration,
    pub throughput: f64,
    pub memory_peak: Option<usize>,
    pub memory_average: Option<usize>,
    pub cpu_usage: Option<f64>,
    pub iterations_completed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub title: String,
    pub description: String,
    pub impact: OptimizationImpact,
    pub difficulty: Difficulty,
    pub code_before: String,
    pub code_after: String,
    pub expected_improvement: f64,
    pub category: OptimizationCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationImpact {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationCategory {
    Algorithm,
    DataStructure,
    Memory,
    Concurrency,
    IO,
    Caching,
    Database,
    Network,
}

pub struct BenchmarkingEngine {
    runners: HashMap<String, Box<dyn BenchmarkRunner>>,
    profilers: HashMap<String, Box<dyn Profiler>>,
    optimizers: HashMap<String, Box<dyn Optimizer>>,
}

impl BenchmarkingEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            runners: HashMap::new(),
            profilers: HashMap::new(),
            optimizers: HashMap::new(),
        };

        engine.register_components();
        engine
    }

    fn register_components(&mut self) {
        // Benchmark runners for different languages
        self.runners.insert("rust".to_string(), Box::new(RustBenchmarkRunner::new()));
        self.runners.insert("python".to_string(), Box::new(PythonBenchmarkRunner::new()));
        self.runners.insert("javascript".to_string(), Box::new(JsBenchmarkRunner::new()));
        self.runners.insert("go".to_string(), Box::new(GoBenchmarkRunner::new()));
        self.runners.insert("java".to_string(), Box::new(JavaBenchmarkRunner::new()));

        // Profilers
        self.profilers.insert("memory".to_string(), Box::new(MemoryProfiler::new()));
        self.profilers.insert("cpu".to_string(), Box::new(CpuProfiler::new()));
        self.profilers.insert("io".to_string(), Box::new(IoProfiler::new()));

        // Optimizers
        self.optimizers.insert("algorithm".to_string(), Box::new(AlgorithmOptimizer::new()));
        self.optimizers.insert("memory".to_string(), Box::new(MemoryOptimizer::new()));
        self.optimizers.insert("concurrency".to_string(), Box::new(ConcurrencyOptimizer::new()));
    }

    pub async fn benchmark_code(
        &self,
        code_path: &Path,
        config: &BenchmarkConfig,
    ) -> Result<Vec<BenchmarkResult>, ServiceError> {
        let runner = self.runners.get(&config.language)
            .ok_or_else(|| ServiceError::ConfigError {
                message: format!("No benchmark runner for language: {}", config.language)
            })?;

        let mut results = Vec::new();

        // Run warmup iterations
        for _ in 0..config.warmup_iterations {
            runner.run_once(code_path).await?;
        }

        // Collect timing data
        let mut timings = Vec::new();
        let mut memory_samples = Vec::new();
        let mut cpu_samples = Vec::new();

        for _ in 0..config.iterations {
            let start = Instant::now();

            // Run with profiling if enabled
            if config.memory_profiling || config.cpu_profiling {
                let (mem, cpu) = self.run_with_profiling(code_path, runner.as_ref()).await?;
                if config.memory_profiling {
                    memory_samples.push(mem);
                }
                if config.cpu_profiling {
                    cpu_samples.push(cpu);
                }
            } else {
                runner.run_once(code_path).await?;
            }

            let elapsed = start.elapsed();
            timings.push(elapsed);
        }

        // Calculate statistics
        let result = self.calculate_statistics(
            "benchmark".to_string(),
            timings,
            memory_samples,
            cpu_samples,
        );

        results.push(result);

        // Compare with baseline if provided
        if let Some(baseline_path) = &config.comparison_baseline {
            let baseline_result = self.load_baseline(baseline_path).await?;
            results.push(baseline_result);
        }

        Ok(results)
    }

    async fn run_with_profiling(
        &self,
        code_path: &Path,
        runner: &dyn BenchmarkRunner,
    ) -> Result<(usize, f64), ServiceError> {
        // Start profilers
        let memory_profiler = self.profilers.get("memory").unwrap();
        let cpu_profiler = self.profilers.get("cpu").unwrap();

        memory_profiler.start().await?;
        cpu_profiler.start().await?;

        // Run code
        runner.run_once(code_path).await?;

        // Stop profilers and get results
        let memory_usage = memory_profiler.stop().await?;
        let cpu_usage = cpu_profiler.stop().await?;

        Ok((memory_usage as usize, cpu_usage))
    }

    fn calculate_statistics(
        &self,
        name: String,
        timings: Vec<Duration>,
        memory_samples: Vec<usize>,
        cpu_samples: Vec<f64>,
    ) -> BenchmarkResult {
        let mut sorted_timings = timings.clone();
        sorted_timings.sort();

        let mean = timings.iter().sum::<Duration>() / timings.len() as u32;
        let median = sorted_timings[sorted_timings.len() / 2];
        let min = *sorted_timings.first().unwrap();
        let max = *sorted_timings.last().unwrap();

        // Calculate standard deviation
        let variance = timings.iter()
            .map(|t| {
                let diff = t.as_secs_f64() - mean.as_secs_f64();
                diff * diff
            })
            .sum::<f64>() / timings.len() as f64;
        let std_dev = Duration::from_secs_f64(variance.sqrt());

        // Calculate throughput (operations per second)
        let throughput = 1.0 / mean.as_secs_f64();

        // Memory statistics
        let memory_peak = memory_samples.iter().max().copied();
        let memory_average = if !memory_samples.is_empty() {
            Some(memory_samples.iter().sum::<usize>() / memory_samples.len())
        } else {
            None
        };

        // CPU statistics
        let cpu_usage = if !cpu_samples.is_empty() {
            Some(cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64)
        } else {
            None
        };

        BenchmarkResult {
            name,
            duration_mean: mean,
            duration_median: median,
            duration_min: min,
            duration_max: max,
            duration_std_dev: std_dev,
            throughput,
            memory_peak,
            memory_average,
            cpu_usage,
            iterations_completed: timings.len(),
            errors: Vec::new(),
        }
    }

    async fn load_baseline(&self, path: &str) -> Result<BenchmarkResult, ServiceError> {
        let content = fs::read_to_string(path).await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        serde_json::from_str(&content)
            .map_err(|e| ServiceError::ConfigError { message: e.to_string() })
    }

    pub async fn optimize_code(
        &self,
        code_path: &Path,
        language: &str,
    ) -> Result<Vec<OptimizationSuggestion>, ServiceError> {
        let code = fs::read_to_string(code_path).await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        let mut suggestions = Vec::new();

        // Run all optimizers
        for (name, optimizer) in &self.optimizers {
            let optimizer_suggestions = optimizer.analyze(&code, language).await?;
            suggestions.extend(optimizer_suggestions);
        }

        // Sort by impact and difficulty
        suggestions.sort_by(|a, b| {
            match (&a.impact, &b.impact) {
                (OptimizationImpact::High, OptimizationImpact::High) => std::cmp::Ordering::Equal,
                (OptimizationImpact::High, _) => std::cmp::Ordering::Less,
                (_, OptimizationImpact::High) => std::cmp::Ordering::Greater,
                (OptimizationImpact::Medium, OptimizationImpact::Medium) => std::cmp::Ordering::Equal,
                (OptimizationImpact::Medium, _) => std::cmp::Ordering::Less,
                (_, OptimizationImpact::Medium) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        Ok(suggestions)
    }

    pub async fn profile_hotspots(
        &self,
        code_path: &Path,
        language: &str,
    ) -> Result<HotspotAnalysis, ServiceError> {
        // Run code with detailed profiling
        let profiling_data = self.collect_profiling_data(code_path, language).await?;

        // Analyze hotspots
        let hotspots = self.identify_hotspots(&profiling_data);

        Ok(HotspotAnalysis {
            hotspots,
            call_graph: self.build_call_graph(&profiling_data),
            memory_allocations: self.analyze_memory_allocations(&profiling_data),
        })
    }

    async fn collect_profiling_data(
        &self,
        code_path: &Path,
        language: &str,
    ) -> Result<ProfilingData, ServiceError> {
        // Language-specific profiling
        match language {
            "rust" => self.profile_rust_code(code_path).await,
            "python" => self.profile_python_code(code_path).await,
            "javascript" => self.profile_js_code(code_path).await,
            _ => Err(ServiceError::ConfigError { message: format!("Profiling not supported for {}", language) }),
        }
    }

    async fn profile_rust_code(&self, code_path: &Path) -> Result<ProfilingData, ServiceError> {
        // Use cargo-flamegraph or similar
        let output = Command::new("cargo")
            .args(&["flamegraph", "--bin", code_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        // Parse flamegraph output
        Ok(ProfilingData::default())
    }

    async fn profile_python_code(&self, code_path: &Path) -> Result<ProfilingData, ServiceError> {
        // Use cProfile
        let output = Command::new("python")
            .args(&["-m", "cProfile", "-o", "profile.stats", code_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        // Parse profile stats
        Ok(ProfilingData::default())
    }

    async fn profile_js_code(&self, code_path: &Path) -> Result<ProfilingData, ServiceError> {
        // Use node --prof
        let output = Command::new("node")
            .args(&["--prof", code_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        // Parse V8 profiling data
        Ok(ProfilingData::default())
    }

    fn identify_hotspots(&self, data: &ProfilingData) -> Vec<Hotspot> {
        let mut hotspots = Vec::new();

        for (function, stats) in &data.function_stats {
            if stats.self_time > Duration::from_millis(100) {
                hotspots.push(Hotspot {
                    function: function.clone(),
                    self_time: stats.self_time,
                    total_time: stats.total_time,
                    call_count: stats.call_count,
                    location: stats.location.clone(),
                });
            }
        }

        hotspots.sort_by_key(|h| std::cmp::Reverse(h.self_time));
        hotspots
    }

    fn build_call_graph(&self, data: &ProfilingData) -> CallGraph {
        CallGraph {
            nodes: data.function_stats.keys().cloned().collect(),
            edges: data.call_edges.clone(),
        }
    }

    fn analyze_memory_allocations(&self, data: &ProfilingData) -> Vec<MemoryAllocation> {
        data.memory_events.iter()
            .map(|event| MemoryAllocation {
                location: event.location.clone(),
                size: event.size,
                count: event.count,
                allocation_type: event.allocation_type.clone(),
            })
            .collect()
    }
}

// Benchmark runner trait and implementations

#[async_trait]
trait BenchmarkRunner: Send + Sync {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError>;
    fn language(&self) -> &str;
}

struct RustBenchmarkRunner;

impl RustBenchmarkRunner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BenchmarkRunner for RustBenchmarkRunner {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError> {
        let output = Command::new("cargo")
            .args(&["bench", "--bench", code_path.file_stem().unwrap().to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        if !output.status.success() {
            return Err(ServiceError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(())
    }

    fn language(&self) -> &str { "rust" }
}

struct PythonBenchmarkRunner;

impl PythonBenchmarkRunner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BenchmarkRunner for PythonBenchmarkRunner {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError> {
        let output = Command::new("python")
            .args(&["-m", "timeit", "-n", "1", "-r", "1",
                   &format!("exec(open('{}').read())", code_path.display())])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        if !output.status.success() {
            return Err(ServiceError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(())
    }

    fn language(&self) -> &str { "python" }
}

struct JsBenchmarkRunner;

impl JsBenchmarkRunner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BenchmarkRunner for JsBenchmarkRunner {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError> {
        let output = Command::new("node")
            .arg(code_path)
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        if !output.status.success() {
            return Err(ServiceError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(())
    }

    fn language(&self) -> &str { "javascript" }
}

struct GoBenchmarkRunner;

impl GoBenchmarkRunner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BenchmarkRunner for GoBenchmarkRunner {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError> {
        let output = Command::new("go")
            .args(&["test", "-bench", ".", code_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        if !output.status.success() {
            return Err(ServiceError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(())
    }

    fn language(&self) -> &str { "go" }
}

struct JavaBenchmarkRunner;

impl JavaBenchmarkRunner {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BenchmarkRunner for JavaBenchmarkRunner {
    async fn run_once(&self, code_path: &Path) -> Result<(), ServiceError> {
        // Use JMH (Java Microbenchmark Harness)
        let output = Command::new("java")
            .args(&["-jar", "jmh.jar", code_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| ServiceError::IoError { message: e.to_string(), path: None })?;

        if !output.status.success() {
            return Err(ServiceError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(())
    }

    fn language(&self) -> &str { "java" }
}

// Profiler trait and implementations

#[async_trait]
trait Profiler: Send + Sync {
    async fn start(&self) -> Result<(), ServiceError>;
    async fn stop(&self) -> Result<f64, ServiceError>;
    fn metric_type(&self) -> &str;
}

struct MemoryProfiler {
    baseline: Option<usize>,
}

impl MemoryProfiler {
    fn new() -> Self {
        Self { baseline: None }
    }
}

#[async_trait]
impl Profiler for MemoryProfiler {
    async fn start(&self) -> Result<(), ServiceError> {
        // Record baseline memory usage
        Ok(())
    }

    async fn stop(&self) -> Result<f64, ServiceError> {
        // Get current memory usage and calculate difference
        Ok(0.0)
    }

    fn metric_type(&self) -> &str { "memory" }
}

struct CpuProfiler {
    start_time: Option<Instant>,
}

impl CpuProfiler {
    fn new() -> Self {
        Self { start_time: None }
    }
}

#[async_trait]
impl Profiler for CpuProfiler {
    async fn start(&self) -> Result<(), ServiceError> {
        // Start CPU time tracking
        Ok(())
    }

    async fn stop(&self) -> Result<f64, ServiceError> {
        // Calculate CPU usage percentage
        Ok(0.0)
    }

    fn metric_type(&self) -> &str { "cpu" }
}

struct IoProfiler {
    io_start_stats: Option<IoStats>,
}

#[derive(Debug, Clone)]
struct IoStats {
    reads: usize,
    writes: usize,
    bytes_read: usize,
    bytes_written: usize,
}

impl IoProfiler {
    fn new() -> Self {
        Self { io_start_stats: None }
    }
}

#[async_trait]
impl Profiler for IoProfiler {
    async fn start(&self) -> Result<(), ServiceError> {
        // Record baseline I/O stats
        Ok(())
    }

    async fn stop(&self) -> Result<f64, ServiceError> {
        // Calculate I/O operations
        Ok(0.0)
    }

    fn metric_type(&self) -> &str { "io" }
}

// Optimizer trait and implementations

#[async_trait]
trait Optimizer: Send + Sync {
    async fn analyze(&self, code: &str, language: &str) -> Result<Vec<OptimizationSuggestion>, ServiceError>;
    fn optimization_type(&self) -> &str;
}

struct AlgorithmOptimizer;

impl AlgorithmOptimizer {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Optimizer for AlgorithmOptimizer {
    async fn analyze(&self, code: &str, _language: &str) -> Result<Vec<OptimizationSuggestion>, ServiceError> {
        let mut suggestions = Vec::new();

        // Check for common algorithm inefficiencies
        if code.contains("for") && code.contains("for") {
            // Nested loops
            suggestions.push(OptimizationSuggestion {
                title: "Nested Loop Optimization".to_string(),
                description: "Consider using a more efficient algorithm to avoid O(n²) complexity".to_string(),
                impact: OptimizationImpact::High,
                difficulty: Difficulty::Medium,
                code_before: "for i in items:\n    for j in items:".to_string(),
                code_after: "# Use hash map or sorting for better performance".to_string(),
                expected_improvement: 50.0,
                category: OptimizationCategory::Algorithm,
            });
        }

        if code.contains(".sort()") && code.contains(".sort()") {
            // Multiple sorts
            suggestions.push(OptimizationSuggestion {
                title: "Combine Sorting Operations".to_string(),
                description: "Multiple sorting operations can be combined".to_string(),
                impact: OptimizationImpact::Medium,
                difficulty: Difficulty::Easy,
                code_before: "list.sort()\n...\nlist.sort()".to_string(),
                code_after: "# Sort once with composite key".to_string(),
                expected_improvement: 30.0,
                category: OptimizationCategory::Algorithm,
            });
        }

        Ok(suggestions)
    }

    fn optimization_type(&self) -> &str { "algorithm" }
}

struct MemoryOptimizer;

impl MemoryOptimizer {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Optimizer for MemoryOptimizer {
    async fn analyze(&self, code: &str, _language: &str) -> Result<Vec<OptimizationSuggestion>, ServiceError> {
        let mut suggestions = Vec::new();

        // Check for memory inefficiencies
        if code.contains("clone()") && code.matches("clone()").count() > 3 {
            suggestions.push(OptimizationSuggestion {
                title: "Reduce Unnecessary Cloning".to_string(),
                description: "Consider using references instead of cloning".to_string(),
                impact: OptimizationImpact::Medium,
                difficulty: Difficulty::Medium,
                code_before: "let copy = data.clone();".to_string(),
                code_after: "let copy = &data;".to_string(),
                expected_improvement: 20.0,
                category: OptimizationCategory::Memory,
            });
        }

        if code.contains("Vec::new()") && code.contains("push") {
            suggestions.push(OptimizationSuggestion {
                title: "Pre-allocate Vector Capacity".to_string(),
                description: "Use Vec::with_capacity when size is known".to_string(),
                impact: OptimizationImpact::Low,
                difficulty: Difficulty::Easy,
                code_before: "let mut v = Vec::new();".to_string(),
                code_after: "let mut v = Vec::with_capacity(100);".to_string(),
                expected_improvement: 10.0,
                category: OptimizationCategory::Memory,
            });
        }

        Ok(suggestions)
    }

    fn optimization_type(&self) -> &str { "memory" }
}

struct ConcurrencyOptimizer;

impl ConcurrencyOptimizer {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Optimizer for ConcurrencyOptimizer {
    async fn analyze(&self, code: &str, _language: &str) -> Result<Vec<OptimizationSuggestion>, ServiceError> {
        let mut suggestions = Vec::new();

        // Check for concurrency opportunities
        if code.contains("for") && code.contains("process") && !code.contains("parallel") {
            suggestions.push(OptimizationSuggestion {
                title: "Parallelize Loop Processing".to_string(),
                description: "This loop could benefit from parallel processing".to_string(),
                impact: OptimizationImpact::High,
                difficulty: Difficulty::Medium,
                code_before: "for item in items { process(item); }".to_string(),
                code_after: "items.par_iter().for_each(|item| process(item));".to_string(),
                expected_improvement: 60.0,
                category: OptimizationCategory::Concurrency,
            });
        }

        if code.contains("Mutex") && code.contains("lock()") {
            suggestions.push(OptimizationSuggestion {
                title: "Consider Lock-Free Data Structures".to_string(),
                description: "Lock-free alternatives might improve performance".to_string(),
                impact: OptimizationImpact::Medium,
                difficulty: Difficulty::Hard,
                code_before: "let data = mutex.lock().unwrap();".to_string(),
                code_after: "// Use Arc<RwLock> or lock-free structures".to_string(),
                expected_improvement: 25.0,
                category: OptimizationCategory::Concurrency,
            });
        }

        Ok(suggestions)
    }

    fn optimization_type(&self) -> &str { "concurrency" }
}

// Supporting data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotAnalysis {
    pub hotspots: Vec<Hotspot>,
    pub call_graph: CallGraph,
    pub memory_allocations: Vec<MemoryAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub function: String,
    pub self_time: Duration,
    pub total_time: Duration,
    pub call_count: usize,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub location: String,
    pub size: usize,
    pub count: usize,
    pub allocation_type: String,
}

#[derive(Debug, Clone, Default)]
struct ProfilingData {
    function_stats: HashMap<String, FunctionStats>,
    call_edges: Vec<(String, String, usize)>,
    memory_events: Vec<MemoryEvent>,
}

#[derive(Debug, Clone)]
struct FunctionStats {
    self_time: Duration,
    total_time: Duration,
    call_count: usize,
    location: String,
}

#[derive(Debug, Clone)]
struct MemoryEvent {
    location: String,
    size: usize,
    count: usize,
    allocation_type: String,
}

// Performance comparison

pub struct PerformanceComparator {
    baseline_results: HashMap<String, BenchmarkResult>,
}

impl PerformanceComparator {
    pub fn new() -> Self {
        Self {
            baseline_results: HashMap::new(),
        }
    }

    pub fn compare(
        &self,
        before: &BenchmarkResult,
        after: &BenchmarkResult,
    ) -> ComparisonReport {
        let speedup = before.duration_mean.as_secs_f64() / after.duration_mean.as_secs_f64();
        let memory_reduction = if let (Some(before_mem), Some(after_mem)) = (before.memory_peak, after.memory_peak) {
            Some((before_mem as f64 - after_mem as f64) / before_mem as f64 * 100.0)
        } else {
            None
        };

        ComparisonReport {
            speedup,
            memory_reduction,
            duration_change: after.duration_mean.as_secs_f64() - before.duration_mean.as_secs_f64(),
            throughput_change: after.throughput - before.throughput,
            regression: speedup < 0.95,
            improvement: speedup > 1.05,
        }
    }

    pub async fn generate_report(
        &self,
        results: Vec<BenchmarkResult>,
    ) -> Result<String, ServiceError> {
        let mut report = String::from("# Performance Benchmark Report\n\n");

        for result in results {
            report.push_str(&format!("## {}\n\n", result.name));
            report.push_str(&format!("- Mean Duration: {:?}\n", result.duration_mean));
            report.push_str(&format!("- Median Duration: {:?}\n", result.duration_median));
            report.push_str(&format!("- Min/Max: {:?}/{:?}\n", result.duration_min, result.duration_max));
            report.push_str(&format!("- Throughput: {:.2} ops/sec\n", result.throughput));

            if let Some(mem) = result.memory_peak {
                report.push_str(&format!("- Peak Memory: {} MB\n", mem / 1024 / 1024));
            }

            if let Some(cpu) = result.cpu_usage {
                report.push_str(&format!("- CPU Usage: {:.1}%\n", cpu));
            }

            report.push_str("\n");
        }

        Ok(report)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub speedup: f64,
    pub memory_reduction: Option<f64>,
    pub duration_change: f64,
    pub throughput_change: f64,
    pub regression: bool,
    pub improvement: bool,
}