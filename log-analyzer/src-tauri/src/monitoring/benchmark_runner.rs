//! Automated benchmark runner for continuous performance monitoring

use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{error, info, warn};

/// Benchmark result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean_time: Duration,
    pub std_deviation: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub throughput: Option<f64>,
    pub timestamp: SystemTime,
    pub git_commit: Option<String>,
}

/// Benchmark regression detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAlert {
    pub benchmark_name: String,
    pub current_time: Duration,
    pub baseline_time: Duration,
    pub regression_percentage: f64,
    pub timestamp: SystemTime,
}

/// Automated benchmark runner
pub struct BenchmarkRunner {
    baseline_results: parking_lot::RwLock<HashMap<String, BenchmarkResult>>,
    recent_results: parking_lot::RwLock<Vec<BenchmarkResult>>,
    regression_threshold: f64, // Percentage threshold for regression detection
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new() -> Self {
        Self {
            baseline_results: parking_lot::RwLock::new(HashMap::new()),
            recent_results: parking_lot::RwLock::new(Vec::new()),
            regression_threshold: 20.0, // 20% regression threshold
        }
    }

    /// Start automated benchmark monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting automated benchmark monitoring");

        // Load baseline results
        self.load_baselines().await?;

        // Start periodic benchmark runs
        let runner = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Run every hour
            loop {
                interval.tick().await;
                if let Err(e) = runner.run_benchmarks().await {
                    error!("Failed to run automated benchmarks: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Run all benchmarks and collect results
    pub async fn run_benchmarks(&self) -> Result<Vec<BenchmarkResult>> {
        info!("Running performance benchmarks");

        let mut results = Vec::new();

        // Run cache benchmarks
        if let Ok(cache_results) = self.run_benchmark_suite("cache_benchmarks").await {
            results.extend(cache_results);
        }

        // Run search benchmarks
        if let Ok(search_results) = self.run_benchmark_suite("search_benchmarks").await {
            results.extend(search_results);
        }

        // Run validation benchmarks
        if let Ok(validation_results) = self.run_benchmark_suite("validation_benchmarks").await {
            results.extend(validation_results);
        }

        // Store results
        {
            let mut recent = self.recent_results.write();
            recent.extend(results.clone());

            // Keep only last 100 results
            if recent.len() > 100 {
                let excess = recent.len() - 100;
                recent.drain(0..excess);
            }
        }

        // Check for regressions
        self.check_for_regressions(&results).await?;

        // Report to Sentry
        self.report_benchmark_results(&results).await?;

        info!("Completed benchmark run with {} results", results.len());
        Ok(results)
    }

    /// Run a specific benchmark suite
    async fn run_benchmark_suite(&self, suite_name: &str) -> Result<Vec<BenchmarkResult>> {
        info!("Running benchmark suite: {}", suite_name);

        let output = Command::new("cargo")
            .args(&[
                "bench",
                "--bench",
                suite_name,
                "--",
                "--output-format",
                "json",
            ])
            .current_dir("log-analyzer/src-tauri")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Benchmark suite {} failed: {}", suite_name, stderr);
            return Ok(Vec::new());
        }

        // Parse criterion JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_criterion_output(&stdout, suite_name)
    }

    /// Parse criterion benchmark output
    fn parse_criterion_output(
        &self,
        output: &str,
        suite_name: &str,
    ) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::new();

        // Parse criterion's JSON output format
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(reason) = json.get("reason").and_then(|r| r.as_str()) {
                    if reason == "benchmark-complete" {
                        if let Some(result) = self.parse_benchmark_result(&json, suite_name)? {
                            results.push(result);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Parse individual benchmark result from criterion JSON
    fn parse_benchmark_result(
        &self,
        json: &serde_json::Value,
        suite_name: &str,
    ) -> Result<Option<BenchmarkResult>> {
        let id = json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
        let mean = json
            .get("mean")
            .and_then(|m| m.get("estimate"))
            .and_then(|e| e.as_f64())
            .unwrap_or(0.0);

        let std_dev = json
            .get("std_dev")
            .and_then(|s| s.get("estimate"))
            .and_then(|e| e.as_f64())
            .unwrap_or(0.0);

        // Get throughput if available
        let throughput = json
            .get("throughput")
            .and_then(|t| t.get("per_iteration"))
            .and_then(|p| p.as_f64());

        let result = BenchmarkResult {
            name: format!("{}::{}", suite_name, id),
            mean_time: Duration::from_nanos(mean as u64),
            std_deviation: Duration::from_nanos(std_dev as u64),
            min_time: Duration::from_nanos((mean - std_dev) as u64),
            max_time: Duration::from_nanos((mean + std_dev) as u64),
            throughput,
            timestamp: SystemTime::now(),
            git_commit: self.get_git_commit(),
        };

        Ok(Some(result))
    }

    /// Get current git commit hash
    fn get_git_commit(&self) -> Option<String> {
        Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    /// Check for performance regressions
    async fn check_for_regressions(&self, results: &[BenchmarkResult]) -> Result<()> {
        let baselines = self.baseline_results.read();
        let mut regressions = Vec::new();

        for result in results {
            if let Some(baseline) = baselines.get(&result.name) {
                let current_ms = result.mean_time.as_millis() as f64;
                let baseline_ms = baseline.mean_time.as_millis() as f64;

                if baseline_ms > 0.0 {
                    let regression_percentage = ((current_ms - baseline_ms) / baseline_ms) * 100.0;

                    if regression_percentage > self.regression_threshold {
                        let alert = RegressionAlert {
                            benchmark_name: result.name.clone(),
                            current_time: result.mean_time,
                            baseline_time: baseline.mean_time,
                            regression_percentage,
                            timestamp: SystemTime::now(),
                        };

                        regressions.push(alert);

                        warn!(
                            benchmark = result.name,
                            current_ms = current_ms,
                            baseline_ms = baseline_ms,
                            regression_pct = regression_percentage,
                            "Performance regression detected"
                        );
                    }
                }
            }
        }

        // Send regression alerts
        if !regressions.is_empty() {
            self.send_regression_alerts(&regressions).await?;
        }

        Ok(())
    }

    /// Send regression alerts to monitoring systems
    async fn send_regression_alerts(&self, regressions: &[RegressionAlert]) -> Result<()> {
        for regression in regressions {
            // Send to Sentry
            sentry::with_scope(
                |scope| {
                    scope.set_tag("alert_type", "performance_regression");
                    scope.set_tag("benchmark", &regression.benchmark_name);
                    scope.set_extra(
                        "current_time_ms",
                        (regression.current_time.as_millis() as u64).into(),
                    );
                    scope.set_extra(
                        "baseline_time_ms",
                        (regression.baseline_time.as_millis() as u64).into(),
                    );
                    scope.set_extra(
                        "regression_percentage",
                        regression.regression_percentage.into(),
                    );
                },
                || {
                    sentry::capture_message(
                        &format!(
                            "Performance regression in {}: {:.1}% slower than baseline",
                            regression.benchmark_name, regression.regression_percentage
                        ),
                        sentry::Level::Warning,
                    );
                },
            );

            // Log structured alert
            error!(
                benchmark = regression.benchmark_name,
                current_ms = regression.current_time.as_millis(),
                baseline_ms = regression.baseline_time.as_millis(),
                regression_pct = regression.regression_percentage,
                "PERFORMANCE REGRESSION ALERT"
            );
        }

        Ok(())
    }

    /// Report benchmark results to monitoring systems
    async fn report_benchmark_results(&self, results: &[BenchmarkResult]) -> Result<()> {
        // Create performance summary for Sentry
        let summary = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "mean_time_ms": r.mean_time.as_millis(),
                    "throughput": r.throughput
                })
            })
            .collect::<Vec<_>>();

        sentry::add_breadcrumb(sentry::Breadcrumb {
            ty: "info".into(),
            category: Some("benchmark".into()),
            message: Some(format!(
                "Completed benchmark run with {} results",
                results.len()
            )),
            data: {
                let mut map = sentry::protocol::Map::new();
                map.insert("results_count".to_string(), results.len().into());
                map.insert("summary".to_string(), summary.into());
                map
            },
            ..Default::default()
        });

        Ok(())
    }

    /// Load baseline benchmark results
    async fn load_baselines(&self) -> Result<()> {
        // In a real implementation, this would load from persistent storage
        // For now, we'll run benchmarks once to establish baselines
        info!("Loading benchmark baselines");

        // Run initial benchmarks to establish baselines if none exist
        let baselines = self.baseline_results.read();
        if baselines.is_empty() {
            drop(baselines);

            info!("No baselines found, running initial benchmark to establish baselines");
            if let Ok(results) = self.run_benchmarks().await {
                let mut baselines = self.baseline_results.write();
                for result in results {
                    baselines.insert(result.name.clone(), result);
                }
                info!("Established {} benchmark baselines", baselines.len());
            }
        }

        Ok(())
    }

    /// Get recent benchmark results
    pub fn get_recent_results(&self, limit: usize) -> Vec<BenchmarkResult> {
        let results = self.recent_results.read();
        results.iter().rev().take(limit).cloned().collect()
    }

    /// Get baseline results
    pub fn get_baselines(&self) -> HashMap<String, BenchmarkResult> {
        self.baseline_results.read().clone()
    }
}

impl Clone for BenchmarkRunner {
    fn clone(&self) -> Self {
        Self {
            baseline_results: parking_lot::RwLock::new(self.baseline_results.read().clone()),
            recent_results: parking_lot::RwLock::new(self.recent_results.read().clone()),
            regression_threshold: self.regression_threshold,
        }
    }
}
