//! 应用状态管理 - 简化版本

use crate::search_engine::manager::SearchEngineManager;
use crate::services::file_watcher::WatcherState;
use crate::state_sync::StateSync;
use crate::storage::ContentAddressableStorage;
use crate::storage::MetadataStore;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::AsyncResourceManager;
use crate::utils::cache_manager::CacheManager;
use crate::utils::cleanup::CleanupQueue;
use crossbeam::queue::SegQueue;
use moka::sync::Cache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// 简化应用状态
pub struct AppState {
    pub workspace_dirs: Arc<Mutex<HashMap<String, std::path::PathBuf>>>,
    pub cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,
    pub metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,
    pub task_manager: Arc<Mutex<Option<TaskManager>>>,
    pub search_cancellation_tokens:
        Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    pub total_searches: Arc<Mutex<u64>>,
    pub cache_hits: Arc<Mutex<u64>>,
    pub last_search_duration: Arc<Mutex<std::time::Duration>>,
    pub watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    pub cleanup_queue: Arc<CleanupQueue>,
    /// 使用 CacheManager 提供高级缓存功能
    pub cache_manager: Arc<Mutex<CacheManager>>,
    pub state_sync: Arc<Mutex<Option<StateSync>>>,
    /// 异步资源管理器，支持搜索取消和超时
    pub async_resource_manager: Arc<AsyncResourceManager>,
    /// 搜索引擎管理器映射 (每个工作区独立)
    /// 用于增量索引时持久化新日志条目到 Tantivy 索引
    pub search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,
}

impl Default for AppState {
    fn default() -> Self {
        // 创建同步缓存 (L1 cache)
        let sync_cache = Cache::builder().max_capacity(1000).build();

        // 使用 CacheManager 包装缓存
        let cache_manager = CacheManager::new(Arc::new(sync_cache));

        Self {
            workspace_dirs: Arc::new(Mutex::new(HashMap::new())),
            cas_instances: Arc::new(Mutex::new(HashMap::new())),
            metadata_stores: Arc::new(Mutex::new(HashMap::new())),
            task_manager: Arc::new(Mutex::new(None)),
            search_cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            total_searches: Arc::new(Mutex::new(0)),
            cache_hits: Arc::new(Mutex::new(0)),
            last_search_duration: Arc::new(Mutex::new(std::time::Duration::from_secs(0))),
            watchers: Arc::new(Mutex::new(HashMap::new())),
            cleanup_queue: Arc::new(SegQueue::new()),
            cache_manager: Arc::new(Mutex::new(cache_manager)),
            state_sync: Arc::new(Mutex::new(None)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
            search_engine_managers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_workspace_dir(&self, workspace_id: &str) -> Option<std::path::PathBuf> {
        let dirs = self.workspace_dirs.lock();
        dirs.get(workspace_id).cloned()
    }

    pub fn set_workspace_dir(&self, workspace_id: String, path: std::path::PathBuf) {
        let mut dirs = self.workspace_dirs.lock();
        dirs.insert(workspace_id, path);
    }
}

// ============================================================================
// CacheManager 访问方法 - 封装 mutex 锁，提供线程安全的访问接口
// ============================================================================

use crate::utils::cache_manager::{
    AccessPatternStats, CacheDashboardData, CacheHealthCheck, CacheMetricsSnapshot,
    CachePerformanceReport, CacheStatistics, CompressionStats, L2CacheConfig,
};

impl AppState {
    /// 获取缓存统计信息
    pub fn get_cache_statistics(&self) -> CacheStatistics {
        let cache = self.cache_manager.lock();
        cache.get_cache_statistics()
    }

    /// 获取异步缓存统计信息
    ///
    /// 内部调用 CacheManager 的异步方法（实际上执行的是同步操作）
    /// 由于方法声明为 async 且需要保持 API 兼容性，这里使用 block_on 包装
    #[allow(clippy::await_holding_lock)]
    pub async fn get_async_cache_statistics(&self) -> CacheStatistics {
        let cache = self.cache_manager.lock();
        // 方法是 async 但执行的是同步操作，直接调用
        tauri::async_runtime::block_on(cache.get_async_cache_statistics())
    }

    /// 清理工作区缓存
    pub fn invalidate_workspace_cache(&self, workspace_id: &str) -> Result<usize, String> {
        let cache = self.cache_manager.lock();
        cache
            .invalidate_workspace_cache(workspace_id)
            .map_err(|e| e.to_string())
    }

    /// 清理过期缓存条目
    pub fn cleanup_expired_entries(&self) -> Result<(), String> {
        let cache = self.cache_manager.lock();
        cache.cleanup_expired_entries().map_err(|e| e.to_string())
    }

    /// 清理异步缓存条目
    #[allow(clippy::await_holding_lock)]
    pub async fn cleanup_expired_entries_async(&self) -> Result<(), String> {
        let cache = self.cache_manager.lock();
        let result = tauri::async_runtime::block_on(cache.cleanup_expired_entries_async());
        result.map_err(|e| e.to_string())
    }

    /// 获取缓存性能指标
    pub fn get_cache_performance_metrics(&self) -> CacheMetricsSnapshot {
        let cache = self.cache_manager.lock();
        cache.get_performance_metrics()
    }

    /// 生成缓存性能报告
    pub fn get_cache_performance_report(&self) -> CachePerformanceReport {
        let cache = self.cache_manager.lock();
        cache.generate_performance_report()
    }

    /// 执行缓存健康检查
    #[allow(clippy::await_holding_lock)]
    pub async fn cache_health_check(&self) -> CacheHealthCheck {
        let cache = self.cache_manager.lock();
        tauri::async_runtime::block_on(cache.health_check())
    }

    /// 获取访问模式统计
    pub fn get_access_pattern_stats(&self) -> AccessPatternStats {
        let cache = self.cache_manager.lock();
        cache.get_access_pattern_stats()
    }

    /// 获取压缩统计信息
    pub fn get_compression_stats(&self) -> CompressionStats {
        let cache = self.cache_manager.lock();
        cache.get_compression_stats()
    }

    /// 获取 L2 缓存配置
    pub fn get_l2_cache_config(&self) -> L2CacheConfig {
        let cache = self.cache_manager.lock();
        cache.get_l2_config()
    }

    /// 智能缓存驱逐
    #[allow(clippy::await_holding_lock)]
    pub async fn intelligent_cache_eviction(
        &self,
        target_reduction_percent: f64,
    ) -> Result<usize, String> {
        let cache = self.cache_manager.lock();
        let result =
            tauri::async_runtime::block_on(cache.intelligent_eviction(target_reduction_percent));
        result.map_err(|e| e.to_string())
    }

    /// 重置缓存性能指标
    pub fn reset_cache_metrics(&self) {
        let cache = self.cache_manager.lock();
        cache.reset_metrics();
    }

    /// 重置访问模式追踪器
    pub fn reset_access_tracker(&self) {
        let cache = self.cache_manager.lock();
        cache.reset_access_tracker();
    }

    /// 获取缓存仪表板数据
    pub fn get_cache_dashboard_data(&self) -> CacheDashboardData {
        let cache = self.cache_manager.lock();
        cache.get_dashboard_data()
    }
}

// ============================================================================
// AsyncResourceManager 访问方法 - 提供异步资源管理功能
// ============================================================================

use crate::utils::async_resource_manager::OperationType;
use tokio_util::sync::CancellationToken;

impl AppState {
    /// 注册异步操作
    pub async fn register_async_operation(
        &self,
        operation_id: String,
        operation_type: OperationType,
        workspace_id: Option<String>,
    ) -> CancellationToken {
        self.async_resource_manager
            .register_operation(operation_id, operation_type, workspace_id)
            .await
    }

    /// 取消异步操作
    pub async fn cancel_async_operation(&self, operation_id: &str) -> Result<(), String> {
        self.async_resource_manager
            .cancel_operation(operation_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// 获取活跃操作数量
    pub async fn get_active_operations_count(&self) -> usize {
        self.async_resource_manager.active_operations_count().await
    }
}
