//! Performance tracking and baseline management

use eyre::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

/// Performance measurement data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMeasurement {
    pub operation: String,
    pub duration: Duration,
    pub timestamp: SystemTime,
    pub metadata: HashMap<String, String>,
}

/// Performance statistics for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operation: String,
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
    pub p95_duration: Duration,
    pub p99_duration: Duration,
    pub last_updated: SystemTime,
}

impl PerformanceStats {
    fn new(operation: String, first_measurement: Duration) -> Self {
        Self {
            operation,
            count: 1,
            total_duration: first_measurement,
            min_duration: first_measurement,
            max_duration: first_measurement,
            avg_duration: first_measurement,
            p95_duration: first_measurement,
            p99_duration: first_measurement,
            last_updated: SystemTime::now(),
        }
    }

    fn update(&mut self, duration: Duration, all_durations: &[Duration]) {
        self.count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.count as u32;
        self.last_updated = SystemTime::now();

        // Calculate percentiles
        let mut sorted_durations = all_durations.to_vec();
        sorted_durations.sort();

        if !sorted_durations.is_empty() {
            let p95_index = (sorted_durations.len() as f64 * 0.95) as usize;
            let p99_index = (sorted_durations.len() as f64 * 0.99) as usize;

            self.p95_duration = sorted_durations
                .get(p95_index.saturating_sub(1))
                .copied()
                .unwrap_or(duration);
            self.p99_duration = sorted_durations
                .get(p99_index.saturating_sub(1))
                .copied()
                .unwrap_or(duration);
        }
    }
}

/// Performance tracker for monitoring operation performance
pub struct PerformanceTracker {
    measurements: RwLock<Vec<PerformanceMeasurement>>,
    stats: RwLock<HashMap<String, PerformanceStats>>,
    baselines: RwLock<HashMap<String, Duration>>,
    max_measurements: usize,
}

impl PerformanceTracker {
    /// Create a new performance tracker
    pub fn new() -> Result<Self> {
        Ok(Self {
            measurements: RwLock::new(Vec::new()),
            stats: RwLock::new(HashMap::new()),
            baselines: RwLock::new(HashMap::new()),
            max_measurements: 10000, // Keep last 10k measurements
        })
    }

    /// Start the performance tracker
    pub async fn start(&self) -> Result<()> {
        info!("Starting performance tracker");

        // Load baselines from previous runs if available
        self.load_baselines().await?;

        // Start periodic cleanup task
        let tracker = self.clone_for_cleanup();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                tracker.cleanup_old_measurements().await;
            }
        });

        Ok(())
    }

    /// Record a performance measurement
    pub fn record_operation(
        &self,
        operation: &str,
        duration: Duration,
        metadata: HashMap<String, String>,
    ) {
        let measurement = PerformanceMeasurement {
            operation: operation.to_string(),
            duration,
            timestamp: SystemTime::now(),
            metadata,
        };

        debug!(
            operation = operation,
            duration_ms = duration.as_millis(),
            "Recording performance measurement"
        );

        // Add measurement
        {
            let mut measurements = self.measurements.write();
            measurements.push(measurement);

            // Limit memory usage
            if measurements.len() > self.max_measurements {
                measurements.drain(0..1000); // Remove oldest 1000 measurements
            }
        }

        // Update statistics
        self.update_stats(operation, duration);

        // Report to Sentry for production monitoring
        self.report_to_sentry(operation, duration);
    }

    /// Update statistics for an operation
    fn update_stats(&self, operation: &str, duration: Duration) {
        let mut stats = self.stats.write();

        // Get all durations for this operation for percentile calculation
        let measurements = self.measurements.read();
        let operation_durations: Vec<Duration> = measurements
            .iter()
            .filter(|m| m.operation == operation)
            .map(|m| m.duration)
            .collect();

        match stats.get_mut(operation) {
            Some(stat) => {
                stat.update(duration, &operation_durations);
            }
            None => {
                stats.insert(
                    operation.to_string(),
                    PerformanceStats::new(operation.to_string(), duration),
                );
            }
        }
    }

    /// Get baseline performance for an operation
    pub fn get_baseline(&self, operation: &str) -> Option<Duration> {
        self.baselines.read().get(operation).copied()
    }

    /// Set baseline performance for an operation
    pub fn set_baseline(&self, operation: &str, duration: Duration) {
        self.baselines
            .write()
            .insert(operation.to_string(), duration);
    }

    /// Get performance summary
    pub fn get_summary(&self) -> HashMap<String, PerformanceStats> {
        self.stats.read().clone()
    }

    /// Get recent measurements for an operation
    pub fn get_recent_measurements(
        &self,
        operation: &str,
        limit: usize,
    ) -> Vec<PerformanceMeasurement> {
        let measurements = self.measurements.read();
        measurements
            .iter()
            .filter(|m| m.operation == operation)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Report performance data to Sentry
    fn report_to_sentry(&self, operation: &str, duration: Duration) {
        // Only report if duration is significant or represents a regression
        if duration.as_millis() > 1000 {
            // > 1 second
            sentry::add_breadcrumb(sentry::Breadcrumb {
                ty: "performance".into(),
                category: Some("operation".into()),
                message: Some(format!(
                    "Operation '{}' took {}ms",
                    operation,
                    duration.as_millis()
                )),
                data: {
                    let mut map = sentry::protocol::Map::new();
                    map.insert("operation".to_string(), operation.into());
                    map.insert(
                        "duration_ms".to_string(),
                        (duration.as_millis() as u64).into(),
                    );
                    map
                },
                ..Default::default()
            });
        }

        // Create performance transaction for Sentry APM
        let transaction = sentry::start_transaction(sentry::TransactionContext::new(
            operation,
            "performance.operation",
        ));
        transaction.set_data(
            "duration_ms",
            sentry::protocol::Value::from(duration.as_millis() as u64),
        );
        transaction.finish();
    }

    /// Load baselines from storage
    async fn load_baselines(&self) -> Result<()> {
        // In a real implementation, this would load from a persistent store
        // For now, we'll set some reasonable defaults based on operation types
        let mut baselines = self.baselines.write();

        // Default baselines (in milliseconds)
        baselines.insert("search_operation".to_string(), Duration::from_millis(500));
        baselines.insert("cache_operation".to_string(), Duration::from_millis(10));
        baselines.insert("validation_operation".to_string(), Duration::from_millis(5));
        baselines.insert("file_operation".to_string(), Duration::from_millis(100));
        baselines.insert(
            "workspace_operation".to_string(),
            Duration::from_millis(200),
        );

        info!("Loaded performance baselines");
        Ok(())
    }

    /// Cleanup old measurements to prevent memory leaks
    async fn cleanup_old_measurements(&self) {
        let cutoff = SystemTime::now() - Duration::from_secs(3600); // Keep last hour

        let mut measurements = self.measurements.write();
        let original_len = measurements.len();
        measurements.retain(|m| m.timestamp > cutoff);

        let removed = original_len - measurements.len();
        if removed > 0 {
            debug!("Cleaned up {} old performance measurements", removed);
        }
    }

    /// Clone for use in async cleanup task
    fn clone_for_cleanup(&self) -> Self {
        Self {
            measurements: RwLock::new(Vec::new()), // Empty for cleanup task
            stats: RwLock::new(HashMap::new()),
            baselines: RwLock::new(HashMap::new()),
            max_measurements: self.max_measurements,
        }
    }
}

impl Clone for PerformanceTracker {
    fn clone(&self) -> Self {
        Self {
            measurements: RwLock::new(self.measurements.read().clone()),
            stats: RwLock::new(self.stats.read().clone()),
            baselines: RwLock::new(self.baselines.read().clone()),
            max_measurements: self.max_measurements,
        }
    }
}
