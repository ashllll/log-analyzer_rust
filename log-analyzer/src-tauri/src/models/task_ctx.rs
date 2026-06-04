//! 任务上下文 — 管理异步任务和资源生命周期

use std::sync::Arc;

use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::{AsyncResourceError, AsyncResourceManager, OperationType};

/// 任务上下文，持有任务调度器和异步资源管理器。
///
/// - `task_manager`: 任务调度器。Default 时为 None，setup() 中初始化。
/// - `async_resource_manager`: 异步资源管理器，管理搜索/导入/监听等操作的注册和取消。
///
/// 注意：async_resource_manager 的方法都是 async fn，调用时需要 .await。
/// 遵循 "lock → clone → unlock → await" 模式。
pub struct TaskContext {
    task_manager: Arc<Mutex<Option<TaskManager>>>,
    async_resource_manager: Arc<AsyncResourceManager>,
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            task_manager: Arc::new(Mutex::new(None)),
            async_resource_manager: Arc::new(AsyncResourceManager::new()),
        }
    }
}

impl TaskContext {
    // ── TaskManager ──

    /// 初始化 TaskManager。
    ///
    /// 由 setup() 调用，在应用启动时创建并注入。
    pub fn init_task_manager(&self, task_manager: TaskManager) {
        *self.task_manager.lock() = Some(task_manager);
    }

    /// 取出 TaskManager 所有权（用于退出时 shutdown）。
    ///
    /// 调用后 task_manager 变为 None。
    pub fn take_task_manager(&self) -> Option<TaskManager> {
        self.task_manager.lock().take()
    }

    /// 获取 TaskManager 的克隆（不清空）。
    ///
    /// 用于导入命令等需要获取 TaskManager 引用但不移除它的场景。
    pub fn get_task_manager_clone(&self) -> Option<TaskManager> {
        self.task_manager.lock().clone()
    }

    // ── AsyncResourceManager 委托 ──

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
