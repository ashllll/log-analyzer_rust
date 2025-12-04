//! 工作区管理命令
//!
//! 提供工作区的删除和管理功能,包括:
//! - 删除工作区及其所有相关资源
//! - 清理解压目录
//! - 清除内存状态
//! - 删除索引文件
//!
//! # 设计原则
//!
//! - 按正确的依赖顺序清理资源
//! - 单步失败不中断流程
//! - 提供友好的错误提示
//! - 支持重试和清理队列机制

use std::fs;
use tauri::{command, AppHandle, Manager, State};

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
    match state.watchers.lock() {
        Ok(mut watchers) => {
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
        Err(e) => {
            let error = format!("Failed to lock watchers: {}", e);
            eprintln!("[WARNING] [delete_workspace] Step 1 failed: {}", error);
            errors.push(error);
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

    // 3.1 清除 path_map
    match state.path_map.lock() {
        Ok(mut path_map) => {
            let before_count = path_map.len();
            path_map.retain(|_, virtual_path| !virtual_path.starts_with(workspace_id));
            let removed = before_count - path_map.len();
            eprintln!(
                "[INFO] [delete_workspace] Removed {} entries from path_map",
                removed
            );
        }
        Err(e) => {
            let error = format!("Failed to lock path_map: {}", e);
            eprintln!("[WARNING] [delete_workspace] Step 3.1 failed: {}", error);
            errors.push(error);
        }
    }

    // 3.2 清除 file_metadata
    match state.file_metadata.lock() {
        Ok(mut file_metadata) => {
            let before_count = file_metadata.len();
            file_metadata.retain(|key, _| !key.starts_with(workspace_id));
            let removed = before_count - file_metadata.len();
            eprintln!(
                "[INFO] [delete_workspace] Removed {} entries from file_metadata",
                removed
            );
        }
        Err(e) => {
            let error = format!("Failed to lock file_metadata: {}", e);
            eprintln!("[WARNING] [delete_workspace] Step 3.2 failed: {}", error);
            errors.push(error);
        }
    }

    // 3.3 清除 workspace_indices
    match state.workspace_indices.lock() {
        Ok(mut workspace_indices) => {
            if workspace_indices.remove(workspace_id).is_some() {
                eprintln!("[INFO] [delete_workspace] Removed workspace from workspace_indices");
            } else {
                eprintln!("[INFO] [delete_workspace] Workspace not found in workspace_indices");
            }
        }
        Err(e) => {
            let error = format!("Failed to lock workspace_indices: {}", e);
            eprintln!("[WARNING] [delete_workspace] Step 3.3 failed: {}", error);
            errors.push(error);
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

    for index_path in [compressed_index, uncompressed_index] {
        if index_path.exists() {
            match fs::remove_file(&index_path) {
                Ok(_) => {
                    eprintln!(
                        "[INFO] [delete_workspace] Deleted index file: {}",
                        index_path.display()
                    );
                }
                Err(e) => {
                    let error = format!(
                        "Failed to delete index file {}: {}",
                        index_path.display(),
                        e
                    );
                    eprintln!("[WARNING] [delete_workspace] {}", error);
                    errors.push(error);
                }
            }
        }
    }

    eprintln!("[INFO] [delete_workspace] Step 4 completed: Index files processed");

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

            // 使用 cleanup 工具的重试机制
            try_cleanup_temp_dir(&extracted_dir, &state.cleanup_queue);

            // 注: try_cleanup_temp_dir 内部处理失败,失败时会自动加入清理队列
            // 不需要额外的错误处理

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
    eprintln!(
        "[INFO] [delete_workspace] Command called for workspace: {}",
        workspaceId
    );

    // 参数验证
    validate_workspace_id(&workspaceId)?;

    // 执行清理
    cleanup_workspace_resources(&workspaceId, &state, &app)?;

    eprintln!(
        "[INFO] [delete_workspace] Command completed for workspace: {}",
        workspaceId
    );

    Ok(())
}
