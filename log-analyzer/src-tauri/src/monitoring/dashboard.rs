//! Production monitoring dashboard and reporting

use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::info;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub refresh_interval: Duration,
    pub retention_period: Duration,
    pub alert_thresholds: HashMap<String, f64>,
    pub enabled_metrics: Vec<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert("cpu_usage".to_string(), 80.0);
        alert_thresholds.insert("memory_usage".to_string(), 85.0);
        alert_thresholds.insert("error_rate".to_string(), 5.0);
        alert_thresholds.insert("response_time_p95".to_string(), 2000.0); // 2 seconds

        Self {
            refresh_interval: Duration::from_secs(30),
            retention_period: Duration::from_secs(86400), // 24 hours
            alert_thresholds,
            enabled_metrics: vec![
                "cpu_usage".to_string(),
                "memory_usage".to_string(),
                "cache_hit_rate".to_string(),
                "search_operations_per_second".to_string(),
                "error_rate".to_string(),
                "response_time_p95".to_string(),
            ],
        }
    }
}

/// Dashboard metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: SystemTime,
    pub status: MetricStatus,
    pub trend: MetricTrend,
}

/// Metric status based on thresholds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Metric trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricTrend {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

/// Performance dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: SystemTime,
    pub system_health: SystemHealthSummary,
    pub performance_metrics: Vec<DashboardMetric>,
    pub recent_alerts: Vec<crate::monitoring::alerting::Alert>,
    pub benchmark_summary: BenchmarkSummary,
}

/// System health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthSummary {
    pub overall_status: MetricStatus,
    pub uptime: Duration,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub active_connections: u32,
    pub error_rate: f64,
}

/// Benchmark performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub last_run: SystemTime,
    pub total_benchmarks: u32,
    pub performance_score: f64, // 0-100 score based on baseline comparison
    pub regressions_detected: u32,
    pub top_performers: Vec<BenchmarkResult>,
    pub concerning_results: Vec<BenchmarkResult>,
}

/// Simplified benchmark result for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean_time_ms: f64,
    pub change_percentage: f64, // Compared to baseline
    pub status: MetricStatus,
}

/// Production monitoring dashboard
pub struct MonitoringDashboard {
    _config: DashboardConfig,
    _performance_tracker: crate::monitoring::performance_tracker::PerformanceTracker,
    metrics_collector: crate::monitoring::metrics_collector::MetricsCollector,
    alerting_system: crate::monitoring::alerting::AlertingSystem,
    benchmark_runner: crate::monitoring::benchmark_runner::BenchmarkRunner,
}

impl MonitoringDashboard {
    /// Create a new monitoring dashboard
    pub fn new(
        performance_tracker: crate::monitoring::performance_tracker::PerformanceTracker,
        metrics_collector: crate::monitoring::metrics_collector::MetricsCollector,
        alerting_system: crate::monitoring::alerting::AlertingSystem,
        benchmark_runner: crate::monitoring::benchmark_runner::BenchmarkRunner,
    ) -> Self {
        Self {
            _config: DashboardConfig::default(),
            _performance_tracker: performance_tracker,
            metrics_collector,
            alerting_system,
            benchmark_runner,
        }
    }

    /// Generate current dashboard data
    pub async fn generate_dashboard_data(&self) -> Result<DashboardData> {
        info!("Generating dashboard data");

        let system_health = self.collect_system_health().await?;
        let performance_metrics = self.collect_performance_metrics().await?;
        let recent_alerts = self.alerting_system.get_active_alerts();
        let benchmark_summary = self.generate_benchmark_summary().await?;

        Ok(DashboardData {
            timestamp: SystemTime::now(),
            system_health,
            performance_metrics,
            recent_alerts,
            benchmark_summary,
        })
    }

    /// Collect system health information
    async fn collect_system_health(&self) -> Result<SystemHealthSummary> {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage() as f64;
        let memory_usage = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

        // Calculate disk usage (simplified for compatibility)
        let disk_usage = 50.0; // Placeholder value

        // Determine overall status
        let overall_status = if cpu_usage > 90.0 || memory_usage > 90.0 || disk_usage > 95.0 {
            MetricStatus::Critical
        } else if cpu_usage > 80.0 || memory_usage > 80.0 || disk_usage > 85.0 {
            MetricStatus::Warning
        } else {
            MetricStatus::Healthy
        };

        Ok(SystemHealthSummary {
            overall_status,
            uptime: Duration::from_secs(sysinfo::System::uptime()),
            cpu_usage,
            memory_usage,
            disk_usage,
            active_connections: 0, // Would be implemented with actual connection tracking
            error_rate: 0.0,       // Would be calculated from metrics
        })
    }

    /// Collect performance metrics for dashboard
    async fn collect_performance_metrics(&self) -> Result<Vec<DashboardMetric>> {
        let mut metrics = Vec::new();
        let raw_metrics = self.metrics_collector.get_current_metrics();

        // Process each enabled metric
        for metric_name in &self._config.enabled_metrics {
            if let Some(threshold) = self._config.alert_thresholds.get(metric_name) {
                // Find matching raw metric
                let value = self.extract_metric_value(&raw_metrics, metric_name);
                let status = self.determine_metric_status(value, *threshold, metric_name);
                let trend = self.calculate_metric_trend(metric_name, value).await;

                let dashboard_metric = DashboardMetric {
                    name: metric_name.clone(),
                    value,
                    unit: self.get_metric_unit(metric_name),
                    timestamp: SystemTime::now(),
                    status,
                    trend,
                };

                metrics.push(dashboard_metric);
            }
        }

        Ok(metrics)
    }

    /// Extract metric value from raw metrics
    fn extract_metric_value(
        &self,
        raw_metrics: &HashMap<String, serde_json::Value>,
        metric_name: &str,
    ) -> f64 {
        // This would implement logic to extract specific metric values
        // from the raw metrics HashMap based on metric name
        match metric_name {
            "cpu_usage" => {
                // Extract from system metrics
                raw_metrics
                    .get("gauge_cpu_usage_percent")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            }
            "memory_usage" => raw_metrics
                .get("gauge_memory_usage_bytes")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            "cache_hit_rate" => {
                // Calculate hit rate from counters
                let hits = raw_metrics
                    .get("counter_cache_hits_total")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let misses = raw_metrics
                    .get("counter_cache_misses_total")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                if hits + misses > 0.0 {
                    (hits / (hits + misses)) * 100.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Determine metric status based on thresholds
    fn determine_metric_status(
        &self,
        value: f64,
        threshold: f64,
        metric_name: &str,
    ) -> MetricStatus {
        match metric_name {
            "cache_hit_rate" => {
                // Higher is better for cache hit rate
                if value >= threshold {
                    MetricStatus::Healthy
                } else if value >= threshold * 0.8 {
                    MetricStatus::Warning
                } else {
                    MetricStatus::Critical
                }
            }
            _ => {
                // Lower is better for most metrics
                if value <= threshold {
                    MetricStatus::Healthy
                } else if value <= threshold * 1.2 {
                    MetricStatus::Warning
                } else {
                    MetricStatus::Critical
                }
            }
        }
    }

    /// Calculate metric trend
    async fn calculate_metric_trend(&self, _metric_name: &str, _current_value: f64) -> MetricTrend {
        // In a real implementation, this would analyze historical data
        // to determine if the metric is trending up, down, or stable
        MetricTrend::Stable
    }

    /// Get metric unit
    fn get_metric_unit(&self, metric_name: &str) -> String {
        match metric_name {
            "cpu_usage" | "memory_usage" | "cache_hit_rate" | "error_rate" => "%".to_string(),
            "response_time_p95" => "ms".to_string(),
            "search_operations_per_second" => "ops/sec".to_string(),
            _ => "".to_string(),
        }
    }

    /// Generate benchmark summary
    async fn generate_benchmark_summary(&self) -> Result<BenchmarkSummary> {
        let recent_results = self.benchmark_runner.get_recent_results(50);
        let baselines = self.benchmark_runner.get_baselines();

        let mut performance_score: f64 = 100.0;
        let mut regressions_detected = 0;
        let mut top_performers = Vec::new();
        let mut concerning_results = Vec::new();

        for result in &recent_results {
            if let Some(baseline) = baselines.get(&result.name) {
                let current_ms = result.mean_time.as_millis() as f64;
                let baseline_ms = baseline.mean_time.as_millis() as f64;

                let change_percentage = if baseline_ms > 0.0 {
                    ((current_ms - baseline_ms) / baseline_ms) * 100.0
                } else {
                    0.0
                };

                let status = if change_percentage > 20.0 {
                    regressions_detected += 1;
                    performance_score -= 5.0; // Reduce score for regressions
                    MetricStatus::Critical
                } else if change_percentage > 10.0 {
                    performance_score -= 2.0;
                    MetricStatus::Warning
                } else {
                    MetricStatus::Healthy
                };

                let benchmark_result = BenchmarkResult {
                    name: result.name.clone(),
                    mean_time_ms: current_ms,
                    change_percentage,
                    status: status.clone(),
                };

                if status == MetricStatus::Critical || status == MetricStatus::Warning {
                    concerning_results.push(benchmark_result);
                } else if change_percentage < -5.0 {
                    // Improved performance
                    top_performers.push(benchmark_result);
                }
            }
        }

        // Sort results
        top_performers.sort_by(|a, b| {
            a.change_percentage
                .partial_cmp(&b.change_percentage)
                .unwrap()
        });
        concerning_results.sort_by(|a, b| {
            b.change_percentage
                .partial_cmp(&a.change_percentage)
                .unwrap()
        });

        // Limit results
        top_performers.truncate(5);
        concerning_results.truncate(10);

        let last_run = recent_results
            .first()
            .map(|r| r.timestamp)
            .unwrap_or_else(SystemTime::now);

        Ok(BenchmarkSummary {
            last_run,
            total_benchmarks: recent_results.len() as u32,
            performance_score: performance_score.max(0.0).min(100.0),
            regressions_detected,
            top_performers,
            concerning_results,
        })
    }

    /// Export dashboard data as JSON
    pub async fn export_json(&self) -> Result<String> {
        let data = self.generate_dashboard_data().await?;
        Ok(serde_json::to_string_pretty(&data)?)
    }

    /// Export dashboard data as HTML report
    pub async fn export_html(&self) -> Result<String> {
        let data = self.generate_dashboard_data().await?;

        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Log Analyzer - Production Monitoring Dashboard</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f5f5f5; padding: 20px; border-radius: 5px; }}
        .metric {{ margin: 10px 0; padding: 10px; border-left: 4px solid #ccc; }}
        .healthy {{ border-left-color: #4CAF50; }}
        .warning {{ border-left-color: #FF9800; }}
        .critical {{ border-left-color: #F44336; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; }}
        .card {{ background: white; border: 1px solid #ddd; border-radius: 5px; padding: 15px; }}
        .score {{ font-size: 2em; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Log Analyzer - Production Monitoring Dashboard</h1>
        <p>Generated: {}</p>
        <p>System Status: <strong>{:?}</strong></p>
    </div>
    
    <div class="grid">
        <div class="card">
            <h2>System Health</h2>
            <div class="metric">CPU Usage: {:.1}%</div>
            <div class="metric">Memory Usage: {:.1}%</div>
            <div class="metric">Disk Usage: {:.1}%</div>
            <div class="metric">Uptime: {} hours</div>
        </div>
        
        <div class="card">
            <h2>Performance Score</h2>
            <div class="score">{:.0}/100</div>
            <p>Benchmarks: {}</p>
            <p>Regressions: {}</p>
        </div>
        
        <div class="card">
            <h2>Active Alerts</h2>
            <p>{} active alerts</p>
        </div>
    </div>
</body>
</html>
        "#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            data.system_health.overall_status,
            data.system_health.cpu_usage,
            data.system_health.memory_usage,
            data.system_health.disk_usage,
            data.system_health.uptime.as_secs() / 3600,
            data.benchmark_summary.performance_score,
            data.benchmark_summary.total_benchmarks,
            data.benchmark_summary.regressions_detected,
            data.recent_alerts.len()
        );

        Ok(html)
    }
}
