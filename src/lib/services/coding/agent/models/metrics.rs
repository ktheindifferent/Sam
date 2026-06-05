//! Performance and metrics models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_time: Duration,
    pub memory_usage: MemoryUsage,
    pub cpu_usage: f32,
    pub io_operations: IoMetrics,
    pub cache_hits: CacheMetrics,
}

/// Memory usage details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub heap_used: usize,
    pub heap_allocated: usize,
    pub stack_used: usize,
    pub virtual_memory: usize,
    pub resident_set_size: usize,
}

/// I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoMetrics {
    pub reads: usize,
    pub writes: usize,
    pub bytes_read: usize,
    pub bytes_written: usize,
    pub read_time: Duration,
    pub write_time: Duration,
}

/// Cache metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub hit_rate: f32,
}

/// Performance suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSuggestions {
    pub optimizations: Vec<PerformanceOptimization>,
    pub bottlenecks: Vec<Bottleneck>,
    pub estimated_improvement: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceOptimization {
    pub optimization_type: OptimizationType,
    pub description: String,
    pub location: String,
    pub impact: PerformanceImpact,
    pub implementation_effort: EffortLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    Algorithmic,
    Caching,
    Parallelization,
    DatabaseQuery,
    NetworkOptimization,
    MemoryManagement,
    CodeRefactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceImpact {
    Major,    // >50% improvement
    Moderate, // 20-50% improvement
    Minor,    // <20% improvement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Trivial,
    Easy,
    Moderate,
    Hard,
    VeryHard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub location: String,
    pub bottleneck_type: BottleneckType,
    pub severity: f32,
    pub description: String,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    Cpu,
    Memory,
    Io,
    Network,
    Lock,
    Algorithm,
}

/// Profiling results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingResult {
    pub profile_type: ProfileType,
    pub duration: Duration,
    pub samples: Vec<ProfileSample>,
    pub summary: ProfileSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileType {
    Cpu,
    Memory,
    Io,
    Network,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSample {
    pub timestamp: u64,
    pub stack_trace: Vec<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub total_samples: usize,
    pub hottest_functions: Vec<HotFunction>,
    pub memory_allocations: Vec<MemoryAllocation>,
    pub io_patterns: Vec<IoPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFunction {
    pub name: String,
    pub self_time_percent: f32,
    pub total_time_percent: f32,
    pub call_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub location: String,
    pub size: usize,
    pub count: usize,
    pub allocation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoPattern {
    pub pattern_type: IoPatternType,
    pub frequency: usize,
    pub impact: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoPatternType {
    Sequential,
    Random,
    Burst,
    Streaming,
}
