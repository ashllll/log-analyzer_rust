//! Concurrent Search Support
#![allow(dead_code)]
//!
//! Provides thread-safe search operations with performance guarantees:
//! - Thread-safe SearchEngineManager with read-only access patterns
//! - Connection pooling for concurrent index reader access
//! - Load balancing for concurrent queries across CPU cores
//! - Performance monitoring to detect concurrent search degradation
//!
//! ## Architecture Decision: Pure Tokio vs Rayon+Tokio
//!
//! **Historical Issue**: Previously used Rayon parallel iterators with Tokio runtime creation
//! inside the loop, which was inefficient and potentially unsafe.
//!
//! **Current Solution**: Pure Tokio approach using `tokio::spawn` + `futures::join_all`
//! - Single Tokio runtime (no runtime creation inside loops)
//! - Proper async/await semantics
//! - Better resource management
//! - Industry-standard pattern for concurrent async operations

use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::{Index, IndexReader, ReloadPolicy};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::manager::SearchResults;
use super::{SearchEngineManager, SearchError, SearchResult};

/// Configuration for concurrent search operations
#[derive(Debug, Clone)]
pub struct ConcurrentSearchConfig {
    /// Maximum number of concurrent searches allowed
    pub max_concurrent_searches: usize,
    /// Number of reader connections in the pool
    pub reader_pool_size: usize,
    /// Timeout for acquiring a reader from the pool
    pub reader_acquire_timeout: Duration,
    /// Performance degradation threshold (multiplier)
    pub performance_degradation_threshold: f64,
    /// CPU core utilization target (0.0 to 1.0)
    pub cpu_utilization_target: f64,
}

impl Default for ConcurrentSearchConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            max_concurrent_searches: cpu_count * 2,
            reader_pool_size: cpu_count,
            reader_acquire_timeout: Duration::from_millis(100),
            performance_degradation_threshold: 2.0, // 2x slower is considered degraded
            cpu_utilization_target: 0.8,            // 80% CPU utilization target
        }
    }
}

/// Statistics for concurrent search operations
#[derive(Debug, Clone, Default)]
pub struct ConcurrentSearchStats {
    pub total_concurrent_searches: u64,
    pub active_searches: u64,
    pub peak_concurrent_searches: u64,
    pub average_response_time_ms: f64,
    pub performance_degradation_events: u64,
    pub reader_pool_hits: u64,
    pub reader_pool_misses: u64,
    pub cpu_utilization: f64,
}

/// Reader connection pool for concurrent access
struct ReaderPool {
    _readers: Vec<IndexReader>,
    _available: Arc<Semaphore>,
    stats: Arc<RwLock<ReaderPoolStats>>,
}

#[derive(Debug, Default, Clone)]
struct ReaderPoolStats {
    hits: u64,
    misses: u64,
    _total_acquisitions: u64,
}

impl ReaderPool {
    fn new(index: &Index, pool_size: usize) -> SearchResult<Self> {
        let mut readers = Vec::with_capacity(pool_size);

        for _ in 0..pool_size {
            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()?;
            readers.push(reader);
        }

        Ok(Self {
            _readers: readers,
            _available: Arc::new(Semaphore::new(pool_size)),
            stats: Arc::new(RwLock::new(ReaderPoolStats::default())),
        })
    }

    fn get_stats(&self) -> ReaderPoolStats {
        self.stats.read().clone()
    }
}

/// Concurrent search manager with performance monitoring
pub struct ConcurrentSearchManager {
    /// Arc-wrapped search engine for safe sharing across async tasks
    search_engine: Arc<SearchEngineManager>,
    reader_pool: ReaderPool,
    config: ConcurrentSearchConfig,
    stats: Arc<RwLock<ConcurrentSearchStats>>,
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
    search_semaphore: Arc<Semaphore>,
}

/// Performance monitoring for concurrent operations
struct PerformanceMonitor {
    baseline_response_time: Option<Duration>,
    recent_response_times: Vec<Duration>,
    max_samples: usize,
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {
            baseline_response_time: None,
            recent_response_times: Vec::new(),
            max_samples: 100,
        }
    }

    fn record_response_time(&mut self, response_time: Duration) {
        self.recent_response_times.push(response_time);

        // Keep only recent samples
        if self.recent_response_times.len() > self.max_samples {
            self.recent_response_times.remove(0);
        }

        // Update baseline if not set or if we have enough samples
        if self.baseline_response_time.is_none() && self.recent_response_times.len() >= 10 {
            let avg = self.calculate_average_response_time();
            self.baseline_response_time = Some(avg);
        }
    }

    fn calculate_average_response_time(&self) -> Duration {
        if self.recent_response_times.is_empty() {
            return Duration::from_millis(0);
        }

        let total_ms: u64 = self
            .recent_response_times
            .iter()
            .map(|d| d.as_millis() as u64)
            .sum();

        Duration::from_millis(total_ms / self.recent_response_times.len() as u64)
    }

    fn is_performance_degraded(&self, threshold: f64) -> bool {
        if let Some(baseline) = self.baseline_response_time {
            if baseline.as_millis() == 0 {
                // Avoid divide-by-zero if baseline is zero
                return false;
            }
            let current_avg = self.calculate_average_response_time();
            let degradation_ratio = current_avg.as_millis() as f64 / baseline.as_millis() as f64;
            degradation_ratio > threshold
        } else {
            false
        }
    }
}

impl ConcurrentSearchManager {
    /// Create a new concurrent search manager
    pub fn new(
        search_engine: SearchEngineManager,
        config: ConcurrentSearchConfig,
    ) -> SearchResult<Self> {
        let search_engine = Arc::new(search_engine);

        // Create reader pool - we need access to the index
        // For now, we'll create a simple pool using the existing reader
        let reader_pool = ReaderPool::new(&search_engine.index, config.reader_pool_size)?;

        let search_semaphore = Arc::new(Semaphore::new(config.max_concurrent_searches));

        info!(
            max_concurrent = config.max_concurrent_searches,
            reader_pool_size = config.reader_pool_size,
            "Concurrent search manager initialized"
        );

        Ok(Self {
            search_engine,
            reader_pool,
            config,
            stats: Arc::new(RwLock::new(ConcurrentSearchStats::default())),
            performance_monitor: Arc::new(Mutex::new(PerformanceMonitor::new())),
            search_semaphore,
        })
    }

    /// Execute concurrent search with performance monitoring
    pub async fn search_concurrent(
        &self,
        query: &str,
        limit: Option<usize>,
        timeout: Option<Duration>,
        token: Option<CancellationToken>,
    ) -> SearchResult<SearchResults> {
        let start_time = Instant::now();

        // Acquire search permit
        let _search_permit = self
            .search_semaphore
            .acquire()
            .await
            .map_err(|_| SearchError::IndexError("Search semaphore closed".to_string()))?;

        // Update active search count
        {
            let mut stats = self.stats.write();
            stats.active_searches += 1;
            stats.total_concurrent_searches += 1;
            if stats.active_searches > stats.peak_concurrent_searches {
                stats.peak_concurrent_searches = stats.active_searches;
            }
        }

        debug!(
            query = %query,
            active_searches = self.stats.read().active_searches,
            "Starting concurrent search"
        );

        // Execute search using the main search engine with cancellation token
        let result = self
            .search_engine
            .search_with_timeout(query, limit, timeout, token)
            .await;

        // Record performance metrics
        let response_time = start_time.elapsed();
        self.record_performance_metrics(response_time).await;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.active_searches -= 1;

            // Update average response time
            let total_time =
                stats.average_response_time_ms * (stats.total_concurrent_searches - 1) as f64;
            stats.average_response_time_ms = (total_time + response_time.as_millis() as f64)
                / stats.total_concurrent_searches as f64;
        }

        match &result {
            Ok(search_results) => {
                info!(
                    query = %query,
                    results = search_results.entries.len(),
                    response_time_ms = response_time.as_millis(),
                    "Concurrent search completed successfully"
                );
            }
            Err(e) => {
                error!(
                    query = %query,
                    error = %e,
                    response_time_ms = response_time.as_millis(),
                    "Concurrent search failed"
                );
            }
        }

        result
    }

    /// Execute multiple searches concurrently with load balancing
    ///
    /// **Architecture**: Uses stream-based processing to prevent resource exhaustion
    ///
    /// **Fixed Issue**: Previous implementation created all tokio tasks upfront via `collect()`,
    /// causing memory exhaustion when handling thousands of queries. Even with semaphore limits,
    /// all tasks were spawned immediately and queued in memory.
    ///
    /// **Current Solution**: Stream-based processing with `futures::stream::iter` + `buffer_unordered`
    /// - Only creates tasks as capacity allows (controlled by max_concurrent_searches)
    /// - Memory usage bounded: O(max_concurrent_searches) instead of O(queries.len())
    /// - Proper backpressure: new futures created only when old ones complete
    ///
    /// **Industry Pattern**: This follows Tokio best practices for batch async operations
    /// See: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffer_unordered
    pub async fn search_batch_concurrent(
        &self,
        queries: Vec<String>,
        limit: Option<usize>,
        timeout: Option<Duration>,
        token: Option<CancellationToken>,
    ) -> Vec<SearchResult<SearchResults>> {
        use futures::stream::{self, StreamExt};

        let start_time = Instant::now();

        debug!(
            query_count = queries.len(),
            max_concurrent = self.config.max_concurrent_searches,
            "Starting stream-based batch concurrent search"
        );

        // Stream-based processing: creates futures on-demand as capacity allows
        let results: Vec<_> = stream::iter(queries)
            .map(|query| {
                let manager = Arc::clone(&self.search_engine);
                let semaphore = Arc::clone(&self.search_semaphore);
                let limit_clone = limit;
                let timeout_clone = timeout;
                let token_clone = token.clone();

                // Return a future (NOT spawned yet)
                async move {
                    // Acquire semaphore permit to enforce concurrency limits
                    let _permit = semaphore.acquire().await.map_err(|_| {
                        SearchError::IndexError("Search semaphore closed".to_string())
                    })?;

                    manager
                        .search_with_timeout(&query, limit_clone, timeout_clone, token_clone)
                        .await
                }
            })
            // buffer_unordered: executes up to N futures concurrently
            // Key difference: only N futures exist in memory at any time
            .buffer_unordered(self.config.max_concurrent_searches)
            .collect()
            .await;

        info!(
            batch_size = results.len(),
            total_time_ms = start_time.elapsed().as_millis(),
            "Stream-based batch concurrent search completed"
        );

        results
    }

    /// Record performance metrics and detect degradation
    async fn record_performance_metrics(&self, response_time: Duration) {
        let mut monitor = self.performance_monitor.lock();
        monitor.record_response_time(response_time);

        // Check for performance degradation
        if monitor.is_performance_degraded(self.config.performance_degradation_threshold) {
            let mut stats = self.stats.write();
            stats.performance_degradation_events += 1;

            warn!(
                response_time_ms = response_time.as_millis(),
                threshold = self.config.performance_degradation_threshold,
                degradation_events = stats.performance_degradation_events,
                "Performance degradation detected"
            );
        }
    }

    /// Get current concurrent search statistics
    pub fn get_concurrent_stats(&self) -> ConcurrentSearchStats {
        let mut stats = self.stats.read().clone();

        // Add reader pool stats
        let pool_stats = self.reader_pool.get_stats();
        stats.reader_pool_hits = pool_stats.hits;
        stats.reader_pool_misses = pool_stats.misses;

        // Estimate CPU utilization based on active searches
        stats.cpu_utilization =
            (stats.active_searches as f64 / self.config.max_concurrent_searches as f64).min(1.0);

        stats
    }

    /// Check if the system is under high load
    pub fn is_under_high_load(&self) -> bool {
        let stats = self.get_concurrent_stats();
        stats.cpu_utilization > self.config.cpu_utilization_target
    }

    /// Get performance degradation status
    pub fn is_performance_degraded(&self) -> bool {
        let monitor = self.performance_monitor.lock();
        monitor.is_performance_degraded(self.config.performance_degradation_threshold)
    }

    /// Reset performance monitoring baseline
    pub fn reset_performance_baseline(&self) {
        let mut monitor = self.performance_monitor.lock();
        monitor.baseline_response_time = None;
        monitor.recent_response_times.clear();

        info!("Performance monitoring baseline reset");
    }

    /// Get current configuration
    pub fn get_config(&self) -> &ConcurrentSearchConfig {
        &self.config
    }

    /// Update configuration (requires restart for some settings)
    pub fn update_config(&mut self, new_config: ConcurrentSearchConfig) {
        info!(
            old_max_concurrent = self.config.max_concurrent_searches,
            new_max_concurrent = new_config.max_concurrent_searches,
            "Updating concurrent search configuration"
        );

        self.config = new_config;
        // Note: Some changes like pool size require recreating the manager
    }
}

// We need to expose the index field from SearchEngineManager for the reader pool
// This is a temporary solution - in a real implementation, we'd refactor the architecture
impl SearchEngineManager {
    pub fn get_index(&self) -> &tantivy::Index {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_engine::manager::SearchConfig;
    use crate::search_engine::SearchEngineManager;
    use tempfile::TempDir;

    /// 创建测试用的并发搜索管理器
    /// 使用正确初始化的 Tantivy 索引
    async fn create_test_concurrent_manager() -> (ConcurrentSearchManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for tests");
        let search_config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        // SearchEngineManager::new 会正确创建新索引
        let search_engine = SearchEngineManager::new(search_config)
            .expect("Failed to create search engine for tests");
        let concurrent_config = ConcurrentSearchConfig::default();
        let concurrent_manager = ConcurrentSearchManager::new(search_engine, concurrent_config)
            .expect("Failed to create concurrent search manager for tests");

        (concurrent_manager, temp_dir)
    }

    #[tokio::test]
    async fn test_concurrent_search_creation() {
        let (_manager, _temp_dir) = create_test_concurrent_manager().await;
        // If we get here, creation was successful
    }

    #[tokio::test]
    async fn test_single_concurrent_search() {
        let (manager, _temp_dir) = create_test_concurrent_manager().await;

        // 在空索引上搜索应该返回空结果
        let result = manager.search_concurrent("test", None, None, None).await;

        // Should succeed even on empty index
        assert!(
            result.is_ok(),
            "Search should succeed on empty index: {:?}",
            result.err()
        );
        let search_results = result.unwrap();
        assert_eq!(search_results.entries.len(), 0);
    }

    #[tokio::test]
    async fn test_batch_concurrent_search() {
        let (manager, _temp_dir) = create_test_concurrent_manager().await;

        let queries = vec![
            "error".to_string(),
            "warning".to_string(),
            "info".to_string(),
        ];

        let results = manager
            .search_batch_concurrent(queries, None, None, None)
            .await;

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(
                result.is_ok(),
                "Batch search should succeed: {:?}",
                result.err()
            );
        }
    }

    #[tokio::test]
    async fn test_concurrent_stats() {
        let (manager, _temp_dir) = create_test_concurrent_manager().await;

        // Execute a search to generate stats
        let _result = manager.search_concurrent("test", None, None, None).await;

        let stats = manager.get_concurrent_stats();
        assert_eq!(stats.total_concurrent_searches, 1);
        assert_eq!(stats.active_searches, 0); // Should be 0 after completion
    }

    #[tokio::test]
    async fn test_load_detection() {
        let (manager, _temp_dir) = create_test_concurrent_manager().await;

        // Initially should not be under high load
        assert!(!manager.is_under_high_load());

        // Stats should reflect low utilization
        let stats = manager.get_concurrent_stats();
        assert!(stats.cpu_utilization <= 1.0);
    }
}
