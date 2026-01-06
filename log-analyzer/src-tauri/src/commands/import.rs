//! 导入相关命令实现
//! 包含工作区导入与 RAR 支持检查

use std::{fs, path::Path};

use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::error;
use uuid::Uuid;

use crate::archive::rar_handler::{get_unrar_path, validate_unrar_binary};
use crate::models::AppState;
use crate::storage::{verify_after_import, MetadataStore};
use crate::utils::{canonicalize_path, validate_path_param, validate_workspace_id};

/**
 * 检查 RAR 支持状态
 *
 * 实际检查 unrar 二进制文件是否存在并验证完整性（运行时验证）
 * 返回详细的诊断信息
 */
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    let unrar_path = get_unrar_path();
    let validation = validate_unrar_binary(&unrar_path);
    
    if !validation.exists {
        warn!(
            "unrar binary not found at: {}",
            unrar_path.display()
        );
    }
    
    if !validation.is_valid {
        for error in &validation.errors {
            warn!("unrar binary validation error: {}", error);
        }
    }
    
    Ok(serde_json::json!({
        "available": validation.is_valid,
        "path": unrar_path.display().to_string(),
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "file_exists": validation.exists,
        "is_executable": validation.is_executable,
        "version_info": validation.version_info,
        "validation_errors": validation.errors,
        "bundled": true,
        "install_guide": if !validation.is_valid {
            Some("unrar 二进制文件似乎缺失或已损坏。请从官方源重新安装应用程序。")
        } else {
            None
        }
    }))
}

    let canonical_path = match canonicalize_path(source_path) {
        Ok(path) => path,
        Err(e) => {
            tracing::warn!("Path canonicalization failed: {}, using original path", e);
            source_path.to_path_buf()
        }
    };

    let workspace_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("workspaces")
        .join(&workspaceId);

    fs::create_dir_all(&workspace_dir)
        .map_err(|e| format!("Failed to create workspace dir: {}", e))?;

    // 使用 TaskManager 创建任务
    let target_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    // Extract task_manager before await
    let task_manager = state.task_manager.lock().clone();
    let _task = if let Some(task_manager) = task_manager.as_ref() {
        task_manager
            .create_task_async(
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
    let task_manager_clone = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(task_manager) = task_manager_clone {
        let _ = task_manager
            .update_task_async(
                &task_id_clone,
                10,
                "Scanning...".to_string(),
                crate::task_manager::TaskStatus::Running,
            )
            .await;
    }

    // Initialize CAS and MetadataStore for this workspace
    let cas = {
        let mut cas_instances = state.cas_instances.lock();
        cas_instances
            .entry(workspace_id_clone.clone())
            .or_insert_with(|| {
                std::sync::Arc::new(crate::storage::ContentAddressableStorage::new(
                    workspace_dir.clone(),
                ))
            })
            .clone()
    };

    let metadata_store = {
        // 第一步：检查是否已存在
        let store_opt = {
            let metadata_stores = state.metadata_stores.lock();
            metadata_stores.get(&workspace_id_clone).cloned()
        };

        if let Some(store) = store_opt {
            store
        } else {
            // 第二步：创建新的 store（不持锁）
            let store = std::sync::Arc::new(
                MetadataStore::new(&workspace_dir)
                    .await
                    .map_err(|e| format!("Failed to create metadata store: {}", e))?,
            );

            // 第三步：插入到 map（短暂持锁）
            {
                let mut metadata_stores = state.metadata_stores.lock();
                metadata_stores.insert(workspace_id_clone.clone(), store.clone());
            }

            store
        }
    };

    // Store workspace directory mapping
    {
        let mut workspace_dirs = state.workspace_dirs.lock();
        workspace_dirs.insert(workspace_id_clone.clone(), workspace_dir.clone());
    }

    // Process the path using CAS architecture
    use crate::archive::processor::process_path_with_cas;

    if let Err(e) = process_path_with_cas(
        source_path,
        &root_name,
        &workspace_dir,
        &cas,
        metadata_store.clone(),
        &app_handle,
        &task_id_clone,
        &workspace_id_clone,
        None, // parent_archive_id
        0,    // depth_level
    )
    .await
    {
        error!(error = %e, "Failed to process path");

        // Update task with error
        let task_manager_clone = {
            let guard = state.task_manager.lock();
            guard.as_ref().cloned()
        };

        if let Some(task_manager) = task_manager_clone {
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
                tracing::error!(
                    task_id = %task_id_clone,
                    error = %update_err,
                    "Failed to update task status to Failed. Not sending fallback event."
                );
            }
        }

        return Err(format!("Failed to process path: {}", e));
    }

    // Verify integrity after import (Task 5.2)
    // This generates a validation report to ensure all imported files are accessible
    // and have valid hashes in the CAS
    // 老王备注：TaskManager.UpdateTask 会自动发送 task-update 事件，不需要重复发送
    let task_manager_clone = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(task_manager) = task_manager_clone {
        let _ = task_manager
            .update_task_async(
                &task_id_clone,
                95,
                "Verifying integrity...".to_string(),
                crate::task_manager::TaskStatus::Running,
            )
            .await;
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
            tracing::error!(
                workspace_id = %workspace_id_clone,
                error = %e,
                "Failed to verify integrity after import"
            );
        }
    }

    // 导入完成，使用 TaskManager 更新任务状态
    // 老王备注：TaskManager.UpdateTask 会自动发送 task-update 事件，不需要重复发送
    let task_manager_clone = {
        let guard = state.task_manager.lock();
        guard.as_ref().cloned()
    };

    if let Some(task_manager) = task_manager_clone {
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
            tracing::error!(
                task_id = %task_id_clone,
                error = %e,
                "Failed to update task status to Completed. Not sending fallback event."
            );
        }
    }

    let _ = app_handle.emit("import-complete", task_id_clone);

    Ok(task_id)
}

/// 检查RAR支持状态（内置unrar始终可用）
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "available": true,
        "install_guide": null,
        "bundled": true,
    }))
}
