//! Real-Time State Synchronization using Tauri Events
//!
//! P7 简化：移除未读 state_cache 和 event_history（零调用者）。
//! 降级为纯事件发射器——前端自行管理状态。
//! P7-续：重试逻辑下沉至 TauriEventPublisher::emit_workspace_event_with_retry。

use crate::infrastructure::TauriEventPublisher;

pub mod models;

#[cfg(test)]
mod property_tests;

pub use models::{WorkspaceEvent, WorkspaceStatus};

/// State synchronization — 纯事件发射器。
///
/// 重试逻辑已下沉至 TauriEventPublisher 适配器。
/// StateSync 仅做薄层委托。
#[derive(Clone)]
pub struct StateSync {
    publisher: TauriEventPublisher,
}

impl StateSync {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            publisher: TauriEventPublisher { app_handle },
        }
    }

    /// 广播工作区事件到前端（委托给 TauriEventPublisher，含重试）。
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> Result<(), String> {
        self.publisher.emit_workspace_event_with_retry(&event).await
    }
}
