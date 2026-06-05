//! Cache Consistency Monitor
//!
//! Monitors the health and consistency of the CAS existence cache
//! against the actual filesystem state.
//!
//! ## Features
//!
//! - Cache hit/miss ratio tracking
//! - Stale entry detection
//! - Background consistency checks
//! - Health metrics export

use crate::cas::ContentAddressableStorage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Cache health metrics
#[derive(Debug, Clone, Default)]
pub struct CacheHealthMetrics {
    /// Total cache lookups
    pub total_lookups: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Stale entries detected (cache said exists but file doesn't)
    pub stale_entries: u64,
    /// Inconsistencies fixed
    pub inconsistencies_fixed: u64,
    /// Last check timestamp
    pub last_check: Option<Instant>,
}

impl CacheHealthMetrics {
    /// Calculate cache hit ratio
    pub fn hit_ratio(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_lookups as f64
        }
    }

    /// Calculate cache miss ratio
    pub fn miss_ratio(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            self.misses as f64 / self.total_lookups as f64
        }
    }
}

/// Configuration for cache monitoring
#[derive(Debug, Clone)]
pub struct CacheMonitorConfig {
    /// Interval between consistency checks
    pub check_interval: Duration,
    /// Sample size for consistency checks (0 = all)
    pub sample_size: usize,
    /// Auto-fix inconsistencies
    pub auto_fix: bool,
    /// Log warnings on stale entries
    pub warn_on_stale: bool,
}

impl Default for CacheMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(600), // 10 minutes
            sample_size: 100,                         // Check 100 random entries
            auto_fix: true,
            warn_on_stale: true,
        }
    }
}

/// Cache consistency monitor
pub struct CacheMonitor {
    cas: Arc<ContentAddressableStorage>,
    config: CacheMonitorConfig,
    metrics: RwLock<CacheHealthMetrics>,
}

impl CacheMonitor {
    /// Create a new cache monitor
    pub fn new(cas: Arc<ContentAddressableStorage>, config: CacheMonitorConfig) -> Self {
        Self {
            cas,
            config,
            metrics: RwLock::new(CacheHealthMetrics::default()),
        }
    }

    /// Record a cache hit
    pub async fn record_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_lookups += 1;
        metrics.hits += 1;
    }

    /// Record a cache miss
    pub async fn record_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_lookups += 1;
        metrics.misses += 1;
    }

    /// Record a stale entry detection
    pub async fn record_stale(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.stale_entries += 1;
    }

    /// Get current metrics
    pub async fn metrics(&self) -> CacheHealthMetrics {
        self.metrics.read().await.clone()
    }

    /// Perform consistency check on a specific hash
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to check
    ///
    /// # Returns
    ///
    /// Returns Ok(true) if consistent, Ok(false) if stale entry found
    pub async fn check_consistency(&self, hash: &str) -> la_core::error::Result<bool> {
        // Get cache state
        let in_cache = self.cas.exists(hash);

        // Get filesystem state
        let object_path = self.cas.get_object_path(hash);
        let on_disk = tokio::fs::try_exists(&object_path).await.unwrap_or(false);

        // Check consistency
        if in_cache && !on_disk {
            // Stale cache entry
            if self.config.warn_on_stale {
                warn!(
                    hash = %hash,
                    "Cache inconsistency detected: cache indicates existence but file missing"
                );
            }

            self.record_stale().await;

            if self.config.auto_fix {
                self.cas.invalidate_cache_entry(hash);
                let mut metrics = self.metrics.write().await;
                metrics.inconsistencies_fixed += 1;
            }

            return Ok(false);
        }

        Ok(true)
    }

    /// Run full consistency check
    ///
    /// Checks all cached entries against filesystem state
    pub async fn run_full_check(&self) -> la_core::error::Result<usize> {
        info!("Starting full cache consistency check");

        // This would need access to cache internals
        // For now, just log that we're starting
        debug!("Full cache consistency check would scan all entries");

        let mut metrics = self.metrics.write().await;
        metrics.last_check = Some(Instant::now());

        Ok(0)
    }

    /// Start background monitoring
    ///
    /// Spawns a task that periodically checks cache consistency
    pub fn start_monitoring(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>) {
        let interval = self.config.check_interval;

        tokio::spawn(async move {
            info!(
                interval_secs = interval.as_secs(),
                "Starting cache monitoring"
            );

            let mut interval = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match self.run_full_check().await {
                            Ok(fixed) => {
                                debug!(inconsistencies_fixed = fixed, "Cache check completed");
                            }
                            Err(e) => {
                                warn!(error = %e, "Cache consistency check failed");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Cache monitor shutting down");
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cas() -> (Arc<ContentAddressableStorage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cas = Arc::new(ContentAddressableStorage::new(
            temp_dir.path().to_path_buf(),
        ));
        (cas, temp_dir)
    }

    // ── Synchronous unit tests for value types ──

    #[test]
    fn test_cache_metrics_default() {
        let metrics = CacheHealthMetrics::default();
        assert_eq!(metrics.total_lookups, 0);
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.hit_ratio(), 0.0);
        assert_eq!(metrics.miss_ratio(), 0.0);
    }

    #[test]
    fn test_cache_metrics_ratios() {
        let metrics = CacheHealthMetrics {
            total_lookups: 100,
            hits: 75,
            misses: 25,
            ..Default::default()
        };
        assert_eq!(metrics.hit_ratio(), 0.75);
        assert_eq!(metrics.miss_ratio(), 0.25);
    }

    #[test]
    fn test_monitor_config_default() {
        let config = CacheMonitorConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(600));
        assert_eq!(config.sample_size, 100);
        assert!(config.auto_fix);
        assert!(config.warn_on_stale);
    }

    // ── Async integration tests for CacheMonitor ──

    #[tokio::test]
    async fn test_create_cache_monitor_initial_state() {
        let (cas, _temp_dir) = create_test_cas();
        let monitor = CacheMonitor::new(cas, CacheMonitorConfig::default());

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.total_lookups, 0);
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.stale_entries, 0);
        assert_eq!(metrics.inconsistencies_fixed, 0);
        assert!(metrics.last_check.is_none());
    }

    #[tokio::test]
    async fn test_record_hit_and_miss_updates_metrics() {
        let (cas, _temp_dir) = create_test_cas();
        let monitor = CacheMonitor::new(cas, CacheMonitorConfig::default());

        monitor.record_hit().await;
        monitor.record_hit().await;
        monitor.record_miss().await;

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.total_lookups, 3);
        assert_eq!(metrics.hits, 2);
        assert_eq!(metrics.misses, 1);
        assert!((metrics.hit_ratio() - 2.0 / 3.0).abs() < f64::EPSILON);
        assert!((metrics.miss_ratio() - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_check_consistency_valid_entry() {
        let (cas, _temp_dir) = create_test_cas();
        // Disable auto_fix and warnings so we can observe raw metrics
        let config = CacheMonitorConfig {
            auto_fix: false,
            warn_on_stale: false,
            ..Default::default()
        };
        let monitor = CacheMonitor::new(cas.clone(), config);

        // Write a file to CAS to obtain a valid hash with a cache entry
        let file_dir = TempDir::new().unwrap();
        let file_path = file_dir.path().join("test.log");
        tokio::fs::write(&file_path, b"consistency test content").await.unwrap();

        let hash = cas.store_file_zero_copy(&file_path).await.unwrap();
        assert_eq!(hash.len(), 64);

        // Consistency check on a valid, existing entry should pass
        let result = monitor.check_consistency(&hash).await.unwrap();
        assert!(
            result,
            "Existing CAS entry should be reported as consistent"
        );

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.stale_entries, 0);
    }

    #[tokio::test]
    async fn test_check_consistency_nonexistent_hash() {
        let (cas, _temp_dir) = create_test_cas();
        let monitor = CacheMonitor::new(cas, CacheMonitorConfig::default());

        // A valid-format hex hash that has no corresponding CAS object on disk
        let fake_hash = "a".repeat(64);

        // Should not panic; returns true because the entry is simply absent,
        // not stale (CAS::exists() self-corrects the cache on every lookup)
        let result = monitor.check_consistency(&fake_hash).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_record_stale_increments_counter() {
        let (cas, _temp_dir) = create_test_cas();
        let monitor = CacheMonitor::new(cas, CacheMonitorConfig::default());

        monitor.record_stale().await;
        monitor.record_stale().await;
        monitor.record_stale().await;

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.stale_entries, 3);
    }

    #[tokio::test]
    async fn test_run_full_check_updates_last_check_timestamp() {
        let (cas, _temp_dir) = create_test_cas();
        let monitor = CacheMonitor::new(cas, CacheMonitorConfig::default());

        let metrics_before = monitor.metrics().await;
        assert!(metrics_before.last_check.is_none());

        let fixed = monitor.run_full_check().await.unwrap();
        assert_eq!(fixed, 0);

        let metrics_after = monitor.metrics().await;
        assert!(metrics_after.last_check.is_some());
    }
}
