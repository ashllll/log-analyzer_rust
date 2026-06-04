//! 同步上下文 — 管理前后端状态同步

use std::sync::Arc;

use parking_lot::Mutex;

use crate::state_sync::StateSync;

/// 同步上下文，持有状态同步实例。
///
/// Default 时为 None，由前端调用 `init_state_sync` 命令时初始化。
pub struct SyncContext {
    state_sync: Arc<Mutex<Option<StateSync>>>,
}

impl Default for SyncContext {
    fn default() -> Self {
        Self {
            state_sync: Arc::new(Mutex::new(None)),
        }
    }
}

impl SyncContext {
    /// 初始化状态同步实例。
    pub fn init(&self, sync: StateSync) {
        *self.state_sync.lock() = Some(sync);
    }

    /// 获取 StateSync 的克隆。
    pub fn get(&self) -> Option<StateSync> {
        self.state_sync.lock().clone()
    }

    /// 获取 state_sync Mutex Arc 引用。
    ///
    /// 供需要将 state_sync 传入命令的场景使用（如 workspace 命令的 sync 更新逻辑）。
    pub fn state_sync_arc(&self) -> Arc<Mutex<Option<StateSync>>> {
        Arc::clone(&self.state_sync)
    }
}
