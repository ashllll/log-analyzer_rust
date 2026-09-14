//! CleanupUseCase — 工作区资源清理用例
//!
//! 从 `commands/workspace.rs` 的 `cleanup_workspace_resources` 提取而来，
//! 遵循 Clean Architecture：commands 层委托给 application 用例层。

use std::path::{Path, PathBuf};

use tracing::{error, info, warn};

use crate::application::workspace_service::WorkspaceServiceRef;
use la_core::error::{AppError, CommandError};

/// Framework-independent runtime boundary required by workspace cleanup.
pub trait CleanupRuntime {
    fn workspace_lifecycle(
        &self,
        workspace_id: &str,
    ) -> std::sync::Arc<super::workspace_lifecycle::WorkspaceLifecycle>;
    fn workspace_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef>;
    fn remove_workspace_service(&self, workspace_id: &str);
    fn legacy_index_dir(&self) -> Result<PathBuf, String>;
    fn workspace_dir(&self, workspace_id: &str) -> Result<PathBuf, String>;
    /// 取消该工作区的全部进行中搜索（P2#6：删除前必须先取消，
    /// 否则在跑的搜索会与目录删除竞态）。返回取消数量。
    fn cancel_workspace_searches(&self, workspace_id: &str) -> usize;
}

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
        runtime: &impl CleanupRuntime,
    ) -> Result<(), CommandError> {
        info!(workspace_id = %workspace_id, "Starting resource cleanup for workspace");

        // Serialize against service construction and the complete import/rollback
        // pipeline. No producer holding a CAS lease reacquires this gate.
        let lifecycle = runtime.workspace_lifecycle(workspace_id);
        let _deletion = lifecycle.begin_delete().await;
        let service = runtime.workspace_service(workspace_id);
        if let Some(service) = &service {
            service.cas().stop_accepting_activity();
        }
        let mut errors = Vec::new();
        let mut has_critical_error = false;

        // Cancel first; the activity barrier below waits for actual worker exit.
        let cancelled_searches = runtime.cancel_workspace_searches(workspace_id);
        if cancelled_searches > 0 {
            info!(
                workspace_id = %workspace_id,
                cancelled = cancelled_searches,
                "Cancelled in-flight searches before workspace cleanup"
            );
        }

        // Step 1: 停止文件监听器
        Self::stop_watcher(workspace_id, runtime).await;

        // Stop the watcher before waiting for the exclusive activity lease.
        // Retain the lease through database closure and directory removal.
        let _activity = match &service {
            Some(service) => Some(service.cas().drain_activity().await),
            None => None,
        };

        // Step 2: 搜索缓存由 LRU 自动淘汰，无需主动清理
        info!("Step 2: Skipping search cache cleanup (LRU auto-eviction)");

        // Step 3: 关闭数据库连接并移除 workspace service
        Self::close_databases(workspace_id, runtime).await;

        // Step 4: 删除旧版索引文件
        Self::delete_legacy_indices(workspace_id, runtime, &mut errors).await;

        // Step 5: 删除工作区目录
        Self::delete_workspace_directory(
            workspace_id,
            runtime,
            &mut errors,
            &mut has_critical_error,
        )
        .await;

        // 汇总结果
        Self::summarize_results(workspace_id, errors, has_critical_error)
    }

    /// Step 1: 停止文件监听器
    async fn stop_watcher(workspace_id: &str, runtime: &impl CleanupRuntime) {
        info!(workspace_id = %workspace_id, "Step 1: Stopping file watcher");
        if let Some(service) = runtime.workspace_service(workspace_id) {
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
    async fn close_databases(workspace_id: &str, runtime: &impl CleanupRuntime) {
        info!("Step 3: Removing workspace runtime resources");
        if let Some(service) = runtime.workspace_service(workspace_id) {
            // 直接调用 trait 方法，避免从 application 层反向依赖 commands 层
            service.close_databases().await;
            info!(workspace_id = %workspace_id, "Databases closed");
        }
        runtime.remove_workspace_service(workspace_id);
        info!("Step 3 completed: Workspace runtime resources removed");
    }

    /// Step 4: 删除旧版索引文件（向后兼容）
    async fn delete_legacy_indices(
        workspace_id: &str,
        runtime: &impl CleanupRuntime,
        errors: &mut Vec<String>,
    ) {
        info!("Step 4: Checking for legacy index files");

        let index_dir = match runtime.legacy_index_dir() {
            Ok(dir) => dir,
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
            match tokio::fs::metadata(index_path).await {
                Ok(metadata) if metadata.is_file() => {
                    match tokio::fs::remove_file(index_path).await {
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
                    }
                }
                Ok(_) => {
                    warn!(path = %index_path.display(), "Path exists but is not a regular file, skipping");
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    info!(path = %index_path.display(), "Legacy index file does not exist, skipping");
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
        runtime: &impl CleanupRuntime,
        errors: &mut Vec<String>,
        has_critical_error: &mut bool,
    ) {
        info!("Step 5: Checking for workspace directory");

        match runtime.workspace_dir(workspace_id) {
            Ok(workspace_dir) => match tokio::fs::try_exists(&workspace_dir).await {
                Ok(true) => {
                    info!(path = %workspace_dir.display(), "Attempting to delete workspace directory");

                    // CAS 工作区需要额外清理 metadata.db 和 objects/
                    let is_cas = {
                        let metadata_db = workspace_dir.join("metadata.db");
                        let objects_dir = workspace_dir.join("objects");
                        match (
                            tokio::fs::try_exists(&metadata_db).await,
                            tokio::fs::try_exists(&objects_dir).await,
                        ) {
                            (Ok(metadata_exists), Ok(objects_exist)) => {
                                metadata_exists || objects_exist
                            }
                            (metadata_result, objects_result) => {
                                for (path, result) in [
                                    (&metadata_db, metadata_result),
                                    (&objects_dir, objects_result),
                                ] {
                                    if let Err(e) = result {
                                        let message = AppError::io_error(
                                            e.to_string(),
                                            Some(path.to_path_buf()),
                                        )
                                        .to_string();
                                        error!(error = %message);
                                        errors.push(message);
                                    }
                                }
                                false
                            }
                        }
                    };
                    if is_cas {
                        info!("Detected CAS workspace, cleaning up CAS resources");
                        Self::cleanup_cas_resources(&workspace_dir, errors).await;
                    } else {
                        info!("Traditional workspace, no CAS cleanup needed");
                    }

                    // 删除工作区目录
                    if let Err(e) = remove_dir_with_retry(&workspace_dir).await {
                        let error = format!(
                            "Failed to delete workspace directory {}: {}",
                            workspace_dir.display(),
                            e
                        );
                        error!(error = %error);
                        errors.push(error);
                    }

                    // 验证删除是否成功
                    match tokio::fs::try_exists(&workspace_dir).await {
                        Ok(false) => {
                            info!(path = %workspace_dir.display(), "Step 5 completed: Workspace directory successfully deleted");
                        }
                        Ok(true) => {
                            *has_critical_error = true;
                            let error = format!(
                                "Failed to delete workspace directory {} after cleanup",
                                workspace_dir.display()
                            );
                            error!(error = %error);
                            errors.push(error);
                        }
                        Err(e) => {
                            *has_critical_error = true;
                            let error = AppError::io_error(
                                e.to_string(),
                                Some(workspace_dir.to_path_buf()),
                            )
                            .to_string();
                            error!(error = %error);
                            errors.push(error);
                        }
                    }
                }
                Ok(false) => info!("Step 5 skipped: Workspace directory does not exist"),
                Err(e) => {
                    *has_critical_error = true;
                    let error =
                        AppError::io_error(e.to_string(), Some(workspace_dir.to_path_buf()))
                            .to_string();
                    error!(error = %error);
                    errors.push(error);
                }
            },
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
        if path_exists(&metadata_db, errors).await {
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
            if path_exists(&journal_file, errors).await {
                if let Err(e) = remove_file_with_retry(&journal_file).await {
                    warn!(path = %journal_file.display(), error = %e, "Failed to delete journal file");
                }
            }
        }

        // 删除 objects 目录
        let objects_dir = workspace_dir.join("objects");
        if path_exists(&objects_dir, errors).await {
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

async fn path_exists(path: &Path, errors: &mut Vec<String>) -> bool {
    match tokio::fs::try_exists(path).await {
        Ok(exists) => exists,
        Err(e) => {
            let message = AppError::io_error(e.to_string(), Some(path.to_path_buf())).to_string();
            error!(error = %message);
            errors.push(message);
            false
        }
    }
}

/// 带重试的文件删除（处理 Windows 临时文件锁定）
async fn remove_file_with_retry(path: &Path) -> Result<(), std::io::Error> {
    const MAX_ATTEMPTS: usize = 3;
    for attempt in 0..MAX_ATTEMPTS {
        match tokio::fs::remove_file(path).await {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
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
        match tokio::fs::remove_dir_all(path).await {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) if attempt + 1 < MAX_ATTEMPTS => {
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt as u64 + 1)))
                    .await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("remove_dir_with_retry loop must return")
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::application::workspace_lifecycle::WorkspaceLifecycle;
    use crate::application::workspace_service::{
        ImportOptions, ImportResult, ImportService, SearchService, WatchService, WorkspaceService,
    };
    use async_trait::async_trait;
    use futures::FutureExt;
    use la_core::{
        error::Result,
        models::{SearchFilters, SearchQuery},
        traits::AppConfigProvider,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio_util::sync::CancellationToken;

    struct Service {
        path: PathBuf,
        cas: Arc<la_storage::ContentAddressableStorage>,
        metadata: Arc<la_storage::MetadataStore>,
        index: Arc<la_search::SearchEngineManager>,
        closed: AtomicBool,
    }
    #[async_trait]
    impl SearchService for Service {
        async fn search(
            &self,
            _: SearchQuery,
            _: Vec<String>,
            _: SearchFilters,
            _: usize,
        ) -> Result<String> {
            unreachable!()
        }
        async fn fetch_search_page(
            &self,
            _: &str,
            _: usize,
            _: usize,
        ) -> Result<la_search::SearchPageResult> {
            unreachable!()
        }
        async fn cancel_search(&self, _: &str) -> Result<()> {
            unreachable!()
        }
    }
    #[async_trait]
    impl ImportService for Service {
        async fn import_file(
            &self,
            _: &Path,
            _: ImportOptions,
            _: &dyn AppConfigProvider,
            _: &str,
            _: CancellationToken,
        ) -> Result<ImportResult> {
            unreachable!()
        }
    }
    #[async_trait]
    impl WatchService for Service {
        async fn start_watch(&self, _: &str) -> Result<()> {
            unreachable!()
        }
        async fn stop_watch(&self) -> Result<()> {
            Ok(())
        }
    }
    #[async_trait]
    impl WorkspaceService for Service {
        fn workspace_id(&self) -> &str {
            "test"
        }
        fn workspace_dir(&self) -> &PathBuf {
            &self.path
        }
        fn cas(&self) -> &Arc<la_storage::ContentAddressableStorage> {
            &self.cas
        }
        fn metadata_store(&self) -> &Arc<la_storage::MetadataStore> {
            &self.metadata
        }
        fn search_engine(&self) -> &Arc<la_search::SearchEngineManager> {
            &self.index
        }
        async fn close_databases(&self) {
            self.closed.store(true, Ordering::SeqCst);
            self.metadata.close().await;
            self.index.close().await;
        }
    }
    struct Runtime {
        service: Arc<Service>,
        lifecycle: Arc<WorkspaceLifecycle>,
        removed: AtomicBool,
    }
    impl CleanupRuntime for Runtime {
        fn workspace_lifecycle(&self, _: &str) -> Arc<WorkspaceLifecycle> {
            self.lifecycle.clone()
        }
        fn workspace_service(&self, _: &str) -> Option<WorkspaceServiceRef> {
            Some(self.service.clone())
        }
        fn remove_workspace_service(&self, _: &str) {
            self.removed.store(true, Ordering::SeqCst);
        }
        fn legacy_index_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(self.service.path.join("legacy"))
        }
        fn workspace_dir(&self, _: &str) -> std::result::Result<PathBuf, String> {
            Ok(self.service.path.clone())
        }
        fn cancel_workspace_searches(&self, _: &str) -> usize {
            1
        }
    }
    #[tokio::test]
    async fn cleanup_never_closes_or_unlinks_before_real_reader_exit() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("workspace");
        let metadata = Arc::new(la_storage::MetadataStore::new(&path).await.unwrap());
        let cas = Arc::new(la_storage::ContentAddressableStorage::new(path.clone()));
        let hash = cas.store_content(b"held snapshot").await.unwrap();
        let index = Arc::new(
            la_search::SearchEngineManager::new(la_search::manager::SearchConfig {
                index_path: path.join("search_index"),
                ..Default::default()
            })
            .unwrap(),
        );
        let service = Arc::new(Service {
            path: path.clone(),
            cas: cas.clone(),
            metadata,
            index,
            closed: AtomicBool::new(false),
        });
        let runtime = Runtime {
            service: service.clone(),
            lifecycle: Arc::new(WorkspaceLifecycle::default()),
            removed: AtomicBool::new(false),
        };
        let worker = cas.activity_guard().await.unwrap();
        let cleanup = CleanupUseCase::execute("test", &runtime);
        tokio::pin!(cleanup);
        assert!(cleanup.as_mut().now_or_never().is_none());
        assert!(!service.closed.load(Ordering::SeqCst));
        assert!(!runtime.removed.load(Ordering::SeqCst));
        assert_eq!(cas.read_content(&hash).await.unwrap(), b"held snapshot");
        assert!(cas.activity_guard().await.is_err());
        assert!(runtime.lifecycle.enter().await.is_err());
        drop(worker);
        cleanup.await.unwrap();
        assert!(service.closed.load(Ordering::SeqCst));
        assert!(runtime.removed.load(Ordering::SeqCst));
        assert!(!path.exists());
    }
}
