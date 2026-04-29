//! 导入相关命令实现
//! 包含工作区导入与 RAR 支持检查
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use std::{fs, path::Path, sync::Arc};

use serde::Serialize;
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::error;
use uuid::Uuid;

use crate::models::AppState;
use crate::services::parse_log_lines;
use crate::task_manager::TaskManager;
use crate::utils::encoding::decode_log_content;
use crate::utils::workspace_paths::preferred_workspace_dir;
use crate::utils::{canonicalize_path, validate_path_param, validate_workspace_id};
use la_core::error::AppError;
use la_core::models::config::AppConfigLoader;
use la_storage::{verify_after_import, MetadataStore};

const SEARCH_INDEX_DIR_NAME: &str = "search_index";
const SEARCH_INDEX_WRITER_HEAP_BYTES: usize = 50_000_000;
const SEARCH_INDEX_COMMIT_EVERY_FILES: usize = 25;

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
) -> Result<Arc<crate::search_engine::SearchEngineManager>, String> {
    if let Some(manager) = state
        .search_engine_managers
        .lock()
        .get(workspace_id)
        .cloned()
    {
        return Ok(manager);
    }

    let app_search_config = load_workspace_search_config(app);
    let index_path = workspace_dir.join(SEARCH_INDEX_DIR_NAME);
    let manager = Arc::new(
        crate::search_engine::manager::SearchEngineManager::with_app_config(
            app_search_config,
            index_path,
            SEARCH_INDEX_WRITER_HEAP_BYTES,
        )
        .map_err(|e| format!("Failed to initialize search engine: {}", e))?,
    );

    state
        .search_engine_managers
        .lock()
        .insert(workspace_id.to_string(), Arc::clone(&manager));

    Ok(manager)
}

pub(crate) async fn ensure_workspace_runtime_state(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    workspace_dir: &Path,
) -> Result<
    (
        Arc<crate::storage::ContentAddressableStorage>,
        Arc<MetadataStore>,
        Arc<crate::search_engine::SearchEngineManager>,
    ),
    String,
> {
    let cas = {
        let mut cas_instances = state.cas_instances.lock();
        cas_instances
            .entry(workspace_id.to_string())
            .or_insert_with(|| {
                Arc::new(crate::storage::ContentAddressableStorage::new(
                    workspace_dir.to_path_buf(),
                ))
            })
            .clone()
    };

    let metadata_store = {
        let existing = {
            let metadata_stores = state.metadata_stores.lock();
            metadata_stores.get(workspace_id).cloned()
        };

        if let Some(store) = existing {
            store
        } else {
            let store = Arc::new(
                MetadataStore::new(workspace_dir)
                    .await
                    .map_err(|e| format!("Failed to open metadata store: {}", e))?,
            );

            state
                .metadata_stores
                .lock()
                .insert(workspace_id.to_string(), Arc::clone(&store));

            store
        }
    };

    state
        .workspace_dirs
        .lock()
        .insert(workspace_id.to_string(), workspace_dir.to_path_buf());

    let search_manager = ensure_search_engine_manager(app, state, workspace_id, workspace_dir)?;

    Ok((cas, metadata_store, search_manager))
}

async fn rebuild_workspace_search_index(
    metadata_store: Arc<MetadataStore>,
    cas: Arc<crate::storage::ContentAddressableStorage>,
    search_manager: Arc<crate::search_engine::SearchEngineManager>,
) -> Result<usize, String> {
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to enumerate imported files for indexing: {}", e))?;

    tokio::task::spawn_blocking(move || -> Result<usize, String> {
        search_manager
            .clear_index()
            .map_err(|e| format!("Failed to clear search index before rebuild: {}", e))?;

        let mut indexed_lines = 0usize;

        for (file_index, file) in files.into_iter().enumerate() {
            let content = cas.read_content_sync(&file.sha256_hash).map_err(|e| {
                format!(
                    "Failed to read CAS content for {}: {}",
                    file.virtual_path, e
                )
            })?;
            let (content_str, _) = decode_log_content(&content);
            let real_path = format!("cas://{}", file.sha256_hash);

            let mut line_buffer = Vec::with_capacity(1024);
            let mut start_line_number = 1usize;

            for line in content_str.lines() {
                line_buffer.push(line.to_string());

                if line_buffer.len() >= 1024 {
                    let entries = parse_log_lines(
                        &line_buffer,
                        &file.virtual_path,
                        &real_path,
                        indexed_lines,
                        start_line_number,
                    );
                    for entry in &entries {
                        search_manager
                            .add_document(entry)
                            .map_err(|e| format!("Failed to add indexed document: {}", e))?;
                    }
                    indexed_lines += entries.len();
                    start_line_number += line_buffer.len();
                    line_buffer.clear();
                }
            }

            if !line_buffer.is_empty() {
                let entries = parse_log_lines(
                    &line_buffer,
                    &file.virtual_path,
                    &real_path,
                    indexed_lines,
                    start_line_number,
                );
                for entry in &entries {
                    search_manager
                        .add_document(entry)
                        .map_err(|e| format!("Failed to add indexed document: {}", e))?;
                }
                indexed_lines += entries.len();
            }

            if (file_index + 1) % SEARCH_INDEX_COMMIT_EVERY_FILES == 0 {
                search_manager
                    .commit()
                    .map_err(|e| format!("Failed to commit rebuilt search index: {}", e))?;
            }
        }

        search_manager
            .commit()
            .map_err(|e| format!("Failed to finalize rebuilt search index: {}", e))?;

        Ok(indexed_lines)
    })
    .await
    .map_err(|e| format!("Search index rebuild task panicked: {}", e))?
}

#[command]
pub async fn import_folder(
    app: AppHandle,
    path: String,
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_path_param(&path, "path")?;
    validate_workspace_id(&workspaceId)?;

    let app_handle = app.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspaceId.clone();

    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let canonical_path = match canonicalize_path(source_path) {
        Ok(path) => path,
        Err(e) => {
            let error_msg = format!("Path canonicalization failed: {}", e);
            tracing::warn!("{}", error_msg);
            let _ = app_handle.emit("import-error", &error_msg);
            return Err(error_msg);
        }
    };

    let workspace_dir = preferred_workspace_dir(&app, &workspaceId)?;

    fs::create_dir_all(&workspace_dir).map_err(|e| {
        AppError::io_error(
            format!("Failed to create workspace dir: {e}"),
            Some(workspace_dir.clone()),
        )
        .to_string()
    })?;

    // 使用 TaskManager 创建任务
    let target_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    // Extract task_manager before await
    let task_manager: Option<TaskManager> = state.task_manager.lock().clone();
    let _task = if let Some(task_manager) = task_manager.as_ref() {
        let tm: &TaskManager = task_manager;
        tm.create_task_async(
            task_id.clone(),
            "Import".to_string(),
            target_name.clone(),
            Some(workspaceId.clone()),
        )
        .await
        .map_err(|e| format!("Failed to create task: {}", e))?
    } else {
        return Err("Task manager not initialized".to_string());
    };

    // 老王备注：TaskManager.CreateTask 已经自动发送了 task-update 事件，不需要重复发送
    // 直接在当前异步上下文中执行，避免创建新的 runtime
    let source_path = Path::new(&path);
    let root_name = source_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 更新任务进度
    // 老王备注：TaskManager.UpdateTask 会自动发送 task-update 事件，不需要重复发送
    let task_manager_clone: Option<TaskManager> = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(ref task_manager) = task_manager_clone {
        let task_manager: &TaskManager = task_manager;
        if let Err(e) = task_manager
            .update_task_async(
                &task_id_clone,
                10,
                "Scanning...".to_string(),
                crate::task_manager::TaskStatus::Running,
            )
            .await
        {
            let error_msg = format!("Failed to update task progress: {}", e);
            tracing::warn!(task_id = %task_id_clone, error = %e, "{}", error_msg);
            // 发送错误事件给前端
            let _ = app_handle.emit("import-error", &error_msg);
        }
    }

    let (cas, metadata_store, search_manager) =
        ensure_workspace_runtime_state(&app_handle, &state, &workspace_id_clone, &workspace_dir)
            .await
            .inspect_err(|e| {
                let _ = app_handle.emit("import-error", &e);
            })?;

    // Process the path using CAS architecture
    use crate::commands::TauriAppConfigProvider;
    use la_archive::processor::process_path_with_cas;

    let provider = TauriAppConfigProvider(app_handle.clone());

    if let Err(e) = process_path_with_cas(
        source_path,
        &root_name,
        &workspace_dir,
        &cas,
        metadata_store.clone(),
        &provider,
        &task_id_clone,
        &workspace_id_clone,
        None, // parent_archive_id
        0,    // depth_level
    )
    .await
    {
        error!(error = %e, "Failed to process path");

        // 清理导入失败前已插入的内存状态，防止孤儿条目
        state.cas_instances.lock().remove(&workspace_id_clone);
        state.metadata_stores.lock().remove(&workspace_id_clone);
        state
            .search_engine_managers
            .lock()
            .remove(&workspace_id_clone);
        {
            let mut dirs = state.workspace_dirs.lock();
            dirs.remove(&workspace_id_clone);
        }
        // 删除磁盘上不完整的工作区目录
        if workspace_dir.exists() {
            if let Err(rm_err) = std::fs::remove_dir_all(&workspace_dir) {
                let error_msg = format!("Failed to cleanup workspace directory: {}", rm_err);
                tracing::warn!(path = ?workspace_dir, error = %rm_err, "{}", error_msg);
                // 发送错误事件给前端
                let _ = app_handle.emit("import-error", &error_msg);
            }
        }

        // Update task with error
        let task_manager_clone: Option<TaskManager> = {
            let guard = state.task_manager.lock();
            guard.as_ref().cloned()
        };

        if let Some(ref task_manager) = task_manager_clone {
            let task_manager: &TaskManager = task_manager;
            // 添加完整的错误处理和降级方案
            if let Err(update_err) = task_manager
                .update_task_async(
                    &task_id_clone,
                    0,
                    format!("Error: {}", e),
                    crate::task_manager::TaskStatus::Failed,
                )
                .await
            {
                let error_msg = format!(
                    "Failed to update task status to Failed: {}. Task may be in inconsistent state.",
                    update_err
                );
                tracing::error!(
                    task_id = %task_id_clone,
                    error = %update_err,
                    "{}",
                    error_msg
                );
                // 发送错误事件给前端
                let _ = app_handle.emit("import-error", &error_msg);
            }
        }

        return Err(format!("Failed to process path: {}", e));
    }

    if let Some(ref task_manager) = task_manager_clone {
        let task_manager: &TaskManager = task_manager;
        if let Err(e) = task_manager
            .update_task_async(
                &task_id_clone,
                97,
                "Building search index...".to_string(),
                crate::task_manager::TaskStatus::Running,
            )
            .await
        {
            let error_msg = format!("Failed to update task progress during index build: {}", e);
            tracing::warn!(task_id = %task_id_clone, error = %e, "{}", error_msg);
            let _ = app_handle.emit("import-error", &error_msg);
        }
    }

    // Tantivy 索引重建：改为后台异步执行，不阻塞导入完成。
    // 主搜索链路（search_logs）不走 Tantivy，但 get_time_range 依赖它，
    // 因此保留索引但不在关键路径上重建。
    let _rebuild_handle = {
        let metadata_store = metadata_store.clone();
        let cas = Arc::clone(&cas);
        let search_manager = Arc::clone(&search_manager);
        let workspace_id = workspace_id_clone.clone();
        tokio::task::spawn(async move {
            if let Err(e) =
                rebuild_workspace_search_index(metadata_store, cas, search_manager).await
            {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "Background Tantivy index rebuild failed; get_time_range may be stale"
                );
            } else {
                tracing::info!(
                    workspace_id = %workspace_id,
                    "Background Tantivy index rebuild completed"
                );
            }
        })
    };

    // Verify integrity after import (Task 5.2)
    // This generates a validation report to ensure all imported files are accessible
    // and have valid hashes in the CAS
    // 老王备注：TaskManager.UpdateTask 会自动发送 task-update 事件，不需要重复发送
    let task_manager_clone: Option<TaskManager> = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(ref task_manager) = task_manager_clone {
        let task_manager: &TaskManager = task_manager;
        if let Err(e) = task_manager
            .update_task_async(
                &task_id_clone,
                95,
                "Verifying integrity...".to_string(),
                crate::task_manager::TaskStatus::Running,
            )
            .await
        {
            let error_msg = format!("Failed to update task progress during verification: {}", e);
            tracing::warn!(task_id = %task_id_clone, error = %e, "{}", error_msg);
            // 发送错误事件给前端
            let _ = app_handle.emit("import-error", &error_msg);
        }
    }

    match verify_after_import(&workspace_dir).await {
        Ok(report) => {
            if report.is_valid() {
                // Get file count from MetadataStore for logging
                let file_count = metadata_store.count_files().await.unwrap_or(0);

                tracing::info!(
                    workspace_id = %workspace_id_clone,
                    total_files = report.total_files,
                    valid_files = report.valid_files,
                    file_count = file_count,
                    "Import completed successfully with integrity verification"
                );

                // 导入成功后清除该工作区的搜索缓存，避免旧缓存返回过时结果
                {
                    let cache = {
                        let guard = state.cache_manager.lock();
                        guard.clone()
                    };
                    if let Err(e) = cache.invalidate_workspace_cache_async(&workspace_id_clone).await {
                        tracing::warn!(
                            workspace_id = %workspace_id_clone,
                            error = %e,
                            "Failed to invalidate workspace cache after import"
                        );
                    }
                }
            } else {
                tracing::warn!(
                    workspace_id = %workspace_id_clone,
                    total_files = report.total_files,
                    valid_files = report.valid_files,
                    invalid_files = report.invalid_files.len(),
                    missing_objects = report.missing_objects.len(),
                    corrupted_objects = report.corrupted_objects.len(),
                    "Integrity verification found issues"
                );

                // Emit validation report to frontend
                let _ = app_handle.emit(
                    "validation-report",
                    serde_json::json!({
                        "workspace_id": workspace_id_clone,
                        "report": report,
                    }),
                );
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to verify integrity after import: {}", e);
            tracing::error!(
                workspace_id = %workspace_id_clone,
                error = %e,
                "{}",
                error_msg
            );
            // 发送错误事件给前端
            let _ = app_handle.emit("import-error", &error_msg);
        }
    }

    // 导入完成，使用 TaskManager 更新任务状态
    // 老王备注：TaskManager.UpdateTask 会自动发送 task-update 事件，不需要重复发送
    let task_manager_clone: Option<TaskManager> = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(ref task_manager) = task_manager_clone {
        let task_manager: &TaskManager = task_manager;
        // 添加完整的错误处理和降级方案
        if let Err(e) = task_manager
            .update_task_async(
                &task_id_clone,
                100,
                "Done".to_string(),
                crate::task_manager::TaskStatus::Completed,
            )
            .await
        {
            let error_msg = format!(
                "Failed to update task status to Completed: {}. Task may be in inconsistent state.",
                e
            );
            tracing::error!(
                task_id = %task_id_clone,
                error = %e,
                "{}",
                error_msg
            );
            // 发送错误事件给前端
            let _ = app_handle.emit("import-error", &error_msg);
        }
    }

    // 导入完成后尝试触发 Tantivy segment 合并
    // 如果该工作区已有 SearchEngineManager，则等待后台 segment 合并完成
    // 这可以减少段数量，提升后续搜索性能
    {
        let search_engine_opt = Some(Arc::clone(&search_manager));
        if let Some(search_manager) = search_engine_opt {
            tracing::info!(
                workspace_id = %workspace_id_clone,
                "导入完成，开始等待 Tantivy segment 合并"
            );
            if let Err(e) = search_manager.commit_and_wait_merge().await {
                let error_msg = format!(
                    "Tantivy segment merge warning (non-critical): {}. Import completed successfully.",
                    e
                );
                tracing::warn!(
                    workspace_id = %workspace_id_clone,
                    error = %e,
                    "{}",
                    error_msg
                );
                // 发送警告事件给前端（非致命错误）
                let _ = app_handle.emit("import-warning", &error_msg);
            }
        }
    }

    let _ = app_handle.emit("import-complete", task_id_clone);

    Ok(task_id)
}

/// 检查 RAR 支持状态（无 sidecar 依赖）
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    serde_json::to_value(get_rar_support_info())
        .map_err(|error| format!("Failed to serialize RAR support info: {}", error))
}

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
