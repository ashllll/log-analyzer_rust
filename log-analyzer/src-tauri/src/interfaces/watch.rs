//! Watch command interface adapters.

use tauri::{AppHandle, State};

use crate::models::AppState;

#[tauri::command]
pub async fn start_watch(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    #[allow(non_snake_case)] autoSearch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::commands::watch::start_watch_impl(app, workspaceId, path, autoSearch, state).await
}

#[tauri::command]
pub async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::commands::watch::stop_watch_impl(workspaceId, state).await
}
