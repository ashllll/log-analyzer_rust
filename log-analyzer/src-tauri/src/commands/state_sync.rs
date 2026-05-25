//! State synchronization commands
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use la_core::error::CommandError;
use tauri::{AppHandle, State};

use crate::models::AppState;
use crate::state_sync::StateSync;

/// Initialize state synchronization (called once on app startup)
#[tauri::command]
pub async fn init_state_sync(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let mut sync_guard = state.state_sync.lock();

    if sync_guard.is_none() {
        let state_sync = StateSync::new(app);
        *sync_guard = Some(state_sync);
        tracing::info!("State synchronization initialized");
    }

    Ok(())
}
