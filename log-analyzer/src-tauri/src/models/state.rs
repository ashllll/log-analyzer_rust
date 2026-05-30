//! 应用状态管理 - 使用 parking_lot::Mutex 实现高效同步锁

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::services::file_watcher::WatcherState;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::AsyncResourceError;
use crate::utils::async_resource_manager::AsyncResourceManager;
use la_search::DiskResultStore;
use parking_lot::Mutex;
use std::collections::HashMap;
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
    // ── 工作区服务（预组装服务） ──
    pub workspace_services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,

    /// 文件监听器状态（运行时状态，不属于预组装服务）
    pub watchers: Arc<Mutex<HashMap<String, WatcherState>>>,

    // ── 搜索上下文 ──
    pub search_cancellation_tokens:
        Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    /// 搜索指标（合并 total_searches + cache_hits + last_search_duration）
    pub search_metrics: Arc<Mutex<SearchMetrics>>,
    /// M4 Fix: Wrapped in RwLock for app_data_dir path replacement during setup()
    /// FIX(CR-06): Option to avoid panic in default() when DiskResultStore::new fails
    pub disk_result_store: parking_lot::RwLock<Option<Arc<DiskResultStore>>>,
    /// FIX(HI-01): 缓存 rayon ThreadPool，避免每次搜索都新建线程池
    pub search_thread_pool: Arc<rayon::ThreadPool>,

    // ── 任务与同步上下文 ──
    pub task_manager: Arc<Mutex<Option<TaskManager>>>,
    pub state_sync: Arc<Mutex<Option<StateSync>>>,
    pub async_resource_manager: Arc<AsyncResourceManager>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace_services: Arc::new(Mutex::new(HashMap::new())),
            watchers: Arc::new(Mutex::new(HashMap::new())),
            task_manager: Arc::new(Mutex::new(None)),
            search_cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            // 合并搜索指标为单个结构体，减少锁竞争
            search_metrics: Arc::new(Mutex::new(SearchMetrics::default())),
            state_sync: Arc::new(Mutex::new(None)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
            // FIX(CR-06): Initialize as None to avoid IO/panic in default().
            // The actual DiskResultStore is created in setup() via init_disk_result_store_at().
            disk_result_store: parking_lot::RwLock::new(None),
            // FIX(HI-01): 在应用启动时初始化 ThreadPool，后续搜索复用
            search_thread_pool: Arc::new({
                let thread_count = num_cpus::get().max(2);
                rayon::ThreadPoolBuilder::new()
                    .num_threads(thread_count)
                    .build()
                    .expect("Failed to create search thread pool")
            }),
        }
    }
}

impl AppState {
    /// M4 Fix: Replace the default temp_dir DiskResultStore with one rooted at
    /// a persistent application data directory. Call this from setup() once the
    /// app_data_dir is available.
    pub fn init_disk_result_store_at(&self, base_path: std::path::PathBuf) {
        let cache_dir = base_path.join("search-cache");
        match DiskResultStore::new(cache_dir.clone(), 50) {
            Ok(store) => {
                let mut guard = self.disk_result_store.write();
                *guard = Some(Arc::new(store));
                tracing::info!(
                    path = %cache_dir.display(),
                    "DiskResultStore initialized at app data directory"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %cache_dir.display(),
                    "Failed to create DiskResultStore at app data dir; keeping fallback"
                );
            }
        }
    }

    // ── WorkspaceService 管理 ──

    /// 获取已注册的工作区服务。
    pub fn get_workspace_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef> {
        self.workspace_services.lock().get(workspace_id).cloned()
    }

    /// 注册工作区服务。
    ///
    /// 由导入命令在导入完成后调用，将预组装的 WorkspaceServiceImpl 存入。
    pub fn set_workspace_service(
        &self,
        workspace_id: String,
        service: WorkspaceServiceRef,
    ) {
        self.workspace_services.lock().insert(workspace_id, service);
    }

    /// 移除工作区服务。
    ///
    /// 由删除工作区命令调用，清理所有相关资源。
    pub fn remove_workspace_service(&self, workspace_id: &str) {
        self.workspace_services.lock().remove(workspace_id);
        self.watchers.lock().remove(workspace_id);
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
