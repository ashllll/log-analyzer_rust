//! State synchronization commands

use tauri::{command, AppHandle, State};

use crate::models::AppState;
use crate::state_sync::{StateSync, WorkspaceEvent};

/// Initialize state synchronization (called once on app startup)
#[command]
pub async fn init_state_sync(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut sync_guard = state.state_sync.lock();

    if sync_guard.is_none() {
        let state_sync = StateSync::new(app);
        *sync_guard = Some(state_sync);
        tracing::info!("State synchronization initialized");
    }

    Ok(())
}

/// Get workspace state
#[command]
#[allow(clippy::await_holding_lock)]
pub async fn get_workspace_state(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::state_sync::WorkspaceState>, String> {
    let state_sync = {
        let sync_guard = state.state_sync.lock();
        if let Some(state_sync) = sync_guard.as_ref() {
            state_sync.clone()
        } else {
            return Err("State synchronization not initialized".to_string());
        }
    };

    Ok(state_sync.get_workspace_state(&workspaceId).await)
}

/// Get event history for a workspace
#[command]
#[allow(clippy::await_holding_lock)]
pub async fn get_event_history(
    #[allow(non_snake_case)] workspaceId: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceEvent>, String> {
    let state_sync = {
        let sync_guard = state.state_sync.lock();
        if let Some(state_sync) = sync_guard.as_ref() {
            state_sync.clone()
        } else {
            return Err("State synchronization not initialized".to_string());
        }
    };

    let limit = limit.unwrap_or(100);
    Ok(state_sync.get_event_history(&workspaceId, limit).await)
}

/// Broadcast a test event (for debugging)
#[command]
#[allow(clippy::await_holding_lock)]
pub async fn broadcast_test_event(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_sync = {
        let sync_guard = state.state_sync.lock();
        if let Some(state_sync) = sync_guard.as_ref() {
            state_sync.clone()
        } else {
            return Err("State synchronization not initialized".to_string());
        }
    };

    let event = WorkspaceEvent::ProgressUpdate {
        workspace_id: workspaceId,
        progress: 0.5,
    };

    state_sync.broadcast_workspace_event(event).await?;
    Ok(())
}
