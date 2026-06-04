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

use std::{fs, path::Path, sync::Arc};

use serde::Serialize;
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::adapters::tauri_config::TauriAppConfigProvider;
use crate::application::workspace_service::{ImportOptions, WorkspaceServiceRef};
use crate::infrastructure::{TauriEventPublisher, WorkspaceServiceImpl};
use crate::models::AppState;
use crate::task_manager::{TaskManager, TaskStatus};
use crate::utils::workspace_paths::preferred_workspace_dir;
use crate::utils::{canonicalize_path, validate_workspace_id};
use la_core::error::AppError;
use la_core::models::config::AppConfigLoader;
use la_storage::{verify_after_import, ContentAddressableStorage, MetadataStore};

const SEARCH_INDEX_DIR_NAME: &str = "search_index";
const SEARCH_INDEX_WRITER_HEAP_BYTES: usize = 50_000_000;

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

pub(crate) fn load_workspace_search_config(
    app: &AppHandle,
) -> la_core::models::config::SearchConfig {
    let config_path = match app.path().app_config_dir() {
        Ok(dir) => dir.join("config.json"),
        Err(_) => return Default::default(),
    };

    if !config_path.exists() {
        return Default::default();
    }

    AppConfigLoader::load(Some(config_path))
        .ok()
        .map(|loader| loader.get_search_config().clone())
        .unwrap_or_default()
}

pub(crate) fn ensure_search_engine_manager(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    workspace_dir: &Path,
) -> Result<Arc<la_search::SearchEngineManager>, String> {
    // P4 迁移：优先从 workspace_services 获取已预组装的服务
    if let Some(service) = state.get_workspace_service(workspace_id) {
        return Ok(Arc::clone(service.search_engine()));
    }

    let app_search_config = load_workspace_search_config(app);
    let index_path = workspace_dir.join(SEARCH_INDEX_DIR_NAME);
    let manager = Arc::new(
        la_search::SearchEngineManager::with_app_config(
            app_search_config,
            index_path,
            SEARCH_INDEX_WRITER_HEAP_BYTES,
        )
        .map_err(|e| format!("Failed to initialize search engine: {e}"))?,
    );

    Ok(manager)
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

// ============================================================================
// 命令层辅助函数
// ============================================================================

/// 获取或创建工作区服务实例。
///
/// 如果服务已存在于 AppState 中则直接返回，否则创建新实例并存储。
/// P6：提升为 pub(crate)，供 workspace.rs 的 load_workspace / get_workspace_status 使用。
pub(crate) async fn get_or_create_workspace_service(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    workspace_dir: &Path,
) -> Result<WorkspaceServiceRef, String> {
    // 优先返回已存在的服务
    if let Some(service) = state.get_workspace_service(workspace_id) {
        return Ok(service);
    }

    // 创建各运行时组件
    let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.to_path_buf()));

    let metadata_store = Arc::new(
        MetadataStore::new(workspace_dir)
            .await
            .map_err(|e| format!("Failed to open metadata store: {e}"))?,
    );

    let search_manager =
        ensure_search_engine_manager(app, state, workspace_id, workspace_dir)?;

    let disk_result_store = state
        .get_disk_result_store()
        .ok_or("Disk result store not initialized")?;
    let thread_pool = state.get_search_thread_pool();
    let regex_cache_size = load_workspace_search_config(app).regex_cache_size.max(1);

    let service = Arc::new(WorkspaceServiceImpl::new(
        workspace_id.to_string(),
        workspace_dir.to_path_buf(),
        cas,
        metadata_store,
        search_manager,
        disk_result_store,
        Arc::new(TauriEventPublisher {
            app_handle: app.clone(),
        }),
        thread_pool,
        regex_cache_size,
    ));

    state.set_workspace_service(workspace_id.to_string(), service.clone() as WorkspaceServiceRef);
    tracing::info!(
        workspace_id = %workspace_id,
        "WorkspaceService created and registered"
    );

    Ok(service as WorkspaceServiceRef)
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
