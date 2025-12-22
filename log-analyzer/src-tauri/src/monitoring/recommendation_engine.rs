//! Optimization Recommendation Engine
//!
//! **Feature: performance-optimization, Property 19: Optimization Recommendations**
//! Analyzes performance patterns and provides automatic recommendations for:
//! - Query optimization and index tuning
//! - Resource allocation suggestions
//! - Performance trend analysis and capacity planning

use crate::monitoring::metrics_collector::{
    CacheMetricsSnapshot, QueryTimingStats, SystemResourceMetrics,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Types of optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationType {
    QueryOptimization,
    IndexTuning,
    ResourceAllocation,
    CacheTuning,
    CapacityPlanning,
    /// Performance trend warning
    PerformanceTrend,
    /// System configuration suggestion
    SystemConfiguration,
}

/// Priority level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Individual optimization recommendation
/// **Validates: Requirements 4.5** - Optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub rec_type: RecommendationType,
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub action_item: String,
    pub impact_score: f64, // 0.0 to 1.0
    pub timestamp: SystemTime,
    /// Estimated improvement percentage if recommendation is followed
    pub estimated_improvement: Option<f64>,
    /// Related metrics that triggered this recommendation
    pub related_metrics: HashMap<String, f64>,
    /// Specific configuration changes suggested
    pub config_suggestions: Vec<ConfigSuggestion>,
}

/// Configuration suggestion for a recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSuggestion {
    pub config_key: String,
    pub current_value: String,
    pub suggested_value: String,
    pub rationale: String,
}

/// Performance trend data for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub metric_name: String,
    pub trend_direction: TrendDirection,
    pub change_percentage: f64,
    pub time_window_hours: u32,
    pub data_points: Vec<f64>,
}

/// Direction of a performance trend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Capacity planning recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityPlanningInfo {
    pub resource_type: String,
    pub current_usage_percent: f64,
    pub projected_usage_percent: f64,
    pub days_until_threshold: Option<u32>,
    pub recommended_action: String,
}

/// Recommendation engine for performance optimization
/// **Feature: performance-optimization, Property 19: Optimization Recommendations**
pub struct RecommendationEngine {
    last_analysis: parking_lot::RwLock<SystemTime>,
    /// Historical performance data for trend analysis
    performance_history: parking_lot::RwLock<Vec<PerformanceSnapshot>>,
    /// Maximum history entries to retain
    max_history_entries: usize,
}

/// Snapshot of performance metrics at a point in time
#[derive(Debug, Clone)]
struct PerformanceSnapshot {
    #[allow(dead_code)]
    timestamp: SystemTime,
    avg_search_time_ms: f64,
    cache_hit_rate: f64,
    cpu_usage: f64,
    memory_usage: f64,
}

impl RecommendationEngine {
    /// Create a new recommendation engine
    pub fn new() -> Self {
        Self {
            last_analysis: parking_lot::RwLock::new(SystemTime::now()),
            performance_history: parking_lot::RwLock::new(Vec::new()),
            max_history_entries: 1000,
        }
    }

    /// Analyze performance metrics and generate recommendations
    /// **Validates: Requirements 4.5** - Optimization recommendations
    pub fn analyze_and_recommend(
        &self,
        metrics: &HashMap<String, serde_json::Value>,
        cache_metrics: &CacheMetricsSnapshot,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // 1. Analyze Cache Performance
        self.analyze_cache_performance(cache_metrics, &mut recommendations);

        // 2. Analyze Search Performance
        self.analyze_search_performance(metrics, &mut recommendations);

        // 3. Analyze System Resources
        self.analyze_system_resources(metrics, &mut recommendations);

        *self.last_analysis.write() = SystemTime::now();
        recommendations
    }

    /// Extended analysis including query timing stats and system metrics
    /// **Validates: Requirements 4.5** - Resource allocation suggestions based on usage patterns
    pub fn analyze_comprehensive(
        &self,
        metrics: &HashMap<String, serde_json::Value>,
        cache_metrics: &CacheMetricsSnapshot,
        query_stats: Option<&QueryTimingStats>,
        system_metrics: Option<&SystemResourceMetrics>,
    ) -> Vec<Recommendation> {
        let mut recommendations = self.analyze_and_recommend(metrics, cache_metrics);

        // 4. Analyze Query Phase Timing
        if let Some(stats) = query_stats {
            self.analyze_query_phases(stats, &mut recommendations);
        }

        // 5. Analyze System Resource Trends
        if let Some(sys_metrics) = system_metrics {
            self.analyze_resource_trends(sys_metrics, &mut recommendations);

            // Store snapshot for trend analysis
            self.store_performance_snapshot(cache_metrics, sys_metrics);
        }

        // 6. Perform Capacity Planning Analysis
        self.analyze_capacity_planning(&mut recommendations);

        recommendations
    }

    /// Analyze cache metrics for tuning recommendations
    fn analyze_cache_performance(
        &self,
        metrics: &CacheMetricsSnapshot,
        recs: &mut Vec<Recommendation>,
    ) {
        // High L1 miss rate but high L2 hit rate
        if metrics.l1_hit_rate < 0.4 && metrics.l2_hit_rate > 0.7 {
            recs.push(Recommendation {
                id: "cache_l1_size".to_string(),
                rec_type: RecommendationType::CacheTuning,
                priority: RecommendationPriority::Medium,
                title: "Increase L1 Cache Capacity".to_string(),
                description: "High L1 miss rate with high L2 hit rate suggests L1 cache is too small for the hot working set.".to_string(),
                action_item: "Increase 'max_capacity' in CacheConfig.".to_string(),
                impact_score: 0.6,
                timestamp: SystemTime::now(),
                estimated_improvement: Some(20.0),
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert("l1_hit_rate".to_string(), metrics.l1_hit_rate);
                    m.insert("l2_hit_rate".to_string(), metrics.l2_hit_rate);
                    m
                },
                config_suggestions: vec![
                    ConfigSuggestion {
                        config_key: "cache.l1.max_capacity".to_string(),
                        current_value: "1000".to_string(),
                        suggested_value: "2000".to_string(),
                        rationale: "Double L1 capacity to accommodate hot working set".to_string(),
                    }
                ],
            });
        }

        // High eviction rate
        if metrics.eviction_rate_per_minute > 50.0 {
            recs.push(Recommendation {
                id: "cache_eviction_high".to_string(),
                rec_type: RecommendationType::CacheTuning,
                priority: RecommendationPriority::High,
                title: "Cache Thrashing Detected".to_string(),
                description: "High eviction rate indicates the cache is frequently replacing items before they can be reused.".to_string(),
                action_item: "Increase cache capacity or review TTL/TTI settings.".to_string(),
                impact_score: 0.8,
                timestamp: SystemTime::now(),
                estimated_improvement: Some(30.0),
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert("eviction_rate_per_minute".to_string(), metrics.eviction_rate_per_minute);
                    m
                },
                config_suggestions: vec![
                    ConfigSuggestion {
                        config_key: "cache.max_capacity".to_string(),
                        current_value: "current".to_string(),
                        suggested_value: "increase by 50%".to_string(),
                        rationale: "Reduce eviction pressure by increasing capacity".to_string(),
                    }
                ],
            });
        }

        // Low overall hit rate
        if metrics.l1_hit_rate < 0.3 && metrics.l2_hit_rate < 0.5 {
            recs.push(Recommendation {
                id: "cache_low_hit_rate".to_string(),
                rec_type: RecommendationType::CacheTuning,
                priority: RecommendationPriority::High,
                title: "Low Cache Hit Rate".to_string(),
                description: "Both L1 and L2 cache hit rates are below optimal levels.".to_string(),
                action_item: "Review cache key strategy and access patterns.".to_string(),
                impact_score: 0.7,
                timestamp: SystemTime::now(),
                estimated_improvement: Some(40.0),
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert("l1_hit_rate".to_string(), metrics.l1_hit_rate);
                    m.insert("l2_hit_rate".to_string(), metrics.l2_hit_rate);
                    m
                },
                config_suggestions: vec![],
            });
        }
    }

    /// Analyze search metrics for query optimization recommendations
    fn analyze_search_performance(
        &self,
        metrics: &HashMap<String, serde_json::Value>,
        recs: &mut Vec<Recommendation>,
    ) {
        // Check for slow searches in histograms
        if let Some(search_duration) = metrics.get("histogram_search_duration_ms:{}") {
            if let Ok(hist) = serde_json::from_value::<
                crate::monitoring::metrics_collector::HistogramMetric,
            >(search_duration.clone())
            {
                let avg_duration = if hist.total_count > 0 {
                    hist.sum / hist.total_count as f64
                } else {
                    0.0
                };

                if avg_duration > 500.0 {
                    recs.push(Recommendation {
                        id: "search_slow_avg".to_string(),
                        rec_type: RecommendationType::QueryOptimization,
                        priority: RecommendationPriority::High,
                        title: "Slow Average Search Performance".to_string(),
                        description: format!(
                            "Average search duration ({:.1}ms) is above the 200ms target.",
                            avg_duration
                        ),
                        action_item:
                            "Analyze common query patterns and consider adding specialized indexes."
                                .to_string(),
                        impact_score: 0.9,
                        timestamp: SystemTime::now(),
                        estimated_improvement: Some(50.0),
                        related_metrics: {
                            let mut m = HashMap::new();
                            m.insert("avg_search_duration_ms".to_string(), avg_duration);
                            m
                        },
                        config_suggestions: vec![],
                    });
                }
            }
        }
    }

    /// Analyze system resources for allocation recommendations
    fn analyze_system_resources(
        &self,
        metrics: &HashMap<String, serde_json::Value>,
        recs: &mut Vec<Recommendation>,
    ) {
        if let Some(cpu_usage) = metrics.get("gauge_cpu_usage_percent:{}") {
            if let Some(val) = cpu_usage.get("value").and_then(|v| v.as_f64()) {
                if val > 85.0 {
                    recs.push(Recommendation {
                        id: "cpu_high_usage".to_string(),
                        rec_type: RecommendationType::ResourceAllocation,
                        priority: RecommendationPriority::High,
                        title: "High CPU Utilization".to_string(),
                        description: format!("System is running at {:.1}% CPU capacity.", val),
                        action_item:
                            "Scale up CPU resources or limit concurrent search operations."
                                .to_string(),
                        impact_score: 0.7,
                        timestamp: SystemTime::now(),
                        estimated_improvement: Some(20.0),
                        related_metrics: {
                            let mut m = HashMap::new();
                            m.insert("cpu_usage_percent".to_string(), val);
                            m
                        },
                        config_suggestions: vec![],
                    });
                }
            }
        }
    }

    /// Analyze query phase timing for optimization opportunities
    fn analyze_query_phases(&self, stats: &QueryTimingStats, recs: &mut Vec<Recommendation>) {
        if stats.query_count == 0 {
            return;
        }

        // Check if execution is the bottleneck
        let execution_ratio = stats.avg_execution_ms / stats.avg_total_ms.max(1.0);
        if execution_ratio > 0.8 && stats.avg_execution_ms > 100.0 {
            recs.push(Recommendation {
                id: "query_execution_bottleneck".to_string(),
                rec_type: RecommendationType::IndexTuning,
                priority: RecommendationPriority::High,
                title: "Query Execution Bottleneck".to_string(),
                description: format!(
                    "Query execution ({:.1}ms) accounts for {:.0}% of total query time.",
                    stats.avg_execution_ms,
                    execution_ratio * 100.0
                ),
                action_item: "Consider adding specialized indexes for frequently searched terms."
                    .to_string(),
                impact_score: 0.8,
                timestamp: SystemTime::now(),
                estimated_improvement: Some(40.0),
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert("avg_execution_ms".to_string(), stats.avg_execution_ms);
                    m.insert("execution_ratio".to_string(), execution_ratio);
                    m
                },
                config_suggestions: vec![],
            });
        }

        // Check P99 latency
        if stats.p99_total_ms > 1000.0 {
            recs.push(Recommendation {
                id: "query_p99_high".to_string(),
                rec_type: RecommendationType::QueryOptimization,
                priority: RecommendationPriority::High,
                title: "High P99 Query Latency".to_string(),
                description: format!(
                    "P99 query latency ({:.1}ms) exceeds 1 second.",
                    stats.p99_total_ms
                ),
                action_item: "Implement query timeouts and identify slow query patterns."
                    .to_string(),
                impact_score: 0.7,
                timestamp: SystemTime::now(),
                estimated_improvement: Some(30.0),
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert("p99_total_ms".to_string(), stats.p99_total_ms);
                    m
                },
                config_suggestions: vec![],
            });
        }
    }

    /// Analyze resource trends for capacity planning
    fn analyze_resource_trends(
        &self,
        current: &SystemResourceMetrics,
        recs: &mut Vec<Recommendation>,
    ) {
        if current.memory_usage_percent > 75.0 {
            recs.push(Recommendation {
                id: "memory_trend_warning".to_string(),
                rec_type: RecommendationType::CapacityPlanning,
                priority: if current.memory_usage_percent > 85.0 {
                    RecommendationPriority::High
                } else {
                    RecommendationPriority::Medium
                },
                title: "Memory Usage Approaching Limit".to_string(),
                description: format!(
                    "Current memory usage is {:.1}%.",
                    current.memory_usage_percent
                ),
                action_item: "Plan for memory capacity increase or optimize memory usage."
                    .to_string(),
                impact_score: 0.6,
                timestamp: SystemTime::now(),
                estimated_improvement: None,
                related_metrics: {
                    let mut m = HashMap::new();
                    m.insert(
                        "memory_usage_percent".to_string(),
                        current.memory_usage_percent,
                    );
                    m
                },
                config_suggestions: vec![],
            });
        }
    }

    /// Store performance snapshot for trend analysis
    fn store_performance_snapshot(
        &self,
        cache_metrics: &CacheMetricsSnapshot,
        system_metrics: &SystemResourceMetrics,
    ) {
        let snapshot = PerformanceSnapshot {
            timestamp: SystemTime::now(),
            avg_search_time_ms: cache_metrics.avg_access_time_ms,
            cache_hit_rate: cache_metrics.l1_hit_rate,
            cpu_usage: system_metrics.cpu_usage_percent,
            memory_usage: system_metrics.memory_usage_percent,
        };

        let mut history = self.performance_history.write();
        history.push(snapshot);

        if history.len() > self.max_history_entries {
            history.drain(0..100);
        }
    }

    /// Analyze capacity planning based on historical trends
    fn analyze_capacity_planning(&self, recs: &mut Vec<Recommendation>) {
        let history = self.performance_history.read();

        if history.len() < 20 {
            return;
        }

        let recent: Vec<f64> = history
            .iter()
            .rev()
            .take(10)
            .map(|s| s.memory_usage)
            .collect();
        let older: Vec<f64> = history
            .iter()
            .rev()
            .skip(10)
            .take(10)
            .map(|s| s.memory_usage)
            .collect();

        if !recent.is_empty() && !older.is_empty() {
            let recent_avg: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
            let older_avg: f64 = older.iter().sum::<f64>() / older.len() as f64;

            if recent_avg > older_avg * 1.1 && recent_avg > 60.0 {
                recs.push(Recommendation {
                    id: "capacity_memory_growth".to_string(),
                    rec_type: RecommendationType::CapacityPlanning,
                    priority: RecommendationPriority::Medium,
                    title: "Memory Usage Growth Trend".to_string(),
                    description: format!(
                        "Memory usage increased from {:.1}% to {:.1}%.",
                        older_avg, recent_avg
                    ),
                    action_item: "Monitor memory growth and plan for capacity increase."
                        .to_string(),
                    impact_score: 0.5,
                    timestamp: SystemTime::now(),
                    estimated_improvement: None,
                    related_metrics: {
                        let mut m = HashMap::new();
                        m.insert("recent_avg_memory".to_string(), recent_avg);
                        m.insert("older_avg_memory".to_string(), older_avg);
                        m
                    },
                    config_suggestions: vec![],
                });
            }
        }
    }

    /// Get performance trend for a specific metric
    pub fn get_performance_trend(&self, metric_name: &str) -> Option<PerformanceTrend> {
        let history = self.performance_history.read();

        if history.len() < 5 {
            return None;
        }

        let data_points: Vec<f64> = match metric_name {
            "search_time" => history.iter().map(|s| s.avg_search_time_ms).collect(),
            "cache_hit_rate" => history.iter().map(|s| s.cache_hit_rate).collect(),
            "cpu_usage" => history.iter().map(|s| s.cpu_usage).collect(),
            "memory_usage" => history.iter().map(|s| s.memory_usage).collect(),
            _ => return None,
        };

        if data_points.len() < 2 {
            return None;
        }

        let first_half_avg: f64 = data_points[..data_points.len() / 2].iter().sum::<f64>()
            / (data_points.len() / 2) as f64;
        let second_half_avg: f64 = data_points[data_points.len() / 2..].iter().sum::<f64>()
            / (data_points.len() - data_points.len() / 2) as f64;

        let change_percentage = if first_half_avg > 0.0 {
            ((second_half_avg - first_half_avg) / first_half_avg) * 100.0
        } else {
            0.0
        };

        let trend_direction = if change_percentage > 5.0 {
            if metric_name == "cache_hit_rate" {
                TrendDirection::Improving
            } else {
                TrendDirection::Degrading
            }
        } else if change_percentage < -5.0 {
            if metric_name == "cache_hit_rate" {
                TrendDirection::Degrading
            } else {
                TrendDirection::Improving
            }
        } else {
            TrendDirection::Stable
        };

        Some(PerformanceTrend {
            metric_name: metric_name.to_string(),
            trend_direction,
            change_percentage,
            time_window_hours: 1,
            data_points,
        })
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}
