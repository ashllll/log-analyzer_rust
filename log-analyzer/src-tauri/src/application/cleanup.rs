//! CleanupUseCase — 工作区资源清理用例
//!
//! 从 `commands/workspace.rs` 的 `cleanup_workspace_resources` 提取而来，
//! 遵循 Clean Architecture：commands 层委托给 application 用例层。

use std::path::Path;

use tauri::{AppHandle, Manager};
use tracing::{error, info, warn};

use crate::models::AppState;
use la_core::error::{AppError, CommandError};

/// 工作区资源清理用例
///
/// 按顺序执行 5 个清理步骤，单步失败不中断流程：
/// 1. 停止文件监听器
/// 2. 跳过搜索缓存清理（LRU 自动淘汰）
/// 3. 关闭数据库连接并移除 workspace service
/// 4. 删除旧版索引文件
/// 5. 删除工作区目录（含 CAS 子资源）
pub struct CleanupUseCase;

impl CleanupUseCase {
    /// 执行工作区资源清理
    pub async fn execute(
        workspace_id: &str,
        state: &AppState,
        app: &AppHandle,
    ) -> Result<(), CommandError> {
        info!(workspace_id = %workspace_id, "Starting resource cleanup for workspace");

        let mut errors = Vec::new();
        let mut has_critical_error = false;

        // Step 1: 停止文件监听器
        Self::stop_watcher(workspace_id, state).await;

        // Step 2: 搜索缓存由 LRU 自动淘汰，无需主动清理
        info!("Step 2: Skipping search cache cleanup (LRU auto-eviction)");

        // Step 3: 关闭数据库连接并移除 workspace service
        Self::close_databases(workspace_id, state).await;

        // Step 4: 删除旧版索引文件
        Self::delete_legacy_indices(workspace_id, app, &mut errors);

        // Step 5: 删除工作区目录
        Self::delete_workspace_directory(workspace_id, app, &mut errors, &mut has_critical_error)
            .await;

        // 汇总结果
        Self::summarize_results(workspace_id, errors, has_critical_error)
    }

    /// Step 1: 停止文件监听器
    async fn stop_watcher(workspace_id: &str, state: &AppState) {
        info!(workspace_id = %workspace_id, "Step 1: Stopping file watcher");
        if let Some(service) = state.get_workspace_service(workspace_id) {
            if let Err(e) = service.stop_watch().await {
                warn!(workspace_id = %workspace_id, error = %e, "Failed to stop file watcher");
            } else {
                info!(workspace_id = %workspace_id, "File watcher stopped via service");
            }
        } else {
            info!(workspace_id = %workspace_id, "No workspace service found, skipping watcher stop");
        }
    }

    /// Step 3: 关闭数据库连接并移除 workspace service
    async fn close_databases(workspace_id: &str, state: &AppState) {
        info!("Step 3: Removing workspace runtime resources");
        if let Some(service) = state.get_workspace_service(workspace_id) {
            // 直接调用 trait 方法，避免从 application 层反向依赖 commands 层
            service.close_databases().await;
            info!(workspace_id = %workspace_id, "Databases closed");
        }
        state.remove_workspace_service(workspace_id);
        info!("Step 3 completed: Workspace runtime resources removed");
    }

    /// Step 4: 删除旧版索引文件（向后兼容）
    fn delete_legacy_indices(workspace_id: &str, app: &AppHandle, errors: &mut Vec<String>) {
        info!("Step 4: Checking for legacy index files");

        let index_dir = match app.path().app_data_dir() {
            Ok(dir) => dir.join("indices"),
            Err(e) => {
                let error = format!("Failed to get app data dir: {e}");
                error!(error = %error);
                errors.push(error);
                return;
            }
        };

        let compressed_index = index_dir.join(format!("{workspace_id}.idx.gz"));
        let uncompressed_index = index_dir.join(format!("{workspace_id}.idx"));

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for index_path in [&compressed_index, &uncompressed_index] {
            if !index_path.exists() {
                info!(path = %index_path.display(), "Legacy index file does not exist, skipping");
                continue;
            }

            match std::fs::metadata(index_path) {
                Ok(metadata) if metadata.is_file() => match std::fs::remove_file(index_path) {
                    Ok(_) => {
                        deleted_count += 1;
                        info!(path = %index_path.display(), size = metadata.len(), "Deleted legacy index file");
                    }
                    Err(e) => {
                        failed_count += 1;
                        let error =
                            AppError::io_error(e.to_string(), Some(index_path.to_path_buf()))
                                .to_string();
                        error!(error = %error);
                        errors.push(error);
                    }
                },
                Ok(_) => {
                    warn!(path = %index_path.display(), "Path exists but is not a regular file, skipping");
                }
                Err(e) => {
                    failed_count += 1;
                    let error = AppError::io_error(e.to_string(), Some(index_path.to_path_buf()))
                        .to_string();
                    error!(error = %error);
                    errors.push(error);
                }
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

        if failed_count > 0 && deleted_count == 0 {
            warn!(workspace_id = %workspace_id, "All legacy index file deletions failed");
        }
    }

    /// Step 5: 删除工作区目录（含 CAS 子资源）
    async fn delete_workspace_directory(
        workspace_id: &str,
        app: &AppHandle,
        errors: &mut Vec<String>,
        has_critical_error: &mut bool,
    ) {
        info!("Step 5: Checking for workspace directory");

        match crate::utils::workspace_paths::resolve_workspace_dir(app, workspace_id) {
            Ok(workspace_dir) if workspace_dir.exists() => {
                info!(path = %workspace_dir.display(), "Attempting to delete workspace directory");

                // CAS 工作区需要额外清理 metadata.db 和 objects/
                let is_cas = {
                    let metadata_db = workspace_dir.join("metadata.db");
                    let objects_dir = workspace_dir.join("objects");
                    metadata_db.exists() || objects_dir.exists()
                };
                if is_cas {
                    info!("Detected CAS workspace, cleaning up CAS resources");
                    Self::cleanup_cas_resources(&workspace_dir, errors).await;
                } else {
                    info!("Traditional workspace, no CAS cleanup needed");
                }

                // 删除工作区目录
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

                // 验证删除是否成功
                if workspace_dir.exists() {
                    *has_critical_error = true;
                    let error = format!(
                        "Failed to delete workspace directory {} after cleanup",
                        workspace_dir.display()
                    );
                    error!(error = %error);
                    errors.push(error);
                } else {
                    info!(path = %workspace_dir.display(), "Step 5 completed: Workspace directory successfully deleted");
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
    }

    /// 清理 CAS 工作区的 SQLite 数据库和 objects 目录
    async fn cleanup_cas_resources(workspace_dir: &Path, errors: &mut Vec<String>) {
        // 删除 metadata.db
        let metadata_db = workspace_dir.join("metadata.db");
        if metadata_db.exists() {
            if let Err(e) = remove_file_with_retry(&metadata_db).await {
                let error =
                    AppError::io_error(e.to_string(), Some(metadata_db.to_path_buf())).to_string();
                error!(error = %error);
                errors.push(error);
            } else {
                info!(path = %metadata_db.display(), "Deleted SQLite database");
            }
        }

        // 删除 journal 文件
        for journal_ext in &["-wal", "-shm", "-journal"] {
            let journal_file = workspace_dir.join(format!("metadata.db{journal_ext}"));
            if journal_file.exists() {
                if let Err(e) = remove_file_with_retry(&journal_file).await {
                    warn!(path = %journal_file.display(), error = %e, "Failed to delete journal file");
                }
            }
        }

        // 删除 objects 目录
        let objects_dir = workspace_dir.join("objects");
        if objects_dir.exists() {
            if let Err(e) = remove_dir_with_retry(&objects_dir).await {
                let error =
                    AppError::io_error(e.to_string(), Some(objects_dir.to_path_buf())).to_string();
                error!(error = %error);
                errors.push(error);
            } else {
                info!(path = %objects_dir.display(), "Deleted CAS objects directory");
            }
        }
    }

    /// 汇总清理结果，区分严重错误和非严重错误
    fn summarize_results(
        workspace_id: &str,
        errors: Vec<String>,
        has_critical_error: bool,
    ) -> Result<(), CommandError> {
        if errors.is_empty() {
            info!(workspace_id = %workspace_id, "All cleanup steps completed successfully");
            Ok(())
        } else if has_critical_error {
            let error_summary = errors.join("; ");
            error!(
                workspace_id = %workspace_id,
                error_count = errors.len(),
                error_summary = %error_summary,
                "Critical error: workspace directory still exists after cleanup"
            );
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
            Ok(())
        }
    }
}

/// 带重试的文件删除（处理 Windows 临时文件锁定）
async fn remove_file_with_retry(path: &Path) -> Result<(), std::io::Error> {
    const MAX_ATTEMPTS: usize = 3;
    for attempt in 0..MAX_ATTEMPTS {
        match std::fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(_) if attempt + 1 < MAX_ATTEMPTS => {
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt as u64 + 1)))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("remove_file_with_retry loop must return")
}

/// 带重试的目录删除（处理 Windows 临时文件锁定）
async fn remove_dir_with_retry(path: &Path) -> Result<(), std::io::Error> {
    const MAX_ATTEMPTS: usize = 3;
    for attempt in 0..MAX_ATTEMPTS {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(_) if attempt + 1 < MAX_ATTEMPTS => {
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt as u64 + 1)))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("remove_dir_with_retry loop must return")
}
