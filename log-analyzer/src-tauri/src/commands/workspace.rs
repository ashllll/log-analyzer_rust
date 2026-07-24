//! 工作区管理命令
//!
//! 提供工作区的删除和管理功能,包括:
//! - 删除工作区及其所有相关资源
//! - 清理解压目录
//! - 清除内存状态
//! - 工作区格式检测
//!
//! # 设计原则
//!
//! - 按正确的依赖顺序清理资源
//! - 单步失败不中断流程
//! - 提供友好的错误提示
//! - 支持重试和清理队列机制
//! - 只支持CAS格式工作区
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。
//! 这确保了前后端接口的一致性，避免了参数名转换带来的混乱。
//!
//! Tauri command 默认使用 camelCase 参数映射，因此 Rust 侧使用 snake_case，
//! 前端仍通过 `{ workspaceId }` 传参。
//!
//! 对应的前端调用：
//! ```typescript
//! await invoke('load_workspace', { workspaceId: 'workspace-123' });
//! ```

use std::{path::Path, sync::Arc};

use la_core::error::CommandError;
use tauri::{AppHandle, State};
use tracing::info;

use crate::commands::import::import_folder;
use crate::models::AppState;
use crate::utils::validation::validate_workspace_id;
use crate::utils::workspace_paths::{build_workspace_id, resolve_workspace_dir};

/// Workspace load response
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLoadResponse {
    /// Whether the workspace was loaded successfully
    pub success: bool,
    /// Number of files loaded
    pub file_count: usize,
}

/// 加载工作区索引
///
/// 只支持CAS格式工作区：
/// - 检查工作区是否存在metadata.db和objects目录
/// - 返回文件数量信息
///
/// # 前后端集成规范
/// 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名，
/// 与前端 invoke('load_workspace', { workspaceId }) 调用保持一致
#[tauri::command]
pub async fn load_workspace(
    app: AppHandle,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceLoadResponse, CommandError> {
    // ── Acquire workspace service (validates ID, resolves dir, checks CAS, creates service) ──
    let (service, _workspace_dir) =
        crate::utils::workspace_guard::require_cas_workspace(&app, &state, &workspace_id).await?;

    let file_count =
        service.metadata_store().count_files().await.map_err(|e| {
            CommandError::new("DATABASE_ERROR", format!("Failed to count files: {e}"))
        })? as usize;

    // Broadcast workspace loaded event
    let state_sync_opt = state.get_state_sync();
    if let Some(state_sync) = state_sync_opt {
        use crate::state_sync::models::{WorkspaceEvent, WorkspaceStatus};
        use std::time::Duration;
        let state_sync: crate::state_sync::StateSync = state_sync;
        let _ = state_sync
            .broadcast_workspace_event(WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: WorkspaceStatus::Completed {
                    duration: Duration::from_secs(0),
                },
            })
            .await;
    }

    Ok(WorkspaceLoadResponse {
        success: true,
        file_count,
    })
}

/// 增量刷新工作区索引
///
/// 注意：CAS架构下，刷新操作等同于重新导入
/// 因为CAS自动处理去重，重新导入是最简单可靠的方式
#[tauri::command]
pub async fn refresh_workspace(
    app: AppHandle,
    workspace_id: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let path = resolve_refresh_source_path(&app, &workspace_id, path)?;

    info!(
        workspace_id = %workspace_id,
        path = %path,
        "Refresh requested for workspace"
    );

    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(CommandError::new("NOT_FOUND", "Source path does not exist")
            .with_help("The source folder may have been moved or deleted"));
    }

    let workspace_dir = resolve_workspace_dir(&app, &workspace_id)
        .map_err(|e| CommandError::new("NOT_FOUND", e))?;

    // Check if workspace exists and is CAS format
    if !workspace_dir.exists() {
        info!("Workspace not found, performing fresh import");
        return import_folder(app, path, workspace_id, state)
            .await
            .map_err(|e| CommandError::new("IMPORT_ERROR", e));
    }

    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    if !metadata_db.exists() || !objects_dir.exists() {
        info!("Workspace is not CAS format, performing fresh import");
        return import_folder(app, path, workspace_id, state)
            .await
            .map_err(|e| CommandError::new("IMPORT_ERROR", e));
    }

    // For CAS workspaces, refresh is equivalent to re-import
    // CAS handles deduplication automatically, so re-importing is safe and simple
    info!("CAS workspace detected, re-importing for refresh");

    import_folder(app, path, workspace_id, state)
        .await
        .map_err(|e| CommandError::new("IMPORT_ERROR", e))
}

#[derive(Debug, serde::Deserialize)]
struct StoredWorkspaceConfig {
    id: String,
    name: Option<String>,
    path: Option<String>,
}

fn load_stored_workspaces(
    app: &AppHandle,
    action: &str,
) -> Result<Vec<StoredWorkspaceConfig>, CommandError> {
    let config = crate::utils::load_app_config(app);
    match config {
        Some(c) => serde_json::from_value(c.workspaces).map_err(|e| {
            CommandError::new(
                "CONFIG_ERROR",
                format!("Failed to parse stored workspaces while {action}: {e}"),
            )
        }),
        None => Ok(Vec::new()),
    }
}

fn resolve_refresh_source_path(
    app: &AppHandle,
    workspace_id: &str,
    path: Option<String>,
) -> Result<String, CommandError> {
    if let Some(path) = path.filter(|value| !value.trim().is_empty()) {
        // FIX(HI-05): 验证用户传入的路径，防止路径遍历攻击
        crate::utils::validation::prevent_path_traversal(&path)
            .map_err(|e| CommandError::new("VALIDATION_ERROR", e))?;
        return Ok(path);
    }

    if let Some(path) = load_stored_workspaces(app, "resolving workspace path")?
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
        .and_then(|workspace| workspace.path)
        .filter(|value| !value.trim().is_empty())
    {
        info!(
            workspace_id = %workspace_id,
            path = %path,
            "Resolved workspace source path from saved config"
        );
        return Ok(path);
    }

    Err(
        CommandError::new("CONFIG_ERROR", "Workspace source path missing")
            .with_help("Please refresh the workspace list or re-import it"),
    )
}

fn resolve_workspace_display_name(app: &AppHandle, workspace_id: &str) -> Option<String> {
    load_stored_workspaces(app, "resolving workspace display name")
        .ok()?
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
        .and_then(|workspace| workspace.name)
        .filter(|name| !name.trim().is_empty())
}

/// 清理工作区所有资源
///
/// 委托给 `CleanupUseCase`，保持命令层精简。
async fn cleanup_workspace_resources(
    workspace_id: &str,
    state: &AppState,
    app: &AppHandle,
) -> Result<(), CommandError> {
    crate::application::cleanup::CleanupUseCase::execute(workspace_id, state, app).await
}

/// 删除工作区命令
///
/// Tauri命令接口,用于删除工作区及其所有相关资源。
#[tauri::command]
pub async fn delete_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), CommandError> {
    info!(
        workspace_id = %workspace_id,
        "Delete workspace command called"
    );

    // 参数验证
    validate_workspace_id(&workspace_id).map_err(|e| CommandError::new("VALIDATION_ERROR", e))?;

    // 执行清理
    cleanup_workspace_resources(&workspace_id, &state, &app).await?;

    // 广播工作区删除事件
    // 注意：先克隆 state_sync，释放锁后再 await，避免跨 await 点持有锁
    let state_sync_opt = state.get_state_sync();
    if let Some(state_sync) = state_sync_opt {
        use crate::state_sync::models::{WorkspaceEvent, WorkspaceStatus};
        use std::time::SystemTime;
        let state_sync: crate::state_sync::StateSync = state_sync;
        let _ = state_sync
            .broadcast_workspace_event(WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: WorkspaceStatus::Cancelled {
                    cancelled_at: SystemTime::now(),
                },
            })
            .await;
    }

    info!(
        workspace_id = %workspace_id,
        "Delete workspace command completed"
    );

    Ok(())
}

/// 取消任务命令
///
/// 将任务状态设置为 Stopped
#[tauri::command]
pub async fn cancel_task(task_id: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    info!(
        task_id = %task_id,
        "Cancel task command called"
    );

    let task_manager = state
        .get_task_manager_clone()
        .ok_or_else(|| CommandError::new("NOT_INITIALIZED", "Task manager not initialized"))?;
    // FIX(HI-03): 使用 ? 传播错误，避免静默丢弃
    task_manager
        .update_task_async(
            &task_id,
            0, // progress 保持不变
            "Task cancelled by user".to_string(),
            crate::task_manager::TaskStatus::Stopped,
        )
        .await
        .map_err(|e| CommandError::new("TASK_ERROR", format!("Failed to cancel task: {e}")))?;

    info!(
        task_id = %task_id,
        "Task cancelled successfully"
    );

    Ok(())
}

/// 工作区状态响应
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceStatusResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub size: String,
    pub files: usize,
}

/// 获取工作区状态命令
///
/// 返回工作区的详细信息
#[tauri::command]
pub async fn get_workspace_status(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WorkspaceStatusResponse, CommandError> {
    // ── Acquire workspace service (validates ID, resolves dir, checks CAS, creates service) ──
    let (service, _workspace_dir) =
        crate::utils::workspace_guard::require_cas_workspace(&app, &state, &workspace_id).await?;

    let file_count: i64 =
        service.metadata_store().count_files().await.map_err(|e| {
            CommandError::new("DATABASE_ERROR", format!("Failed to count files: {e}"))
        })?;

    // 用 SQL 聚合查询替代目录遍历，O(1) 而非 O(n)
    let total_size: u64 = service
        .metadata_store()
        .sum_file_sizes()
        .await
        .map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to sum file sizes: {e}")))?
        .try_into()
        .unwrap_or(0);

    let size_mb = total_size / (1024 * 1024);
    let size_str = if size_mb >= 1024 {
        format!("{:.1}GB", size_mb as f64 / 1024.0)
    } else {
        format!("{size_mb}MB")
    };

    Ok(WorkspaceStatusResponse {
        id: workspace_id.clone(),
        name: resolve_workspace_display_name(&app, &workspace_id)
            .unwrap_or_else(|| workspace_id.clone()),
        status: "READY".to_string(),
        size: size_str,
        files: file_count as usize,
    })
}

/// 获取工作区日志时间范围
///
/// 从 Tantivy 索引中查询最早和最晚的日志时间戳
#[tauri::command]
pub async fn get_workspace_time_range(
    app: AppHandle,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<la_core::models::search::WorkspaceTimeRange, CommandError> {
    use chrono::DateTime;
    use la_core::models::search::WorkspaceTimeRange;

    // ── Acquire workspace service ──
    let (service, _workspace_dir) =
        crate::utils::workspace_guard::require_cas_workspace(&app, &state, &workspace_id).await?;
    let manager: Arc<la_search::SearchEngineManager> = Arc::clone(service.search_engine());
    let (min_ts, max_ts, total_logs) = manager.get_time_range().map_err(|e| {
        CommandError::new(
            "SEARCH_ERROR",
            format!("Failed to get time range from index: {e}"),
        )
    })?;

    // Convert timestamps to ISO 8601 format
    let min_timestamp = if min_ts > 0 {
        Some(
            DateTime::from_timestamp(min_ts, 0)
                .map_or_else(|| min_ts.to_string(), |dt| dt.to_rfc3339()),
        )
    } else {
        None
    };

    let max_timestamp = if max_ts > 0 {
        Some(
            DateTime::from_timestamp(max_ts, 0)
                .map_or_else(|| max_ts.to_string(), |dt| dt.to_rfc3339()),
        )
    } else {
        None
    };

    Ok(WorkspaceTimeRange {
        min_timestamp,
        max_timestamp,
        total_logs,
    })
}

/// 创建工作区命令（import_folder 的语义化别名）
///
/// 提供更符合用户预期的命令名来创建工作区
#[tauri::command]
pub async fn create_workspace(
    name: String,
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    info!(
        name = %name,
        path = %path,
        "Create workspace command called"
    );

    // 验证并规范化导入源路径，避免在 import_folder 前使用未经校验的输入
    let canonical_path = crate::utils::validation::validate_import_source_path(&path, "path")
        .map_err(|e| CommandError::new("VALIDATION_ERROR", e))?;

    // 生成带随机后缀的 workspace ID，避免同名工作区覆盖
    let workspace_id = build_workspace_id(&name);

    // 验证生成的 workspace ID 合法性
    validate_workspace_id(&workspace_id).map_err(|e| CommandError::new("VALIDATION_ERROR", e))?;

    // 调用 import_folder 逻辑
    import_folder(
        app,
        canonical_path.to_string_lossy().into_owned(),
        workspace_id,
        state,
    )
    .await
    .map_err(|e| CommandError::new("IMPORT_ERROR", e))
}
