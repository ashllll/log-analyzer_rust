//! 应用状态管理 - 使用 parking_lot::Mutex 实现高效同步锁

use crate::services::file_watcher::WatcherState;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::AsyncResourceError;
use crate::utils::async_resource_manager::AsyncResourceManager;
use crate::utils::cache_manager::{CacheError, CacheManager};
use crate::utils::cleanup::CleanupQueue;
use crossbeam::queue::SegQueue;
use la_search::DiskResultStore;
use la_search::SearchEngineManager;
use la_search::VirtualSearchManager;
use la_storage::ContentAddressableStorage;
use la_storage::MetadataStore;
use moka::sync::Cache;
use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// 应用状态 - 使用 parking_lot::Mutex 实现高效同步锁
///
/// # 锁策略
///
/// 所有字段使用 `parking_lot::Mutex`：
///
/// 1. **高性能**：parking_lot::Mutex 比 std::sync::Mutex 更快，无 poison 状态
/// 2. **简洁API**：使用 `.lock()` 获取锁，无需处理 unwrap
/// 3. **不跨 await**：锁不跨 `.await` 点持有，先克隆数据再释放锁
///
/// # 注意事项
///
/// - 使用 `.lock()` 获取锁守卫
/// - 锁守卫不能跨 `.await` 传递，需要时先克隆数据
/// - 遵循 "lock → clone → unlock → await" 模式
pub struct AppState {
    /// 工作区目录映射
    pub workspace_dirs: Arc<Mutex<BTreeMap<String, std::path::PathBuf>>>,
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
    pub cache_manager: Arc<Mutex<CacheManager>>,
    pub state_sync: Arc<Mutex<Option<StateSync>>>,
    pub async_resource_manager: Arc<AsyncResourceManager>,
    pub search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,
    pub virtual_search_manager: Arc<VirtualSearchManager>,
    pub disk_result_store: Arc<DiskResultStore>,
}

impl Default for AppState {
    fn default() -> Self {
        // 创建同步缓存 (L1 cache)
        let sync_cache = Cache::builder().max_capacity(1000).build();

        // 使用 CacheManager 包装缓存
        let cache_manager = CacheManager::new(Arc::new(sync_cache));

        Self {
            // C-H1 优化: BTreeMap 保证迭代顺序确定性
            workspace_dirs: Arc::new(Mutex::new(BTreeMap::new())),
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
            virtual_search_manager: Arc::new(VirtualSearchManager::new(100)),
            disk_result_store: Arc::new({
                // 使用系统临时目录存储搜索结果缓存
                // main.rs setup 阶段可替换为应用专属目录
                let cache_dir = std::env::temp_dir().join("log-analyzer-search-cache");
                DiskResultStore::new(cache_dir, 50).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "无法创建搜索磁盘缓存，尝试降级路径");
                    // 降级：尝试另一个临时目录，仍失败时记录错误并 panic（输出清晰诊断信息）
                    DiskResultStore::new(std::env::temp_dir().join("la-sc-fallback"), 20)
                        .unwrap_or_else(|e2| {
                            tracing::error!(
                                primary_error = %e,
                                fallback_error = %e2,
                                tmp_dir = ?std::env::temp_dir(),
                                "所有搜索缓存目录均初始化失败，请检查临时目录权限或磁盘空间"
                            );
                            panic!(
                                "无法初始化搜索缓存（主路径: {}, 降级路径: {}）。\
                                 请确认临时目录可写且磁盘空间充足。",
                                e, e2
                            )
                        })
                })
            }),
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
    /// 注意：此方法直接使用 CacheManager 的同步方法获取统计信息，
    /// 避免在持锁期间调用 block_on，消除死锁风险
    pub fn get_async_cache_statistics(&self) -> CacheStatistics {
        // 缩小锁作用域：获取统计信息后立即释放锁
        let cache = self.cache_manager.lock();
        cache.get_cache_statistics()
    }

    /// 清理工作区缓存
    pub fn invalidate_workspace_cache(&self, workspace_id: &str) -> Result<usize, String> {
        let cache = self.cache_manager.lock();
        cache
            .invalidate_workspace_cache(workspace_id)
            .map_err(|e: CacheError| e.to_string())
    }

    /// 清理过期缓存条目
    pub fn cleanup_expired_entries(&self) -> Result<(), String> {
        let cache = self.cache_manager.lock();
        cache
            .cleanup_expired_entries()
            .map_err(|e: CacheError| e.to_string())
    }

    /// 清理异步缓存条目
    ///
    /// 注意：此方法先克隆 CacheManager，释放锁后再调用异步操作，避免死锁风险
    pub fn cleanup_expired_entries_async(&self) -> Result<(), String> {
        // 缩小锁作用域：克隆 CacheManager 后立即释放锁
        let cache = {
            let guard = self.cache_manager.lock();
            // CacheManager 内部使用 Arc，克隆是廉价的
            guard.clone()
        };
        let result = tauri::async_runtime::block_on(cache.cleanup_expired_entries_async());
        result.map_err(|e: CacheError| e.to_string())
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
    ///
    /// 注意：此方法先克隆 CacheManager，释放锁后再调用异步操作，避免死锁风险
    pub fn cache_health_check(&self) -> CacheHealthCheck {
        // 缩小锁作用域：克隆 CacheManager 后立即释放锁
        let cache = {
            let guard = self.cache_manager.lock();
            guard.clone()
        };
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
    ///
    /// 注意：此方法先克隆 CacheManager，释放锁后再调用异步操作，避免死锁风险
    pub fn intelligent_cache_eviction(
        &self,
        target_reduction_percent: f64,
    ) -> Result<usize, String> {
        // 缩小锁作用域：克隆 CacheManager 后立即释放锁
        let cache = {
            let guard = self.cache_manager.lock();
            guard.clone()
        };
        let result =
            tauri::async_runtime::block_on(cache.intelligent_eviction(target_reduction_percent));
        result.map_err(|e: CacheError| e.to_string())
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
            .map_err(|e: AsyncResourceError| e.to_string())
    }

    /// 获取活跃操作数量
    pub async fn get_active_operations_count(&self) -> usize {
        self.async_resource_manager.active_operations_count().await
    }
}
