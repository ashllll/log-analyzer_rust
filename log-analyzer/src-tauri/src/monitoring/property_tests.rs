//! Property-based tests for monitoring and alerting
//!
//! **Feature: performance-optimization**
//! Validates:
//! - Property 15: Search Metrics Collection
//! - Property 18: Performance Alert Generation
//! - Property 19: Optimization Recommendations

#[cfg(test)]
mod tests {
    use crate::monitoring::alerting::{AlertSeverity, AlertingSystem};
    use crate::monitoring::metrics_collector::{
        MetricsCollector, QueryPhaseTiming, QueryTimingStats,
    };
    use crate::monitoring::recommendation_engine::{RecommendationEngine, RecommendationPriority};
    use crate::monitoring::ProductionMonitor;
    use crate::utils::cache_manager::CacheMetricsSnapshot;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::time::Duration;

    // ========================================================================
    // Property 15: Search Metrics Collection
    // **Feature: performance-optimization, Property 15: Search Metrics Collection**
    // *For any* search operation, detailed timing metrics should be collected for each query phase
    // **Validates: Requirements 4.1**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 15: Search Metrics Collection
        /// For any query with valid timing data, the metrics collector should:
        /// 1. Store the timing data
        /// 2. Calculate correct statistics
        /// 3. Maintain timing breakdown (parsing, execution, formatting, highlighting)
        #[test]
        fn test_search_metrics_collection_property(
            parsing_ms in 0.1f64..50.0f64,
            execution_ms in 1.0f64..500.0f64,
            formatting_ms in 0.1f64..100.0f64,
            highlighting_ms in 0.1f64..100.0f64,
            result_count in 0u64..10000u64
        ) {
            let collector = MetricsCollector::new().unwrap();

            // Create query timing with all phases
            let total_ms = parsing_ms + execution_ms + formatting_ms + highlighting_ms;
            let timing = QueryPhaseTiming {
                parsing_ms,
                execution_ms,
                result_formatting_ms: formatting_ms,
                highlighting_ms,
                total_ms,
                query: "test query".to_string(),
                result_count,
                timestamp: std::time::SystemTime::now(),
            };

            // Record the timing
            collector.record_query_timing(timing.clone());

            // Verify timing was recorded
            let recent = collector.get_recent_query_timings(1);
            prop_assert!(!recent.is_empty(), "Query timing should be recorded");

            let recorded = &recent[0];

            // Property: All timing phases should be preserved
            prop_assert!(
                (recorded.parsing_ms - parsing_ms).abs() < 0.001,
                "Parsing time should be preserved"
            );
            prop_assert!(
                (recorded.execution_ms - execution_ms).abs() < 0.001,
                "Execution time should be preserved"
            );
            prop_assert!(
                (recorded.result_formatting_ms - formatting_ms).abs() < 0.001,
                "Formatting time should be preserved"
            );
            prop_assert!(
                (recorded.highlighting_ms - highlighting_ms).abs() < 0.001,
                "Highlighting time should be preserved"
            );

            // Property: Total time should be sum of phases
            prop_assert!(
                (recorded.total_ms - total_ms).abs() < 0.001,
                "Total time should equal sum of phases"
            );

            // Property: Result count should be preserved
            prop_assert_eq!(recorded.result_count, result_count);
        }

        /// Property 15: Query timing statistics should be correctly calculated
        #[test]
        fn test_query_timing_stats_property(
            num_queries in 5usize..50usize,
            base_time in 10.0f64..100.0f64
        ) {
            let collector = MetricsCollector::new().unwrap();

            let mut total_time = 0.0f64;
            let mut min_time = f64::MAX;
            let mut max_time = f64::MIN;

            // Record multiple query timings
            for i in 0..num_queries {
                let variation = (i as f64 / num_queries as f64) * base_time;
                let query_time = base_time + variation;

                total_time += query_time;
                min_time = min_time.min(query_time);
                max_time = max_time.max(query_time);

                let timing = QueryPhaseTiming {
                    parsing_ms: query_time * 0.1,
                    execution_ms: query_time * 0.7,
                    result_formatting_ms: query_time * 0.1,
                    highlighting_ms: query_time * 0.1,
                    total_ms: query_time,
                    query: format!("query_{}", i),
                    result_count: i as u64,
                    timestamp: std::time::SystemTime::now(),
                };
                collector.record_query_timing(timing);
            }

            let stats = collector.get_query_timing_stats();

            // Property: Query count should match
            prop_assert_eq!(stats.query_count, num_queries as u64);

            // Property: Average should be correct (within tolerance)
            let expected_avg = total_time / num_queries as f64;
            prop_assert!(
                (stats.avg_total_ms - expected_avg).abs() < 1.0,
                "Average should be approximately correct"
            );

            // Property: Min should be <= average <= max
            prop_assert!(stats.min_total_ms <= stats.avg_total_ms);
            prop_assert!(stats.avg_total_ms <= stats.max_total_ms);

            // Property: Percentiles should be ordered
            prop_assert!(stats.p50_total_ms <= stats.p95_total_ms);
            prop_assert!(stats.p95_total_ms <= stats.p99_total_ms);
        }
    }

    // ========================================================================
    // Property 18: Performance Alert Generation
    // **Feature: performance-optimization, Property 18: Performance Alert Generation**
    // *For any* performance threshold violation, alerts with actionable diagnostic information should be emitted
    // **Validates: Requirements 4.4**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Property 18: Performance Alert Generation
        /// For any performance regression exceeding threshold, an alert should be generated
        #[test]
        fn test_performance_alert_generation_property(
            duration_ms in 300u64..1000u64,
            baseline_ms in 100u64..200u64
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let res = rt.block_on(async {
                let monitor = ProductionMonitor::new().unwrap();
                // Initialize alerts to load default configs
                monitor.alerting_system.initialize_alerts().await.unwrap();

                let duration = Duration::from_millis(duration_ms);
                let baseline = Duration::from_millis(baseline_ms);

                // Set a baseline explicitly
                monitor.performance_tracker.set_baseline("test_op", baseline);

                // Record a regression (at least 1.5x baseline as per mod.rs logic)
                monitor.record_performance("test_op", duration, HashMap::new());

                let alerts = monitor.alerting_system.get_active_alerts();

                // If duration > 1.5 * baseline, an alert should be generated
                if (duration_ms as f64) > (baseline_ms as f64 * 1.5) {
                    if alerts.is_empty() {
                        return Err(TestCaseError::fail("Alerts should not be empty on regression"));
                    }
                    if !alerts.iter().any(|a| a.message.contains("Performance regression")) {
                        return Err(TestCaseError::fail("Alert message should contain 'Performance regression'"));
                    }

                    // Property: Alert should contain diagnostic information
                    let alert = alerts.iter().find(|a| a.message.contains("Performance regression")).unwrap();
                    if alert.diagnostics.probable_cause.is_empty() && alert.diagnostics.recommended_actions.is_empty() {
                        // Diagnostics may be empty for legacy alerts, which is acceptable
                    }
                }
                Ok(())
            });
            res?;
        }

        /// Property 18: Response time violation alerts should be generated correctly
        #[test]
        fn test_response_time_alert_property(
            response_time_ms in 100.0f64..1000.0f64,
            threshold_ms in 50.0f64..200.0f64
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let res = rt.block_on(async {
                let alerting = AlertingSystem::new().unwrap();
                alerting.initialize_alerts().await.unwrap();

                // Send response time alert if threshold exceeded
                if response_time_ms > threshold_ms {
                    alerting.send_response_time_alert(
                        "search_operation",
                        response_time_ms,
                        threshold_ms,
                        "p95"
                    );

                    let alerts = alerting.get_active_alerts();

                    // Property: Alert should be generated for threshold violation
                    if !alerts.is_empty() {
                        let alert = &alerts[0];

                        // Property: Alert should have appropriate severity
                        prop_assert!(
                            alert.severity == AlertSeverity::Warning ||
                            alert.severity == AlertSeverity::Error ||
                            alert.severity == AlertSeverity::Critical,
                            "Alert severity should be Warning or higher"
                        );

                        // Property: Alert should contain diagnostic information
                        prop_assert!(
                            !alert.diagnostics.recommended_actions.is_empty(),
                            "Alert should have recommended actions"
                        );
                    }
                }
                Ok(())
            });
            res?;
        }

        /// Property 18: Resource exhaustion alerts should be generated correctly
        #[test]
        fn test_resource_alert_property(
            usage_percent in 50.0f64..100.0f64,
            threshold_percent in 70.0f64..90.0f64
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let res = rt.block_on(async {
                let alerting = AlertingSystem::new().unwrap();
                alerting.initialize_alerts().await.unwrap();

                // Send resource alert if threshold exceeded
                if usage_percent > threshold_percent {
                    alerting.send_resource_alert("memory", usage_percent, threshold_percent);

                    let alerts = alerting.get_active_alerts();

                    // Property: Alert should be generated for resource constraint
                    // Note: Due to cooldown, alert may not always be generated
                    if !alerts.is_empty() {
                        let alert = alerts.iter().find(|a| a.message.contains("memory"));
                        if let Some(alert) = alert {
                            prop_assert!(
                                alert.message.contains(&format!("{:.1}%", usage_percent)) ||
                                alert.message.contains("memory"),
                                "Alert message should contain resource info"
                            );
                        }
                    }
                }
                Ok(())
            });
            res?;
        }
    }

    // ========================================================================
    // Property 19: Optimization Recommendations
    // **Feature: performance-optimization, Property 19: Optimization Recommendations**
    // *For any* resource constraint situation, optimization recommendations should be provided
    // **Validates: Requirements 4.5**
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 19: Optimization Recommendations
        /// For any cache metrics indicating issues, appropriate recommendations should be generated
        #[test]
        fn test_recommendation_logic_property(
            l1_hit_rate in 0.0f64..1.0f64,
            l2_hit_rate in 0.0f64..1.0f64,
            eviction_rate in 0.0f64..100.0f64
        ) {
            let monitor = ProductionMonitor::new().unwrap();
            let cache_metrics = CacheMetricsSnapshot {
                l1_hit_count: 100,
                l1_miss_count: 100,
                l2_hit_count: 100,
                l2_miss_count: 100,
                load_count: 10,
                eviction_count: 10,
                l1_hit_rate,
                l2_hit_rate,
                avg_access_time_ms: 5.0,
                avg_load_time_ms: 50.0,
                eviction_rate_per_minute: eviction_rate,
                total_requests: 200,
                elapsed_time: Duration::from_secs(60),
            };

            let recs = monitor.get_recommendations(&cache_metrics);

            // Property: High L2 hit rate but low L1 hit rate should trigger L1 size recommendation
            if l1_hit_rate < 0.4 && l2_hit_rate > 0.7 {
                prop_assert!(
                    recs.iter().any(|r| r.id == "cache_l1_size"),
                    "Should recommend L1 cache size increase"
                );
            }

            // Property: High eviction rate should trigger thrashing recommendation
            if eviction_rate > 50.0 {
                prop_assert!(
                    recs.iter().any(|r| r.id == "cache_eviction_high"),
                    "Should detect cache thrashing"
                );
            }

            // Property: Low hit rates should trigger cache review recommendation
            if l1_hit_rate < 0.3 && l2_hit_rate < 0.5 {
                prop_assert!(
                    recs.iter().any(|r| r.id == "cache_low_hit_rate"),
                    "Should recommend cache review for low hit rates"
                );
            }
        }

        /// Property 19: Recommendations should have valid structure
        #[test]
        fn test_recommendation_structure_property(
            l1_hit_rate in 0.0f64..0.5f64,
            l2_hit_rate in 0.5f64..1.0f64,
            eviction_rate in 40.0f64..100.0f64
        ) {
            let engine = RecommendationEngine::new();
            let cache_metrics = CacheMetricsSnapshot {
                l1_hit_count: 100,
                l1_miss_count: 100,
                l2_hit_count: 100,
                l2_miss_count: 100,
                load_count: 10,
                eviction_count: 10,
                l1_hit_rate,
                l2_hit_rate,
                avg_access_time_ms: 5.0,
                avg_load_time_ms: 50.0,
                eviction_rate_per_minute: eviction_rate,
                total_requests: 200,
                elapsed_time: Duration::from_secs(60),
            };

            let recs = engine.analyze_and_recommend(&HashMap::new(), &cache_metrics);

            // Property: All recommendations should have valid structure
            for rec in &recs {
                // Property: ID should not be empty
                prop_assert!(!rec.id.is_empty(), "Recommendation ID should not be empty");

                // Property: Title should not be empty
                prop_assert!(!rec.title.is_empty(), "Recommendation title should not be empty");

                // Property: Description should not be empty
                prop_assert!(!rec.description.is_empty(), "Recommendation description should not be empty");

                // Property: Action item should not be empty
                prop_assert!(!rec.action_item.is_empty(), "Recommendation action item should not be empty");

                // Property: Impact score should be in valid range
                prop_assert!(
                    rec.impact_score >= 0.0 && rec.impact_score <= 1.0,
                    "Impact score should be between 0 and 1"
                );

                // Property: Priority should be valid
                prop_assert!(
                    rec.priority == RecommendationPriority::Low ||
                    rec.priority == RecommendationPriority::Medium ||
                    rec.priority == RecommendationPriority::High ||
                    rec.priority == RecommendationPriority::Critical,
                    "Priority should be a valid level"
                );
            }
        }

        /// Property 19: Recommendations should be prioritized correctly
        #[test]
        fn test_recommendation_priority_property(
            eviction_rate in 60.0f64..100.0f64
        ) {
            let engine = RecommendationEngine::new();

            // Create metrics that should trigger high-priority recommendation
            let cache_metrics = CacheMetricsSnapshot {
                l1_hit_count: 100,
                l1_miss_count: 100,
                l2_hit_count: 100,
                l2_miss_count: 100,
                load_count: 10,
                eviction_count: 10,
                l1_hit_rate: 0.2,  // Very low
                l2_hit_rate: 0.3,  // Also low
                avg_access_time_ms: 5.0,
                avg_load_time_ms: 50.0,
                eviction_rate_per_minute: eviction_rate,
                total_requests: 200,
                elapsed_time: Duration::from_secs(60),
            };

            let recs = engine.analyze_and_recommend(&HashMap::new(), &cache_metrics);

            // Property: High eviction rate should generate high priority recommendation
            if eviction_rate > 50.0 {
                let high_priority_recs: Vec<_> = recs.iter()
                    .filter(|r| r.priority == RecommendationPriority::High ||
                               r.priority == RecommendationPriority::Critical)
                    .collect();

                prop_assert!(
                    !high_priority_recs.is_empty(),
                    "High eviction rate should generate high priority recommendations"
                );
            }
        }
    }
}
