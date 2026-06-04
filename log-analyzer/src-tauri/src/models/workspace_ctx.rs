//! 工作区上下文 — 管理已注册的工作区服务
//!
//! P5 迁移后：watchers 状态已移入每个 WorkspaceServiceImpl 实例内部，
//! 不再在此处维护全局 HashMap。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::workspace_service::WorkspaceServiceRef;

/// 工作区上下文，持有所有已导入工作区的运行时状态。
///
/// - `services`: 预组装的 WorkspaceServiceImpl 实例，一个工作区一个。
///
/// 所有字段通过 Mutex 保护，遵循 "lock → clone → unlock → await" 模式。
pub struct WorkspaceContext {
    services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,
}

impl Default for WorkspaceContext {
    fn default() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl WorkspaceContext {
    // ── Service CRUD ──

    /// 获取已注册的工作区服务。
    pub fn get_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef> {
        self.services.lock().get(workspace_id).cloned()
    }

    /// 注册工作区服务。
    ///
    /// 由导入命令在导入完成后调用。
    pub fn set_service(&self, workspace_id: String, service: WorkspaceServiceRef) {
        self.services.lock().insert(workspace_id, service);
    }

    /// 移除工作区服务。
    ///
    /// 注意：watcher 清理现在由调用方在删除前通过 `service.stop_watch()` 完成。
    pub fn remove_service(&self, workspace_id: &str) {
        self.services.lock().remove(workspace_id);
    }

    /// 获取所有工作区服务的快照（用于退出时批量清理）。
    pub fn all_services(&self) -> Vec<WorkspaceServiceRef> {
        self.services.lock().values().cloned().collect()
    }

    /// 获取所有已注册的工作区 ID。
    pub fn workspace_ids(&self) -> Vec<String> {
        self.services.lock().keys().cloned().collect()
    }
}
