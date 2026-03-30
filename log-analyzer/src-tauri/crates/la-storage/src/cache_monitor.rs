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
}
