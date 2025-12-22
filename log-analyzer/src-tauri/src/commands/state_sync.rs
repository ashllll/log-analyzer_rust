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
pub async fn get_workspace_state(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::state_sync::WorkspaceState>, String> {
    let sync_guard = state.state_sync.lock();

    if let Some(state_sync) = sync_guard.as_ref() {
        Ok(state_sync.get_workspace_state(&workspaceId).await)
    } else {
        Err("State synchronization not initialized".to_string())
    }
}

/// Get event history for a workspace
#[command]
pub async fn get_event_history(
    #[allow(non_snake_case)] workspaceId: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceEvent>, String> {
    let sync_guard = state.state_sync.lock();

    if let Some(state_sync) = sync_guard.as_ref() {
        let limit = limit.unwrap_or(100);
        Ok(state_sync.get_event_history(&workspaceId, limit).await)
    } else {
        Err("State synchronization not initialized".to_string())
    }
}

/// Broadcast a test event (for debugging)
#[command]
pub async fn broadcast_test_event(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let sync_guard = state.state_sync.lock();

    if let Some(state_sync) = sync_guard.as_ref() {
        let event = WorkspaceEvent::ProgressUpdate {
            workspace_id: workspaceId,
            progress: 0.5,
        };

        state_sync.broadcast_workspace_event(event).await?;
        Ok(())
    } else {
        Err("State synchronization not initialized".to_string())
    }
}
