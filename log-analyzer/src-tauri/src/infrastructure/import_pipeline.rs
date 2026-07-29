//! ImportPipeline — 导入生命周期编排器。
//!
//! 将 `import_folder` Tauri 命令中原本混杂的验证、目录创建、
//! TaskScheduler 生命周期、WorkspaceService 创建、导入调用、失败清理、
//! 后台完整性验证和 Tantivy 段合并抽取到本模块，使命令层保持薄。
//!
//! # 职责边界
//!
//! - **本模块**：拥有导入的完整生命周期编排。通过 trait 引用接收依赖，不绑定 Tauri。
//! - **`ImportService::import_file`**：保持行为不变，负责具体的文件导入逻辑。
//! - **命令层**：构造 adapter 参数并调用 `run_import`，返回 task id。

use std::fs;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::workspace_service::ImportOptions;
use crate::infrastructure::workspace_service_factory::get_or_create_workspace_service;
use crate::models::AppState;
use crate::utils::canonicalize_path;
use crate::utils::validation::{validate_import_source_path, validate_workspace_id};
use la_core::domain::event::EventPublisher;
use la_core::domain::{TaskHandle, TaskScheduler, WorkspacePaths};
use la_core::error::{AppError, ErrorCategory};
use la_core::traits::AppConfigProvider;
use la_storage::verify_after_import;

fn cleanup_failed_import_workspace(
    workspace_dir: &Path,
    existed_before_import: bool,
) -> std::io::Result<()> {
    if existed_before_import {
        return Ok(());
    }

    if workspace_dir.exists() {
        fs::remove_dir_all(workspace_dir)?;
    }
    Ok(())
}

struct ImportInitialization<T> {
    service: T,
    scheduler: Arc<dyn TaskScheduler>,
    handle: TaskHandle,
    workspace_existed_before_import: bool,
}

async fn rollback_import_initialization(
    scheduler: &dyn TaskScheduler,
    handle: &TaskHandle,
    workspace_dir: &Path,
    workspace_existed_before_import: bool,
    error: String,
) -> String {
    let mut rollback_errors = Vec::new();

    if let Err(cleanup_error) =
        cleanup_failed_import_workspace(workspace_dir, workspace_existed_before_import)
    {
        rollback_errors.push(format!("workspace cleanup failed: {cleanup_error}"));
    }

    if let Err(task_error) = scheduler.fail(handle, &error).await {
        rollback_errors.push(format!("task failure update failed: {task_error}"));
    }

    if rollback_errors.is_empty() {
        error
    } else {
        format!("{error}; rollback errors: {}", rollback_errors.join("; "))
    }
}

async fn initialize_import<T, F, Fut>(
    workspace_dir: &Path,
    scheduler: Option<Arc<dyn TaskScheduler>>,
    task_id: &str,
    target_name: &str,
    workspace_id: &str,
    create_service: F,
) -> Result<ImportInitialization<T>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let scheduler = scheduler.ok_or("Task manager not initialized")?;
    let handle = scheduler
        .create(task_id, "Import", target_name, Some(workspace_id))
        .await
        .map_err(|e| format!("Failed to create task: {e}"))?;

    let workspace_existed_before_import = workspace_dir.exists();
    if let Err(error) = fs::create_dir_all(workspace_dir) {
        let error = AppError::io_error(
            format!("Failed to create workspace dir: {error}"),
            Some(workspace_dir.to_path_buf()),
        )
        .to_string();
        return Err(rollback_import_initialization(
            scheduler.as_ref(),
            &handle,
            workspace_dir,
            workspace_existed_before_import,
            error,
        )
        .await);
    }

    let service = match create_service().await {
        Ok(service) => service,
        Err(error) => {
            return Err(rollback_import_initialization(
                scheduler.as_ref(),
                &handle,
                workspace_dir,
                workspace_existed_before_import,
                error,
            )
            .await);
        }
    };

    Ok(ImportInitialization {
        service,
        scheduler,
        handle,
        workspace_existed_before_import,
    })
}

async fn complete_import_task(
    scheduler: &dyn TaskScheduler,
    handle: &TaskHandle,
) -> Result<(), String> {
    let terminal_result = async {
        scheduler
            .update(handle, 100, "Import complete")
            .await
            .map_err(|error| format!("Failed to update completed import task: {error}"))?;
        scheduler
            .complete(handle)
            .await
            .map_err(|error| format!("Failed to complete import task: {error}"))
    }
    .await;

    match terminal_result {
        Ok(()) => Ok(()),
        Err(error) => {
            if let Err(fail_error) = scheduler.fail(handle, &error).await {
                Err(format!(
                    "{error}; additionally failed to mark task as failed: {fail_error}"
                ))
            } else {
                Err(error)
            }
        }
    }
}

/// 运行导入生命周期，返回 TaskScheduler 创建的任务 ID。
///
/// 通过 trait 引用接收基础设施依赖，不绑定 Tauri 具体类型，可独立测试。
///
/// `event_publisher` 使用 `Arc` 以支持后台任务的 fire-and-forget 事件发送。
pub async fn run_import(
    event_publisher: Arc<dyn EventPublisher>,
    workspace_paths: &dyn WorkspacePaths,
    config_provider: &dyn AppConfigProvider,
    app_handle_for_factory: &tauri::AppHandle,
    state: &AppState,
    workspace_id: &str,
    path: &str,
) -> Result<String, String> {
    validate_workspace_id(workspace_id)?;

    let validated_path = validate_import_source_path(path, "path")?;
    let canonical_path = match canonicalize_path(&validated_path) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Path canonicalization failed: {e}");
            warn!("{msg}");
            event_publisher.emit_import_error(&msg).await;
            return Err(msg);
        }
    };

    let workspace_dir = workspace_paths.workspace_data_dir(workspace_id)?;
    let target_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();

    // ── TaskScheduler 任务创建 ──
    let task_id = Uuid::new_v4().to_string();
    let ImportInitialization {
        service,
        scheduler,
        handle,
        workspace_existed_before_import,
    } = initialize_import(
        &workspace_dir,
        state.get_task_scheduler(),
        &task_id,
        &target_name,
        workspace_id,
        || {
            get_or_create_workspace_service(
                app_handle_for_factory,
                state,
                workspace_id,
                &workspace_dir,
            )
        },
    )
    .await
    .inspect_err(|e| {
        let publisher = Arc::clone(&event_publisher);
        let error_msg = e.clone();
        tokio::spawn(async move {
            publisher.emit_import_error(&error_msg).await;
        });
    })?;

    // ── 更新任务进度 ──
    let _ = scheduler.update(&handle, 10, "Scanning...").await;

    // ── 调用 ImportService ──
    let cancel_token = tokio_util::sync::CancellationToken::new();
    state.register_task_cancellation(&task_id, cancel_token.clone());

    let import_result = service
        .import_file(
            &canonical_path,
            ImportOptions::default(),
            config_provider,
            &task_id,
            cancel_token.clone(),
        )
        .await;
    state.cleanup_task_cancellation(&task_id);
    let import_result = if cancel_token.is_cancelled() && import_result.is_ok() {
        Err(AppError::Archive {
            category: ErrorCategory::CancellationRequested,
            _message: "Import cancelled".to_string(),
            _path: Some(canonical_path.clone()),
        })
    } else {
        import_result
    };

    let _import_result = match import_result {
        Ok(result) => result,
        Err(e) => {
            let was_cancelled = e.category() == ErrorCategory::CancellationRequested;
            if was_cancelled {
                info!(task_id = %task_id, "Import cancelled by user");
            } else {
                error!(error = %e, "Failed to import file");
            }
            service.close_databases().await;
            state.remove_workspace_service(workspace_id);
            drop(service);
            if let Err(rm_err) =
                cleanup_failed_import_workspace(&workspace_dir, workspace_existed_before_import)
            {
                warn!(path = ?workspace_dir, error = %rm_err, "Failed to cleanup workspace");
                let publisher = Arc::clone(&event_publisher);
                let cleanup_msg = format!("Cleanup failed: {rm_err}");
                tokio::spawn(async move {
                    publisher.emit_import_error(&cleanup_msg).await;
                });
            }
            if was_cancelled {
                let _ = scheduler.cancel(&handle).await;
                return Err("Import cancelled".to_string());
            }
            let msg = format!("Failed to import: {e}");
            let _ = scheduler.fail(&handle, &msg).await;
            return Err(msg);
        }
    };

    // ── 完成 ──
    complete_import_task(scheduler.as_ref(), &handle).await?;
    event_publisher.emit_import_complete(&task_id).await;

    // ── 完整性验证（后台执行）──
    let verify_publisher = Arc::clone(&event_publisher);
    let verify_workspace_id = workspace_id.to_string();
    let verify_workspace_dir = workspace_dir.clone();
    tokio::spawn(async move {
        match verify_after_import(&verify_workspace_dir).await {
            Ok(report) => {
                if report.is_valid() {
                    info!(
                        workspace_id = %verify_workspace_id,
                        total_files = report.total_files,
                        valid_files = report.valid_files,
                        "Import integrity verification completed"
                    );
                } else {
                    warn!(
                        workspace_id = %verify_workspace_id,
                        total_files = report.total_files,
                        valid_files = report.valid_files,
                        invalid = report.invalid_files.len(),
                        missing = report.missing_objects.len(),
                        corrupted = report.corrupted_objects.len(),
                        "Integrity verification found issues"
                    );
                    let report_json = serde_json::json!({
                        "workspace_id": verify_workspace_id,
                        "report": report,
                    });
                    let _ = verify_publisher
                        .emit_validation_report(&verify_workspace_id, &report_json.to_string())
                        .await;
                }
            }
            Err(e) => {
                let msg = format!("Failed to verify integrity: {e}");
                error!(workspace_id = %verify_workspace_id, error = %e, "{msg}");
                let _ = verify_publisher.emit_import_error(&msg).await;
            }
        }
    });

    // ── Tantivy segment 合并（后台执行）──
    let search_engine = Arc::clone(service.search_engine());
    let ws_id = workspace_id.to_string();
    tokio::spawn(async move {
        info!(workspace_id = %ws_id, "Starting Tantivy segment merge");
        if let Err(e) = search_engine.commit_and_wait_merge().await {
            warn!(
                workspace_id = %ws_id,
                error = %e,
                "Tantivy segment merge warning (non-critical)"
            );
        }
    });

    Ok(task_id)
}

/// 便捷函数：从字符串路径构造 [`Path`] 后运行导入生命周期。
///
/// 主要用于测试或需要显式传入 [`Path`] 的场景。
pub async fn run_import_with_path(
    event_publisher: Arc<dyn EventPublisher>,
    workspace_paths: &dyn WorkspacePaths,
    config_provider: &dyn AppConfigProvider,
    app_handle_for_factory: &tauri::AppHandle,
    state: &AppState,
    workspace_id: &str,
    source_path: &Path,
) -> Result<String, String> {
    let path_str = source_path.to_string_lossy().to_string();
    run_import(
        event_publisher,
        workspace_paths,
        config_provider,
        app_handle_for_factory,
        state,
        workspace_id,
        &path_str,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use tempfile::TempDir;

    #[derive(Default)]
    struct RecordingScheduler {
        fail_create: bool,
        fail_complete: bool,
        created: Mutex<Vec<String>>,
        failed: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl TaskScheduler for RecordingScheduler {
        async fn create(
            &self,
            id: &str,
            _task_type: &str,
            _target: &str,
            _workspace_id: Option<&str>,
        ) -> la_core::error::Result<TaskHandle> {
            if self.fail_create {
                return Err(AppError::internal_error("injected task creation failure"));
            }
            self.created.lock().unwrap().push(id.to_string());
            Ok(TaskHandle::new(id))
        }

        async fn update(
            &self,
            _handle: &TaskHandle,
            _progress: u8,
            _message: &str,
        ) -> la_core::error::Result<()> {
            Ok(())
        }

        async fn complete(&self, _handle: &TaskHandle) -> la_core::error::Result<()> {
            if self.fail_complete {
                return Err(AppError::internal_error("injected task completion failure"));
            }
            Ok(())
        }

        async fn fail(&self, handle: &TaskHandle, _error: &str) -> la_core::error::Result<()> {
            self.failed.lock().unwrap().push(handle.id().to_string());
            Ok(())
        }

        async fn cancel(&self, _handle: &TaskHandle) -> la_core::error::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn missing_scheduler_does_not_create_a_workspace_directory() {
        let parent = TempDir::new().unwrap();
        let workspace = parent.path().join("missing-scheduler");

        let result = initialize_import(
            &workspace,
            None,
            "task-1",
            "source.log",
            "workspace-1",
            || async { Ok(()) },
        )
        .await;

        let error = match result {
            Ok(_) => panic!("initialization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error, "Task manager not initialized");
        assert!(!workspace.exists());
    }

    #[tokio::test]
    async fn task_creation_failure_does_not_create_a_workspace_directory() {
        let parent = TempDir::new().unwrap();
        let workspace = parent.path().join("failed-task");
        let scheduler = Arc::new(RecordingScheduler {
            fail_create: true,
            ..RecordingScheduler::default()
        });

        let result = initialize_import(
            &workspace,
            Some(scheduler as Arc<dyn TaskScheduler>),
            "task-2",
            "source.log",
            "workspace-2",
            || async { Ok(()) },
        )
        .await;

        let error = match result {
            Ok(_) => panic!("initialization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(error.contains("Failed to create task"));
        assert!(!workspace.exists());
    }

    #[tokio::test]
    async fn service_initialization_failure_rolls_back_task_and_new_workspace() {
        let parent = TempDir::new().unwrap();
        let workspace = parent.path().join("failed-service");
        let scheduler = Arc::new(RecordingScheduler::default());

        let result = initialize_import(
            &workspace,
            Some(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>),
            "task-3",
            "source.log",
            "workspace-3",
            || async { Err::<(), _>("injected service initialization failure".to_string()) },
        )
        .await;

        let error = match result {
            Ok(_) => panic!("initialization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error, "injected service initialization failure");
        assert!(!workspace.exists());
        assert_eq!(scheduler.failed.lock().unwrap().as_slice(), ["task-3"]);
    }

    #[tokio::test]
    async fn task_completion_failure_is_not_reported_as_import_success() {
        let scheduler = RecordingScheduler {
            fail_complete: true,
            ..RecordingScheduler::default()
        };
        let handle = TaskHandle::new("task-4");

        let result = complete_import_task(&scheduler, &handle).await;

        assert!(result.is_err());
        assert_eq!(scheduler.failed.lock().unwrap().as_slice(), ["task-4"]);
    }

    #[test]
    fn failed_refresh_preserves_an_existing_workspace() {
        let parent = TempDir::new().unwrap();
        let workspace = parent.path().join("existing-workspace");
        fs::create_dir_all(&workspace).unwrap();
        let sentinel = workspace.join("metadata.db");
        fs::write(&sentinel, b"existing metadata").unwrap();

        cleanup_failed_import_workspace(&workspace, true).unwrap();

        assert!(sentinel.exists());
    }

    #[test]
    fn failed_initial_import_removes_its_new_workspace() {
        let parent = TempDir::new().unwrap();
        let workspace = parent.path().join("new-workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("partial-object"), b"partial").unwrap();

        cleanup_failed_import_workspace(&workspace, false).unwrap();

        assert!(!workspace.exists());
    }
}
