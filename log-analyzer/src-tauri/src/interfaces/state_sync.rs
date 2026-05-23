//! State synchronization command interface adapters.

use tauri::{AppHandle, State};

use la_core::error::CommandError;

use crate::models::AppState;

#[tauri::command]
pub async fn init_state_sync(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    crate::commands::state_sync::init_state_sync_impl(app, state).await
}
