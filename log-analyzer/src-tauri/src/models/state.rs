//! 应用状态管理 - 使用 parking_lot::Mutex 实现高效同步锁

use crate::services::file_watcher::WatcherState;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::AsyncResourceError;
use crate::utils::async_resource_manager::AsyncResourceManager;
use la_search::DiskResultStore;
use la_search::SearchEngineManager;
use la_storage::ContentAddressableStorage;
use la_storage::MetadataStore;
use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// 搜索指标 - 合并相关计数器以减少锁竞争
#[derive(Debug, Default)]
pub struct SearchMetrics {
    pub total_searches: u64,
    pub cache_hits: u64,
    pub last_search_duration: std::time::Duration,
}

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
///
/// # 搜索指标优化
/// 将 total_searches、cache_hits、last_search_duration 合并为 SearchMetrics 结构体，
/// 使用单个 Mutex 保护，减少锁竞争和缓存行伪共享。
pub struct AppState {
    /// 工作区目录映射
    pub workspace_dirs: Arc<Mutex<BTreeMap<String, std::path::PathBuf>>>,
    pub cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,
    pub metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,
    pub task_manager: Arc<Mutex<Option<TaskManager>>>,
    pub search_cancellation_tokens:
        Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    /// 搜索指标（合并 total_searches + cache_hits + last_search_duration）
    pub search_metrics: Arc<Mutex<SearchMetrics>>,
    pub watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    pub state_sync: Arc<Mutex<Option<StateSync>>>,
    pub async_resource_manager: Arc<AsyncResourceManager>,
    pub search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,
    /// M4 Fix: Wrapped in RwLock to allow replacement with app_data_dir path
    /// during setup(). Read-heavy access pattern — read lock is shared.
    pub disk_result_store: parking_lot::RwLock<Arc<DiskResultStore>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            // C-H1 优化: BTreeMap 保证迭代顺序确定性
            workspace_dirs: Arc::new(Mutex::new(BTreeMap::new())),
            cas_instances: Arc::new(Mutex::new(HashMap::new())),
            metadata_stores: Arc::new(Mutex::new(HashMap::new())),
            task_manager: Arc::new(Mutex::new(None)),
            search_cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            // 合并搜索指标为单个结构体，减少锁竞争
            search_metrics: Arc::new(Mutex::new(SearchMetrics::default())),
            watchers: Arc::new(Mutex::new(HashMap::new())),
            state_sync: Arc::new(Mutex::new(None)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
            search_engine_managers: Arc::new(Mutex::new(HashMap::new())),
            disk_result_store: parking_lot::RwLock::new(Arc::new({
                // M4 Fix: Default uses temp_dir as fallback; setup() stage replaces
                // with app-specific data directory via init_disk_result_store_at()
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
            })),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// M4 Fix: Replace the default temp_dir DiskResultStore with one rooted at
    /// a persistent application data directory. Call this from setup() once the
    /// app_data_dir is available.
    pub fn init_disk_result_store_at(&self, base_path: std::path::PathBuf) {
        let cache_dir = base_path.join("search-cache");
        match DiskResultStore::new(cache_dir.clone(), 50) {
            Ok(store) => {
                let mut guard = self.disk_result_store.write();
                *guard = Arc::new(store);
                tracing::info!(
                    path = %cache_dir.display(),
                    "DiskResultStore initialized at app data directory"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %cache_dir.display(),
                    "Failed to create DiskResultStore at app data dir; keeping temp_dir fallback"
                );
            }
        }
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
