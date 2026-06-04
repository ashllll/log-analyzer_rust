//! 文件监听命令 — thin glue over WorkspaceServiceImpl::WatchService.
//!
//! P5 迁移后：watcher 状态内嵌于 WorkspaceServiceImpl 实例，命令层仅做
//! 参数验证和服务查找，直接委托给 service 的 WatchService 实现。
//!
//! 已移除：
//! - WatchEventAdapter（搜索索引更新现在由 WorkspaceServiceImpl 内部直接访问 search_engine）
//! - StopOnlyCas / StopOnlyMeta / StopOnlyEvents 存根（stop_watch 不再需要外部依赖注入）
//! - WatchUseCase 创建（逻辑已内联到 WorkspaceServiceImpl）

use tauri::{AppHandle, State};

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::models::AppState;
use crate::utils::{validate_path_param, validate_workspace_id};

/// Start watching a workspace directory for file changes.
///
/// Thin glue: validates parameters, looks up the workspace service, delegates.
#[tauri::command]
pub async fn start_watch(
    _app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    #[allow(non_snake_case)] _autoSearch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_workspace_id(&workspaceId)?;
    validate_path_param(&path, "path")?;

    // ── Look up workspace service ──
    let workspace: WorkspaceServiceRef = state
        .get_workspace_service(&workspaceId)
        .ok_or_else(|| {
            format!(
                "Workspace {} not found. Please import or reload the workspace.",
                workspaceId
            )
        })?;

    // ── Delegate to service (watcher state lives inside the instance) ──
    workspace
        .start_watch(&path)
        .await
        .map_err(|e| e.to_string())
}

/// Stop watching a workspace.
///
/// Thin glue: looks up the workspace service, delegates.
#[tauri::command]
pub async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let workspace: WorkspaceServiceRef = state
        .get_workspace_service(&workspaceId)
        .ok_or_else(|| {
            format!(
                "Workspace {} not found. Please import or reload the workspace.",
                workspaceId
            )
        })?;

    workspace
        .stop_watch()
        .await
        .map_err(|e| e.to_string())
}
