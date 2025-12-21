//! Query Optimization Engine
//!
//! Analyzes query patterns and provides optimization suggestions:
//! - Query rewriting suggestions for slow queries
//! - Specialized index recommendations based on query frequency
//! - Query complexity analysis and automatic simplification
//! - Performance pattern detection

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Query performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub query: String,
    pub execution_count: u64,
    pub total_time_ms: u64,
    pub average_time_ms: f64,
    pub last_executed: u64, // Unix timestamp
    pub result_count_avg: f64,
    pub complexity_score: f32,
}

/// Index recommendation based on query patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub recommendation_type: IndexRecommendationType,
    pub field_name: String,
    pub reason: String,
    pub estimated_improvement: f32, // Percentage improvement estimate
    pub priority: RecommendationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexRecommendationType {
    SpecializedTermIndex,
    TimePartitionedIndex,
    CompositeIndex,
    PrefixIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimized query with suggestions
#[derive(Debug, Clone)]
pub struct OptimizedQuery {
    pub original_query: String,
    pub optimized_query: String,
    pub suggestions: Vec<QuerySuggestion>,
    pub complexity_reduction: f32,
    pub estimated_speedup: f32,
}

/// Query optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub original_part: String,
    pub suggested_part: String,
    pub estimated_improvement: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    TermReordering,
    QuerySimplification,
    OperatorOptimization,
    WildcardOptimization,
    RegexOptimization,
    TimeRangeOptimization,
}

/// Query complexity analysis
#[derive(Debug, Clone)]
pub struct ComplexityAnalysis {
    pub score: f32,
    pub factors: Vec<ComplexityFactor>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ComplexityFactor {
    pub factor_type: String,
    pub impact: f32,
    pub description: String,
}

/// Query optimization engine
pub struct QueryOptimizer {
    query_stats: Arc<RwLock<HashMap<String, QueryStats>>>,
    index_stats: Arc<RwLock<IndexStatistics>>,
    optimization_rules: Vec<OptimizationRule>,
}

#[derive(Debug, Default)]
struct IndexStatistics {
    _term_frequencies: HashMap<String, u64>,
    _field_usage: HashMap<String, u64>,
    _query_patterns: HashMap<String, u64>,
}

type OptimizationRule = Box<dyn Fn(&str) -> Vec<QuerySuggestion> + Send + Sync>;

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        let mut optimizer = Self {
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            index_stats: Arc::new(RwLock::new(IndexStatistics::default())),
            optimization_rules: Vec::new(),
        };

        optimizer.initialize_rules();
        optimizer
    }

    /// Initialize optimization rules
    fn initialize_rules(&mut self) {
        // Rule 1: Term reordering based on selectivity
        self.optimization_rules.push(Box::new(|query: &str| {
            let mut suggestions = Vec::new();

            // Simple heuristic: shorter terms are usually more selective
            let terms: Vec<&str> = query.split_whitespace().collect();
            if terms.len() > 2 {
                let mut sorted_terms = terms.clone();
                sorted_terms.sort_by_key(|t| t.len());

                if sorted_terms != terms {
                    suggestions.push(QuerySuggestion {
                        suggestion_type: SuggestionType::TermReordering,
                        description: "Reorder terms by selectivity (shorter terms first)"
                            .to_string(),
                        original_part: terms.join(" "),
                        suggested_part: sorted_terms.join(" "),
                        estimated_improvement: 15.0,
                    });
                }
            }

            suggestions
        }));

        // Rule 2: Wildcard optimization
        self.optimization_rules.push(Box::new(|query: &str| {
            let mut suggestions = Vec::new();

            if query.contains('*') && query.len() > 10 {
                suggestions.push(QuerySuggestion {
                    suggestion_type: SuggestionType::WildcardOptimization,
                    description: "Consider using prefix queries instead of wildcards".to_string(),
                    original_part: query.to_string(),
                    suggested_part: query.replace('*', ""),
                    estimated_improvement: 25.0,
                });
            }

            suggestions
        }));

        // Rule 3: Regex optimization
        self.optimization_rules.push(Box::new(|query: &str| {
            let mut suggestions = Vec::new();

            if query.contains(".*") {
                suggestions.push(QuerySuggestion {
                    suggestion_type: SuggestionType::RegexOptimization,
                    description: "Avoid .* patterns in regex queries for better performance"
                        .to_string(),
                    original_part: query.to_string(),
                    suggested_part: "Consider using more specific patterns".to_string(),
                    estimated_improvement: 40.0,
                });
            }

            suggestions
        }));
    }

    /// Optimize a query and provide suggestions
    pub fn optimize_query(&self, query: &str) -> OptimizedQuery {
        debug!(query = %query, "Optimizing query");

        let complexity = self.analyze_complexity(query);
        let mut suggestions = Vec::new();

        // Apply all optimization rules
        for rule in &self.optimization_rules {
            suggestions.extend(rule(query));
        }

        // Generate optimized query based on suggestions
        let optimized_query = self.apply_optimizations(query, &suggestions);

        // Calculate estimated improvements
        let complexity_reduction =
            (complexity.score - self.analyze_complexity(&optimized_query).score).max(0.0);
        let estimated_speedup = suggestions
            .iter()
            .map(|s| s.estimated_improvement)
            .fold(0.0, |acc, x| acc + x * 0.01) // Convert percentage to multiplier
            .min(0.8); // Cap at 80% improvement

        OptimizedQuery {
            original_query: query.to_string(),
            optimized_query,
            suggestions,
            complexity_reduction,
            estimated_speedup,
        }
    }

    /// Apply optimization suggestions to generate optimized query
    fn apply_optimizations(&self, query: &str, suggestions: &[QuerySuggestion]) -> String {
        let mut optimized = query.to_string();

        // Apply suggestions in order of estimated improvement
        let mut sorted_suggestions = suggestions.to_vec();
        sorted_suggestions.sort_by(|a, b| {
            b.estimated_improvement
                .partial_cmp(&a.estimated_improvement)
                .unwrap()
        });

        for suggestion in sorted_suggestions.iter().take(3) {
            // Apply top 3 suggestions
            match suggestion.suggestion_type {
                SuggestionType::TermReordering => {
                    optimized = suggestion.suggested_part.clone();
                }
                SuggestionType::WildcardOptimization => {
                    optimized =
                        optimized.replace(&suggestion.original_part, &suggestion.suggested_part);
                }
                SuggestionType::RegexOptimization => {
                    // For regex, we provide guidance rather than automatic replacement
                }
                _ => {}
            }
        }

        optimized
    }

    /// Analyze query complexity
    pub fn analyze_complexity(&self, query: &str) -> ComplexityAnalysis {
        let mut score = 0.0;
        let mut factors = Vec::new();
        let mut recommendations = Vec::new();

        // Factor 1: Query length
        let length_factor = (query.len() as f32 / 100.0).min(2.0);
        score += length_factor;
        factors.push(ComplexityFactor {
            factor_type: "Length".to_string(),
            impact: length_factor,
            description: format!("Query length: {} characters", query.len()),
        });

        // Factor 2: Number of terms
        let term_count = query.split_whitespace().count() as f32;
        let term_factor = (term_count / 10.0).min(2.0);
        score += term_factor;
        factors.push(ComplexityFactor {
            factor_type: "Terms".to_string(),
            impact: term_factor,
            description: format!("Number of terms: {}", term_count as usize),
        });

        // Factor 3: Wildcards and regex
        let wildcard_count = query.matches('*').count() + query.matches('?').count();
        if wildcard_count > 0 {
            let wildcard_factor = wildcard_count as f32 * 0.5;
            score += wildcard_factor;
            factors.push(ComplexityFactor {
                factor_type: "Wildcards".to_string(),
                impact: wildcard_factor,
                description: format!("Wildcard patterns: {}", wildcard_count),
            });
            recommendations.push("Consider using prefix queries instead of wildcards".to_string());
        }

        // Factor 4: Boolean operators
        let bool_ops = query.matches(" AND ").count()
            + query.matches(" OR ").count()
            + query.matches(" NOT ").count();
        if bool_ops > 0 {
            let bool_factor = bool_ops as f32 * 0.3;
            score += bool_factor;
            factors.push(ComplexityFactor {
                factor_type: "Boolean".to_string(),
                impact: bool_factor,
                description: format!("Boolean operators: {}", bool_ops),
            });
        }

        // Generate recommendations based on complexity
        if score > 3.0 {
            recommendations
                .push("Consider simplifying the query for better performance".to_string());
        }
        if term_count > 5.0 {
            recommendations.push("Try using fewer, more specific terms".to_string());
        }

        ComplexityAnalysis {
            score,
            factors,
            recommendations,
        }
    }

    /// Record query execution statistics
    pub fn record_query_execution(
        &self,
        query: &str,
        execution_time: Duration,
        result_count: usize,
    ) {
        let mut stats = self.query_stats.write();

        let query_stats = stats
            .entry(query.to_string())
            .or_insert_with(|| QueryStats {
                query: query.to_string(),
                execution_count: 0,
                total_time_ms: 0,
                average_time_ms: 0.0,
                last_executed: 0,
                result_count_avg: 0.0,
                complexity_score: self.analyze_complexity(query).score,
            });

        query_stats.execution_count += 1;
        query_stats.total_time_ms += execution_time.as_millis() as u64;
        query_stats.average_time_ms =
            query_stats.total_time_ms as f64 / query_stats.execution_count as f64;
        query_stats.last_executed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update rolling average for result count
        let alpha = 0.1; // Smoothing factor
        query_stats.result_count_avg =
            alpha * result_count as f64 + (1.0 - alpha) * query_stats.result_count_avg;

        debug!(
            query = %query,
            execution_time_ms = execution_time.as_millis(),
            result_count = result_count,
            avg_time_ms = query_stats.average_time_ms,
            "Recorded query execution"
        );
    }

    /// Get recommendations for specialized indexes
    pub fn get_index_recommendations(&self) -> Vec<IndexRecommendation> {
        let stats = self.query_stats.read();
        let mut recommendations = Vec::new();

        // Analyze frequently executed slow queries
        for query_stats in stats.values() {
            if query_stats.execution_count >= 10 && query_stats.average_time_ms > 100.0 {
                // Recommend specialized index for frequently used terms
                let terms: Vec<&str> = query_stats.query.split_whitespace().collect();
                for term in terms {
                    if term.len() > 3 && !term.contains('*') {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: IndexRecommendationType::SpecializedTermIndex,
                            field_name: format!("content_{}", term),
                            reason: format!(
                                "Term '{}' appears in {} slow queries",
                                term, query_stats.execution_count
                            ),
                            estimated_improvement: 30.0,
                            priority: if query_stats.average_time_ms > 500.0 {
                                RecommendationPriority::High
                            } else {
                                RecommendationPriority::Medium
                            },
                        });
                    }
                }
            }
        }

        // Sort by priority and estimated improvement
        recommendations.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then(
                b.estimated_improvement
                    .partial_cmp(&a.estimated_improvement)
                    .unwrap(),
            )
        });

        // Limit to top 10 recommendations
        recommendations.truncate(10);

        info!(
            recommendation_count = recommendations.len(),
            "Generated index recommendations"
        );
        recommendations
    }

    /// Check if a specialized index should be created for a query pattern
    pub fn should_create_specialized_index(&self, query_pattern: &str) -> bool {
        let stats = self.query_stats.read();

        // Look for similar queries
        let similar_queries: Vec<_> = stats
            .values()
            .filter(|s| s.query.contains(query_pattern) || query_pattern.contains(&s.query))
            .collect();

        if similar_queries.is_empty() {
            return false;
        }

        let total_executions: u64 = similar_queries.iter().map(|s| s.execution_count).sum();
        let avg_time: f64 = similar_queries
            .iter()
            .map(|s| s.average_time_ms)
            .sum::<f64>()
            / similar_queries.len() as f64;

        // Create specialized index if:
        // 1. Pattern appears in many queries (>= 5)
        // 2. Total executions are high (>= 50)
        // 3. Average time is slow (>= 200ms)
        total_executions >= 50 && avg_time >= 200.0 && similar_queries.len() >= 5
    }

    /// Get query statistics
    pub fn get_query_stats(&self) -> HashMap<String, QueryStats> {
        self.query_stats.read().clone()
    }

    /// Clear statistics (for testing or reset)
    pub fn clear_stats(&self) {
        self.query_stats.write().clear();
        *self.index_stats.write() = IndexStatistics::default();
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_optimizer_creation() {
        let optimizer = QueryOptimizer::new();
        assert!(!optimizer.optimization_rules.is_empty());
    }

    #[test]
    fn test_complexity_analysis() {
        let optimizer = QueryOptimizer::new();

        // Simple query
        let simple_analysis = optimizer.analyze_complexity("error");
        assert!(simple_analysis.score < 1.0);

        // Complex query
        let complex_analysis = optimizer
            .analyze_complexity("error AND warning OR debug WITH wildcards * AND regex .*");
        assert!(complex_analysis.score > simple_analysis.score);
    }

    #[test]
    fn test_query_optimization() {
        let optimizer = QueryOptimizer::new();

        let optimized = optimizer.optimize_query("very long term short a");
        assert!(!optimized.suggestions.is_empty());
        assert_ne!(optimized.original_query, optimized.optimized_query);
    }

    #[test]
    fn test_query_stats_recording() {
        let optimizer = QueryOptimizer::new();

        optimizer.record_query_execution("test query", Duration::from_millis(150), 100);
        optimizer.record_query_execution("test query", Duration::from_millis(200), 150);

        let stats = optimizer.get_query_stats();
        let query_stats = stats.get("test query").unwrap();

        assert_eq!(query_stats.execution_count, 2);
        assert_eq!(query_stats.total_time_ms, 350);
        assert_eq!(query_stats.average_time_ms, 175.0);
    }

    #[test]
    fn test_index_recommendations() {
        let optimizer = QueryOptimizer::new();

        // Record multiple slow executions of similar queries
        for _ in 0..15 {
            optimizer.record_query_execution(
                "error database connection",
                Duration::from_millis(300),
                50,
            );
        }

        let recommendations = optimizer.get_index_recommendations();
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_specialized_index_decision() {
        let optimizer = QueryOptimizer::new();

        // Record many executions of queries containing "database"
        for i in 0..60 {
            optimizer.record_query_execution(
                &format!("database error {}", i % 10),
                Duration::from_millis(250),
                30,
            );
        }

        assert!(optimizer.should_create_specialized_index("database"));
        assert!(!optimizer.should_create_specialized_index("nonexistent"));
    }
}
