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
//! ```rust
//! #[allow(non_snake_case)]
//! pub async fn load_workspace(
//!     workspaceId: String,  // 对应前端 invoke('load_workspace', { workspaceId })
//!     // ...
//! ) -> Result<WorkspaceLoadResponse, String>
//! ```
//!
//! 对应的前端调用：
//! ```typescript
//! await invoke('load_workspace', { workspaceId: 'workspace-123' });
//! ```

/// Workspace load response
#[derive(Debug, Clone, serde::Serialize)]
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
#[command]
pub async fn load_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String, // 对应前端 invoke('load_workspace', { workspaceId })
    state: State<'_, AppState>,
) -> Result<WorkspaceLoadResponse, String> {
    validate_workspace_id(&workspaceId)?;

    // Get workspace directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let workspace_dir = app_data_dir.join("extracted").join(&workspaceId);

    // Check if workspace exists and is CAS format
    if !workspace_dir.exists() {
        return Err(format!("Workspace not found: {}", workspaceId));
    }

    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    if !metadata_db.exists() || !objects_dir.exists() {
        return Err(format!(
            "Workspace {} is not in CAS format. Please create a new workspace.",
            workspaceId
        ));
    }

    // Open metadata store and get file count
    let metadata_store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to open metadata store: {}", e))?;

    let file_count = metadata_store
        .count_files()
        .await
        .map_err(|e| format!("Failed to count files: {}", e))? as usize;

    // Broadcast workspace loaded event
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
        file_count,
    })
}

/// 增量刷新工作区索引
///
/// 注意：CAS架构下，刷新操作等同于重新导入
/// 因为CAS自动处理去重，重新导入是最简单可靠的方式
#[command]
pub async fn refresh_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!(
        workspace_id = %workspaceId,
        path = %path,
        "Refresh requested for workspace"
    );

    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Get workspace directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let workspace_dir = app_data_dir.join("extracted").join(&workspaceId);

    // Check if workspace exists and is CAS format
    if !workspace_dir.exists() {
        info!("Workspace not found, performing fresh import");
        return import_folder(app, path, workspaceId, state).await;
    }

    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    if !metadata_db.exists() || !objects_dir.exists() {
        info!("Workspace is not CAS format, performing fresh import");
        return import_folder(app, path, workspaceId, state).await;
    }

    // For CAS workspaces, refresh is equivalent to re-import
    // CAS handles deduplication automatically, so re-importing is safe and simple
    info!("CAS workspace detected, re-importing for refresh");

    import_folder(app, path, workspaceId, state).await
}

use std::{fs, path::Path};

use tauri::{command, AppHandle, Manager, State};
use tracing::{error, info, warn};

use crate::commands::import::import_folder;
use crate::models::AppState;
use crate::utils::{cleanup::try_cleanup_temp_dir, validation::validate_workspace_id};

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
/// 3. 清除内存状态(CAS架构下无需清理)
/// 4. 删除旧的索引文件(向后兼容)
/// 5. 删除解压目录(包括CAS数据)
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
    info!(
        workspace_id = %workspace_id,
        "Starting resource cleanup for workspace"
    );

    let mut errors = Vec::new();

    // ===== 步骤1: 停止文件监听器 =====
    info!(
        workspace_id = %workspace_id,
        "Step 1: Stopping file watcher"
    );
    {
        let mut watchers = state.watchers.lock();
        if let Some(mut watcher_state) = watchers.remove(workspace_id) {
            watcher_state.is_active = false;
            info!(
                workspace_id = %workspace_id,
                "File watcher stopped"
            );
        } else {
            info!(
                workspace_id = %workspace_id,
                "No active watcher found"
            );
        }
    }

    // ===== 步骤2: 清除搜索缓存 =====
    // 优化决策: 不主动清理搜索缓存,依赖LRU自动淘汰机制
    // 这样可以避免遍历缓存键的性能开销
    info!("Step 2: Skipping search cache cleanup (LRU auto-eviction)");

    // ===== 步骤3: 清除内存状态 =====
    // CAS架构下，不需要清理全局的 path_map 和 file_metadata
    // 这些已经在之前的任务中被移除
    info!("Step 3: No memory state to clear (CAS architecture)");

    info!("Step 3 completed: Memory state cleared");

    // ===== 步骤4: 清理旧的索引文件（如果存在）=====
    // 为了向后兼容，检查并删除旧的 .idx.gz 文件
    info!("Step 4: Checking for legacy index files");

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
                                info!(
                                    path = %index_path.display(),
                                    size = metadata.len(),
                                    "Deleted legacy index file"
                                );
                            }
                            Err(e) => {
                                failed_count += 1;
                                let error = format!(
                                    "Failed to delete legacy index file {}: {}",
                                    index_path.display(),
                                    e
                                );
                                error!(error = %error);
                                errors.push(error);
                            }
                        }
                    } else {
                        warn!(
                            path = %index_path.display(),
                            "Path exists but is not a regular file, skipping"
                        );
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    let error = format!(
                        "Failed to get metadata for legacy index file {}: {}",
                        index_path.display(),
                        e
                    );
                    error!(error = %error);
                    errors.push(error);
                }
            }
        } else {
            info!(
                path = %index_path.display(),
                "Legacy index file does not exist, skipping"
            );
        }
    }

    if deleted_count > 0 {
        info!(
            count = deleted_count,
            "Step 4 completed: legacy index files deleted"
        );
    } else {
        info!("Step 4 completed: No legacy index files found");
    }

    // 如果关键文件删除失败，记录警告但不中断清理流程
    if failed_count > 0 && deleted_count == 0 {
        warn!(
            workspace_id = %workspace_id,
            "All legacy index file deletions failed"
        );
    }

    // ===== 步骤5: 删除解压目录 =====
    info!("Step 5: Checking for extracted directory");

    match is_extracted_workspace(workspace_id, app) {
        Ok(true) => {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            let extracted_dir = app_data_dir.join("extracted").join(workspace_id);

            info!(
                path = %extracted_dir.display(),
                "Attempting to delete extracted directory"
            );

            // Check if this is a CAS workspace
            match is_cas_workspace(workspace_id, app) {
                Ok(true) => {
                    info!("Detected CAS workspace, cleaning up CAS resources");

                    // Step 5.1: Delete SQLite database
                    let metadata_db = extracted_dir.join("metadata.db");
                    if metadata_db.exists() {
                        match fs::remove_file(&metadata_db) {
                            Ok(_) => {
                                info!(
                                    path = %metadata_db.display(),
                                    "Deleted SQLite database"
                                );
                            }
                            Err(e) => {
                                let error = format!(
                                    "Failed to delete SQLite database {}: {}",
                                    metadata_db.display(),
                                    e
                                );
                                error!(error = %error);
                                errors.push(error);
                            }
                        }
                    }

                    // Also try to delete SQLite journal files
                    for journal_ext in &["-wal", "-shm", "-journal"] {
                        let journal_file =
                            extracted_dir.join(format!("metadata.db{}", journal_ext));
                        if journal_file.exists() {
                            if let Err(e) = fs::remove_file(&journal_file) {
                                warn!(
                                    path = %journal_file.display(),
                                    error = %e,
                                    "Failed to delete journal file"
                                );
                            }
                        }
                    }

                    // Step 5.2: Delete CAS objects directory
                    let objects_dir = extracted_dir.join("objects");
                    if objects_dir.exists() {
                        match fs::remove_dir_all(&objects_dir) {
                            Ok(_) => {
                                info!(
                                    path = %objects_dir.display(),
                                    "Deleted CAS objects directory"
                                );
                            }
                            Err(e) => {
                                let error = format!(
                                    "Failed to delete CAS objects directory {}: {}",
                                    objects_dir.display(),
                                    e
                                );
                                error!(error = %error);
                                errors.push(error);
                            }
                        }
                    }
                }
                Ok(false) => {
                    info!("Traditional workspace, no CAS cleanup needed");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to check CAS status");
                }
            }

            // Use cleanup tool's retry mechanism for the entire extracted directory
            try_cleanup_temp_dir(&extracted_dir, &state.cleanup_queue);

            // Note: try_cleanup_temp_dir handles failures internally
            // Failed deletions are automatically added to cleanup queue

            info!("Step 5 completed: Extracted directory cleanup initiated");
        }
        Ok(false) => {
            info!("Step 5 skipped: Not an extracted workspace");
        }
        Err(e) => {
            let error = format!("Failed to check if workspace is extracted: {}", e);
            warn!(error = %error, "Step 5 failed");
            errors.push(error);
        }
    }

    // ===== 汇总结果 =====
    if errors.is_empty() {
        info!(
            workspace_id = %workspace_id,
            "All cleanup steps completed successfully"
        );
        Ok(())
    } else {
        let error_summary = errors.join("; ");
        warn!(
            workspace_id = %workspace_id,
            error_count = errors.len(),
            error_summary = %error_summary,
            "Cleanup completed with errors"
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
    info!(
        workspace_id = %workspaceId,
        "Delete workspace command called"
    );

    // 参数验证
    validate_workspace_id(&workspaceId)?;

    // 执行清理
    cleanup_workspace_resources(&workspaceId, &state, &app)?;

    // 失效该工作区的所有缓存
    // 使用 CacheManager 的 clear 方法
    {
        let cache = state.cache_manager.lock();
        cache.clear();
    }
    info!(
        workspace_id = %workspaceId,
        "Successfully invalidated cache for workspace"
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

    info!(
        workspace_id = %workspaceId,
        "Delete workspace command completed"
    );

    Ok(())
}

/// 取消任务命令
///
/// 将任务状态设置为 Stopped
///
/// # 参数
///
/// - `taskId` - 任务ID
/// - `state` - 全局状态
///
/// # 返回值
///
/// - `Ok(())` - 取消成功
/// - `Err(String)` - 取消失败
#[command]
pub async fn cancel_task(
    #[allow(non_snake_case)] taskId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(
        task_id = %taskId,
        "Cancel task command called"
    );

    let task_manager = state
        .task_manager
        .lock()
        .as_ref()
        .ok_or_else(|| "Task manager not initialized".to_string())?
        .clone();

    // 更新任务状态为 Stopped
    let _ = task_manager
        .update_task_async(
            &taskId,
            0, // progress 保持不变
            "Task cancelled by user".to_string(),
            crate::task_manager::TaskStatus::Stopped,
        )
        .await
        .map_err(|e| format!("Failed to cancel task: {}", e))?;

    info!(
        task_id = %taskId,
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
///
/// # 参数
///
/// - `workspaceId` - 工作区ID
/// - `app` - Tauri应用句柄
/// - `state` - 全局状态
///
/// # 返回值
///
/// - `Ok(WorkspaceStatusResponse)` - 工作区状态信息
/// - `Err(String)` - 获取失败
#[command]
pub async fn get_workspace_status(
    #[allow(non_snake_case)] workspaceId: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WorkspaceStatusResponse, String> {
    validate_workspace_id(&workspaceId)?;

    // 获取工作区目录
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let workspace_dir = app_data_dir.join("extracted").join(&workspaceId);

    // 检查工作区是否存在
    if !workspace_dir.exists() {
        return Err(format!("Workspace not found: {}", workspaceId));
    }

    // 检查是否为 CAS 格式
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    let is_cas = metadata_db.exists() && objects_dir.exists();

    if !is_cas {
        return Err(format!(
            "Workspace {} is not in CAS format. Please create a new workspace.",
            workspaceId
        ));
    }

    // 获取 MetadataStore
    let metadata_store = state
        .metadata_stores
        .lock()
        .get(&workspaceId)
        .cloned()
        .ok_or_else(|| format!("Workspace store not initialized: {}", workspaceId))?;

    let file_count = metadata_store.count_files().await.unwrap_or(0);

    // 计算目录大小
    let total_size = walkdir::WalkDir::new(&workspace_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum::<u64>();

    let size_mb = total_size / (1024 * 1024);
    let size_str = if size_mb > 1024 {
        format!("{:.1}GB", size_mb as f64 / 1024.0)
    } else {
        format!("{}MB", size_mb)
    };

    Ok(WorkspaceStatusResponse {
        id: workspaceId.clone(),
        name: workspaceId, // TODO: 从配置中读取实际名称
        status: "READY".to_string(),
        size: size_str,
        files: file_count as usize,
    })
}

/// 创建工作区命令（import_folder 的语义化别名）
///
/// 提供更符合用户预期的命令名来创建工作区
///
/// # 参数
///
/// - `name` - 工作区名称
/// - `path` - 文件夹路径
/// - `app` - Tauri应用句柄
/// - `state` - 全局状态
///
/// # 返回值
///
/// - `Ok(String)` - 返回任务ID
/// - `Err(String)` - 创建失败
#[command]
pub async fn create_workspace(
    name: String,
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!(
        name = %name,
        path = %path,
        "Create workspace command called"
    );

    // 验证路径存在
    let path_obj = std::path::Path::new(&path);
    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // 生成 workspace ID（使用名称作为基础，转换为合法 ID）
    let workspace_id = format!("ws-{}", name.to_lowercase().replace([' ', '/', '\\'], "-"));

    // 调用 import_folder 逻辑
    import_folder(app, path, workspace_id, state).await
}
