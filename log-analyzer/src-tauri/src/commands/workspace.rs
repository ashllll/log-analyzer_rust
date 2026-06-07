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

use std::{fs, path::Path, sync::Arc};

use la_core::error::{AppError, CommandError};
use tauri::{AppHandle, Manager, State};
use tracing::{error, info, warn};

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::commands::import::import_folder;
use crate::models::AppState;
use crate::utils::validation::validate_workspace_id;
use crate::utils::workspace_paths::resolve_workspace_dir;

/// 关闭工作区数据库连接（MetadataStore + SearchEngine）。
/// 消除 workspace.rs 和 main.rs 之间的重复 close 模式。
async fn close_workspace_databases(service: &WorkspaceServiceRef) {
    service.metadata_store().close().await;
    service.search_engine().close().await;
}
use uuid::Uuid;

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

fn build_workspace_id(name: &str) -> String {
    let mut slug = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "workspace" } else { slug };
    let suffix = Uuid::new_v4().to_string();
    let suffix = &suffix[..8];
    let max_slug_len = 50usize.saturating_sub("ws-".len() + "-".len() + suffix.len());
    let mut slug = slug.chars().take(max_slug_len).collect::<String>();
    slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        slug = "workspace".to_string();
    }

    format!("ws-{slug}-{suffix}")
}

/// 判断工作区是否使用CAS存储
///
/// 检查工作区目录下是否存在CAS相关文件（metadata.db或objects目录）
fn is_cas_workspace(workspace_id: &str, app: &AppHandle) -> Result<bool, CommandError> {
    let workspace_dir =
        resolve_workspace_dir(app, workspace_id).map_err(|e| CommandError::new("NOT_FOUND", e))?;

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
/// # 错误处理
///
/// 单步失败不中断流程,记录日志并继续清理其他资源。
async fn cleanup_workspace_resources(
    workspace_id: &str,
    state: &AppState,
    app: &AppHandle,
) -> Result<(), CommandError> {
    info!(
        workspace_id = %workspace_id,
        "Starting resource cleanup for workspace"
    );

    let mut errors = Vec::new();

    // 辅助闭包：统一将 io::Error 映射为结构化错误消息
    let io_err = |e: std::io::Error, path: &std::path::Path| -> String {
        AppError::io_error(e.to_string(), Some(path.to_path_buf())).to_string()
    };

    // 辅助函数：带重试的文件删除（处理 Windows 临时文件锁定）
    async fn remove_file_with_retry(path: &std::path::Path) -> Result<(), std::io::Error> {
        const MAX_ATTEMPTS: usize = 3;
        for attempt in 0..MAX_ATTEMPTS {
            match std::fs::remove_file(path) {
                Ok(()) => return Ok(()),
                Err(_) if attempt + 1 < MAX_ATTEMPTS => {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (attempt as u64 + 1),
                    ))
                    .await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!("remove_file_with_retry loop must return")
    }

    // 辅助函数：带重试的目录删除（处理 Windows 临时文件锁定）
    async fn remove_dir_with_retry(path: &std::path::Path) -> Result<(), std::io::Error> {
        const MAX_ATTEMPTS: usize = 3;
        for attempt in 0..MAX_ATTEMPTS {
            match std::fs::remove_dir_all(path) {
                Ok(()) => return Ok(()),
                Err(_) if attempt + 1 < MAX_ATTEMPTS => {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (attempt as u64 + 1),
                    ))
                    .await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!("remove_dir_with_retry loop must return")
    }

    // ===== 步骤1: 停止文件监听器 =====
    // P5 迁移：watcher 状态内嵌于 WorkspaceServiceImpl，通过 service.stop_watch() 清理
    info!(
        workspace_id = %workspace_id,
        "Step 1: Stopping file watcher"
    );
    if let Some(service) = state.get_workspace_service(workspace_id) {
        let _ = service.stop_watch().await; // watcher 可能未激活，忽略错误
        info!(
            workspace_id = %workspace_id,
            "File watcher stopped via service"
        );
    } else {
        info!(
            workspace_id = %workspace_id,
            "No workspace service found, skipping watcher stop"
        );
    }

    // ===== 步骤2: 清除搜索缓存 =====
    // 优化决策: 不主动清理搜索缓存,依赖LRU自动淘汰机制
    // 这样可以避免遍历缓存键的性能开销
    info!("Step 2: Skipping search cache cleanup (LRU auto-eviction)");

    // ===== 步骤3: 清除工作区运行态资源 =====
    info!("Step 3: Removing workspace runtime resources");

    // P4 迁移：先关闭资源，再统一清理（workspace_services 替代分散的旧 HashMap）
    if let Some(service) = state.get_workspace_service(workspace_id) {
        close_workspace_databases(&service).await;
        info!(workspace_id = %workspace_id, "Databases closed");
    }
    state.remove_workspace_service(workspace_id);
    info!("Step 3 completed: Workspace runtime resources removed");

    // ===== 步骤4: 清理旧的索引文件（如果存在）=====
    // 为了向后兼容，检查并删除旧的 .idx.gz 文件
    info!("Step 4: Checking for legacy index files");

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::new("IO_ERROR", format!("Failed to get app data dir: {e}")))?
        .join("indices");

    // 尝试删除压缩版本和未压缩版本(兼容性)
    let compressed_index = index_dir.join(format!("{workspace_id}.idx.gz"));
    let uncompressed_index = index_dir.join(format!("{workspace_id}.idx"));

    let mut deleted_count = 0;
    let mut failed_count = 0;

    for index_path in [compressed_index, uncompressed_index].iter() {
        let index_path: &std::path::PathBuf = index_path;
        let exists = index_path.exists();
        if exists {
            match fs::metadata(index_path) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        match fs::remove_file(index_path) {
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
                                let error = io_err(e, index_path);
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
                    let error = io_err(e, index_path);
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

    // ===== 步骤5: 删除工作区目录 =====
    info!("Step 5: Checking for workspace directory");

    match resolve_workspace_dir(app, workspace_id) {
        Ok(workspace_dir) if workspace_dir.exists() => {
            info!(
                path = %workspace_dir.display(),
                "Attempting to delete workspace directory"
            );

            match is_cas_workspace(workspace_id, app) {
                Ok(true) => {
                    info!("Detected CAS workspace, cleaning up CAS resources");

                    let metadata_db = workspace_dir.join("metadata.db");
                    if metadata_db.exists() {
                        if let Err(e) = remove_file_with_retry(&metadata_db).await {
                            let error = io_err(e, &metadata_db);
                            error!(error = %error);
                            errors.push(error);
                        } else {
                            info!(path = %metadata_db.display(), "Deleted SQLite database");
                        }
                    }

                    for journal_ext in &["-wal", "-shm", "-journal"] {
                        let journal_file = workspace_dir.join(format!("metadata.db{journal_ext}"));
                        if journal_file.exists() {
                            if let Err(e) = remove_file_with_retry(&journal_file).await {
                                warn!(
                                    path = %journal_file.display(),
                                    error = %e,
                                    "Failed to delete journal file"
                                );
                            }
                        }
                    }

                    let objects_dir = workspace_dir.join("objects");
                    if objects_dir.exists() {
                        if let Err(e) = remove_dir_with_retry(&objects_dir).await {
                            let error = io_err(e, &objects_dir);
                            error!(error = %error);
                            errors.push(error);
                        } else {
                            info!(path = %objects_dir.display(), "Deleted CAS objects directory");
                        }
                    }
                }
                Ok(false) => info!("Traditional workspace, no CAS cleanup needed"),
                Err(e) => warn!(error = %e, "Failed to check CAS status"),
            }

            if workspace_dir.exists() {
                if let Err(e) = remove_dir_with_retry(&workspace_dir).await {
                    let error = format!(
                        "Failed to delete workspace directory {}: {}",
                        workspace_dir.display(),
                        e
                    );
                    error!(error = %error);
                    errors.push(error);
                }
            }

            // 检查工作区目录是否仍然存在（验证删除是否成功）
            if workspace_dir.exists() {
                let error = format!(
                    "Failed to delete workspace directory {} after cleanup",
                    workspace_dir.display()
                );
                error!(error = %error);
                errors.push(error);
            } else {
                info!(
                    path = %workspace_dir.display(),
                    "Step 5 completed: Workspace directory successfully deleted"
                );
            }
        }
        Ok(_) => {
            info!("Step 5 skipped: Workspace directory does not exist");
        }
        Err(e) => {
            let error = format!("Failed to resolve workspace directory: {e}");
            warn!(error = %error, "Step 5 failed");
            errors.push(error);
        }
    }

    // ===== 汇总结果 =====
    // 区分严重错误和非严重错误：
    // - 严重错误：工作区目录仍然存在（用户无法重新导入同一工作区）
    // - 非严重错误：辅助文件（journal files 等）删除失败（不影响重新导入）
    let has_directory_exists_error = errors
        .iter()
        .any(|e| e.contains("Failed to delete workspace directory"));

    if errors.is_empty() {
        info!(
            workspace_id = %workspace_id,
            "All cleanup steps completed successfully"
        );
        Ok(())
    } else if has_directory_exists_error {
        let error_summary = errors.join("; ");
        error!(
            workspace_id = %workspace_id,
            error_count = errors.len(),
            error_summary = %error_summary,
            "Critical error: workspace directory still exists after cleanup"
        );
        // 工作区目录未删除，这是严重错误，必须返回错误
        Err(
            CommandError::new("CLEANUP_ERROR", "Workspace directory deletion failed")
                .with_help("Please close all programs that may have files open and retry")
                .with_details(serde_json::json!({ "details": error_summary })),
        )
    } else {
        let error_summary = errors.join("; ");
        warn!(
            workspace_id = %workspace_id,
            error_count = errors.len(),
            error_summary = %error_summary,
            "Cleanup completed with non-critical errors (auxiliary files)"
        );
        // 非严重错误：辅助文件删除失败，但工作区目录已删除，用户可以正常使用
        Ok(())
    }
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

    let file_count: i64 = service.metadata_store().count_files().await.unwrap_or(0);

    // 用 SQL 聚合查询替代目录遍历，O(1) 而非 O(n)
    let total_size: u64 = service
        .metadata_store()
        .sum_file_sizes()
        .await
        .unwrap_or(0)
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
