//! Import command interface adapters.

use tauri::{AppHandle, State};

use crate::models::AppState;

#[tauri::command]
pub async fn import_folder(
    app: AppHandle,
    path: String,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::commands::import::import_folder_impl(app, path, workspace_id, state).await
}
