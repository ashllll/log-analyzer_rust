//! Import command interface adapters.
//!
//! ## Migration path (UC-02)
//!
//! When CAS / MetadataStore / SearchEngineManager are trait-ised,
//! replace the legacy delegation with ImportUseCase wiring:
//!
//! ```ignore
//! use crate::application::ImportUseCase;
//! use crate::infrastructure::{ArchiveManagerAdapter, TaskManagerAdapter};
//!
//! let archive_adapter = ArchiveManagerAdapter::new(Arc::new(ArchiveManager::new()));
//! let task_adapter = TaskManagerAdapter::new(Arc::clone(&state.task_manager.lock().unwrap()));
//! let use_case = ImportUseCase::new(
//!     log_files,
//!     events,
//!     Arc::new(archive_adapter),
//!     Arc::new(task_adapter),
//!     cas,
//! );
//! use_case.execute(&path, &workspace_id, &task_id).await
//! ```
//!
//! ImportUseCase now accepts ArchiveExtractor + TaskScheduler traits
//! (AD-06/07 complete) and has unit-testable orchestration logic.

use tauri::{AppHandle, State};

use crate::models::AppState;

#[tauri::command]
pub async fn import_folder(
    app: AppHandle,
    path: String,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // TODO(UC-02): Wire ImportUseCase once CAS/MetadataStore are trait-ised.
    // Currently the full import flow requires AppState fields
    // (cas_instances, metadata_stores, search_engine_managers) that are
    // not yet behind domain traits.
    crate::commands::import::import_folder_impl(app, path, workspace_id, state).await
}
