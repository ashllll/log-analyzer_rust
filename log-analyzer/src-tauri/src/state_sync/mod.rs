//! Real-Time State Synchronization using Tauri Events
//!
//! P7 简化：移除未读 state_cache 和 event_history（零调用者）。
//! 降级为纯事件发射器——前端自行管理状态。

use tauri::{AppHandle, Emitter};

pub mod models;

#[cfg(test)]
mod property_tests;

pub use models::{WorkspaceEvent, WorkspaceState, WorkspaceStatus};

/// State synchronization — 纯事件发射器。
///
/// P7: 移除了内部缓存和历史队列（get_workspace_state / get_event_history 零调用者）。
/// 前端通过 Tauri 事件自行管理状态，后端不再维护冗余缓存。
#[derive(Clone)]
pub struct StateSync {
    app_handle: AppHandle,
}

impl StateSync {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// 广播工作区事件到前端，带重试（最多 3 次）。
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> Result<(), String> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 10;
        let mut last_error: Option<String> = None;

        for attempt in 0..MAX_RETRIES {
            match self.app_handle.emit("workspace-event", &event) {
                Ok(()) => {
                    tracing::debug!(
                        event_type = ?event,
                        attempt = attempt + 1,
                        "Broadcasted workspace event"
                    );
                    return Ok(());
                }
                Err(e) => {
                    let msg = format!("Failed to emit event: {e}");
                    last_error = Some(msg.clone());
                    if attempt + 1 < MAX_RETRIES {
                        tracing::warn!(
                            error = %e,
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            "Emit failed, retrying..."
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        let error_msg = last_error.unwrap_or_else(|| "Unknown emit failure".to_string());
        tracing::error!(
            error = %error_msg,
            event_type = ?event,
            max_retries = MAX_RETRIES,
            "Failed to emit workspace event after all retries"
        );
        Err(error_msg)
    }
}
