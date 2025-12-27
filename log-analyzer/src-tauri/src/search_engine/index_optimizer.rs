//! Automatic Index Optimizer
#![allow(dead_code)]
//!
//! Detects frequently searched terms and automatically creates optimized indexes
//! or suggests specialized index structures for common query patterns.
//!
//! **Feature: performance-optimization, Property 25: Automatic Index Optimization**
//! **Validates: Requirements 7.1**

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

/// Statistics for a specific query pattern
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryPatternStats {
    pub count: u64,
    pub total_duration_ms: u64,
    pub last_seen: Option<SystemTime>,
    pub avg_result_count: f64,
    pub slow_query_count: u64,
}

impl QueryPatternStats {
    /// Calculate average duration in milliseconds
    pub fn avg_duration_ms(&self) -> f64 {
        if self.count > 0 {
            self.total_duration_ms as f64 / self.count as f64
        } else {
            0.0
        }
    }
}

/// Specialized index recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializedIndexRecommendation {
    pub index_type: SpecializedIndexType,
    pub target_terms: Vec<String>,
    pub reason: String,
    pub estimated_improvement_percent: f32,
    pub priority: IndexPriority,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpecializedIndexType {
    /// Index optimized for specific frequently searched terms
    TermSpecific,
    /// Index partitioned by time ranges
    TimePartitioned,
    /// Composite index for multi-term queries
    Composite,
    /// Prefix-optimized index for autocomplete
    PrefixOptimized,
    /// Regex-optimized index
    RegexOptimized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndexPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Index maintenance task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMaintenanceTask {
    pub task_type: MaintenanceTaskType,
    pub scheduled_at: SystemTime,
    pub priority: IndexPriority,
    pub description: String,
    pub estimated_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenanceTaskType {
    /// Merge small index segments
    SegmentMerge,
    /// Optimize index for read performance
    OptimizeForRead,
    /// Clean up deleted documents
    GarbageCollection,
    /// Rebuild specialized indexes
    RebuildSpecialized,
    /// Update term statistics
    UpdateStatistics,
}

/// Index performance analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPerformanceAnalysis {
    pub total_queries_analyzed: u64,
    pub hot_query_count: usize,
    pub slow_query_count: u64,
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub recommendations: Vec<SpecializedIndexRecommendation>,
    pub maintenance_tasks: Vec<IndexMaintenanceTask>,
    pub health_score: f64,
    pub analysis_timestamp: SystemTime,
}

/// Configuration for the index optimizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexOptimizerConfig {
    /// Minimum query count to consider a pattern "hot"
    pub optimization_threshold: u64,
    /// Time window for analyzing query patterns
    pub analysis_window: Duration,
    /// Threshold for considering a query "slow" (ms)
    pub slow_query_threshold_ms: u64,
    /// Maximum number of specialized indexes to recommend
    pub max_recommendations: usize,
    /// Interval for automatic maintenance scheduling
    pub maintenance_interval: Duration,
    /// Enable automatic index creation
    pub auto_create_indexes: bool,
}

impl Default for IndexOptimizerConfig {
    fn default() -> Self {
        Self {
            optimization_threshold: 100,
            analysis_window: Duration::from_secs(3600), // 1 hour
            slow_query_threshold_ms: 200,
            max_recommendations: 10,
            maintenance_interval: Duration::from_secs(3600 * 6), // 6 hours
            auto_create_indexes: false,                          // Disabled by default for safety
        }
    }
}

/// Index optimizer for detecting and optimizing hot query paths
///
/// **Feature: performance-optimization, Property 25: Automatic Index Optimization**
/// **Validates: Requirements 7.1**
pub struct IndexOptimizer {
    query_patterns: Arc<RwLock<HashMap<String, QueryPatternStats>>>,
    specialized_indexes: Arc<RwLock<Vec<SpecializedIndexRecommendation>>>,
    maintenance_schedule: Arc<RwLock<Vec<IndexMaintenanceTask>>>,
    config: IndexOptimizerConfig,
    last_analysis: Arc<RwLock<Option<IndexPerformanceAnalysis>>>,
    created_indexes: Arc<RwLock<Vec<String>>>,
}

impl IndexOptimizer {
    /// Create a new index optimizer with default threshold
    pub fn new(optimization_threshold: u64) -> Self {
        Self::with_config(IndexOptimizerConfig {
            optimization_threshold,
            ..Default::default()
        })
    }

    /// Create a new index optimizer with custom configuration
    pub fn with_config(config: IndexOptimizerConfig) -> Self {
        Self {
            query_patterns: Arc::new(RwLock::new(HashMap::new())),
            specialized_indexes: Arc::new(RwLock::new(Vec::new())),
            maintenance_schedule: Arc::new(RwLock::new(Vec::new())),
            config,
            last_analysis: Arc::new(RwLock::new(None)),
            created_indexes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a query execution for analysis
    ///
    /// **Feature: performance-optimization, Property 25: Automatic Index Optimization**
    pub fn record_query(&self, query: &str, duration: Duration) {
        self.record_query_with_results(query, duration, 0);
    }

    /// Record a query execution with result count
    pub fn record_query_with_results(&self, query: &str, duration: Duration, result_count: usize) {
        let mut patterns = self.query_patterns.write();
        let stats = patterns.entry(query.to_string()).or_default();

        let duration_ms = duration.as_millis() as u64;
        stats.count += 1;
        stats.total_duration_ms += duration_ms;
        stats.last_seen = Some(SystemTime::now());

        // Update rolling average for result count
        let alpha = 0.1;
        stats.avg_result_count =
            alpha * result_count as f64 + (1.0 - alpha) * stats.avg_result_count;

        // Track slow queries
        if duration_ms > self.config.slow_query_threshold_ms {
            stats.slow_query_count += 1;
        }

        if stats.count >= self.config.optimization_threshold {
            debug!(
                query = %query,
                count = stats.count,
                avg_ms = stats.avg_duration_ms(),
                "Query pattern reached optimization threshold"
            );
        }
    }

    /// Identify "hot" queries that need optimization
    pub fn identify_hot_queries(&self) -> Vec<(String, QueryPatternStats)> {
        let patterns = self.query_patterns.read();
        let now = SystemTime::now();

        patterns
            .iter()
            .filter(|(_, stats)| {
                // Only consider queries seen within the analysis window
                if let Some(last_seen) = stats.last_seen {
                    if let Ok(elapsed) = now.duration_since(last_seen) {
                        return elapsed <= self.config.analysis_window
                            && stats.count >= self.config.optimization_threshold;
                    }
                }
                false
            })
            .map(|(q, s)| (q.clone(), s.clone()))
            .collect()
    }

    /// Suggest optimizations for hot queries
    pub fn suggest_optimizations(&self) -> Vec<String> {
        let hot_queries = self.identify_hot_queries();
        let mut suggestions = Vec::new();

        for (query, stats) in hot_queries {
            let avg_ms = stats.avg_duration_ms();
            if avg_ms > self.config.slow_query_threshold_ms as f64 {
                suggestions.push(format!(
                    "Query '{}' is hot ({} hits) and slow ({:.1}ms avg). Consider creating a specialized index or pre-calculating results.",
                    query, stats.count, avg_ms
                ));
            }
        }

        suggestions
    }

    /// Generate specialized index recommendations based on query patterns
    ///
    /// **Feature: performance-optimization, Property 25: Automatic Index Optimization**
    /// **Validates: Requirements 7.1**
    pub fn generate_index_recommendations(&self) -> Vec<SpecializedIndexRecommendation> {
        let hot_queries = self.identify_hot_queries();
        let mut recommendations = Vec::new();

        // Analyze hot queries for patterns
        for (query, stats) in &hot_queries {
            let avg_ms = stats.avg_duration_ms();

            // Skip if query is already fast enough
            if avg_ms < 50.0 {
                continue;
            }

            // Extract terms from query
            let terms: Vec<String> = query
                .split_whitespace()
                .filter(|t| t.len() > 2 && !t.contains('*'))
                .map(|s| s.to_lowercase())
                .collect();

            if terms.is_empty() {
                continue;
            }

            // Determine index type based on query characteristics
            let (index_type, estimated_improvement) = if query.contains('*') || query.contains('?')
            {
                (SpecializedIndexType::PrefixOptimized, 40.0)
            } else if query.contains("..") || query.contains('-') {
                // Time range pattern
                (SpecializedIndexType::TimePartitioned, 35.0)
            } else if terms.len() > 2 {
                (SpecializedIndexType::Composite, 30.0)
            } else {
                (SpecializedIndexType::TermSpecific, 25.0)
            };

            // Calculate priority based on query frequency and slowness
            let priority = if stats.count > 500 && avg_ms > 500.0 {
                IndexPriority::Critical
            } else if stats.count > 200 && avg_ms > 200.0 {
                IndexPriority::High
            } else if stats.count > 100 {
                IndexPriority::Medium
            } else {
                IndexPriority::Low
            };

            recommendations.push(SpecializedIndexRecommendation {
                index_type,
                target_terms: terms,
                reason: format!(
                    "Query executed {} times with avg {:.1}ms. {} slow queries detected.",
                    stats.count, avg_ms, stats.slow_query_count
                ),
                estimated_improvement_percent: estimated_improvement,
                priority,
                created_at: SystemTime::now(),
            });
        }

        // Sort by priority and limit
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));
        recommendations.truncate(self.config.max_recommendations);

        // Store recommendations
        *self.specialized_indexes.write() = recommendations.clone();

        info!(
            recommendation_count = recommendations.len(),
            "Generated specialized index recommendations"
        );

        recommendations
    }

    /// Schedule index maintenance tasks based on usage patterns
    ///
    /// **Validates: Requirements 7.1** - Index maintenance scheduling
    pub fn schedule_maintenance(&self) -> Vec<IndexMaintenanceTask> {
        let mut tasks = Vec::new();
        let now = SystemTime::now();
        let patterns = self.query_patterns.read();

        // Calculate overall query statistics
        let total_queries: u64 = patterns.values().map(|s| s.count).sum();
        let total_slow_queries: u64 = patterns.values().map(|s| s.slow_query_count).sum();
        let slow_query_ratio = if total_queries > 0 {
            total_slow_queries as f64 / total_queries as f64
        } else {
            0.0
        };

        // Schedule segment merge if many queries
        if total_queries > 10000 {
            tasks.push(IndexMaintenanceTask {
                task_type: MaintenanceTaskType::SegmentMerge,
                scheduled_at: now,
                priority: IndexPriority::Medium,
                description: "Merge index segments to improve read performance".to_string(),
                estimated_duration_ms: 5000,
            });
        }

        // Schedule optimization if slow query ratio is high
        if slow_query_ratio > 0.1 {
            tasks.push(IndexMaintenanceTask {
                task_type: MaintenanceTaskType::OptimizeForRead,
                scheduled_at: now,
                priority: IndexPriority::High,
                description: format!(
                    "Optimize index for read performance. {:.1}% queries are slow.",
                    slow_query_ratio * 100.0
                ),
                estimated_duration_ms: 10000,
            });
        }

        // Schedule statistics update periodically
        tasks.push(IndexMaintenanceTask {
            task_type: MaintenanceTaskType::UpdateStatistics,
            scheduled_at: now,
            priority: IndexPriority::Low,
            description: "Update term frequency statistics for query optimization".to_string(),
            estimated_duration_ms: 2000,
        });

        // Check if specialized indexes need rebuilding
        let recommendations = self.specialized_indexes.read();
        if !recommendations.is_empty() {
            let high_priority_count = recommendations
                .iter()
                .filter(|r| r.priority >= IndexPriority::High)
                .count();

            if high_priority_count > 0 {
                tasks.push(IndexMaintenanceTask {
                    task_type: MaintenanceTaskType::RebuildSpecialized,
                    scheduled_at: now,
                    priority: IndexPriority::High,
                    description: format!(
                        "Rebuild {} high-priority specialized indexes",
                        high_priority_count
                    ),
                    estimated_duration_ms: high_priority_count as u64 * 3000,
                });
            }
        }

        // Sort by priority
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Store schedule
        *self.maintenance_schedule.write() = tasks.clone();

        info!(
            task_count = tasks.len(),
            "Scheduled index maintenance tasks"
        );

        tasks
    }

    /// Perform comprehensive index performance analysis
    ///
    /// **Feature: performance-optimization, Property 25: Automatic Index Optimization**
    /// **Validates: Requirements 7.1**
    pub fn analyze_performance(&self) -> IndexPerformanceAnalysis {
        let patterns = self.query_patterns.read();
        let hot_queries = self.identify_hot_queries();

        // Calculate statistics
        let total_queries: u64 = patterns.values().map(|s| s.count).sum();
        let total_slow_queries: u64 = patterns.values().map(|s| s.slow_query_count).sum();

        let total_duration: u64 = patterns.values().map(|s| s.total_duration_ms).sum();
        let avg_query_time = if total_queries > 0 {
            total_duration as f64 / total_queries as f64
        } else {
            0.0
        };

        // Calculate p95 (simplified - using hot queries)
        let mut query_times: Vec<f64> = patterns.values().map(|s| s.avg_duration_ms()).collect();
        query_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_idx = (query_times.len() as f64 * 0.95) as usize;
        let p95_query_time = query_times
            .get(p95_idx.saturating_sub(1))
            .copied()
            .unwrap_or(0.0);

        // Generate recommendations and maintenance tasks
        drop(patterns); // Release lock before calling methods that need it
        let recommendations = self.generate_index_recommendations();
        let maintenance_tasks = self.schedule_maintenance();

        // Calculate health score (0-100)
        let health_score = self.calculate_health_score(
            avg_query_time,
            total_slow_queries as f64 / total_queries.max(1) as f64,
            hot_queries.len(),
        );

        let analysis = IndexPerformanceAnalysis {
            total_queries_analyzed: total_queries,
            hot_query_count: hot_queries.len(),
            slow_query_count: total_slow_queries,
            avg_query_time_ms: avg_query_time,
            p95_query_time_ms: p95_query_time,
            recommendations,
            maintenance_tasks,
            health_score,
            analysis_timestamp: SystemTime::now(),
        };

        // Store analysis
        *self.last_analysis.write() = Some(analysis.clone());

        info!(
            total_queries = total_queries,
            hot_queries = hot_queries.len(),
            slow_queries = total_slow_queries,
            health_score = health_score,
            "Completed index performance analysis"
        );

        analysis
    }

    /// Calculate index health score (0-100)
    fn calculate_health_score(
        &self,
        avg_query_time_ms: f64,
        slow_query_ratio: f64,
        hot_query_count: usize,
    ) -> f64 {
        let mut score = 100.0;

        // Penalize for slow average query time
        if avg_query_time_ms > 200.0 {
            score -= ((avg_query_time_ms - 200.0) / 10.0).min(30.0);
        }

        // Penalize for high slow query ratio
        score -= (slow_query_ratio * 100.0).min(30.0);

        // Penalize for many unoptimized hot queries
        if hot_query_count > 10 {
            score -= ((hot_query_count - 10) as f64 * 2.0).min(20.0);
        }

        score.max(0.0)
    }

    /// Check if automatic index creation should be triggered
    ///
    /// **Feature: performance-optimization, Property 25: Automatic Index Optimization**
    pub fn should_auto_create_index(&self, query_pattern: &str) -> bool {
        if !self.config.auto_create_indexes {
            return false;
        }

        let patterns = self.query_patterns.read();

        if let Some(stats) = patterns.get(query_pattern) {
            // Auto-create if:
            // 1. Query is very hot (>= 2x threshold)
            // 2. Query is slow (>= 2x slow threshold)
            // 3. Not already created
            let is_very_hot = stats.count >= self.config.optimization_threshold * 2;
            let is_very_slow =
                stats.avg_duration_ms() >= (self.config.slow_query_threshold_ms * 2) as f64;
            let not_created = !self
                .created_indexes
                .read()
                .contains(&query_pattern.to_string());

            is_very_hot && is_very_slow && not_created
        } else {
            false
        }
    }

    /// Mark an index as created (for tracking)
    pub fn mark_index_created(&self, query_pattern: &str) {
        self.created_indexes.write().push(query_pattern.to_string());
        info!(query_pattern = %query_pattern, "Marked specialized index as created");
    }

    /// Get the last performance analysis
    pub fn get_last_analysis(&self) -> Option<IndexPerformanceAnalysis> {
        self.last_analysis.read().clone()
    }

    /// Get current recommendations
    pub fn get_recommendations(&self) -> Vec<SpecializedIndexRecommendation> {
        self.specialized_indexes.read().clone()
    }

    /// Get scheduled maintenance tasks
    pub fn get_maintenance_schedule(&self) -> Vec<IndexMaintenanceTask> {
        self.maintenance_schedule.read().clone()
    }

    /// Clear old patterns to free memory
    pub fn cleanup_old_patterns(&self) {
        let mut patterns = self.query_patterns.write();
        let now = SystemTime::now();
        let cleanup_window = self.config.analysis_window * 2;

        let before_count = patterns.len();
        patterns.retain(|_, stats| {
            if let Some(last_seen) = stats.last_seen {
                if let Ok(elapsed) = now.duration_since(last_seen) {
                    return elapsed <= cleanup_window;
                }
            }
            false
        });

        let removed = before_count - patterns.len();
        if removed > 0 {
            debug!(removed = removed, "Cleaned up old query patterns");
        }
    }

    /// Get query pattern statistics
    pub fn get_query_patterns(&self) -> HashMap<String, QueryPatternStats> {
        self.query_patterns.read().clone()
    }

    /// Reset all statistics (for testing)
    pub fn reset(&self) {
        self.query_patterns.write().clear();
        self.specialized_indexes.write().clear();
        self.maintenance_schedule.write().clear();
        self.created_indexes.write().clear();
        *self.last_analysis.write() = None;
    }

    /// Get configuration
    pub fn get_config(&self) -> IndexOptimizerConfig {
        self.config.clone()
    }
}

impl Default for IndexOptimizer {
    fn default() -> Self {
        Self::new(100) // Default threshold of 100 hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_optimizer_creation() {
        let optimizer = IndexOptimizer::new(50);
        assert_eq!(optimizer.config.optimization_threshold, 50);
    }

    #[test]
    fn test_query_recording() {
        let optimizer = IndexOptimizer::new(10);

        for _ in 0..15 {
            optimizer.record_query("test query", Duration::from_millis(100));
        }

        let patterns = optimizer.get_query_patterns();
        let stats = patterns.get("test query").unwrap();

        assert_eq!(stats.count, 15);
        assert_eq!(stats.total_duration_ms, 1500);
    }

    #[test]
    fn test_hot_query_identification() {
        let optimizer = IndexOptimizer::new(10);

        // Record enough queries to be "hot"
        for _ in 0..15 {
            optimizer.record_query("hot query", Duration::from_millis(100));
        }

        // Record not enough queries
        for _ in 0..5 {
            optimizer.record_query("cold query", Duration::from_millis(100));
        }

        let hot_queries = optimizer.identify_hot_queries();
        assert_eq!(hot_queries.len(), 1);
        assert_eq!(hot_queries[0].0, "hot query");
    }

    #[test]
    fn test_optimization_suggestions() {
        let optimizer = IndexOptimizer::new(10);

        // Record slow hot queries
        for _ in 0..15 {
            optimizer.record_query("slow query", Duration::from_millis(300));
        }

        let suggestions = optimizer.suggest_optimizations();
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("slow query"));
    }

    #[test]
    fn test_index_recommendations() {
        let optimizer = IndexOptimizer::new(10);

        // Record various query patterns
        for _ in 0..20 {
            optimizer.record_query("error database connection", Duration::from_millis(250));
        }
        for _ in 0..15 {
            optimizer.record_query("warn*", Duration::from_millis(300));
        }

        let recommendations = optimizer.generate_index_recommendations();
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_performance_analysis() {
        let optimizer = IndexOptimizer::new(10);

        for _ in 0..20 {
            optimizer.record_query("test query", Duration::from_millis(150));
        }
        for _ in 0..5 {
            optimizer.record_query("slow query", Duration::from_millis(500));
        }

        let analysis = optimizer.analyze_performance();
        assert_eq!(analysis.total_queries_analyzed, 25);
        assert!(analysis.health_score > 0.0);
    }

    #[test]
    fn test_maintenance_scheduling() {
        let optimizer = IndexOptimizer::new(10);

        // Record many queries to trigger maintenance
        for i in 0..100 {
            optimizer.record_query(
                &format!("query {}", i % 10),
                Duration::from_millis(if i % 5 == 0 { 300 } else { 50 }),
            );
        }

        let tasks = optimizer.schedule_maintenance();
        assert!(!tasks.is_empty());
    }
}
