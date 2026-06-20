//! ImportPipeline — 导入生命周期编排器。
//!
//! 将 `import_folder` Tauri 命令中原本混杂的验证、目录创建、
//! TaskScheduler 生命周期、WorkspaceService 创建、导入调用、失败清理、
//! 后台完整性验证和 Tantivy 段合并抽取到本模块，使命令层保持薄。
//!
//! # 职责边界
//!
//! - **本模块**：拥有导入的完整生命周期编排。通过 trait 引用接收依赖，不绑定 Tauri。
//! - **`ImportService::import_file`**：保持行为不变，负责具体的文件导入逻辑。
//! - **命令层**：构造 adapter 参数并调用 `run_import`，返回 task id。

use std::fs;
use std::path::Path;
use std::sync::Arc;

use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::workspace_service::ImportOptions;
use crate::infrastructure::workspace_service_factory::get_or_create_workspace_service;
use crate::models::AppState;
use crate::utils::canonicalize_path;
use crate::utils::validation::{validate_import_source_path, validate_workspace_id};
use la_core::domain::event::EventPublisher;
use la_core::domain::{TaskHandle, WorkspacePaths};
use la_core::error::AppError;
use la_core::traits::AppConfigProvider;
use la_storage::verify_after_import;

/// 运行导入生命周期，返回 TaskScheduler 创建的任务 ID。
///
/// 通过 trait 引用接收基础设施依赖，不绑定 Tauri 具体类型，可独立测试。
///
/// `event_publisher` 使用 `Arc` 以支持后台任务的 fire-and-forget 事件发送。
pub async fn run_import(
    event_publisher: Arc<dyn EventPublisher>,
    workspace_paths: &dyn WorkspacePaths,
    config_provider: &dyn AppConfigProvider,
    app_handle_for_factory: &tauri::AppHandle,
    state: &AppState,
    workspace_id: &str,
    path: &str,
) -> Result<String, String> {
    validate_workspace_id(workspace_id)?;

    let validated_path = validate_import_source_path(path, "path")?;
    let canonical_path = match canonicalize_path(&validated_path) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Path canonicalization failed: {e}");
            warn!("{msg}");
            event_publisher.emit_import_error(&msg).await;
            return Err(msg);
        }
    };

    let workspace_dir = workspace_paths.workspace_data_dir(workspace_id)?;
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
        .unwrap_or(path)
        .to_string();

    // ── TaskScheduler 任务创建 ──
    let task_id = Uuid::new_v4().to_string();
    let scheduler = state
        .get_task_scheduler()
        .ok_or("Task manager not initialized")?;
    let handle = TaskHandle::new(&task_id);
    scheduler
        .create(&task_id, "Import", &target_name, Some(workspace_id))
        .await
        .map_err(|e| format!("Failed to create task: {e}"))?;

    // ── 获取或创建 WorkspaceService ──
    let service =
        get_or_create_workspace_service(app_handle_for_factory, state, workspace_id, &workspace_dir)
            .await
            .inspect_err(|e| {
                let publisher = Arc::clone(&event_publisher);
                let error_msg = e.clone();
                tokio::spawn(async move {
                    publisher.emit_import_error(&error_msg).await;
                });
            })?;

    // ── 更新任务进度 ──
    let _ = scheduler.update(&handle, 10, "Scanning...").await;

    // ── 调用 ImportService ──
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let _import_result = match service
        .import_file(
            &canonical_path,
            ImportOptions::default(),
            config_provider,
            &task_id,
            cancel_token,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!(error = %e, "Failed to import file");
            state.remove_workspace_service(workspace_id);
            if workspace_dir.exists() {
                if let Err(rm_err) = fs::remove_dir_all(&workspace_dir) {
                    warn!(path = ?workspace_dir, error = %rm_err, "Failed to cleanup workspace");
                    let publisher = Arc::clone(&event_publisher);
                    let cleanup_msg = format!("Cleanup failed: {rm_err}");
                    tokio::spawn(async move {
                        publisher.emit_import_error(&cleanup_msg).await;
                    });
                }
            }
            let msg = format!("Failed to import: {e}");
            let _ = scheduler.fail(&handle, &msg).await;
            return Err(msg);
        }
    };

    // ── 完成 ──
    let _ = scheduler.update(&handle, 100, "Import complete").await;
    let _ = scheduler.complete(&handle).await;
    event_publisher.emit_import_complete(&task_id).await;

    // ── 完整性验证（后台执行）──
    let verify_publisher = Arc::clone(&event_publisher);
    let verify_workspace_id = workspace_id.to_string();
    let verify_workspace_dir = workspace_dir.clone();
    tokio::spawn(async move {
        match verify_after_import(&verify_workspace_dir).await {
            Ok(report) => {
                if report.is_valid() {
                    info!(
                        workspace_id = %verify_workspace_id,
                        total_files = report.total_files,
                        valid_files = report.valid_files,
                        "Import integrity verification completed"
                    );
                } else {
                    warn!(
                        workspace_id = %verify_workspace_id,
                        total_files = report.total_files,
                        valid_files = report.valid_files,
                        invalid = report.invalid_files.len(),
                        missing = report.missing_objects.len(),
                        corrupted = report.corrupted_objects.len(),
                        "Integrity verification found issues"
                    );
                    let report_json = serde_json::json!({
                        "workspace_id": verify_workspace_id,
                        "report": report,
                    });
                    let _ = verify_publisher
                        .emit_validation_report(&verify_workspace_id, &report_json.to_string())
                        .await;
                }
            }
            Err(e) => {
                let msg = format!("Failed to verify integrity: {e}");
                error!(workspace_id = %verify_workspace_id, error = %e, "{msg}");
                let _ = verify_publisher.emit_import_error(&msg).await;
            }
        }
    });

    // ── Tantivy segment 合并（后台执行）──
    let search_engine = Arc::clone(service.search_engine());
    let ws_id = workspace_id.to_string();
    tokio::spawn(async move {
        info!(workspace_id = %ws_id, "Starting Tantivy segment merge");
        if let Err(e) = search_engine.commit_and_wait_merge().await {
            warn!(
                workspace_id = %ws_id,
                error = %e,
                "Tantivy segment merge warning (non-critical)"
            );
        }
    });

    Ok(task_id)
}

/// 便捷函数：从字符串路径构造 [`Path`] 后运行导入生命周期。
///
/// 主要用于测试或需要显式传入 [`Path`] 的场景。
pub async fn run_import_with_path(
    event_publisher: Arc<dyn EventPublisher>,
    workspace_paths: &dyn WorkspacePaths,
    config_provider: &dyn AppConfigProvider,
    app_handle_for_factory: &tauri::AppHandle,
    state: &AppState,
    workspace_id: &str,
    source_path: &Path,
) -> Result<String, String> {
    let path_str = source_path.to_string_lossy().to_string();
    run_import(
        event_publisher,
        workspace_paths,
        config_provider,
        app_handle_for_factory,
        state,
        workspace_id,
        &path_str,
    )
    .await
}
