//! Dynamic Performance Optimizer
//!
//! Implements automatic performance optimization based on usage patterns:
//! - Dynamic resource allocation and scaling
//! - Self-tuning cache management
//! - System load-aware optimization
//! - Automatic cache size adjustment
//! - CPU core scaling for indexing and search operations
//! - Load balancing for concurrent operations
//!
//! **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
//! **Validates: Requirements 7.5**

use crate::monitoring::metrics_collector::MetricsCollector;
use crate::utils::cache_manager::CacheManager;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, info};

/// Configuration for the dynamic optimizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicOptimizerConfig {
    pub check_interval: Duration,
    pub cpu_high_threshold: f64,
    pub cpu_low_threshold: f64,
    pub memory_high_threshold: f64,
    pub memory_low_threshold: f64,
    pub min_cache_size: u64,
    pub max_cache_size: u64,
    /// Target hit rate for cache optimization
    pub target_hit_rate: f64,
    /// Eviction threshold (percentage of cache to evict under pressure)
    pub eviction_threshold: f64,
    /// Minimum worker threads
    pub min_workers: usize,
    /// Maximum worker threads
    pub max_workers: usize,
    /// Load history window size
    pub load_history_size: usize,
}

impl Default for DynamicOptimizerConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            cpu_high_threshold: 80.0,
            cpu_low_threshold: 30.0,
            memory_high_threshold: 85.0,
            memory_low_threshold: 50.0,
            min_cache_size: 100,
            max_cache_size: 5000,
            target_hit_rate: 0.7,
            eviction_threshold: 20.0,
            min_workers: 1,
            max_workers: num_cpus::get(),
            load_history_size: 60, // 30 minutes at 30s intervals
        }
    }
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub recommendation_type: RecommendationType,
    pub description: String,
    pub priority: Priority,
    pub estimated_impact: String,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationType {
    IncreaseCacheSize,
    DecreaseCacheSize,
    EnableCompression,
    EnableL2Cache,
    AdjustTTL,
    ReduceConcurrency,
    IncreaseConcurrency,
    OptimizeQueries,
    ScaleUpWorkers,
    ScaleDownWorkers,
    TriggerGC,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Resource allocation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Current number of active worker threads
    pub active_workers: usize,
    /// Current cache size allocation
    pub cache_size: u64,
    /// Current memory budget (bytes)
    pub memory_budget: u64,
    /// Timestamp of last adjustment
    pub last_adjusted: SystemTime,
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        Self {
            active_workers: num_cpus::get().max(2),
            cache_size: 1000,
            memory_budget: 512 * 1024 * 1024, // 512MB default
            last_adjusted: SystemTime::now(),
        }
    }
}

/// Load history entry for trend analysis
#[derive(Debug, Clone)]
struct LoadHistoryEntry {
    #[allow(dead_code)]
    timestamp: Instant,
    cpu_usage: f64,
    memory_usage: f64,
    #[allow(dead_code)]
    active_operations: u64,
    #[allow(dead_code)]
    cache_hit_rate: f64,
}

/// Resource manager for dynamic allocation
///
/// **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
/// **Validates: Requirements 7.5**
pub struct ResourceManager {
    config: DynamicOptimizerConfig,
    current_allocation: Arc<RwLock<ResourceAllocation>>,
    load_history: Arc<RwLock<VecDeque<LoadHistoryEntry>>>,
    active_operations: AtomicU64,
    pending_operations: AtomicU64,
    completed_operations: AtomicU64,
    current_workers: AtomicUsize,
    scaling_cooldown: Arc<RwLock<Option<Instant>>>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(config: DynamicOptimizerConfig) -> Self {
        let initial_workers = (config.min_workers + config.max_workers) / 2;

        Self {
            config,
            current_allocation: Arc::new(RwLock::new(ResourceAllocation::default())),
            load_history: Arc::new(RwLock::new(VecDeque::new())),
            active_operations: AtomicU64::new(0),
            pending_operations: AtomicU64::new(0),
            completed_operations: AtomicU64::new(0),
            current_workers: AtomicUsize::new(initial_workers),
            scaling_cooldown: Arc::new(RwLock::new(None)),
        }
    }

    /// Record start of an operation
    pub fn operation_started(&self) {
        self.active_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record completion of an operation
    pub fn operation_completed(&self) {
        self.active_operations.fetch_sub(1, Ordering::Relaxed);
        self.completed_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Queue an operation
    pub fn operation_queued(&self) {
        self.pending_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Dequeue an operation
    pub fn operation_dequeued(&self) {
        self.pending_operations.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current active operation count
    pub fn get_active_operations(&self) -> u64 {
        self.active_operations.load(Ordering::Relaxed)
    }

    /// Get current worker count
    pub fn get_current_workers(&self) -> usize {
        self.current_workers.load(Ordering::Relaxed)
    }

    /// Set current worker count (for testing)
    #[cfg(test)]
    pub fn set_current_workers(&self, count: usize) {
        self.current_workers.store(count, Ordering::Relaxed);
    }

    /// Record load metrics for trend analysis
    pub fn record_load(&self, cpu_usage: f64, memory_usage: f64, cache_hit_rate: f64) {
        let entry = LoadHistoryEntry {
            timestamp: Instant::now(),
            cpu_usage,
            memory_usage,
            active_operations: self.active_operations.load(Ordering::Relaxed),
            cache_hit_rate,
        };

        let mut history = self.load_history.write();
        history.push_back(entry);

        // Maintain window size
        while history.len() > self.config.load_history_size {
            history.pop_front();
        }
    }

    /// Calculate load trend (positive = increasing, negative = decreasing)
    pub fn calculate_load_trend(&self) -> f64 {
        let history = self.load_history.read();

        if history.len() < 5 {
            return 0.0;
        }

        // Compare recent average to older average
        let mid = history.len() / 2;
        let recent_avg: f64 = history.iter().skip(mid).map(|e| e.cpu_usage).sum::<f64>()
            / (history.len() - mid) as f64;
        let older_avg: f64 =
            history.iter().take(mid).map(|e| e.cpu_usage).sum::<f64>() / mid as f64;

        recent_avg - older_avg
    }

    /// Determine optimal worker count based on load
    ///
    /// **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
    pub fn calculate_optimal_workers(&self, cpu_usage: f64, pending_ops: u64) -> usize {
        let current = self.current_workers.load(Ordering::Relaxed);
        let trend = self.calculate_load_trend();

        // Scale up conditions
        if cpu_usage < self.config.cpu_low_threshold && pending_ops > 0 {
            // Low CPU but work pending - scale up
            return (current + 1).min(self.config.max_workers);
        }

        // Scale down conditions
        if cpu_usage > self.config.cpu_high_threshold {
            // High CPU - scale down to reduce contention
            return (current.saturating_sub(1)).max(self.config.min_workers);
        }

        // Trend-based adjustment
        if trend > 10.0 && current < self.config.max_workers {
            // Load increasing rapidly - preemptively scale up
            return current + 1;
        } else if trend < -10.0 && current > self.config.min_workers {
            // Load decreasing - scale down
            return current.saturating_sub(1);
        }

        current
    }

    /// Apply worker scaling with cooldown
    pub fn apply_worker_scaling(&self, target_workers: usize) -> bool {
        // Check cooldown
        {
            let cooldown = self.scaling_cooldown.read();
            if let Some(last_scale) = *cooldown {
                if last_scale.elapsed() < Duration::from_secs(60) {
                    return false; // Still in cooldown
                }
            }
        }

        let current = self.current_workers.load(Ordering::Relaxed);
        if target_workers == current {
            return false;
        }

        // Apply scaling
        self.current_workers
            .store(target_workers, Ordering::Relaxed);
        *self.scaling_cooldown.write() = Some(Instant::now());

        // Update allocation record
        {
            let mut allocation = self.current_allocation.write();
            allocation.active_workers = target_workers;
            allocation.last_adjusted = SystemTime::now();
        }

        info!(
            from = current,
            to = target_workers,
            "Applied worker scaling"
        );

        true
    }

    /// Get current resource allocation
    pub fn get_allocation(&self) -> ResourceAllocation {
        self.current_allocation.read().clone()
    }

    /// Get load statistics
    pub fn get_load_stats(&self) -> LoadStats {
        let history = self.load_history.read();

        if history.is_empty() {
            return LoadStats::default();
        }

        let cpu_values: Vec<f64> = history.iter().map(|e| e.cpu_usage).collect();
        let mem_values: Vec<f64> = history.iter().map(|e| e.memory_usage).collect();

        LoadStats {
            avg_cpu_usage: cpu_values.iter().sum::<f64>() / cpu_values.len() as f64,
            max_cpu_usage: cpu_values.iter().cloned().fold(0.0, f64::max),
            avg_memory_usage: mem_values.iter().sum::<f64>() / mem_values.len() as f64,
            max_memory_usage: mem_values.iter().cloned().fold(0.0, f64::max),
            load_trend: self.calculate_load_trend(),
            sample_count: history.len(),
        }
    }
}

/// Load statistics summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadStats {
    pub avg_cpu_usage: f64,
    pub max_cpu_usage: f64,
    pub avg_memory_usage: f64,
    pub max_memory_usage: f64,
    pub load_trend: f64,
    pub sample_count: usize,
}

/// Dynamic optimizer for self-tuning system performance
///
/// **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
/// **Validates: Requirements 7.5**
pub struct DynamicOptimizer {
    cache_manager: Arc<CacheManager>,
    metrics_collector: Arc<MetricsCollector>,
    resource_manager: Arc<ResourceManager>,
    config: DynamicOptimizerConfig,
    recommendations_history: Arc<RwLock<Vec<OptimizationRecommendation>>>,
}

impl DynamicOptimizer {
    /// Create a new dynamic optimizer
    pub fn new(
        cache_manager: Arc<CacheManager>,
        metrics_collector: Arc<MetricsCollector>,
        config: DynamicOptimizerConfig,
    ) -> Self {
        let resource_manager = Arc::new(ResourceManager::new(config.clone()));

        Self {
            cache_manager,
            metrics_collector,
            resource_manager,
            config,
            recommendations_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the resource manager
    pub fn resource_manager(&self) -> Arc<ResourceManager> {
        self.resource_manager.clone()
    }

    /// Start the optimization loop
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let cache_mgr = self.cache_manager.clone();
        let metrics = self.metrics_collector.clone();
        let resource_mgr = self.resource_manager.clone();
        let config = self.config.clone();
        let recommendations = self.recommendations_history.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(config.check_interval);
            loop {
                interval.tick().await;

                // 1. Collect current system state
                let cache_snapshot = cache_mgr.get_performance_metrics();
                let system_metrics = metrics.get_current_metrics();

                // Extract metrics
                let cpu_usage = system_metrics
                    .get("gauge_cpu_usage_percent:{}")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                let mem_usage = system_metrics
                    .get("gauge_memory_usage_bytes:{}")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                // Record load for trend analysis
                resource_mgr.record_load(cpu_usage, mem_usage, cache_snapshot.l1_hit_rate);

                // 2. Perform dynamic resource allocation
                Self::perform_resource_allocation(&resource_mgr, cpu_usage, &config).await;

                // 3. Perform self-tuning cache
                Self::tune_cache_size(&cache_mgr, &cache_snapshot, cpu_usage, mem_usage, &config)
                    .await;

                // 4. Check for intelligent eviction needs
                Self::check_eviction_needs(&cache_mgr, mem_usage, &config).await;

                // 5. Generate and store recommendations
                let new_recommendations = Self::generate_recommendations_internal(
                    &cache_snapshot,
                    &resource_mgr,
                    cpu_usage,
                    mem_usage,
                    &config,
                );

                if !new_recommendations.is_empty() {
                    let mut history = recommendations.write();
                    history.extend(new_recommendations);
                    // Keep only recent recommendations
                    if history.len() > 100 {
                        history.drain(0..50);
                    }
                }
            }
        })
    }

    /// Perform dynamic resource allocation based on system load
    ///
    /// **Feature: performance-optimization, Property 29: Dynamic Resource Allocation**
    async fn perform_resource_allocation(
        resource_mgr: &ResourceManager,
        cpu_usage: f64,
        _config: &DynamicOptimizerConfig,
    ) {
        let pending_ops = resource_mgr.pending_operations.load(Ordering::Relaxed);
        let optimal_workers = resource_mgr.calculate_optimal_workers(cpu_usage, pending_ops);

        if resource_mgr.apply_worker_scaling(optimal_workers) {
            debug!(
                cpu = %cpu_usage,
                pending = pending_ops,
                workers = optimal_workers,
                "Adjusted worker count based on load"
            );
        }
    }

    /// Automatically adjust cache size based on performance and system load
    async fn tune_cache_size(
        cache_mgr: &CacheManager,
        snapshot: &crate::utils::cache_manager::CacheMetricsSnapshot,
        cpu_usage: f64,
        mem_usage: f64,
        config: &DynamicOptimizerConfig,
    ) {
        // Logic for cache size adjustment
        if cpu_usage > config.cpu_high_threshold || mem_usage > config.memory_high_threshold {
            // System under pressure, reduce cache size to free resources
            if snapshot.total_requests > 0 {
                debug!(
                    cpu = %cpu_usage,
                    memory = %mem_usage,
                    "System under pressure, triggering cache eviction"
                );
                let _ = cache_mgr
                    .intelligent_eviction(config.eviction_threshold)
                    .await;
            }
        } else if snapshot.l1_hit_rate < config.target_hit_rate && snapshot.total_requests > 100 {
            // Low hit rate and system has headroom, suggest increasing cache size
            info!(
                hit_rate = %snapshot.l1_hit_rate,
                target = %config.target_hit_rate,
                "Low hit rate detected, cache may benefit from size increase"
            );
        }
    }

    /// Check if intelligent eviction is needed based on memory pressure
    async fn check_eviction_needs(
        cache_mgr: &CacheManager,
        mem_usage: f64,
        config: &DynamicOptimizerConfig,
    ) {
        if mem_usage > config.memory_high_threshold {
            info!(
                memory = %mem_usage,
                threshold = %config.memory_high_threshold,
                "Memory pressure detected, triggering intelligent eviction"
            );
            let _ = cache_mgr
                .intelligent_eviction(config.eviction_threshold)
                .await;
        }
    }

    /// Generate optimization recommendations based on current state
    fn generate_recommendations_internal(
        cache_metrics: &crate::utils::cache_manager::CacheMetricsSnapshot,
        resource_mgr: &ResourceManager,
        cpu_usage: f64,
        mem_usage: f64,
        config: &DynamicOptimizerConfig,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        let load_stats = resource_mgr.get_load_stats();

        // CPU-based recommendations
        if cpu_usage > config.cpu_high_threshold {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::ReduceConcurrency,
                description: format!(
                    "CPU usage ({:.1}%) exceeds threshold. Consider reducing concurrent operations.",
                    cpu_usage
                ),
                priority: Priority::High,
                estimated_impact: "Could reduce CPU usage by 20-30%".to_string(),
                created_at: SystemTime::now(),
            });
        } else if cpu_usage < config.cpu_low_threshold && resource_mgr.get_active_operations() > 0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::IncreaseConcurrency,
                description: format!(
                    "CPU usage ({:.1}%) is low with active operations. Can increase parallelism.",
                    cpu_usage
                ),
                priority: Priority::Low,
                estimated_impact: "Could improve throughput by 10-20%".to_string(),
                created_at: SystemTime::now(),
            });
        }

        // Memory-based recommendations
        if mem_usage > config.memory_high_threshold {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::TriggerGC,
                description: format!(
                    "Memory usage ({:.1}%) is high. Consider triggering garbage collection.",
                    mem_usage
                ),
                priority: Priority::High,
                estimated_impact: "Could free 10-30% memory".to_string(),
                created_at: SystemTime::now(),
            });
        }

        // Cache hit rate recommendations
        if cache_metrics.l1_hit_rate < config.target_hit_rate && cache_metrics.total_requests > 50 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::IncreaseCacheSize,
                description: format!(
                    "Cache hit rate ({:.1}%) is below target ({:.1}%).",
                    cache_metrics.l1_hit_rate * 100.0,
                    config.target_hit_rate * 100.0
                ),
                priority: Priority::Medium,
                estimated_impact: "Could improve hit rate by 10-20%".to_string(),
                created_at: SystemTime::now(),
            });
        }

        // Eviction rate recommendations
        if cache_metrics.eviction_rate_per_minute > 5.0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::IncreaseCacheSize,
                description: format!(
                    "High eviction rate ({:.1}/min). Cache may be undersized.",
                    cache_metrics.eviction_rate_per_minute
                ),
                priority: Priority::High,
                estimated_impact: "Could reduce evictions by 50%+".to_string(),
                created_at: SystemTime::now(),
            });
        }

        // Worker scaling recommendations based on trend
        if load_stats.load_trend > 15.0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::ScaleUpWorkers,
                description: format!(
                    "Load trend is increasing ({:.1}). Consider scaling up workers.",
                    load_stats.load_trend
                ),
                priority: Priority::Medium,
                estimated_impact: "Could handle increased load better".to_string(),
                created_at: SystemTime::now(),
            });
        } else if load_stats.load_trend < -15.0
            && resource_mgr.get_current_workers() > config.min_workers
        {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::ScaleDownWorkers,
                description: format!(
                    "Load trend is decreasing ({:.1}). Can scale down workers.",
                    load_stats.load_trend
                ),
                priority: Priority::Low,
                estimated_impact: "Could reduce resource usage".to_string(),
                created_at: SystemTime::now(),
            });
        }

        recommendations
    }

    /// Generate optimization recommendations (public API)
    pub fn generate_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let cache_metrics = self.cache_manager.get_performance_metrics();
        let access_stats = self.cache_manager.get_access_pattern_stats();
        let system_metrics = self.metrics_collector.get_current_metrics();

        let cpu_usage = system_metrics
            .get("gauge_cpu_usage_percent:{}")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mem_usage = system_metrics
            .get("gauge_memory_usage_bytes:{}")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mut recommendations = Self::generate_recommendations_internal(
            &cache_metrics,
            &self.resource_manager,
            cpu_usage,
            mem_usage,
            &self.config,
        );

        // Additional recommendations based on cache manager state
        if !self.cache_manager.is_l2_connected() && access_stats.hot_keys > 10 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::EnableL2Cache,
                description:
                    "L2 cache (Redis) is not connected. Could improve distributed performance."
                        .to_string(),
                priority: Priority::Low,
                estimated_impact: "Could provide distributed caching benefits".to_string(),
                created_at: SystemTime::now(),
            });
        }

        let compression_stats = self.cache_manager.get_compression_stats();
        if !compression_stats.compression_enabled && cache_metrics.total_requests > 100 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::EnableCompression,
                description: "Cache compression is disabled. Could reduce memory usage."
                    .to_string(),
                priority: Priority::Low,
                estimated_impact: "Could reduce memory usage by 30-50% for large entries"
                    .to_string(),
                created_at: SystemTime::now(),
            });
        }

        if cache_metrics.avg_access_time_ms > 10.0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::OptimizeQueries,
                description: format!(
                    "Average cache access time ({:.2}ms) is high.",
                    cache_metrics.avg_access_time_ms
                ),
                priority: Priority::Medium,
                estimated_impact: "Could reduce access time by 50%+".to_string(),
                created_at: SystemTime::now(),
            });
        }

        recommendations
    }

    /// Get recommendation history
    pub fn get_recommendation_history(&self) -> Vec<OptimizationRecommendation> {
        self.recommendations_history.read().clone()
    }

    /// Get current resource allocation
    pub fn get_resource_allocation(&self) -> ResourceAllocation {
        self.resource_manager.get_allocation()
    }

    /// Get load statistics
    pub fn get_load_stats(&self) -> LoadStats {
        self.resource_manager.get_load_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_manager_creation() {
        let config = DynamicOptimizerConfig::default();
        let manager = ResourceManager::new(config.clone());

        assert!(manager.get_current_workers() >= config.min_workers);
        assert!(manager.get_current_workers() <= config.max_workers);
    }

    #[test]
    fn test_operation_tracking() {
        let config = DynamicOptimizerConfig::default();
        let manager = ResourceManager::new(config);

        assert_eq!(manager.get_active_operations(), 0);

        manager.operation_started();
        assert_eq!(manager.get_active_operations(), 1);

        manager.operation_started();
        assert_eq!(manager.get_active_operations(), 2);

        manager.operation_completed();
        assert_eq!(manager.get_active_operations(), 1);
    }

    #[test]
    fn test_load_recording() {
        let config = DynamicOptimizerConfig {
            load_history_size: 10,
            ..Default::default()
        };
        let manager = ResourceManager::new(config);

        for i in 0..15 {
            manager.record_load(i as f64 * 5.0, 50.0, 0.8);
        }

        let stats = manager.get_load_stats();
        assert_eq!(stats.sample_count, 10); // Should be capped at window size
    }

    #[test]
    fn test_worker_scaling_calculation() {
        let config = DynamicOptimizerConfig {
            min_workers: 1,
            max_workers: 8,
            cpu_high_threshold: 80.0,
            cpu_low_threshold: 30.0,
            ..Default::default()
        };
        let manager = ResourceManager::new(config);

        // Low CPU with pending work should scale up
        let optimal = manager.calculate_optimal_workers(20.0, 5);
        assert!(optimal > manager.get_current_workers());

        // High CPU should scale down
        let optimal = manager.calculate_optimal_workers(90.0, 0);
        assert!(optimal < manager.get_current_workers() || optimal == 1);
    }

    #[test]
    fn test_scaling_cooldown() {
        let config = DynamicOptimizerConfig {
            min_workers: 1,
            max_workers: 8,
            ..Default::default()
        };
        let manager = ResourceManager::new(config);

        // Initial workers is (1+8)/2 = 4, so scale to a different value
        // First scaling should succeed
        assert!(manager.apply_worker_scaling(6));

        // Immediate second scaling should fail (cooldown)
        assert!(!manager.apply_worker_scaling(3));
    }

    #[test]
    fn test_load_trend_calculation() {
        let config = DynamicOptimizerConfig {
            load_history_size: 20,
            ..Default::default()
        };
        let manager = ResourceManager::new(config);

        // Record increasing load
        for i in 0..10 {
            manager.record_load(30.0 + i as f64 * 5.0, 50.0, 0.8);
        }

        let trend = manager.calculate_load_trend();
        assert!(trend > 0.0, "Trend should be positive for increasing load");
    }
}
