//! Property-based tests for auto-tuning and dynamic optimization
//!
//! Validates:
//! - Property 25: Automatic Index Optimization
//! - Property 27: Automatic Cache Tuning
//! - Property 28: Query Rewrite Suggestions
//! - Property 29: Dynamic Resource Allocation
//!
//! **Feature: performance-optimization**
//! **Validates: Requirements 7.1, 7.3, 7.4, 7.5**

#[cfg(test)]
mod tests {
    use crate::search_engine::index_optimizer::{IndexOptimizer, IndexOptimizerConfig};
    use crate::search_engine::query_optimizer::QueryOptimizer;
    use crate::utils::cache_tuner::{
        CacheTuner, CacheTunerConfig, TuningActionType, TuningMetrics,
    };
    use crate::utils::dynamic_optimizer::{DynamicOptimizerConfig, ResourceManager};
    use proptest::prelude::*;
    use std::time::Duration;

    // ========================================================================
    // Property 25: Automatic Index Optimization
    // **Feature: performance-optimization, Property 25: Automatic Index Optimization**
    // **Validates: Requirements 7.1**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 25.1: Hot queries are identified when count >= threshold
        /// *For any* query pattern with execution count >= threshold, the optimizer
        /// SHALL identify it as a hot query requiring optimization.
        #[test]
        fn prop_hot_query_identification_threshold(
            query_count in 10u64..500u64,
            threshold in 50u64..200u64
        ) {
            // **Feature: performance-optimization, Property 25: Automatic Index Optimization**
            // **Validates: Requirements 7.1**
            let optimizer = IndexOptimizer::new(threshold);
            let query = "test_query_pattern";

            for _ in 0..query_count {
                optimizer.record_query(query, Duration::from_millis(150));
            }

            let hot_queries = optimizer.identify_hot_queries();

            // Property: Query should be identified as hot only if count >= threshold
            if query_count >= threshold {
                prop_assert!(!hot_queries.is_empty(),
                    "Query with {} executions should be hot (threshold: {})",
                    query_count, threshold);
                prop_assert_eq!(&hot_queries[0].0, query);
            } else {
                prop_assert!(hot_queries.is_empty(),
                    "Query with {} executions should NOT be hot (threshold: {})",
                    query_count, threshold);
            }
        }

        /// Property 25.2: Slow hot queries generate optimization suggestions
        /// *For any* hot query with average duration > slow threshold, the optimizer
        /// SHALL generate optimization suggestions.
        #[test]
        fn prop_slow_query_suggestions(
            avg_duration_ms in 50u64..1000u64,
            threshold in 10u64..50u64
        ) {
            // **Feature: performance-optimization, Property 25: Automatic Index Optimization**
            // **Validates: Requirements 7.1**
            let config = IndexOptimizerConfig {
                optimization_threshold: threshold,
                slow_query_threshold_ms: 200,
                ..Default::default()
            };
            let optimizer = IndexOptimizer::with_config(config);
            let query = "slow_query_pattern";

            for _ in 0..threshold {
                optimizer.record_query(query, Duration::from_millis(avg_duration_ms));
            }

            let suggestions = optimizer.suggest_optimizations();

            // Property: Slow hot queries (>200ms) should trigger suggestions
            if avg_duration_ms > 200 {
                prop_assert!(!suggestions.is_empty(),
                    "Slow query ({} ms avg) should generate suggestions", avg_duration_ms);
                prop_assert!(suggestions[0].contains(query),
                    "Suggestion should reference the slow query");
            } else {
                prop_assert!(suggestions.is_empty(),
                    "Fast query ({} ms avg) should NOT generate suggestions", avg_duration_ms);
            }
        }

        /// Property 25.3: Index recommendations are prioritized by query frequency and slowness
        /// *For any* set of hot queries, recommendations SHALL be sorted by priority
        /// (Critical > High > Medium > Low).
        #[test]
        fn prop_recommendation_priority_ordering(
            query_counts in prop::collection::vec(100u64..1000u64, 3..10),
            durations in prop::collection::vec(100u64..800u64, 3..10)
        ) {
            // **Feature: performance-optimization, Property 25: Automatic Index Optimization**
            // **Validates: Requirements 7.1**
            let optimizer = IndexOptimizer::new(50);

            // Record multiple query patterns with varying frequencies and durations
            for (i, (&count, &duration)) in query_counts.iter().zip(durations.iter()).enumerate() {
                let query = format!("query_pattern_{}", i);
                for _ in 0..count.min(200) { // Cap to avoid test timeout
                    optimizer.record_query(&query, Duration::from_millis(duration));
                }
            }

            let recommendations = optimizer.generate_index_recommendations();

            // Property: Recommendations should be sorted by priority (descending)
            for window in recommendations.windows(2) {
                prop_assert!(window[0].priority >= window[1].priority,
                    "Recommendations should be sorted by priority");
            }
        }
    }

    // ========================================================================
    // Property 27: Automatic Cache Tuning
    // **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
    // **Validates: Requirements 7.3**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 27.1: Low hit rate triggers cache size increase
        /// *For any* cache with hit rate below minimum threshold, the tuner
        /// SHALL recommend increasing cache size.
        #[test]
        fn prop_low_hit_rate_triggers_size_increase(
            hit_rate in 0.1f64..0.5f64,
            min_threshold in 0.55f64..0.7f64
        ) {
            // **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
            // **Validates: Requirements 7.3**
            let config = CacheTunerConfig {
                min_acceptable_hit_rate: min_threshold,
                adjustment_cooldown: Duration::from_secs(0), // Disable cooldown for test
                ..Default::default()
            };
            let tuner = CacheTuner::new(config);

            let metrics = TuningMetrics {
                hit_rate,
                eviction_rate: 5.0,
                avg_access_time_ms: 1.0,
                cache_size: 1000,
                memory_usage_bytes: 1024 * 1024,
                hot_keys_count: 10,
            };

            let action = tuner.analyze_and_tune(&metrics);

            // Property: Low hit rate should trigger size increase
            if hit_rate < min_threshold {
                match action.action_type {
                    TuningActionType::IncreaseCacheSize { from, to } => {
                        prop_assert!(to > from,
                            "Cache size should increase when hit rate ({:.2}) < threshold ({:.2})",
                            hit_rate, min_threshold);
                    }
                    _ => prop_assert!(false,
                        "Expected IncreaseCacheSize action for low hit rate ({:.2})",
                        hit_rate),
                }
            }
        }

        /// Property 27.2: High eviction rate triggers cache size increase
        /// *For any* cache with eviction rate above maximum threshold, the tuner
        /// SHALL recommend increasing cache size.
        #[test]
        fn prop_high_eviction_triggers_size_increase(
            eviction_rate in 1.0f64..30.0f64,
            max_threshold in 5.0f64..15.0f64
        ) {
            // **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
            // **Validates: Requirements 7.3**
            let config = CacheTunerConfig {
                max_eviction_rate: max_threshold,
                min_acceptable_hit_rate: 0.3, // Set low to not trigger hit rate action
                adjustment_cooldown: Duration::from_secs(0),
                ..Default::default()
            };
            let tuner = CacheTuner::new(config);

            let metrics = TuningMetrics {
                hit_rate: 0.8, // Good hit rate
                eviction_rate,
                avg_access_time_ms: 1.0,
                cache_size: 1000,
                memory_usage_bytes: 1024 * 1024,
                hot_keys_count: 10,
            };

            let action = tuner.analyze_and_tune(&metrics);

            // Property: High eviction rate should trigger size increase
            if eviction_rate > max_threshold {
                match action.action_type {
                    TuningActionType::IncreaseCacheSize { from, to } => {
                        prop_assert!(to > from,
                            "Cache size should increase when eviction rate ({:.2}) > threshold ({:.2})",
                            eviction_rate, max_threshold);
                    }
                    _ => prop_assert!(false,
                        "Expected IncreaseCacheSize action for high eviction rate ({:.2})",
                        eviction_rate),
                }
            }
        }

        /// Property 27.3: Cache size adjustments respect bounds
        /// *For any* tuning action, the resulting cache size SHALL remain within
        /// configured min and max bounds.
        #[test]
        fn prop_cache_size_respects_bounds(
            current_size in 50u64..5000u64,  // Keep current_size within reasonable range
            min_size in 10u64..100u64,
            max_size in 5000u64..20000u64,
            increase in prop::bool::ANY
        ) {
            // **Feature: performance-optimization, Property 27: Automatic Cache Tuning**
            // **Validates: Requirements 7.3**
            let config = CacheTunerConfig {
                min_cache_size: min_size,
                max_cache_size: max_size,
                size_adjustment_step: 10.0,
                ..Default::default()
            };
            let tuner = CacheTuner::new(config.clone());

            let new_size = tuner.calculate_new_size(current_size, increase);

            // Property: New size must be within bounds
            prop_assert!(new_size >= config.min_cache_size,
                "New size {} should be >= min {}", new_size, config.min_cache_size);
            prop_assert!(new_size <= config.max_cache_size,
                "New size {} should be <= max {}", new_size, config.max_cache_size);
        }
    }

    // ========================================================================
    // Property 28: Query Rewrite Suggestions
    // **Feature: performance-optimization, Property 28: Query Rewrite Suggestions**
    // **Validates: Requirements 7.4**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 28.1: Complex queries receive optimization suggestions
        /// *For any* query with multiple terms of varying lengths, the optimizer SHALL provide
        /// term reordering suggestions when terms are not already optimally ordered.
        #[test]
        fn prop_multi_term_queries_get_suggestions(
            term_count in 4usize..8usize  // Need at least 4 terms for reordering to be meaningful
        ) {
            // **Feature: performance-optimization, Property 28: Query Rewrite Suggestions**
            // **Validates: Requirements 7.4**
            let optimizer = QueryOptimizer::new();

            // Generate a query with multiple terms of varying lengths (longest first to trigger reordering)
            let terms: Vec<String> = (0..term_count)
                .rev()  // Reverse to put longest first (suboptimal order)
                .map(|i| format!("term{}", "x".repeat(i + 1)))
                .collect();
            let query = terms.join(" ");

            let optimized = optimizer.optimize_query(&query);

            // Property: Multi-term queries with suboptimal ordering should receive suggestions
            // The optimizer suggests reordering when terms are not sorted by length
            prop_assert!(!optimized.suggestions.is_empty() || optimized.optimized_query != query,
                "Query with {} terms in suboptimal order should receive optimization suggestions or be rewritten",
                term_count);
        }

        /// Property 28.2: Wildcard queries receive optimization suggestions
        /// *For any* query containing wildcards, the optimizer SHALL suggest
        /// wildcard optimization.
        #[test]
        fn prop_wildcard_queries_get_suggestions(
            prefix in "[a-z]{5,15}",
            has_wildcard in prop::bool::ANY
        ) {
            // **Feature: performance-optimization, Property 28: Query Rewrite Suggestions**
            // **Validates: Requirements 7.4**
            let optimizer = QueryOptimizer::new();

            let query = if has_wildcard {
                format!("{}*", prefix)
            } else {
                prefix.clone()
            };

            let optimized = optimizer.optimize_query(&query);

            // Property: Wildcard queries should receive wildcard-specific suggestions
            if has_wildcard && query.len() > 10 {
                let has_wildcard_suggestion = optimized.suggestions.iter()
                    .any(|s| matches!(s.suggestion_type,
                        crate::search_engine::query_optimizer::SuggestionType::WildcardOptimization));
                prop_assert!(has_wildcard_suggestion,
                    "Wildcard query should receive wildcard optimization suggestion");
            }
        }

        /// Property 28.3: Query complexity analysis is consistent
        /// *For any* query, complexity score SHALL be non-negative and
        /// increase with query complexity factors.
        #[test]
        fn prop_complexity_analysis_consistency(
            base_term in "[a-z]{3,10}",
            additional_terms in 0usize..5usize,
            wildcards in 0usize..3usize
        ) {
            // **Feature: performance-optimization, Property 28: Query Rewrite Suggestions**
            // **Validates: Requirements 7.4**
            let optimizer = QueryOptimizer::new();

            // Build simple query
            let simple_query = base_term.clone();
            let simple_analysis = optimizer.analyze_complexity(&simple_query);

            // Build complex query
            let mut complex_parts = vec![base_term];
            for i in 0..additional_terms {
                complex_parts.push(format!("term{}", i));
            }
            for _ in 0..wildcards {
                complex_parts.push("*".to_string());
            }
            let complex_query = complex_parts.join(" ");
            let complex_analysis = optimizer.analyze_complexity(&complex_query);

            // Property: Complexity score should be non-negative
            prop_assert!(simple_analysis.score >= 0.0,
                "Simple query complexity should be non-negative");
            prop_assert!(complex_analysis.score >= 0.0,
                "Complex query complexity should be non-negative");

            // Property: More complex queries should have higher scores
            if additional_terms > 0 || wildcards > 0 {
                prop_assert!(complex_analysis.score >= simple_analysis.score,
                    "Complex query ({}) should have >= complexity than simple query ({})",
                    complex_analysis.score, simple_analysis.score);
            }
        }

        /// Property 28.4: Slow queries trigger index recommendations
        /// *For any* frequently executed slow query, the optimizer SHALL
        /// generate index recommendations.
        #[test]
        fn prop_slow_queries_get_index_recommendations(
            execution_count in 10u64..100u64,
            avg_time_ms in 50u64..500u64
        ) {
            // **Feature: performance-optimization, Property 28: Query Rewrite Suggestions**
            // **Validates: Requirements 7.4**
            let optimizer = QueryOptimizer::new();
            let query = "database connection error";

            for _ in 0..execution_count {
                optimizer.record_query_execution(
                    query,
                    Duration::from_millis(avg_time_ms),
                    50,
                );
            }

            let recommendations = optimizer.get_index_recommendations();

            // Property: Frequently executed slow queries should get recommendations
            if execution_count >= 10 && avg_time_ms > 100 {
                prop_assert!(!recommendations.is_empty(),
                    "Slow query ({} executions, {} ms avg) should get index recommendations",
                    execution_count, avg_time_ms);
            }
        }
    }

    // ========================================================================
    // Property 29: Dynamic Resource Allocation
    // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
    // **Validates: Requirements 7.5**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 29.1: Worker count stays within configured bounds
        /// *For any* system load condition, the calculated optimal worker count
        /// SHALL remain within min_workers and max_workers bounds.
        #[test]
        fn prop_worker_count_within_bounds(
            cpu_usage in 0.0f64..100.0f64,
            pending_ops in 0u64..100u64,
            min_workers in 1usize..4usize,
            max_workers in 4usize..16usize
        ) {
            // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
            // **Validates: Requirements 7.5**
            let config = DynamicOptimizerConfig {
                min_workers,
                max_workers,
                cpu_high_threshold: 80.0,
                cpu_low_threshold: 30.0,
                ..Default::default()
            };
            let manager = ResourceManager::new(config.clone());

            let optimal = manager.calculate_optimal_workers(cpu_usage, pending_ops);

            // Property: Worker count must be within bounds
            prop_assert!(optimal >= config.min_workers,
                "Optimal workers {} should be >= min {}", optimal, config.min_workers);
            prop_assert!(optimal <= config.max_workers,
                "Optimal workers {} should be <= max {}", optimal, config.max_workers);
        }

        /// Property 29.2: High CPU triggers worker reduction
        /// *For any* CPU usage above high threshold, the optimizer SHALL
        /// recommend reducing worker count (unless already at minimum).
        #[test]
        fn prop_high_cpu_reduces_workers(
            cpu_usage in 85.0f64..100.0f64,
            current_workers in 2usize..8usize
        ) {
            // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
            // **Validates: Requirements 7.5**
            let config = DynamicOptimizerConfig {
                min_workers: 1,
                max_workers: 8,
                cpu_high_threshold: 80.0,
                ..Default::default()
            };
            let manager = ResourceManager::new(config);

            // Set current workers using the test setter
            manager.set_current_workers(current_workers);

            let optimal = manager.calculate_optimal_workers(cpu_usage, 0);

            // Property: High CPU should reduce workers (unless at minimum)
            if current_workers > 1 {
                prop_assert!(optimal < current_workers,
                    "High CPU ({:.1}%) should reduce workers from {} to {}",
                    cpu_usage, current_workers, optimal);
            }
        }

        /// Property 29.3: Low CPU with pending work triggers worker increase
        /// *For any* CPU usage below low threshold with pending operations,
        /// the optimizer SHALL recommend increasing worker count.
        #[test]
        fn prop_low_cpu_with_work_increases_workers(
            cpu_usage in 5.0f64..25.0f64,
            pending_ops in 1u64..50u64,
            current_workers in 2usize..6usize
        ) {
            // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
            // **Validates: Requirements 7.5**
            let config = DynamicOptimizerConfig {
                min_workers: 1,
                max_workers: 8,
                cpu_low_threshold: 30.0,
                ..Default::default()
            };
            let manager = ResourceManager::new(config);

            // Set current workers using the test setter
            manager.set_current_workers(current_workers);

            let optimal = manager.calculate_optimal_workers(cpu_usage, pending_ops);

            // Property: Low CPU with pending work should increase workers
            prop_assert!(optimal > current_workers,
                "Low CPU ({:.1}%) with {} pending ops should increase workers from {} to {}",
                cpu_usage, pending_ops, current_workers, optimal);
        }

        /// Property 29.4: Operation tracking is accurate
        /// *For any* sequence of operation starts and completions, the active
        /// operation count SHALL accurately reflect the difference.
        #[test]
        fn prop_operation_tracking_accuracy(
            starts in 0usize..100usize,
            completions_ratio in 0.0f64..1.0f64
        ) {
            // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
            // **Validates: Requirements 7.5**
            let config = DynamicOptimizerConfig::default();
            let manager = ResourceManager::new(config);

            // Start operations
            for _ in 0..starts {
                manager.operation_started();
            }

            // Complete some operations
            let completions = (starts as f64 * completions_ratio) as usize;
            for _ in 0..completions {
                manager.operation_completed();
            }

            let active = manager.get_active_operations() as usize;
            let expected = starts.saturating_sub(completions);

            // Property: Active count should equal starts - completions
            prop_assert_eq!(active, expected,
                "Active operations should be {} (started: {}, completed: {})",
                expected, starts, completions);
        }

        /// Property 29.5: Load trend calculation is bounded
        /// *For any* sequence of load recordings, the calculated trend
        /// SHALL be a finite number (not NaN or infinite).
        #[test]
        fn prop_load_trend_is_finite(
            cpu_values in prop::collection::vec(0.0f64..100.0f64, 10..30)
        ) {
            // **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
            // **Validates: Requirements 7.5**
            let config = DynamicOptimizerConfig {
                load_history_size: 50,
                ..Default::default()
            };
            let manager = ResourceManager::new(config);

            for cpu in cpu_values {
                manager.record_load(cpu, 50.0, 0.8);
            }

            let trend = manager.calculate_load_trend();

            // Property: Trend should be a finite number
            prop_assert!(trend.is_finite(),
                "Load trend should be finite, got: {}", trend);
        }
    }
}
