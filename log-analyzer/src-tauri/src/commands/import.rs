//! 导入相关命令实现
//! 包含工作区导入与 RAR 支持检查

use std::{fs, panic, path::Path, thread};

use tauri::{command, AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::archive::process_path_recursive_with_metadata;
use crate::models::{AppState, TaskProgress};
use crate::services::save_index;
use crate::utils::{canonicalize_path, validate_path_param, validate_workspace_id};

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

    let canonical_path = canonicalize_path(source_path).unwrap_or_else(|e| {
        eprintln!(
            "[WARNING] Path canonicalization failed: {}, using original path",
            e
        );
        source_path.to_path_buf()
    });

    let extracted_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("extracted")
        .join(&workspaceId);
    fs::create_dir_all(&extracted_dir)
        .map_err(|e| format!("Failed to create extracted dir: {}", e))?;

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.clone(),
            task_type: "Import".to_string(),
            target: canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string(),
            status: "RUNNING".to_string(),
            message: "Starting import...".to_string(),
            progress: 0,
            workspace_id: Some(workspaceId.clone()),
        },
    );

    {
        let mut map_guard = state
            .path_map
            .lock()
            .map_err(|e| format!("Failed to acquire path_map lock: {}", e))?;
        let mut metadata_guard = state
            .file_metadata
            .lock()
            .map_err(|e| format!("Failed to acquire metadata lock: {}", e))?;

        map_guard.clear();
        metadata_guard.clear();
    }

    thread::spawn(move || {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = app_handle.state::<AppState>();
            let source_path = Path::new(&path);
            let root_name = source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: root_name.to_string(),
                    status: "RUNNING".to_string(),
                    message: "Scanning...".to_string(),
                    progress: 10,
                    workspace_id: Some(workspace_id_clone.clone()),
                },
            );

            let mut map_guard = state
                .path_map
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            let mut metadata_guard = state
                .file_metadata
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?;

            process_path_recursive_with_metadata(
                source_path,
                &root_name,
                &extracted_dir,
                &mut map_guard,
                &mut metadata_guard,
                &app_handle,
                &task_id_clone,
                &workspace_id_clone,
            );

            match save_index(
                &app_handle,
                &workspace_id_clone,
                &map_guard,
                &metadata_guard,
            ) {
                Ok(index_path) => {
                    let mut indices_guard = state
                        .workspace_indices
                        .lock()
                        .map_err(|e| format!("Lock error: {}", e))?;
                    indices_guard.insert(workspace_id_clone.clone(), index_path);
                }
                Err(e) => {
                    eprintln!("[WARNING] Failed to save index: {}", e);
                }
            }

            Ok::<(), String>(())
        }));

        if let Err(_e) = result {
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name.clone(),
                    status: "FAILED".to_string(),
                    message: "Crashed".to_string(),
                    progress: 0,
                    workspace_id: Some(workspace_id_clone.clone()),
                },
            );
            let _ = app_handle.emit("import-error", "Backend process crashed");
        } else {
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name,
                    status: "COMPLETED".to_string(),
                    message: "Done".to_string(),
                    progress: 100,
                    workspace_id: Some(workspace_id_clone.clone()),
                },
            );
            let _ = app_handle.emit("import-complete", task_id_clone);
        }
    });

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
