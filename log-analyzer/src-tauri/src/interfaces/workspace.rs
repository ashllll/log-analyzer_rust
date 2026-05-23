//! Workspace command interface adapters.

use tauri::{AppHandle, State};

use la_core::error::CommandError;

use crate::commands::workspace::{WorkspaceLoadResponse, WorkspaceStatusResponse};
use crate::models::AppState;

#[tauri::command]
pub async fn load_workspace(
    app: AppHandle,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceLoadResponse, CommandError> {
    crate::commands::workspace::load_workspace(app, workspace_id, state).await
}

#[tauri::command]
pub async fn refresh_workspace(
    app: AppHandle,
    workspace_id: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    crate::commands::workspace::refresh_workspace(app, workspace_id, path, state).await
}

#[tauri::command]
pub async fn delete_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), CommandError> {
    crate::commands::workspace::delete_workspace(workspace_id, state, app).await
}

#[tauri::command]
pub async fn cancel_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    crate::commands::workspace::cancel_task(task_id, state).await
}

#[tauri::command]
pub async fn get_workspace_status(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WorkspaceStatusResponse, CommandError> {
    crate::commands::workspace::get_workspace_status(workspace_id, app, state).await
}

#[tauri::command]
pub async fn get_workspace_time_range(
    app: AppHandle,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<la_core::models::search::WorkspaceTimeRange, CommandError> {
    crate::commands::workspace::get_workspace_time_range(app, workspace_id, state).await
}

#[tauri::command]
pub async fn create_workspace(
    name: String,
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    crate::commands::workspace::create_workspace(name, path, app, state).await
}
