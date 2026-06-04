//! 导入相关命令实现
//!
//! # P4 重构
//!
//! `import_folder` 已精简：核心导入逻辑下沉到 `WorkspaceServiceImpl::import_file`。
//! 命令层仅负责 TaskManager 生命周期、完整性验证（verify_after_import）
//! 和 Tantivy 段合并。
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use std::{fs, sync::Arc};

use serde::Serialize;
use tauri::{command, AppHandle, Emitter, State};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::adapters::tauri_config::TauriAppConfigProvider;
use crate::application::workspace_service::ImportOptions;
use crate::infrastructure::workspace_service_factory::{
    get_or_create_workspace_service,
};
use crate::models::AppState;
use crate::task_manager::{TaskManager, TaskStatus};
use crate::utils::workspace_paths::preferred_workspace_dir;
use crate::utils::{canonicalize_path, validate_workspace_id};
use la_core::error::AppError;
use la_storage::verify_after_import;

// ============================================================================
// 共享工具函数
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct RarSupportInfo {
    compiled: bool,
    available: bool,
    reason: Option<String>,
}

fn get_rar_support_info() -> RarSupportInfo {
    #[cfg(feature = "rar-support")]
    {
        RarSupportInfo {
            compiled: true,
            available: true,
            reason: None,
        }
    }

    #[cfg(not(feature = "rar-support"))]
    {
        RarSupportInfo {
            compiled: false,
            available: false,
            reason: Some("RAR support is not compiled into this build".to_string()),
        }
    }
}

// ============================================================================
// 简化版 import_folder 命令
// ============================================================================

#[tauri::command]
pub async fn import_folder(
    app: AppHandle,
    path: String,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_workspace_id(&workspace_id)?;

    let validated_path =
        crate::utils::validation::validate_import_source_path(&path, "path")?;
    let canonical_path = canonicalize_path(&validated_path).map_err(|e| {
        let msg = format!("Path canonicalization failed: {e}");
        warn!("{msg}");
        let _ = app.emit("import-error", &msg);
        msg
    })?;

    let workspace_dir = preferred_workspace_dir(&app, &workspace_id)?;
    fs::create_dir_all(&workspace_dir).map_err(|e| {
        AppError::io_error(
            format!("Failed to create workspace dir: {e}"),
            Some(workspace_dir.clone()),
        )
        .to_string()
    })?;

    let target_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    // ── TaskManager 任务创建 ──
    let task_id = Uuid::new_v4().to_string();
    let task_manager = state
        .get_task_manager_clone()
        .ok_or("Task manager not initialized")?;
    let tm: &TaskManager = &task_manager;
    tm.create_task_async(
        task_id.clone(),
        "Import".to_string(),
        target_name,
        Some(workspace_id.clone()),
    )
    .await
    .map_err(|e| format!("Failed to create task: {e}"))?;

    // ── 获取或创建 WorkspaceService ──
    let service = get_or_create_workspace_service(
        &app,
        &state,
        &workspace_id,
        &workspace_dir,
    )
    .await
    .map_err(|e| {
        let _ = app.emit("import-error", &e);
        e
    })?;

    // ── 更新任务进度 ──
    update_task(
        tm,
        &task_id,
        10,
        "Scanning...",
        TaskStatus::Running,
        &app,
    )
    .await;

    // ── 调用 ImportService ──
    let config_provider = TauriAppConfigProvider(app.clone());
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let _import_result = match service
        .import_file(
            &canonical_path,
            ImportOptions::default(),
            &config_provider,
            &task_id,
            cancel_token,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!(error = %e, "Failed to import file");
            state.remove_workspace_service(&workspace_id);
            if workspace_dir.exists() {
                if let Err(rm_err) = std::fs::remove_dir_all(&workspace_dir) {
                    warn!(path = ?workspace_dir, error = %rm_err, "Failed to cleanup workspace");
                    let _ = app
                        .emit("import-error", &format!("Cleanup failed: {rm_err}"));
                }
            }
            let msg = format!("Failed to import: {e}");
            update_task(tm, &task_id, 0, &msg, TaskStatus::Failed, &app)
                .await;
            return Err(msg);
        }
    };

    // ── 完整性验证（保留在命令层）──
    update_task(
        tm,
        &task_id,
        95,
        "Verifying integrity...",
        TaskStatus::Running,
        &app,
    )
    .await;

    match verify_after_import(&workspace_dir).await {
        Ok(report) => {
            if report.is_valid() {
                info!(
                    workspace_id = %workspace_id,
                    total_files = report.total_files,
                    valid_files = report.valid_files,
                    "Import completed with integrity verification"
                );
            } else {
                warn!(
                    workspace_id = %workspace_id,
                    total_files = report.total_files,
                    valid_files = report.valid_files,
                    invalid = report.invalid_files.len(),
                    missing = report.missing_objects.len(),
                    corrupted = report.corrupted_objects.len(),
                    "Integrity verification found issues"
                );
                let _ = app.emit(
                    "validation-report",
                    serde_json::json!({
                        "workspace_id": workspace_id,
                        "report": report,
                    }),
                );
            }
        }
        Err(e) => {
            let msg = format!("Failed to verify integrity: {e}");
            error!(workspace_id = %workspace_id, error = %e, "{msg}");
            let _ = app.emit("import-error", &msg);
        }
    }

    // ── 完成 ──
    update_task(
        tm,
        &task_id,
        100,
        "Done",
        TaskStatus::Completed,
        &app,
    )
    .await;

    // ── Tantivy segment 合并（后台执行）──
    let search_engine = Arc::clone(service.search_engine());
    let ws_id = workspace_id.clone();
    tokio::spawn(async move {
        info!(workspace_id = %ws_id, "Starting Tantivy segment merge");
        if let Err(e) = search_engine.commit_and_wait_merge().await {
            warn!(
                workspace_id = %ws_id,
                error = %e,
                "Tantivy segment merge warning (non-critical)"
            );
            // Non-fatal; frontend 通过事件获知
        }
    });

    let _ = app.emit("import-complete", &task_id);
    Ok(task_id)
}

/// 检查 RAR 支持状态（无 sidecar 依赖）
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    serde_json::to_value(get_rar_support_info())
        .map_err(|error| format!("Failed to serialize RAR support info: {error}"))
}

/// 更新任务进度并处理错误。
async fn update_task(
    task_manager: &TaskManager,
    task_id: &str,
    progress: u8,
    message: &str,
    status: TaskStatus,
    app: &AppHandle,
) {
    if let Err(e) = task_manager
        .update_task_async(task_id, progress, message.to_string(), status)
        .await
    {
        warn!(
            task_id = %task_id,
            error = %e,
            "Failed to update task progress"
        );
        let _ = app.emit("import-error", &format!("Task update failed: {e}"));
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::get_rar_support_info;

    #[test]
    fn rar_support_reports_compiled_feature_state() {
        let support = get_rar_support_info();

        #[cfg(feature = "rar-support")]
        {
            assert!(support.compiled);
            assert!(support.available);
            assert!(support.reason.is_none());
        }

        #[cfg(not(feature = "rar-support"))]
        {
            assert!(!support.compiled);
            assert!(!support.available);
            assert!(support
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("not compiled")));
        }
    }
}
