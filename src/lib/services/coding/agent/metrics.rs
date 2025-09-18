use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use super::types::{PerformanceMetrics, LearningMetrics};
use super::providers::ModelPerformanceMetrics;

/// Metrics manager for tracking performance and learning
pub struct MetricsManager {
    performance_metrics: PerformanceMetrics,
    learning_metrics: LearningMetrics,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self {
            performance_metrics: PerformanceMetrics::default(),
            learning_metrics: LearningMetrics::default(),
        }
    }

    /// Record command execution outcome for learning
    pub fn record_command_execution(
        &mut self,
        command: &str,
        success: bool,
        execution_time: Duration,
        task_type: &str,
    ) {
        // Update performance metrics
        self.performance_metrics.total_commands_executed += 1;
        
        // Update average execution time
        let new_time = execution_time.as_secs_f64();
        let total_commands = self.performance_metrics.total_commands_executed as f64;
        self.performance_metrics.average_execution_time = 
            (self.performance_metrics.average_execution_time * (total_commands - 1.0) + new_time) / total_commands;

        // Update success rate
        let success_count = if success { 1.0 } else { 0.0 };
        self.performance_metrics.success_rate = 
            (self.performance_metrics.success_rate * (total_commands - 1.0) + success_count) / total_commands;

        // Track most used commands
        let base_command = command.split_whitespace().next().unwrap_or(command);
        *self.performance_metrics.most_used_commands.entry(base_command.to_string()).or_insert(0) += 1;

        // Update learning metrics - command success patterns
        let current_success_rate = self.learning_metrics.command_success_patterns
            .get(base_command)
            .copied()
            .unwrap_or(0.5);
        
        let new_success_rate = (current_success_rate + success_count as f32) / 2.0;
        self.learning_metrics.command_success_patterns.insert(base_command.to_string(), new_success_rate);

        // Track task completion times
        self.learning_metrics.task_completion_times
            .entry(task_type.to_string())
            .or_insert_with(Vec::new)
            .push(execution_time.as_millis() as u64);

        // Keep only recent completion times (last 10)
        if let Some(times) = self.learning_metrics.task_completion_times.get_mut(task_type) {
            if times.len() > 10 {
                times.remove(0);
            }
        }
    }

    /// Record error patterns for learning
    pub fn record_error_pattern(&mut self, error_message: &str, resolution: Option<&str>) {
        // Extract error pattern (first line or key phrases)
        let error_pattern = self.extract_error_pattern(error_message);
        
        // Update error patterns in performance metrics
        *self.performance_metrics.error_patterns.entry(error_pattern.clone()).or_insert(0) += 1;

        // Record successful resolutions
        if let Some(resolution) = resolution {
            self.learning_metrics.error_resolution_patterns
                .entry(error_pattern)
                .or_insert_with(Vec::new)
                .push(resolution.to_string());
        }
    }

    /// Extract meaningful error pattern from error message
    fn extract_error_pattern(&self, error_message: &str) -> String {
        let error_lower = error_message.to_lowercase();
        
        // Common error patterns
        if error_lower.contains("permission denied") {
            "permission_denied".to_string()
        } else if error_lower.contains("file not found") || error_lower.contains("no such file") {
            "file_not_found".to_string()
        } else if error_lower.contains("command not found") {
            "command_not_found".to_string()
        } else if error_lower.contains("compilation failed") || error_lower.contains("compile error") {
            "compilation_error".to_string()
        } else if error_lower.contains("dependency") || error_lower.contains("package") {
            "dependency_error".to_string()
        } else if error_lower.contains("syntax error") || error_lower.contains("parse error") {
            "syntax_error".to_string()
        } else if error_lower.contains("network") || error_lower.contains("connection") {
            "network_error".to_string()
        } else {
            // Take first meaningful word or fallback to generic
            error_message.split_whitespace()
                .find(|word| word.len() > 3 && !word.chars().all(|c| c.is_numeric()))
                .unwrap_or("unknown_error")
                .to_lowercase()
        }
    }

    /// Record user preference
    pub fn record_user_preference(&mut self, preference_key: &str, preference_value: &str) {
        self.learning_metrics.user_preferences.insert(
            preference_key.to_string(),
            preference_value.to_string(),
        );
    }

    /// Get personalized recommendations based on learning
    pub fn get_personalized_recommendations(&self, task_type: &str) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Recommend commands with high success rates for this user
        let mut successful_commands: Vec<_> = self.learning_metrics.command_success_patterns
            .iter()
            .filter(|(_, &success_rate)| success_rate > 0.7)
            .collect();
        successful_commands.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (command, success_rate) in successful_commands.iter().take(3) {
            recommendations.push(format!(
                "Consider using `{}` ({}% success rate)",
                command,
                (*success_rate * 100.0) as u32
            ));
        }

        // Recommend based on task completion times
        if let Some(times) = self.learning_metrics.task_completion_times.get(task_type) {
            if !times.is_empty() {
                let avg_time = times.iter().sum::<u64>() / times.len() as u64;
                if avg_time > 5000 { // More than 5 seconds
                    recommendations.push(format!(
                        "This task type typically takes {}s - consider breaking it into smaller steps",
                        avg_time / 1000
                    ));
                }
            }
        }

        // Recommend based on common error patterns
        let mut common_errors: Vec<_> = self.performance_metrics.error_patterns.iter().collect();
        common_errors.sort_by(|a, b| b.1.cmp(a.1));

        if let Some((error_pattern, count)) = common_errors.first() {
            if *count > &2 {
                if let Some(resolutions) = self.learning_metrics.error_resolution_patterns.get(*error_pattern) {
                    if let Some(most_common_resolution) = resolutions.last() {
                        recommendations.push(format!(
                            "For '{}' errors, try: {}",
                            error_pattern,
                            most_common_resolution
                        ));
                    }
                }
            }
        }

        // User preference-based recommendations
        if let Some(preferred_editor) = self.learning_metrics.user_preferences.get("preferred_editor") {
            recommendations.push(format!("Use your preferred editor: {}", preferred_editor));
        }

        recommendations
    }

    /// Get learning insights for user
    pub fn get_learning_insights(&self) -> HashMap<String, serde_json::Value> {
        let mut insights = HashMap::new();

        // Command success rates
        insights.insert(
            "command_success_rates".to_string(),
            serde_json::to_value(&self.learning_metrics.command_success_patterns).unwrap_or_default(),
        );

        // Most used commands
        let mut most_used: Vec<_> = self.performance_metrics.most_used_commands.iter().collect();
        most_used.sort_by(|a, b| b.1.cmp(a.1));
        let top_commands: HashMap<String, u32> = most_used.into_iter().take(10).map(|(k, v)| (k.clone(), *v)).collect();
        insights.insert("most_used_commands".to_string(), serde_json::to_value(top_commands).unwrap_or_default());

        // Task completion patterns
        let mut avg_completion_times = HashMap::new();
        for (task_type, times) in &self.learning_metrics.task_completion_times {
            if !times.is_empty() {
                let avg = times.iter().sum::<u64>() / times.len() as u64;
                avg_completion_times.insert(task_type.clone(), avg);
            }
        }
        insights.insert("avg_task_completion_times".to_string(), serde_json::to_value(avg_completion_times).unwrap_or_default());

        // Error patterns
        insights.insert(
            "common_errors".to_string(),
            serde_json::to_value(&self.performance_metrics.error_patterns).unwrap_or_default(),
        );

        // User preferences
        insights.insert(
            "user_preferences".to_string(),
            serde_json::to_value(&self.learning_metrics.user_preferences).unwrap_or_default(),
        );

        // Overall statistics
        let mut stats = HashMap::new();
        stats.insert("total_commands".to_string(), serde_json::Value::Number(self.performance_metrics.total_commands_executed.into()));
        stats.insert("success_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.performance_metrics.success_rate).unwrap_or(serde_json::Number::from(0))));
        stats.insert("avg_execution_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.performance_metrics.average_execution_time).unwrap_or(serde_json::Number::from(0))));
        insights.insert("statistics".to_string(), serde_json::Value::Object(stats.into_iter().collect()));

        insights
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_metrics
    }

    /// Get learning metrics
    pub fn get_learning_metrics(&self) -> &LearningMetrics {
        &self.learning_metrics
    }

    /// Reset metrics (useful for testing or fresh starts)
    pub fn reset_metrics(&mut self) {
        self.performance_metrics = PerformanceMetrics::default();
        self.learning_metrics = LearningMetrics::default();
    }

    /// Export metrics to JSON
    pub fn export_performance_metrics(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&self.performance_metrics)
            .map_err(|e| anyhow::anyhow!("Failed to serialize performance metrics: {}", e))
    }

    /// Export learning metrics as JSON
    pub fn export_learning_metrics(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&self.learning_metrics)
            .map_err(|e| anyhow::anyhow!("Failed to serialize learning metrics: {}", e))
    }

    /// Get command recommendation based on patterns
    pub fn recommend_command(&self, task_description: &str) -> Option<String> {
        let task_lower = task_description.to_lowercase();
        
        // Simple pattern matching for command recommendations
        if task_lower.contains("list") || task_lower.contains("show") {
            Some("ls -la".to_string())
        } else if task_lower.contains("create") && task_lower.contains("file") {
            Some("touch".to_string())
        } else if task_lower.contains("create") && task_lower.contains("directory") {
            Some("mkdir".to_string())
        } else if task_lower.contains("copy") {
            Some("cp".to_string())
        } else if task_lower.contains("move") || task_lower.contains("rename") {
            Some("mv".to_string())
        } else if task_lower.contains("search") || task_lower.contains("find") {
            Some("grep".to_string())
        } else if task_lower.contains("git") && task_lower.contains("status") {
            Some("git status".to_string())
        } else if task_lower.contains("build") || task_lower.contains("compile") {
            // Look for project-specific build commands based on success patterns
            let build_commands = ["cargo build", "npm run build", "make", "go build"];
            for cmd in &build_commands {
                if let Some(&success_rate) = self.learning_metrics.command_success_patterns.get(*cmd) {
                    if success_rate > 0.6 {
                        return Some(cmd.to_string());
                    }
                }
            }
            Some("cargo build".to_string()) // Default for Rust
        } else {
            None
        }
    }

    /// Update metrics from external model performance data
    pub fn update_model_performance(&mut self, provider: &str, metrics: &ModelPerformanceMetrics) {
        // We could integrate model performance into our learning metrics
        // For now, just record as user preference
        self.record_user_preference(&format!("model_performance_{}", provider), &format!("{:.2}", metrics.success_rate));
    }

    /// Get success rate for a specific command
    pub fn get_command_success_rate(&self, command: &str) -> f32 {
        self.learning_metrics.command_success_patterns
            .get(command)
            .copied()
            .unwrap_or(0.5) // Default to 50% if no data
    }

    /// Check if a task type typically takes a long time
    pub fn is_slow_task_type(&self, task_type: &str) -> bool {
        if let Some(times) = self.learning_metrics.task_completion_times.get(task_type) {
            if !times.is_empty() {
                let avg_time = times.iter().sum::<u64>() / times.len() as u64;
                return avg_time > 10000; // More than 10 seconds
            }
        }
        false
    }

    /// Get the most common resolution for an error pattern
    pub fn get_common_error_resolution(&self, error_pattern: &str) -> Option<String> {
        self.learning_metrics.error_resolution_patterns
            .get(error_pattern)
            .and_then(|resolutions| resolutions.last())
            .cloned()
    }
}

impl Default for MetricsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_execution_recording() {
        let mut metrics = MetricsManager::new();
        
        metrics.record_command_execution("ls", true, Duration::from_millis(100), "file_listing");
        metrics.record_command_execution("ls", true, Duration::from_millis(150), "file_listing");
        metrics.record_command_execution("ls", false, Duration::from_millis(200), "file_listing");
        
        assert_eq!(metrics.performance_metrics.total_commands_executed, 3);
        assert!(metrics.performance_metrics.success_rate > 0.6);
        assert!(metrics.performance_metrics.success_rate < 0.7);
        
        let ls_success_rate = metrics.get_command_success_rate("ls");
        assert!(ls_success_rate > 0.6 && ls_success_rate < 0.8);
    }

    #[test]
    fn test_error_pattern_extraction() {
        let metrics = MetricsManager::new();
        
        assert_eq!(metrics.extract_error_pattern("Permission denied"), "permission_denied");
        assert_eq!(metrics.extract_error_pattern("File not found: test.txt"), "file_not_found");
        assert_eq!(metrics.extract_error_pattern("command not found: xyz"), "command_not_found");
    }

    #[test]
    fn test_recommendations() {
        let mut metrics = MetricsManager::new();
        
        // Record successful commands
        metrics.record_command_execution("git", true, Duration::from_millis(100), "version_control");
        metrics.record_command_execution("git", true, Duration::from_millis(120), "version_control");
        metrics.record_command_execution("cargo", true, Duration::from_millis(2000), "build");
        
        let recommendations = metrics.get_personalized_recommendations("version_control");
        assert!(!recommendations.is_empty());
    }
}