//! 应用状态 — 扁平化的运行时状态持有者
//!
//! 将原先分散在 4 个 Context 文件中的字段和方法直接内联到 AppState，
//! 消除薄封装层（Context 只是 Arc<Mutex<T>> 的 getter/setter）。
//!
//! P8: 折叠 WorkspaceContext / SearchContext / TaskContext / SyncContext → AppState。
//! 省去 22 个委托方法和 4 个 Context 文件，AppState 对外 API 不变。
//!
//! # 锁策略
//!
//! 所有字段使用 `parking_lot::Mutex` / `RwLock`：
//! 1. 高性能：parking_lot 比 std::sync::Mutex 更快，无 poison 状态
//! 2. 简洁 API：使用 `.lock()` 获取锁，无需处理 unwrap
//! 3. 不跨 await：锁不跨 `.await` 点持有，先克隆数据再释放锁
//! 4. 遵循 "lock → clone → unlock → await" 模式

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::infrastructure::TaskManagerAdapter;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::{AsyncResourceError, AsyncResourceManager, OperationType};
use la_core::domain::TaskScheduler;
use la_search::DiskResultStore;

// ============================================================================
// AppState
// ============================================================================

pub struct AppState {
    // ── Workspace ──
    services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,

    // ── Search ──
    disk_result_store: RwLock<Option<Arc<DiskResultStore>>>,
    thread_pool: Arc<rayon::ThreadPool>,

    // ── Task ──
    task_manager: Arc<Mutex<Option<TaskManager>>>,
    async_resource_manager: Arc<AsyncResourceManager>,

    // ── Sync ──
    state_sync: Arc<Mutex<Option<StateSync>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            // Workspace
            services: Arc::new(Mutex::new(HashMap::new())),
            // Search
            disk_result_store: RwLock::new(None),
            thread_pool: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(num_cpus::get().max(2))
                    .build()
                    .expect("Failed to create search thread pool"),
            ),
            // Task
            task_manager: Arc::new(Mutex::new(None)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
            // Sync
            state_sync: Arc::new(Mutex::new(None)),
        }
    }
}

// ============================================================================
// Workspace
// ============================================================================

impl AppState {
    /// 获取已注册的工作区服务。
    pub fn get_workspace_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef> {
        self.services.lock().get(workspace_id).cloned()
    }

    /// 注册工作区服务。
    pub fn set_workspace_service(&self, workspace_id: String, service: WorkspaceServiceRef) {
        self.services.lock().insert(workspace_id, service);
    }

    /// 移除工作区服务（watcher 清理由调用方在删除前通过 service.stop_watch() 完成）。
    pub fn remove_workspace_service(&self, workspace_id: &str) {
        self.services.lock().remove(workspace_id);
    }

    /// 获取所有工作区服务的快照（用于退出时批量清理）。
    pub fn all_workspace_services(&self) -> Vec<WorkspaceServiceRef> {
        self.services.lock().values().cloned().collect()
    }

    /// 获取所有已注册的工作区 ID。
    pub fn workspace_ids(&self) -> Vec<String> {
        self.services.lock().keys().cloned().collect()
    }
}

// ============================================================================
// Search
// ============================================================================

impl AppState {
    /// 初始化 DiskResultStore 到指定的持久化目录。
    pub fn init_disk_result_store_at(&self, base_path: PathBuf) {
        let cache_dir = base_path.join("search-cache");
        match DiskResultStore::new(cache_dir.clone(), 50) {
            Ok(store) => {
                *self.disk_result_store.write() = Some(Arc::new(store));
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

    /// 获取 DiskResultStore（如果已初始化）。
    pub fn get_disk_result_store(&self) -> Option<Arc<DiskResultStore>> {
        self.disk_result_store.read().clone()
    }

    /// 获取共享的 Rayon 搜索线程池引用。
    pub fn get_search_thread_pool(&self) -> Arc<rayon::ThreadPool> {
        Arc::clone(&self.thread_pool)
    }

    /// 退出时清理所有搜索结果的磁盘缓存。
    pub fn cleanup_disk_result_store(&self) {
        if let Some(disk_store) = self.disk_result_store.read().as_ref() {
            disk_store.cleanup_all();
        }
    }
}

// ============================================================================
// Task
// ============================================================================

impl AppState {
    /// 初始化 TaskManager（由 setup() 在应用启动时调用）。
    pub fn init_task_manager(&self, task_manager: TaskManager) {
        *self.task_manager.lock() = Some(task_manager);
    }

    /// 取出 TaskManager 所有权（用于退出时 shutdown）。
    pub fn take_task_manager(&self) -> Option<TaskManager> {
        self.task_manager.lock().take()
    }

    /// 获取 TaskManager 的克隆（不清空，用于导入命令等需要引用但不移除的场景）。
    pub fn get_task_manager_clone(&self) -> Option<TaskManager> {
        self.task_manager.lock().clone()
    }

    /// P11: 获取 TaskScheduler trait 对象（推荐方式）。
    ///
    /// 包装 TaskManager 为 TaskManagerAdapter，通过 domain trait 暴露。
    /// 命令层应优先使用此方法而非直接访问 TaskManager。
    pub fn get_task_scheduler(&self) -> Option<Arc<dyn TaskScheduler>> {
        self.task_manager.lock().as_ref().map(|tm| {
            Arc::new(TaskManagerAdapter::new(Arc::new(tm.clone()))) as Arc<dyn TaskScheduler>
        })
    }

    /// 注册一个异步操作。
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

    /// 取消指定操作。
    pub async fn cancel_async_operation(&self, operation_id: &str) -> Result<(), String> {
        self.async_resource_manager
            .cancel_operation(operation_id)
            .await
            .map_err(|e: AsyncResourceError| e.to_string())
    }

    /// 获取活跃操作数量。
    pub async fn get_active_operations_count(&self) -> usize {
        self.async_resource_manager.active_operations_count().await
    }
}

// ============================================================================
// Sync
// ============================================================================

impl AppState {
    /// 初始化状态同步实例（由前端调用 init_state_sync 命令时触发）。
    pub fn init_state_sync(&self, sync: StateSync) {
        *self.state_sync.lock() = Some(sync);
    }

    /// 获取 StateSync 的克隆。
    pub fn get_state_sync(&self) -> Option<StateSync> {
        self.state_sync.lock().clone()
    }

    /// 获取 state_sync Mutex Arc 引用（供 workspace 命令的 sync 更新逻辑使用）。
    pub fn state_sync_arc(&self) -> Arc<Mutex<Option<StateSync>>> {
        Arc::clone(&self.state_sync)
    }
}
