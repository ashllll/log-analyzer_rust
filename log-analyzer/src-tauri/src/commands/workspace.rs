//! 工作区管理命令
//!
//! 提供工作区的删除和管理功能,包括:
//! - 删除工作区及其所有相关资源
//! - 清理解压目录
//! - 清除内存状态
//! - 删除索引文件
//! - 工作区格式检测和迁移提示
//!
//! # 设计原则
//!
//! - 按正确的依赖顺序清理资源
//! - 单步失败不中断流程
//! - 提供友好的错误提示
//! - 支持重试和清理队列机制
//! - 向后兼容旧格式工作区

use crate::commands::import::import_folder;
use crate::migration::{detect_workspace_format, WorkspaceFormat};

/// Workspace load response with format information
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceLoadResponse {
    /// Whether the workspace was loaded successfully
    pub success: bool,
    /// Workspace format: "traditional", "cas", or "unknown"
    pub format: String,
    /// Whether the workspace needs migration
    pub needs_migration: bool,
    /// Number of files loaded
    pub file_count: usize,
}

/// 加载工作区索引
///
/// 支持向后兼容：
/// - 检测工作区格式（传统或CAS）
/// - 传统格式工作区可以正常加载（只读模式）
/// - 返回格式信息，提示用户迁移
#[command]
pub async fn load_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceLoadResponse, String> {
    let start_time = std::time::Instant::now();

    validate_workspace_id(&workspaceId)?;

    // Detect workspace format
    let format = detect_workspace_format(&workspaceId, &app).map_err(|e| e.to_string())?;
    let format_str = match format {
        WorkspaceFormat::Traditional => "traditional",
        WorkspaceFormat::CAS => "cas",
        WorkspaceFormat::Unknown => "unknown",
    };
    let needs_migration = format == WorkspaceFormat::Traditional;

    if needs_migration {
        eprintln!(
            "[INFO] [load_workspace] Workspace {} is in traditional format and can be migrated to CAS",
            workspaceId
        );
    }

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let mut index_path = index_dir.join(format!("{}.idx.gz", workspaceId));
    if !index_path.exists() {
        index_path = index_dir.join(format!("{}.idx", workspaceId));
        if !index_path.exists() {
            return Err(format!("Index not found for workspace: {}", workspaceId));
        }
    }

    let load_start = std::time::Instant::now();
    let (path_map, file_metadata) = load_index(&index_path).map_err(|e| e.to_string())?;
    let load_duration = load_start.elapsed();

    // 记录文件数量（在移动 path_map 之前）
    let file_count = path_map.len();

    {
        let mut map_guard = state.path_map.lock();
        let mut metadata_guard = state.file_metadata.lock();

        *map_guard = path_map;
        *metadata_guard = file_metadata;
    }

    {
        let mut indices_guard = state.workspace_indices.lock();
        indices_guard.insert(workspaceId.clone(), index_path);
    }

    // 记录性能指标
    let total_duration = start_time.elapsed();
    state.metrics_collector.record_workspace_operation(
        "load",
        &workspaceId,
        file_count,
        total_duration,
        vec![("index_load", load_duration)],
        true,
    );

    // 广播工作区加载完成事件
    // 注意：先克隆 state_sync，释放锁后再 await，避免跨 await 点持有锁
    let state_sync_opt = {
        let guard = state.state_sync.lock();
        guard.as_ref().cloned()
    };
    if let Some(state_sync) = state_sync_opt {
        use crate::state_sync::models::{WorkspaceEvent, WorkspaceStatus};
        use std::time::Duration;
        let _ = state_sync
            .broadcast_workspace_event(WorkspaceEvent::StatusChanged {
                workspace_id: workspaceId.clone(),
                status: WorkspaceStatus::Completed {
                    duration: Duration::from_secs(0),
                },
            })
            .await;
    }

    Ok(WorkspaceLoadResponse {
        success: true,
        format: format_str.to_string(),
        needs_migration,
        file_count,
    })
}

/// 增量刷新工作区索引
#[command]
pub async fn refresh_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
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

    // 使用 TaskManager 创建任务（异步版本，避免在 async 上下文中使用 block_on）
    let target_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    let task = if let Some(task_manager) = state.task_manager.lock().as_ref() {
        task_manager.create_task_async(
            task_id.clone(),
            "Refresh".to_string(),
            target_name.clone(),
            Some(workspaceId.clone()),
        ).await.map_err(|e| format!("Failed to create task: {}", e))?
    } else {
        return Err("Task manager not initialized".to_string());
    };

    // 发送初始任务事件
    let _ = app.emit("task-update", task.clone());

    // 同时发送到事件总线（向后兼容）
    let event_bus = get_event_bus();
    let _ = event_bus.publish_task_update(crate::models::TaskProgress {
        task_id: task.id.clone(),
        task_type: task.task_type.clone(),
        target: task.target.clone(),
        status: format!("{:?}", task.status).to_uppercase(),
        message: task.message.clone(),
        progress: task.progress,
        workspace_id: task.workspace_id.clone(),
    });

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let mut index_path = index_dir.join(format!("{}.idx.gz", workspaceId));
    if !index_path.exists() {
        index_path = index_dir.join(format!("{}.idx", workspaceId));
        if !index_path.exists() {
            return import_folder(app, path, workspaceId, state).await;
        }
    }

    // 使用 tauri::async_runtime::spawn 执行后台任务（成熟的异步模式）
    tauri::async_runtime::spawn(async move {
        let operation_start = std::time::Instant::now();
        let _file_name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // 使用 Result 而不是 panic::catch_unwind（更安全的错误处理）
        let result: Result<(), String> = async {
            let state = app_handle.state::<AppState>();

            let (mut existing_path_map, mut existing_metadata) =
                load_index(&index_path).map_err(|e| e.to_string())?;

            // 更新任务进度：扫描文件系统
            if let Some(task_manager) = state.task_manager.lock().as_ref() {
                if let Ok(Some(task)) = task_manager.update_task_async(
                    &task_id_clone,
                    20,
                    "Scanning file system...".to_string(),
                    crate::task_manager::TaskStatus::Running,
                ).await {
                    let _ = app_handle.emit("task-update", task);
                }
            }

            // 使用 spawn_blocking 处理阻塞的文件系统操作
            let current_files = tauri::async_runtime::spawn_blocking({
                let path = path.clone();
                move || {
                    let mut files: HashMap<String, FileMetadata> = HashMap::new();
                    let source_path = Path::new(&path);

                    for entry in WalkDir::new(source_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        let real_path = entry.path().to_string_lossy().to_string();
                        if let Ok(metadata) = get_file_metadata(entry.path()) {
                            files.insert(real_path, metadata);
                        }
                    }
                    files
                }
            }).await.map_err(|e| format!("Failed to scan file system: {}", e))?;

            // 更新任务进度：分析变更
            if let Some(task_manager) = state.task_manager.lock().as_ref() {
                if let Ok(Some(task)) = task_manager.update_task_async(
                    &task_id_clone,
                    40,
                    "Analyzing changes...".to_string(),
                    crate::task_manager::TaskStatus::Running,
                ).await {
                    let _ = app_handle.emit("task-update", task);
                }
            }

            let mut new_files: Vec<String> = Vec::new();
            let mut modified_files: Vec<String> = Vec::new();

            for (real_path, current_meta) in &current_files {
                if let Some(existing_meta) = existing_metadata.get(real_path) {
                    if existing_meta.modified_time != current_meta.modified_time
                        || existing_meta.size != current_meta.size
                    {
                        modified_files.push(real_path.clone());
                    }
                } else {
                    new_files.push(real_path.clone());
                }
            }

            let deleted_files: Vec<String> = existing_metadata
                .keys()
                .filter(|k| !current_files.contains_key(*k))
                .cloned()
                .collect();

            let total_changes = new_files.len() + modified_files.len() + deleted_files.len();

            if total_changes > 0 {
                // 更新任务进度：处理变更
                if let Some(task_manager) = state.task_manager.lock().as_ref() {
                    if let Ok(Some(task)) = task_manager.update_task_async(
                        &task_id_clone,
                        60,
                        format!("Processing {} changes...", total_changes),
                        crate::task_manager::TaskStatus::Running,
                    ).await {
                        let _ = app_handle.emit("task-update", task);
                    }
                }

                let temp_guard = state.temp_dir.lock();

                if let Some(ref _temp_dir) = *temp_guard {
                    let source_path = Path::new(&path);
                    let root_name = source_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();

                    let mut new_entries: HashMap<String, String> = HashMap::new();
                    let mut new_metadata_entries: HashMap<String, FileMetadata> = HashMap::new();

                    for real_path in &new_files {
                        let file_path = Path::new(real_path);
                        if let Ok(relative) = file_path.strip_prefix(source_path) {
                            let virtual_path = format!(
                                "{}/{}",
                                root_name,
                                relative.to_string_lossy().replace('\\', "/")
                            );
                            let normalized_virtual = normalize_path_separator(&virtual_path);

                            new_entries.insert(real_path.clone(), normalized_virtual);
                            if let Some(meta) = current_files.get(real_path) {
                                new_metadata_entries.insert(real_path.clone(), meta.clone());
                            }
                        }
                    }

                    for real_path in &modified_files {
                        let file_path = Path::new(real_path);
                        if let Ok(relative) = file_path.strip_prefix(source_path) {
                            let virtual_path = format!(
                                "{}/{}",
                                root_name,
                                relative.to_string_lossy().replace('\\', "/")
                            );
                            let normalized_virtual = normalize_path_separator(&virtual_path);

                            new_entries.insert(real_path.clone(), normalized_virtual);
                            if let Some(meta) = current_files.get(real_path) {
                                new_metadata_entries.insert(real_path.clone(), meta.clone());
                            }
                        }
                    }

                    for (k, v) in new_entries {
                        existing_path_map.insert(k, v);
                    }
                    for (k, v) in new_metadata_entries {
                        existing_metadata.insert(k, v);
                    }

                    for real_path in &deleted_files {
                        existing_path_map.remove(real_path);
                        existing_metadata.remove(real_path);
                    }
                }

                // 更新任务进度：保存索引
                if let Some(task_manager) = state.task_manager.lock().as_ref() {
                    if let Ok(Some(task)) = task_manager.update_task_async(
                        &task_id_clone,
                        80,
                        "Saving index...".to_string(),
                        crate::task_manager::TaskStatus::Running,
                    ).await {
                        let _ = app_handle.emit("task-update", task);
                    }
                }

                save_index(
                    &app_handle,
                    &workspace_id_clone,
                    &existing_path_map,
                    &existing_metadata,
                )
                .map_err(|e| e.to_string())?;

                let mut map_guard = state.path_map.lock();
                let mut metadata_guard = state.file_metadata.lock();

                *map_guard = existing_path_map;
                *metadata_guard = existing_metadata;
            }

            Ok(())
        }.await;

        let state = app_handle.state::<AppState>();

        if result.is_err() {
            // 更新任务状态为失败
            if let Some(task_manager) = state.task_manager.lock().as_ref() {
                if let Ok(Some(task)) = task_manager.update_task_async(
                    &task_id_clone,
                    0,
                    "Refresh failed".to_string(),
                    crate::task_manager::TaskStatus::Failed,
                ).await {
                    let _ = app_handle.emit("task-update", task);
                }
            }

            // 记录失败的性能指标
            state.metrics_collector.record_workspace_operation(
                "refresh",
                &workspace_id_clone,
                0,
                operation_start.elapsed(),
                vec![],
                false,
            );
        } else {
            // 更新任务状态为完成
            if let Some(task_manager) = state.task_manager.lock().as_ref() {
                if let Ok(Some(task)) = task_manager.update_task_async(
                    &task_id_clone,
                    100,
                    "Refresh complete".to_string(),
                    crate::task_manager::TaskStatus::Completed,
                ).await {
                    let _ = app_handle.emit("task-update", task);
                }
            }

            let _ = app_handle.emit("import-complete", task_id_clone.clone());

            // 失效该工作区的所有缓存（使用异步版本，避免在 async 上下文中调用 block_on）
            if let Err(e) = state
                .cache_manager
                .invalidate_workspace_cache_async(&workspace_id_clone)
                .await
            {
                eprintln!(
                    "[WARNING] Failed to invalidate cache for workspace {}: {}",
                    workspace_id_clone, e
                );
            } else {
                eprintln!(
                    "[INFO] Successfully invalidated cache for workspace: {}",
                    workspace_id_clone
                );
            }

            // 记录成功的性能指标
            let total_duration = operation_start.elapsed();
            state.metrics_collector.record_workspace_operation(
                "refresh",
                &workspace_id_clone,
                0,
                total_duration,
                vec![],
                true,
            );

            // 广播工作区刷新完成事件
            let state_sync_opt = {
                let guard = state.state_sync.lock();
                guard.as_ref().cloned()
            };
            if let Some(state_sync) = state_sync_opt {
                use crate::state_sync::models::{WorkspaceEvent, WorkspaceStatus};
                use std::time::Duration;
                let workspace_id_for_event = workspace_id_clone.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = state_sync
                        .broadcast_workspace_event(WorkspaceEvent::StatusChanged {
                            workspace_id: workspace_id_for_event,
                            status: WorkspaceStatus::Completed {
                                duration: Duration::from_secs(0),
                            },
                        })
                        .await;
                });
            }
        }
    });

    Ok(task_id)
}

use std::{collections::HashMap, fs, panic, path::Path};

use tauri::{command, AppHandle, Emitter, Manager, State};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::models::{AppState, FileMetadata};
use crate::services::{get_event_bus, get_file_metadata, load_index, save_index};
use crate::utils::{
    canonicalize_path, cleanup::try_cleanup_temp_dir, normalize_path_separator,
    validation::validate_workspace_id,
};

/// 判断工作区是否为解压类型
///
/// 检查解压目录是否存在来判断工作区类型。
///
/// # 参数
///
/// - `workspace_id` - 工作区ID
/// - `app` - Tauri应用句柄
///
/// # 返回值
///
/// - `Ok(true)` - 解压目录存在,为解压类型工作区
/// - `Ok(false)` - 解压目录不存在,为普通文件夹工作区
/// - `Err(String)` - 获取应用目录失败
///
/// # 示例
///
/// ```ignore
/// let is_extracted = is_extracted_workspace("workspace-123", &app)?;
/// if is_extracted {
///     // 需要删除解压目录
/// }
/// ```
fn is_extracted_workspace(workspace_id: &str, app: &AppHandle) -> Result<bool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let extracted_dir = app_data_dir.join("extracted").join(workspace_id);

    Ok(extracted_dir.exists())
}

/// 判断工作区是否使用CAS存储
///
/// 检查工作区目录下是否存在CAS相关文件（metadata.db或objects目录）
///
/// # 参数
///
/// - `workspace_id` - 工作区ID
/// - `app` - Tauri应用句柄
///
/// # 返回值
///
/// - `Ok(true)` - 工作区使用CAS存储
/// - `Ok(false)` - 工作区使用传统存储
/// - `Err(String)` - 获取应用目录失败
fn is_cas_workspace(workspace_id: &str, app: &AppHandle) -> Result<bool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let workspace_dir = app_data_dir.join("extracted").join(workspace_id);
    
    // Check for metadata.db or objects directory
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");
    
    Ok(metadata_db.exists() || objects_dir.exists())
}

/// 清理工作区资源
///
/// 按正确的依赖顺序清理工作区的所有相关资源。
///
/// # 清理顺序
///
/// 1. 停止文件监听器(释放目录句柄)
/// 2. 清除搜索缓存(依赖LRU自动淘汰,已优化为不主动清理)
/// 3. 清除内存状态(path_map, file_metadata, workspace_indices)
/// 4. 删除索引文件(磁盘文件)
/// 5. 删除解压目录(仅压缩文件工作区)
///
/// # 参数
///
/// - `workspace_id` - 工作区ID
/// - `state` - 全局状态引用
/// - `app` - Tauri应用句柄
///
/// # 返回值
///
/// - `Ok(())` - 清理成功
/// - `Err(String)` - 清理失败,返回错误信息
///
/// # 错误处理
///
/// 单步失败不中断流程,记录日志并继续清理其他资源。
fn cleanup_workspace_resources(
    workspace_id: &str,
    state: &AppState,
    app: &AppHandle,
) -> Result<(), String> {
    eprintln!(
        "[INFO] [delete_workspace] Starting resource cleanup for workspace: {}",
        workspace_id
    );

    let mut errors = Vec::new();

    // ===== 步骤1: 停止文件监听器 =====
    eprintln!(
        "[INFO] [delete_workspace] Step 1: Stopping file watcher for workspace: {}",
        workspace_id
    );
    {
        let mut watchers = state.watchers.lock();
        if let Some(mut watcher_state) = watchers.remove(workspace_id) {
            watcher_state.is_active = false;
            eprintln!(
                "[INFO] [delete_workspace] File watcher stopped for workspace: {}",
                workspace_id
            );
        } else {
            eprintln!(
                "[INFO] [delete_workspace] No active watcher found for workspace: {}",
                workspace_id
            );
        }
    }

    // ===== 步骤2: 清除搜索缓存 =====
    // 优化决策: 不主动清理搜索缓存,依赖LRU自动淘汰机制
    // 这样可以避免遍历缓存键的性能开销
    eprintln!(
        "[INFO] [delete_workspace] Step 2: Skipping search cache cleanup (LRU auto-eviction)"
    );

    // ===== 步骤3: 清除内存状态 =====
    eprintln!(
        "[INFO] [delete_workspace] Step 3: Clearing memory state for workspace: {}",
        workspace_id
    );

    // 3.1 从 workspace_indices 读取索引文件，精准删除 path_map 与 file_metadata 项
    let indexed_paths: Option<Vec<String>> = {
        let indices = state.workspace_indices.lock();
        match indices.get(workspace_id) {
            Some(index_path) => match load_index(index_path) {
                Ok((paths, _meta)) => {
                    let path_keys: Vec<String> = paths.keys().cloned().collect();
                    eprintln!(
                        "[INFO] [delete_workspace] Step 3.1: Loaded {} paths from index",
                        path_keys.len()
                    );
                    Some(path_keys)
                }
                Err(e) => {
                    let error =
                        format!("Failed to load index file {}: {}", index_path.display(), e);
                    eprintln!("[ERROR] [delete_workspace] Step 3.1 failed: {}", error);
                    errors.push(error);
                    None
                }
            },
            None => {
                eprintln!("[INFO] [delete_workspace] Step 3.1: No index path found for workspace");
                None
            }
        }
    };

    // 3.2 清除 path_map 与 file_metadata（仅在成功获取路径列表时执行）
    if let Some(paths_to_remove) = indexed_paths {
        if paths_to_remove.is_empty() {
            eprintln!("[INFO] [delete_workspace] Step 3.2: No paths to remove from path_map");
        } else {
            {
                let mut path_map = state.path_map.lock();
                let before_count = path_map.len();
                let mut removed_count = 0;
                for p in &paths_to_remove {
                    if path_map.remove(p).is_some() {
                        removed_count += 1;
                    }
                }
                let after_count = path_map.len();
                eprintln!(
                    "[INFO] [delete_workspace] Step 3.2: Removed {} entries from path_map ({} -> {})",
                    removed_count, before_count, after_count
                );

                // 验证清理结果
                if removed_count != paths_to_remove.len() {
                    eprintln!(
                        "[WARNING] [delete_workspace] Step 3.2: Expected to remove {} entries, but only removed {}",
                        paths_to_remove.len(), removed_count
                    );
                }
            }
        }

        if paths_to_remove.is_empty() {
            eprintln!("[INFO] [delete_workspace] Step 3.3: No paths to remove from file_metadata");
        } else {
            {
                let mut file_metadata = state.file_metadata.lock();
                let before_count = file_metadata.len();
                let mut removed_count = 0;
                for p in &paths_to_remove {
                    if file_metadata.remove(p).is_some() {
                        removed_count += 1;
                    }
                }
                let after_count = file_metadata.len();
                eprintln!(
                    "[INFO] [delete_workspace] Step 3.3: Removed {} entries from file_metadata ({} -> {})",
                    removed_count, before_count, after_count
                );

                // 验证清理结果
                if removed_count != paths_to_remove.len() {
                    eprintln!(
                        "[WARNING] [delete_workspace] Step 3.3: Expected to remove {} entries, but only removed {}",
                        paths_to_remove.len(), removed_count
                    );
                }
            }
        }
    } else {
        eprintln!("[WARNING] [delete_workspace] Step 3.2-3.3: Skipping path_map and file_metadata cleanup due to index loading failure");
    }

    // 3.3 清除 workspace_indices（最后移除记录）
    {
        let mut workspace_indices = state.workspace_indices.lock();
        if workspace_indices.remove(workspace_id).is_some() {
            eprintln!("[INFO] [delete_workspace] Removed workspace from workspace_indices");
        } else {
            eprintln!("[INFO] [delete_workspace] Workspace not found in workspace_indices");
        }
    }

    eprintln!("[INFO] [delete_workspace] Step 3 completed: Memory state cleared");

    // ===== 步骤4: 删除索引文件 =====
    eprintln!(
        "[INFO] [delete_workspace] Step 4: Deleting index files for workspace: {}",
        workspace_id
    );

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    // 尝试删除压缩版本和未压缩版本(兼容性)
    let compressed_index = index_dir.join(format!("{}.idx.gz", workspace_id));
    let uncompressed_index = index_dir.join(format!("{}.idx", workspace_id));

    let mut deleted_count = 0;
    let mut failed_count = 0;

    for index_path in [compressed_index, uncompressed_index] {
        if index_path.exists() {
            match fs::metadata(&index_path) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        match fs::remove_file(&index_path) {
                            Ok(_) => {
                                deleted_count += 1;
                                eprintln!(
                                    "[INFO] [delete_workspace] Deleted index file: {} ({} bytes)",
                                    index_path.display(),
                                    metadata.len()
                                );
                            }
                            Err(e) => {
                                failed_count += 1;
                                let error = format!(
                                    "Failed to delete index file {}: {}",
                                    index_path.display(),
                                    e
                                );
                                eprintln!("[ERROR] [delete_workspace] {}", error);
                                errors.push(error);
                            }
                        }
                    } else {
                        eprintln!(
                            "[WARNING] [delete_workspace] Path {} exists but is not a regular file, skipping",
                            index_path.display()
                        );
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    let error = format!(
                        "Failed to get metadata for index file {}: {}",
                        index_path.display(),
                        e
                    );
                    eprintln!("[ERROR] [delete_workspace] {}", error);
                    errors.push(error);
                }
            }
        } else {
            eprintln!(
                "[INFO] [delete_workspace] Index file does not exist, skipping: {}",
                index_path.display()
            );
        }
    }

    eprintln!(
        "[INFO] [delete_workspace] Step 4 completed: {} index files deleted, {} failures",
        deleted_count, failed_count
    );

    // 如果关键文件删除失败，记录警告但不中断清理流程
    if failed_count > 0 && deleted_count == 0 {
        eprintln!(
            "[WARNING] [delete_workspace] All index file deletions failed for workspace: {}",
            workspace_id
        );
    }

    // ===== 步骤5: 删除解压目录 =====
    eprintln!("[INFO] [delete_workspace] Step 5: Checking for extracted directory");

    match is_extracted_workspace(workspace_id, app) {
        Ok(true) => {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            let extracted_dir = app_data_dir.join("extracted").join(workspace_id);

            eprintln!(
                "[INFO] [delete_workspace] Attempting to delete extracted directory: {}",
                extracted_dir.display()
            );

            // Check if this is a CAS workspace
            match is_cas_workspace(workspace_id, app) {
                Ok(true) => {
                    eprintln!("[INFO] [delete_workspace] Detected CAS workspace, cleaning up CAS resources");
                    
                    // Step 5.1: Delete SQLite database
                    let metadata_db = extracted_dir.join("metadata.db");
                    if metadata_db.exists() {
                        match fs::remove_file(&metadata_db) {
                            Ok(_) => {
                                eprintln!(
                                    "[INFO] [delete_workspace] Deleted SQLite database: {}",
                                    metadata_db.display()
                                );
                            }
                            Err(e) => {
                                let error = format!(
                                    "Failed to delete SQLite database {}: {}",
                                    metadata_db.display(),
                                    e
                                );
                                eprintln!("[ERROR] [delete_workspace] {}", error);
                                errors.push(error);
                            }
                        }
                    }
                    
                    // Also try to delete SQLite journal files
                    for journal_ext in &["-wal", "-shm", "-journal"] {
                        let journal_file = extracted_dir.join(format!("metadata.db{}", journal_ext));
                        if journal_file.exists() {
                            if let Err(e) = fs::remove_file(&journal_file) {
                                eprintln!(
                                    "[WARNING] [delete_workspace] Failed to delete journal file {}: {}",
                                    journal_file.display(),
                                    e
                                );
                            }
                        }
                    }
                    
                    // Step 5.2: Delete CAS objects directory
                    let objects_dir = extracted_dir.join("objects");
                    if objects_dir.exists() {
                        match fs::remove_dir_all(&objects_dir) {
                            Ok(_) => {
                                eprintln!(
                                    "[INFO] [delete_workspace] Deleted CAS objects directory: {}",
                                    objects_dir.display()
                                );
                            }
                            Err(e) => {
                                let error = format!(
                                    "Failed to delete CAS objects directory {}: {}",
                                    objects_dir.display(),
                                    e
                                );
                                eprintln!("[ERROR] [delete_workspace] {}", error);
                                errors.push(error);
                            }
                        }
                    }
                }
                Ok(false) => {
                    eprintln!("[INFO] [delete_workspace] Traditional workspace, no CAS cleanup needed");
                }
                Err(e) => {
                    eprintln!("[WARNING] [delete_workspace] Failed to check CAS status: {}", e);
                }
            }

            // Use cleanup tool's retry mechanism for the entire extracted directory
            try_cleanup_temp_dir(&extracted_dir, &state.cleanup_queue);

            // Note: try_cleanup_temp_dir handles failures internally
            // Failed deletions are automatically added to cleanup queue

            eprintln!(
                "[INFO] [delete_workspace] Step 5 completed: Extracted directory cleanup initiated"
            );
        }
        Ok(false) => {
            eprintln!("[INFO] [delete_workspace] Step 5 skipped: Not an extracted workspace");
        }
        Err(e) => {
            let error = format!("Failed to check if workspace is extracted: {}", e);
            eprintln!("[WARNING] [delete_workspace] Step 5 failed: {}", error);
            errors.push(error);
        }
    }

    // ===== 汇总结果 =====
    if errors.is_empty() {
        eprintln!(
            "[INFO] [delete_workspace] All cleanup steps completed successfully for workspace: {}",
            workspace_id
        );
        Ok(())
    } else {
        let error_summary = errors.join("; ");
        eprintln!(
            "[WARNING] [delete_workspace] Cleanup completed with {} errors: {}",
            errors.len(),
            error_summary
        );
        // 部分资源清理失败不影响整体删除操作
        // 主要资源(内存状态)已清理,用户可以正常使用
        Ok(())
    }
}

/// 删除工作区命令
///
/// Tauri命令接口,用于删除工作区及其所有相关资源。
///
/// # 参数
///
/// - `workspaceId` - 工作区ID(需符合验证规则)
/// - `state` - 全局状态
/// - `app` - Tauri应用句柄
///
/// # 返回值
///
/// - `Ok(())` - 删除成功
/// - `Err(String)` - 删除失败,返回错误信息
///
/// # 错误码
///
/// - "Workspace ID cannot be empty" - 工作区ID为空
/// - "Workspace ID contains invalid characters" - 工作区ID包含非法字符
/// - 其他错误信息由cleanup_workspace_resources返回
///
/// # 示例(前端调用)
///
/// ```typescript
/// await invoke('delete_workspace', { workspaceId: 'workspace-123' });
/// ```
#[command]
pub async fn delete_workspace(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let start_time = std::time::Instant::now();

    eprintln!(
        "[INFO] [delete_workspace] Command called for workspace: {}",
        workspaceId
    );

    // 参数验证
    validate_workspace_id(&workspaceId)?;

    // 执行清理
    cleanup_workspace_resources(&workspaceId, &state, &app)?;

    // 失效该工作区的所有缓存（使用异步版本，避免在 async 上下文中调用 block_on）
    if let Err(e) = state
        .cache_manager
        .invalidate_workspace_cache_async(&workspaceId)
        .await
    {
        eprintln!(
            "[WARNING] Failed to invalidate cache for workspace {}: {}",
            workspaceId, e
        );
    } else {
        eprintln!(
            "[INFO] Successfully invalidated cache for workspace: {}",
            workspaceId
        );
    }

    // 记录性能指标
    let total_duration = start_time.elapsed();
    state.metrics_collector.record_workspace_operation(
        "delete",
        &workspaceId,
        0,
        total_duration,
        vec![],
        true,
    );

    // 广播工作区删除事件
    // 注意：先克隆 state_sync，释放锁后再 await，避免跨 await 点持有锁
    let state_sync_opt = {
        let guard = state.state_sync.lock();
        guard.as_ref().cloned()
    };
    if let Some(state_sync) = state_sync_opt {
        use crate::state_sync::models::{WorkspaceEvent, WorkspaceStatus};
        use std::time::SystemTime;
        let _ = state_sync
            .broadcast_workspace_event(WorkspaceEvent::StatusChanged {
                workspace_id: workspaceId.clone(),
                status: WorkspaceStatus::Cancelled {
                    cancelled_at: SystemTime::now(),
                },
            })
            .await;
    }

    eprintln!(
        "[INFO] [delete_workspace] Command completed for workspace: {}",
        workspaceId
    );

    Ok(())
}
