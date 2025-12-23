//! Metrics collection and aggregation for production monitoring
//!
//! **Feature: performance-optimization, Property 15: Search Metrics Collection**
//! This module implements comprehensive metrics collection for search operations,
//! including detailed query phase timing (parsing, execution, result formatting)
//! and system resource monitoring (CPU, memory, disk I/O).

pub use crate::utils::cache_manager::CacheMetricsSnapshot;
use eyre::Result;
use parking_lot::Mutex;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::time::interval;
use tracing::{debug, info};

/// Metric types supported by the collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
}

/// Individual metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub timestamp: SystemTime,
    pub tags: HashMap<String, String>,
}

/// Histogram bucket for latency measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Histogram metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramMetric {
    pub name: String,
    pub buckets: Vec<HistogramBucket>,
    pub total_count: u64,
    pub sum: f64,
    pub timestamp: SystemTime,
}

/// Query phase timing for detailed search metrics
/// **Validates: Requirements 4.1** - Detailed timing metrics for each query phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPhaseTiming {
    /// Time spent parsing the query string
    pub parsing_ms: f64,
    /// Time spent executing the search against the index
    pub execution_ms: f64,
    /// Time spent formatting and preparing results
    pub result_formatting_ms: f64,
    /// Time spent on highlighting matches
    pub highlighting_ms: f64,
    /// Total query time
    pub total_ms: f64,
    /// Query string for reference
    pub query: String,
    /// Number of results returned
    pub result_count: u64,
    /// Timestamp of the query
    pub timestamp: SystemTime,
}

impl Default for QueryPhaseTiming {
    fn default() -> Self {
        Self {
            parsing_ms: 0.0,
            execution_ms: 0.0,
            result_formatting_ms: 0.0,
            highlighting_ms: 0.0,
            total_ms: 0.0,
            query: String::new(),
            result_count: 0,
            timestamp: SystemTime::now(),
        }
    }
}

impl QueryPhaseTiming {
    pub fn new(query: String) -> Self {
        Self {
            query,
            timestamp: SystemTime::now(),
            ..Default::default()
        }
    }
}

/// Builder for tracking query phase timings
pub struct QueryPhaseTimer {
    query: String,
    start_time: Instant,
    parsing_start: Option<Instant>,
    parsing_end: Option<Instant>,
    execution_start: Option<Instant>,
    execution_end: Option<Instant>,
    formatting_start: Option<Instant>,
    formatting_end: Option<Instant>,
    highlighting_start: Option<Instant>,
    highlighting_end: Option<Instant>,
    result_count: u64,
}

impl QueryPhaseTimer {
    pub fn new(query: String) -> Self {
        Self {
            query,
            start_time: Instant::now(),
            parsing_start: None,
            parsing_end: None,
            execution_start: None,
            execution_end: None,
            formatting_start: None,
            formatting_end: None,
            highlighting_start: None,
            highlighting_end: None,
            result_count: 0,
        }
    }

    pub fn start_parsing(&mut self) {
        self.parsing_start = Some(Instant::now());
    }

    pub fn end_parsing(&mut self) {
        self.parsing_end = Some(Instant::now());
    }

    pub fn start_execution(&mut self) {
        self.execution_start = Some(Instant::now());
    }

    pub fn end_execution(&mut self) {
        self.execution_end = Some(Instant::now());
    }

    pub fn start_formatting(&mut self) {
        self.formatting_start = Some(Instant::now());
    }

    pub fn end_formatting(&mut self) {
        self.formatting_end = Some(Instant::now());
    }

    pub fn start_highlighting(&mut self) {
        self.highlighting_start = Some(Instant::now());
    }

    pub fn end_highlighting(&mut self) {
        self.highlighting_end = Some(Instant::now());
    }

    pub fn set_result_count(&mut self, count: u64) {
        self.result_count = count;
    }

    pub fn finish(self) -> QueryPhaseTiming {
        let parsing_ms = match (self.parsing_start, self.parsing_end) {
            (Some(start), Some(end)) => end.duration_since(start).as_secs_f64() * 1000.0,
            _ => 0.0,
        };

        let execution_ms = match (self.execution_start, self.execution_end) {
            (Some(start), Some(end)) => end.duration_since(start).as_secs_f64() * 1000.0,
            _ => 0.0,
        };

        let formatting_ms = match (self.formatting_start, self.formatting_end) {
            (Some(start), Some(end)) => end.duration_since(start).as_secs_f64() * 1000.0,
            _ => 0.0,
        };

        let highlighting_ms = match (self.highlighting_start, self.highlighting_end) {
            (Some(start), Some(end)) => end.duration_since(start).as_secs_f64() * 1000.0,
            _ => 0.0,
        };

        let total_ms = self.start_time.elapsed().as_secs_f64() * 1000.0;

        QueryPhaseTiming {
            parsing_ms,
            execution_ms,
            result_formatting_ms: formatting_ms,
            highlighting_ms,
            total_ms,
            query: self.query,
            result_count: self.result_count,
            timestamp: SystemTime::now(),
        }
    }
}

/// System resource metrics snapshot
/// **Validates: Requirements 4.1** - System resource monitoring (CPU, memory, disk I/O)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: f64,
    /// Memory used in bytes
    pub memory_used_bytes: u64,
    /// Total memory in bytes
    pub memory_total_bytes: u64,
    /// Memory usage percentage (0-100)
    pub memory_usage_percent: f64,
    /// Disk read bytes since last measurement
    pub disk_read_bytes: u64,
    /// Disk write bytes since last measurement
    pub disk_write_bytes: u64,
    /// Number of active processes
    pub process_count: usize,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Timestamp of measurement
    pub timestamp: SystemTime,
}

impl Default for SystemResourceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            memory_usage_percent: 0.0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            process_count: 0,
            uptime_seconds: 0,
            timestamp: SystemTime::now(),
        }
    }
}

/// Aggregated query timing statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryTimingStats {
    /// Total number of queries tracked
    pub query_count: u64,
    /// Average parsing time in milliseconds
    pub avg_parsing_ms: f64,
    /// Average execution time in milliseconds
    pub avg_execution_ms: f64,
    /// Average formatting time in milliseconds
    pub avg_formatting_ms: f64,
    /// Average highlighting time in milliseconds
    pub avg_highlighting_ms: f64,
    /// Average total query time in milliseconds
    pub avg_total_ms: f64,
    /// 50th percentile (median) total time
    pub p50_total_ms: f64,
    /// 95th percentile total time
    pub p95_total_ms: f64,
    /// 99th percentile total time
    pub p99_total_ms: f64,
    /// Minimum total time
    pub min_total_ms: f64,
    /// Maximum total time
    pub max_total_ms: f64,
}

/// Resource constraint status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceConstraintStatus {
    /// Resources are within normal limits
    Normal,
    /// Resources are approaching limits (>80%)
    Warning,
    /// Resources are critically constrained (>90%)
    Critical,
    /// Unable to determine resource status
    Unknown,
}

/// Counter metric (monotonically increasing)
pub struct Counter {
    value: AtomicU64,
    name: String,
    tags: HashMap<String, String>,
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            value: AtomicU64::new(self.value.load(Ordering::Relaxed)),
            name: self.name.clone(),
            tags: self.tags.clone(),
        }
    }
}

impl Counter {
    pub fn new(name: String, tags: HashMap<String, String>) -> Self {
        Self {
            value: AtomicU64::new(0),
            name,
            tags,
        }
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn to_metric_point(&self) -> MetricPoint {
        MetricPoint {
            name: self.name.clone(),
            value: self.get() as f64,
            timestamp: SystemTime::now(),
            tags: self.tags.clone(),
        }
    }
}

/// Gauge metric (can go up or down)
#[derive(Clone)]
pub struct Gauge {
    value: Arc<Mutex<f64>>,
    name: String,
    tags: HashMap<String, String>,
}

impl Gauge {
    pub fn new(name: String, tags: HashMap<String, String>) -> Self {
        Self {
            value: Arc::new(Mutex::new(0.0)),
            name,
            tags,
        }
    }

    pub fn set(&self, value: f64) {
        *self.value.lock() = value;
    }

    pub fn add(&self, value: f64) {
        *self.value.lock() += value;
    }

    pub fn get(&self) -> f64 {
        *self.value.lock()
    }

    pub fn to_metric_point(&self) -> MetricPoint {
        MetricPoint {
            name: self.name.clone(),
            value: self.get(),
            timestamp: SystemTime::now(),
            tags: self.tags.clone(),
        }
    }
}

/// Histogram for latency and size distributions
pub struct Histogram {
    name: String,
    buckets: Vec<(f64, AtomicU64)>, // (upper_bound, count)
    total_count: AtomicU64,
    sum: Arc<Mutex<f64>>,
    tags: HashMap<String, String>,
}

impl Clone for Histogram {
    fn clone(&self) -> Self {
        let cloned_buckets = self
            .buckets
            .iter()
            .map(|(bound, counter)| (*bound, AtomicU64::new(counter.load(Ordering::Relaxed))))
            .collect();

        Self {
            name: self.name.clone(),
            buckets: cloned_buckets,
            total_count: AtomicU64::new(self.total_count.load(Ordering::Relaxed)),
            sum: Arc::new(Mutex::new(*self.sum.lock())),
            tags: self.tags.clone(),
        }
    }
}

impl Histogram {
    pub fn new(name: String, buckets: Vec<f64>, tags: HashMap<String, String>) -> Self {
        let bucket_counters = buckets
            .into_iter()
            .map(|bound| (bound, AtomicU64::new(0)))
            .collect();

        Self {
            name,
            buckets: bucket_counters,
            total_count: AtomicU64::new(0),
            sum: Arc::new(Mutex::new(0.0)),
            tags,
        }
    }

    pub fn observe(&self, value: f64) {
        self.total_count.fetch_add(1, Ordering::Relaxed);
        *self.sum.lock() += value;

        // Find appropriate bucket
        for (upper_bound, counter) in &self.buckets {
            if value <= *upper_bound {
                counter.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    pub fn to_histogram_metric(&self) -> HistogramMetric {
        let buckets = self
            .buckets
            .iter()
            .map(|(bound, counter)| HistogramBucket {
                upper_bound: *bound,
                count: counter.load(Ordering::Relaxed),
            })
            .collect();

        HistogramMetric {
            name: self.name.clone(),
            buckets,
            total_count: self.total_count.load(Ordering::Relaxed),
            sum: *self.sum.lock(),
            timestamp: SystemTime::now(),
        }
    }
}

/// Central metrics collector
/// **Feature: performance-optimization, Property 15: Search Metrics Collection**
pub struct MetricsCollector {
    counters: RwLock<HashMap<String, Counter>>,
    gauges: RwLock<HashMap<String, Gauge>>,
    histograms: RwLock<HashMap<String, Histogram>>,
    /// Query phase timings for detailed search metrics
    query_timings: RwLock<Vec<QueryPhaseTiming>>,
    /// System resource metrics history
    system_metrics: RwLock<Vec<SystemResourceMetrics>>,
    /// Maximum number of query timings to retain
    max_query_timings: usize,
    /// Maximum number of system metrics snapshots to retain
    max_system_metrics: usize,
    collection_interval: Duration,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Result<Self> {
        Ok(Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            query_timings: RwLock::new(Vec::new()),
            system_metrics: RwLock::new(Vec::new()),
            max_query_timings: 1000,
            max_system_metrics: 100,
            collection_interval: Duration::from_secs(60), // Collect every minute
        })
    }

    /// Start metrics collection
    pub async fn start_collection(&self) -> Result<()> {
        info!("Starting metrics collection");

        // Initialize standard metrics
        self.initialize_standard_metrics().await?;

        // Start periodic collection
        let collector = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = interval(collector.collection_interval);
            loop {
                interval.tick().await;
                collector.collect_system_metrics().await;
                collector.report_metrics_to_sentry().await;
            }
        });

        Ok(())
    }

    /// Initialize standard application metrics
    async fn initialize_standard_metrics(&self) -> Result<()> {
        // Performance counters
        self.create_counter("search_operations_total", HashMap::new());
        self.create_counter("cache_hits_total", HashMap::new());
        self.create_counter("cache_misses_total", HashMap::new());
        self.create_counter("validation_errors_total", HashMap::new());
        self.create_counter("workspace_operations_total", HashMap::new());

        // Performance gauges
        self.create_gauge("active_searches", HashMap::new());
        self.create_gauge("cache_size_bytes", HashMap::new());
        self.create_gauge("memory_usage_bytes", HashMap::new());
        self.create_gauge("cpu_usage_percent", HashMap::new());

        // Latency histograms (buckets in milliseconds)
        let latency_buckets = vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
        ];
        self.create_histogram(
            "search_duration_ms",
            latency_buckets.clone(),
            HashMap::new(),
        );
        self.create_histogram(
            "cache_operation_duration_ms",
            latency_buckets.clone(),
            HashMap::new(),
        );
        self.create_histogram("validation_duration_ms", latency_buckets, HashMap::new());

        info!("Initialized standard metrics");
        Ok(())
    }

    /// Create a new counter metric
    pub fn create_counter(&self, name: &str, tags: HashMap<String, String>) -> String {
        let key = format!("{}:{:?}", name, tags);
        let counter = Counter::new(name.to_string(), tags);
        self.counters.write().insert(key.clone(), counter);
        key
    }

    /// Create a new gauge metric
    pub fn create_gauge(&self, name: &str, tags: HashMap<String, String>) -> String {
        let key = format!("{}:{:?}", name, tags);
        let gauge = Gauge::new(name.to_string(), tags);
        self.gauges.write().insert(key.clone(), gauge);
        key
    }

    /// Create a new histogram metric
    pub fn create_histogram(
        &self,
        name: &str,
        buckets: Vec<f64>,
        tags: HashMap<String, String>,
    ) -> String {
        let key = format!("{}:{:?}", name, tags);
        let histogram = Histogram::new(name.to_string(), buckets, tags);
        self.histograms.write().insert(key.clone(), histogram);
        key
    }

    /// Increment a counter
    pub fn increment_counter(&self, key: &str) {
        if let Some(counter) = self.counters.read().get(key) {
            counter.increment();
        }
    }

    /// Add to a counter
    pub fn add_to_counter(&self, key: &str, value: u64) {
        if let Some(counter) = self.counters.read().get(key) {
            counter.add(value);
        }
    }

    /// Set a gauge value
    pub fn set_gauge(&self, key: &str, value: f64) {
        if let Some(gauge) = self.gauges.read().get(key) {
            gauge.set(value);
        }
    }

    /// Add to a gauge
    pub fn add_to_gauge(&self, key: &str, value: f64) {
        if let Some(gauge) = self.gauges.read().get(key) {
            gauge.add(value);
        }
    }

    /// Record a histogram observation
    pub fn observe_histogram(&self, key: &str, value: f64) {
        if let Some(histogram) = self.histograms.read().get(key) {
            histogram.observe(value);
        }
    }

    /// Get current metrics as JSON
    pub fn get_current_metrics(&self) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();

        // Collect counters
        let counters = self.counters.read();
        for (key, counter) in counters.iter() {
            metrics.insert(
                format!("counter_{}", key),
                serde_json::to_value(counter.to_metric_point()).unwrap_or_default(),
            );
        }

        // Collect gauges
        let gauges = self.gauges.read();
        for (key, gauge) in gauges.iter() {
            metrics.insert(
                format!("gauge_{}", key),
                serde_json::to_value(gauge.to_metric_point()).unwrap_or_default(),
            );
        }

        // Collect histograms
        let histograms = self.histograms.read();
        for (key, histogram) in histograms.iter() {
            metrics.insert(
                format!("histogram_{}", key),
                serde_json::to_value(histogram.to_histogram_metric()).unwrap_or_default(),
            );
        }

        metrics
    }

    /// Collect system metrics
    async fn collect_system_metrics(&self) {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        // Update system gauges
        self.set_gauge("memory_usage_bytes", sys.used_memory() as f64);
        self.set_gauge("cpu_usage_percent", sys.global_cpu_usage() as f64);

        debug!("Collected system metrics");
    }

    /// Report metrics to Sentry for monitoring
    async fn report_metrics_to_sentry(&self) {
        let metrics = self.get_current_metrics();

        // Create summary for Sentry
        let summary = serde_json::json!({
            "metrics_count": metrics.len(),
            "timestamp": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs()
        });

        sentry::add_breadcrumb(sentry::Breadcrumb {
            ty: "info".into(),
            category: Some("metrics".into()),
            message: Some("Metrics collection completed".to_string()),
            data: {
                let mut map = sentry::protocol::Map::new();
                map.insert("summary".to_string(), summary.into());
                map
            },
            ..Default::default()
        });
    }

    // ========================================================================
    // Query Phase Timing Methods
    // **Feature: performance-optimization, Property 15: Search Metrics Collection**
    // ========================================================================

    /// Record a query phase timing
    /// **Validates: Requirements 4.1** - Detailed timing metrics for each query phase
    pub fn record_query_timing(&self, timing: QueryPhaseTiming) {
        let mut timings = self.query_timings.write();
        timings.push(timing.clone());

        // Limit memory usage
        if timings.len() > self.max_query_timings {
            timings.drain(0..100); // Remove oldest 100 timings
        }

        // Also record to histogram for aggregated stats
        self.observe_histogram("search_duration_ms:{}", timing.total_ms);

        debug!(
            query = timing.query,
            total_ms = timing.total_ms,
            parsing_ms = timing.parsing_ms,
            execution_ms = timing.execution_ms,
            formatting_ms = timing.result_formatting_ms,
            "Recorded query phase timing"
        );
    }

    /// Get recent query timings
    pub fn get_recent_query_timings(&self, limit: usize) -> Vec<QueryPhaseTiming> {
        let timings = self.query_timings.read();
        timings.iter().rev().take(limit).cloned().collect()
    }

    /// Get query timing statistics
    pub fn get_query_timing_stats(&self) -> QueryTimingStats {
        let timings = self.query_timings.read();

        if timings.is_empty() {
            return QueryTimingStats::default();
        }

        let count = timings.len() as f64;
        let total_parsing: f64 = timings.iter().map(|t| t.parsing_ms).sum();
        let total_execution: f64 = timings.iter().map(|t| t.execution_ms).sum();
        let total_formatting: f64 = timings.iter().map(|t| t.result_formatting_ms).sum();
        let total_highlighting: f64 = timings.iter().map(|t| t.highlighting_ms).sum();
        let total_overall: f64 = timings.iter().map(|t| t.total_ms).sum();

        // Calculate percentiles
        let mut total_times: Vec<f64> = timings.iter().map(|t| t.total_ms).collect();
        total_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50_idx = (total_times.len() as f64 * 0.50) as usize;
        let p95_idx = (total_times.len() as f64 * 0.95) as usize;
        let p99_idx = (total_times.len() as f64 * 0.99) as usize;

        QueryTimingStats {
            query_count: timings.len() as u64,
            avg_parsing_ms: total_parsing / count,
            avg_execution_ms: total_execution / count,
            avg_formatting_ms: total_formatting / count,
            avg_highlighting_ms: total_highlighting / count,
            avg_total_ms: total_overall / count,
            p50_total_ms: total_times.get(p50_idx).copied().unwrap_or(0.0),
            p95_total_ms: total_times
                .get(p95_idx.saturating_sub(1))
                .copied()
                .unwrap_or(0.0),
            p99_total_ms: total_times
                .get(p99_idx.saturating_sub(1))
                .copied()
                .unwrap_or(0.0),
            min_total_ms: total_times.first().copied().unwrap_or(0.0),
            max_total_ms: total_times.last().copied().unwrap_or(0.0),
        }
    }

    // ========================================================================
    // System Resource Monitoring Methods
    // **Feature: performance-optimization, Property 15: Search Metrics Collection**
    // ========================================================================

    /// Collect and store system resource metrics
    /// **Validates: Requirements 4.1** - System resource monitoring (CPU, memory, disk I/O)
    pub fn collect_and_store_system_metrics(&self) -> SystemResourceMetrics {
        use sysinfo::{Disks, System};

        let mut sys = System::new_all();
        sys.refresh_all();

        // Calculate disk I/O (simplified - actual implementation would track deltas)
        let disks = Disks::new_with_refreshed_list();
        let (disk_read, disk_write) = disks.iter().fold((0u64, 0u64), |acc, _disk| {
            // Note: sysinfo doesn't provide disk I/O directly, this is a placeholder
            // In production, you'd use platform-specific APIs or track file operations
            acc
        });

        let memory_total = sys.total_memory();
        let memory_used = sys.used_memory();
        let memory_usage_percent = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64) * 100.0
        } else {
            0.0
        };

        let metrics = SystemResourceMetrics {
            cpu_usage_percent: sys.global_cpu_usage() as f64,
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            memory_usage_percent,
            disk_read_bytes: disk_read,
            disk_write_bytes: disk_write,
            process_count: sys.processes().len(),
            uptime_seconds: sysinfo::System::uptime(),
            timestamp: SystemTime::now(),
        };

        // Store metrics
        {
            let mut system_metrics = self.system_metrics.write();
            system_metrics.push(metrics.clone());

            // Limit memory usage
            if system_metrics.len() > self.max_system_metrics {
                system_metrics.drain(0..10); // Remove oldest 10 snapshots
            }
        }

        // Update gauges
        self.set_gauge("memory_usage_bytes", metrics.memory_used_bytes as f64);
        self.set_gauge("cpu_usage_percent", metrics.cpu_usage_percent);

        debug!(
            cpu = metrics.cpu_usage_percent,
            memory_percent = metrics.memory_usage_percent,
            "Collected system resource metrics"
        );

        metrics
    }

    /// Get recent system resource metrics
    pub fn get_recent_system_metrics(&self, limit: usize) -> Vec<SystemResourceMetrics> {
        let metrics = self.system_metrics.read();
        metrics.iter().rev().take(limit).cloned().collect()
    }

    /// Get current system resource metrics (latest snapshot)
    pub fn get_current_system_metrics(&self) -> Option<SystemResourceMetrics> {
        let metrics = self.system_metrics.read();
        metrics.last().cloned()
    }

    /// Check if system resources are under pressure
    pub fn is_resource_constrained(&self) -> ResourceConstraintStatus {
        let metrics = self.system_metrics.read();
        let latest = match metrics.last() {
            Some(m) => m,
            None => return ResourceConstraintStatus::Unknown,
        };

        if latest.cpu_usage_percent > 90.0 || latest.memory_usage_percent > 90.0 {
            ResourceConstraintStatus::Critical
        } else if latest.cpu_usage_percent > 80.0 || latest.memory_usage_percent > 80.0 {
            ResourceConstraintStatus::Warning
        } else {
            ResourceConstraintStatus::Normal
        }
    }

    // ========================================================================
    // High-Level Operation Recording Methods
    // ========================================================================

    /// Record a search operation with detailed phase timings
    /// **Validates: Requirements 4.1** - Detailed timing metrics for search operations
    pub fn record_search_operation(
        &self,
        query: &str,
        result_count: usize,
        total_duration: Duration,
        phase_timings: Vec<(SearchPhase, Duration)>,
        success: bool,
    ) {
        // Increment operation counter
        self.increment_counter("search_operations_total:{:?}");

        // Record to histogram
        let total_ms = total_duration.as_secs_f64() * 1000.0;
        self.observe_histogram("search_duration_ms:{}", total_ms);

        // Create detailed query timing
        let mut timing = QueryPhaseTiming {
            query: query.to_string(),
            result_count: result_count as u64,
            total_ms,
            timestamp: SystemTime::now(),
            ..Default::default()
        };

        // Fill in phase timings
        for (phase, duration) in phase_timings {
            let ms = duration.as_secs_f64() * 1000.0;
            match phase {
                SearchPhase::Parsing => timing.parsing_ms = ms,
                SearchPhase::Execution => timing.execution_ms = ms,
                SearchPhase::Formatting => timing.result_formatting_ms = ms,
                SearchPhase::Highlighting => timing.highlighting_ms = ms,
            }
        }

        // Record the timing
        self.record_query_timing(timing);

        // Collect system metrics if this is a significant operation
        if total_ms > 100.0 {
            self.collect_and_store_system_metrics();
        }

        debug!(
            query = query,
            result_count = result_count,
            total_ms = total_ms,
            success = success,
            "Recorded search operation"
        );
    }

    /// Record a workspace operation (load, refresh, delete)
    /// **Validates: Requirements 4.1** - Timing metrics for workspace operations
    pub fn record_workspace_operation(
        &self,
        operation_type: &str,
        workspace_id: &str,
        file_count: usize,
        total_duration: Duration,
        _phase_timings: Vec<(&str, Duration)>,
        success: bool,
    ) {
        // Increment operation counter
        self.increment_counter("workspace_operations_total:{:?}");

        // Record to histogram
        let total_ms = total_duration.as_secs_f64() * 1000.0;
        self.observe_histogram("workspace_operation_duration_ms:{}", total_ms);

        // Collect system metrics if this is a significant operation
        if total_ms > 500.0 {
            self.collect_and_store_system_metrics();
        }

        debug!(
            operation_type = operation_type,
            workspace_id = workspace_id,
            file_count = file_count,
            total_ms = total_ms,
            success = success,
            "Recorded workspace operation"
        );
    }

    /// Record a state synchronization operation
    /// **Validates: Requirements 2.1, 4.1** - State sync latency and success rate tracking
    pub fn record_state_sync_operation(
        &self,
        event_type: &str,
        workspace_id: &str,
        latency: Duration,
        success: bool,
    ) {
        // Increment operation counter
        self.increment_counter("state_sync_operations_total:{:?}");

        if success {
            self.increment_counter("state_sync_success_total:{:?}");
        } else {
            self.increment_counter("state_sync_failure_total:{:?}");
        }

        // Record latency to histogram
        let latency_ms = latency.as_secs_f64() * 1000.0;
        self.observe_histogram("state_sync_latency_ms:{}", latency_ms);

        debug!(
            event_type = event_type,
            workspace_id = workspace_id,
            latency_ms = latency_ms,
            success = success,
            "Recorded state sync operation"
        );
    }

    /// Reset all metrics
    pub fn reset_metrics(&self) {
        // Clear query timings
        self.query_timings.write().clear();

        // Clear system metrics
        self.system_metrics.write().clear();

        // Note: Counters, gauges, and histograms cannot be easily reset
        // In production, you might want to recreate them or use a different approach

        info!("Metrics reset completed");
    }
}

/// Search operation phases for detailed timing
#[derive(Debug, Clone, Copy)]
pub enum SearchPhase {
    Parsing,
    Execution,
    Formatting,
    Highlighting,
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            counters: RwLock::new(self.counters.read().clone()),
            gauges: RwLock::new(self.gauges.read().clone()),
            histograms: RwLock::new(self.histograms.read().clone()),
            query_timings: RwLock::new(self.query_timings.read().clone()),
            system_metrics: RwLock::new(self.system_metrics.read().clone()),
            max_query_timings: self.max_query_timings,
            max_system_metrics: self.max_system_metrics,
            collection_interval: self.collection_interval,
        }
    }
}

/// Convenience macros for metrics
#[macro_export]
macro_rules! increment_counter {
    ($collector:expr, $name:expr) => {
        $collector.increment_counter($name)
    };
}

#[macro_export]
macro_rules! set_gauge {
    ($collector:expr, $name:expr, $value:expr) => {
        $collector.set_gauge($name, $value)
    };
}

#[macro_export]
macro_rules! observe_histogram {
    ($collector:expr, $name:expr, $value:expr) => {
        $collector.observe_histogram($name, $value)
    };
}

#[macro_export]
macro_rules! time_histogram {
    ($collector:expr, $name:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        $collector.observe_histogram($name, duration.as_millis() as f64);
        result
    }};
}
