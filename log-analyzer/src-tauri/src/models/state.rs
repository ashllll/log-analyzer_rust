//! 应用状态 — 4 Context 的 Facade
//!
//! 领域拆分后的顶层状态持有者，命令层通过 `State<'_, AppState>` 访问。
//! 所有方法委托给对应的 Context，调用方不应直接访问 pub 字段。
//!
//! # Context 映射
//!
//! | Context | 职责 |
//! |---------|------|
//! | `workspaces` | 工作区服务（预组装） + 文件监听器状态 |
//! | `search` | 搜索取消令牌 + DiskResultStore + Rayon 线程池 |
//! | `tasks` | TaskManager + AsyncResourceManager |
//! | `sync` | 前后端 StateSync |
//!
//! # 锁策略
//!
//! 所有字段使用 `parking_lot::Mutex` / `RwLock`：
//!
//! 1. 高性能：parking_lot 比 std::sync::Mutex 更快，无 poison 状态
//! 2. 简洁 API：使用 `.lock()` 获取锁，无需处理 unwrap
//! 3. 不跨 await：锁不跨 `.await` 点持有，先克隆数据再释放锁
//! 4. 遵循 "lock → clone → unlock → await" 模式

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::models::search_ctx::SearchContext;
use crate::models::sync_ctx::SyncContext;
use crate::models::task_ctx::TaskContext;
use crate::models::workspace_ctx::WorkspaceContext;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use crate::utils::async_resource_manager::OperationType;
use la_search::DiskResultStore;

pub struct AppState {
    pub workspaces: WorkspaceContext,
    pub search: SearchContext,
    pub tasks: TaskContext,
    pub sync: SyncContext,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspaces: WorkspaceContext::default(),
            search: SearchContext::default(),
            tasks: TaskContext::default(),
            sync: SyncContext::default(),
        }
    }
}

// ============================================================================
// Facade — Workspace
// ============================================================================

impl AppState {
    pub fn get_workspace_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef> {
        self.workspaces.get_service(workspace_id)
    }

    pub fn set_workspace_service(&self, workspace_id: String, service: WorkspaceServiceRef) {
        self.workspaces.set_service(workspace_id, service);
    }

    pub fn remove_workspace_service(&self, workspace_id: &str) {
        self.workspaces.remove_service(workspace_id);
    }

    pub fn all_workspace_services(&self) -> Vec<WorkspaceServiceRef> {
        self.workspaces.all_services()
    }
}

// ============================================================================
// Facade — Search
// ============================================================================

impl AppState {
    pub fn init_disk_result_store_at(&self, base_path: PathBuf) {
        self.search.init_disk_result_store_at(base_path);
    }

    pub fn get_disk_result_store(&self) -> Option<Arc<DiskResultStore>> {
        self.search.get_disk_result_store()
    }

    pub fn get_search_thread_pool(&self) -> Arc<rayon::ThreadPool> {
        self.search.get_thread_pool()
    }

    pub fn get_search_cancellation_token(&self, search_id: &str) -> Option<CancellationToken> {
        self.search.get_cancellation_token(search_id)
    }

    pub fn insert_search_cancellation_token(&self, search_id: String, token: CancellationToken) {
        self.search.insert_cancellation_token(search_id, token);
    }

    pub fn remove_search_cancellation_token(
        &self,
        search_id: &str,
    ) -> Option<CancellationToken> {
        self.search.remove_cancellation_token(search_id)
    }

    /// 暴露 cancellation_tokens Arc 引用（供 search_logs 命令在闭包中使用）。
    pub fn cancellation_tokens_arc(
        &self,
    ) -> Arc<Mutex<HashMap<String, CancellationToken>>> {
        self.search.cancellation_tokens_arc()
    }

    /// 退出时清理搜索结果磁盘缓存。
    pub fn cleanup_disk_result_store(&self) {
        self.search.cleanup_disk_result_store();
    }
}

// ============================================================================
// Facade — Task
// ============================================================================

impl AppState {
    pub fn init_task_manager(&self, task_manager: TaskManager) {
        self.tasks.init_task_manager(task_manager);
    }

    pub fn take_task_manager(&self) -> Option<TaskManager> {
        self.tasks.take_task_manager()
    }

    pub fn get_task_manager_clone(&self) -> Option<TaskManager> {
        self.tasks.get_task_manager_clone()
    }

    pub async fn register_async_operation(
        &self,
        operation_id: String,
        operation_type: OperationType,
        workspace_id: Option<String>,
    ) -> CancellationToken {
        self.tasks
            .register_async_operation(operation_id, operation_type, workspace_id)
            .await
    }

    pub async fn cancel_async_operation(&self, operation_id: &str) -> Result<(), String> {
        self.tasks.cancel_async_operation(operation_id).await
    }

    pub async fn get_active_operations_count(&self) -> usize {
        self.tasks.get_active_operations_count().await
    }
}

// ============================================================================
// Facade — Sync
// ============================================================================

impl AppState {
    pub fn init_state_sync(&self, sync: StateSync) {
        self.sync.init(sync);
    }

    pub fn get_state_sync(&self) -> Option<StateSync> {
        self.sync.get()
    }

    /// 暴露 state_sync Arc 引用（供 workspace 命令的同步逻辑使用）。
    pub fn state_sync_arc(&self) -> Arc<Mutex<Option<StateSync>>> {
        self.sync.state_sync_arc()
    }
}
