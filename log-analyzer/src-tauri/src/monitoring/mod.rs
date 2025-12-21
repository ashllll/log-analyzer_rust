//! Production monitoring and performance tracking module
//!
//! This module provides comprehensive monitoring capabilities including:
//! - Performance metrics collection and reporting
//! - Benchmark result tracking and regression detection
//! - Sentry integration for production monitoring
//! - Custom metrics dashboard and alerting

pub mod alerting;
pub mod benchmark_runner;
pub mod dashboard;
pub mod metrics_collector;
pub mod performance_tracker;
pub mod property_tests;
pub mod recommendation_engine;

#[cfg(test)]
mod test;

use eyre::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

/// Central monitoring system for production performance tracking
pub struct ProductionMonitor {
    pub(crate) performance_tracker: performance_tracker::PerformanceTracker,
    pub(crate) metrics_collector: metrics_collector::MetricsCollector,
    pub(crate) alerting_system: alerting::AlertingSystem,
    pub(crate) recommendation_engine: recommendation_engine::RecommendationEngine,
}

impl ProductionMonitor {
    /// Initialize the production monitoring system
    pub fn new() -> Result<Self> {
        info!("Initializing production monitoring system");

        Ok(Self {
            performance_tracker: performance_tracker::PerformanceTracker::new()?,
            metrics_collector: metrics_collector::MetricsCollector::new()?,
            alerting_system: alerting::AlertingSystem::new()?,
            recommendation_engine: recommendation_engine::RecommendationEngine::new(),
        })
    }

    /// Start monitoring critical operations
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting production monitoring");

        // Start performance tracking
        self.performance_tracker.start().await?;

        // Initialize metrics collection
        self.metrics_collector.start_collection().await?;

        // Setup alerting
        self.alerting_system.initialize_alerts().await?;

        info!("Production monitoring system started successfully");
        Ok(())
    }

    /// Record a performance measurement
    pub fn record_performance(
        &self,
        operation: &str,
        duration: Duration,
        metadata: HashMap<String, String>,
    ) {
        self.performance_tracker
            .record_operation(operation, duration, metadata);

        // Check for performance regressions
        if let Some(baseline) = self.performance_tracker.get_baseline(operation) {
            let regression_threshold = baseline.as_millis() as f64 * 1.5; // 50% slower
            if duration.as_millis() as f64 > regression_threshold {
                warn!(
                    operation = operation,
                    duration_ms = duration.as_millis(),
                    baseline_ms = baseline.as_millis(),
                    "Performance regression detected"
                );

                // Send alert
                self.alerting_system
                    .send_performance_alert(operation, duration, baseline);
            }
        }
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        self.metrics_collector.get_current_metrics()
    }

    /// Get optimization recommendations
    pub fn get_recommendations(
        &self,
        cache_metrics: &crate::utils::cache_manager::CacheMetricsSnapshot,
    ) -> Vec<recommendation_engine::Recommendation> {
        let metrics = self.get_metrics();
        self.recommendation_engine
            .analyze_and_recommend(&metrics, cache_metrics)
    }

    /// Generate performance report
    pub async fn generate_report(&self) -> Result<String> {
        let metrics = self.get_metrics();
        let performance_data = self.performance_tracker.get_summary();

        let report = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metrics": metrics,
            "performance": performance_data,
            "system_info": self.collect_system_info()
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }

    /// Collect system information for monitoring
    fn collect_system_info(&self) -> serde_json::Value {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        serde_json::json!({
            "cpu_usage": sys.global_cpu_usage(),
            "memory_used": sys.used_memory(),
            "memory_total": sys.total_memory(),
            "disk_usage": serde_json::json!([])
        })
    }
}

/// Macro for easy performance measurement
#[macro_export]
macro_rules! measure_performance {
    ($monitor:expr, $operation:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        $monitor.record_performance($operation, duration, std::collections::HashMap::new());
        result
    }};

    ($monitor:expr, $operation:expr, $metadata:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        $monitor.record_performance($operation, duration, $metadata);
        result
    }};
}
