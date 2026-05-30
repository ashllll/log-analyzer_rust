//! 文件监听命令 — thin glue over WatchUseCase.
//!
//! These Tauri commands handle parameter extraction, search-index updates,
//! and delegate the core watch lifecycle to `WatchUseCase`.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::application::watch::WatchUseCase;
use crate::application::workspace_service::WorkspaceServiceRef;
use crate::infrastructure::event_publisher::TauriEventPublisher;
use crate::models::AppState;
use crate::utils::{validate_path_param, validate_workspace_id};
use la_core::domain::event::{EventPublisher, SearchSummary};
use la_core::traits::{ContentStorage, MetadataStorage};
use la_search::SearchEngineManager;

/// Adapter that decorates `TauriEventPublisher` with search-index updates
/// on `emit_new_logs`. This keeps the search-index concern out of the
/// domain-layer `WatchUseCase`.
struct WatchEventAdapter {
    inner: TauriEventPublisher,
    search_manager: Arc<SearchEngineManager>,
}

#[async_trait::async_trait]
impl EventPublisher for WatchEventAdapter {
    async fn emit_search_start(&self, id: &str) {
        self.inner.emit_search_start(id).await;
    }
    async fn emit_search_progress(&self, id: &str, count: usize) {
        self.inner.emit_search_progress(id, count).await;
    }
    async fn emit_search_complete(&self, id: &str, summary: SearchSummary) {
        self.inner.emit_search_complete(id, summary).await;
    }
    async fn emit_search_error(&self, id: &str, error: &str) {
        self.inner.emit_search_error(id, error).await;
    }
    async fn emit_search_cancelled(&self, id: &str) {
        self.inner.emit_search_cancelled(id).await;
    }
    async fn emit_search_timeout(&self, id: &str) {
        self.inner.emit_search_timeout(id).await;
    }

    async fn emit_file_changed(
        &self,
        workspace_id: &str,
        event_type: &str,
        file_path: &str,
        timestamp: i64,
    ) {
        self.inner
            .emit_file_changed(workspace_id, event_type, file_path, timestamp)
            .await;
    }

    async fn emit_new_logs(&self, workspace_id: &str, entries_json: &str) {
        // 1. Forward to Tauri frontend
        self.inner.emit_new_logs(workspace_id, entries_json).await;

        // 2. Update Tantivy search index (using per-workspace search_manager)
        if let Ok(entries) =
            serde_json::from_str::<Vec<la_core::models::log_entry::LogEntry>>(entries_json)
        {
            if !entries.is_empty() {
                if let Err(e) = self.search_manager.add_documents(&entries) {
                    tracing::warn!(
                        error = %e,
                        count = entries.len(),
                        workspace_id = %workspace_id,
                        "Failed to add watch documents to search index"
                    );
                }
                if let Err(e) = self.search_manager.commit() {
                    tracing::warn!(
                        error = %e,
                        workspace_id = %workspace_id,
                        "Failed to commit search index after watch update"
                    );
                }
            }
        }
    }
}

/// Start watching a workspace directory for file changes.
///
/// Thin glue over `WatchUseCase::start()`. Handles:
/// - Tauri parameter extraction and validation
/// - CAS / metadata lookup from AppState
/// - Search index updates (via `WatchEventAdapter`)
#[tauri::command]
pub async fn start_watch(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    #[allow(non_snake_case)] _autoSearch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_workspace_id(&workspaceId)?;
    validate_path_param(&path, "path")?;

    // ── Look up workspace service (pure lookup, no fallback) ──
    let workspace: WorkspaceServiceRef = state
        .get_workspace_service(&workspaceId)
        .ok_or_else(|| {
            format!(
                "Workspace {} not found. Please import or reload the workspace.",
                workspaceId
            )
        })?;

    let cas = Arc::clone(workspace.cas());
    let metadata_store = Arc::clone(workspace.metadata_store());
    let search_manager = Arc::clone(workspace.search_engine());

    // ── Build event adapter (Tauri + search-index) ──
    let events = Arc::new(WatchEventAdapter {
        inner: TauriEventPublisher {
            app_handle: app.clone(),
        },
        search_manager,
    });

    // ── Delegate to WatchUseCase ──
    let use_case = WatchUseCase::new(events, cas, metadata_store, Arc::clone(&state.watchers));

    use_case
        .start(&workspaceId, &path)
        .await
        .map(|_result| ())
        .map_err(|e| e.to_string())
}

/// Stop watching a workspace.
///
/// Thin glue over `WatchUseCase::stop()`.
#[tauri::command]
pub async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // stop() only needs the watchers map — CAS and metadata aren't used.
    // We create a minimal WatchUseCase with dummy deps just for stop().
    // (stop() only interacts with the watchers map, not CAS/metadata/events.)
    //
    // This is a pragmatic shortcut; a cleaner long-term approach would
    // store the active WatchUseCase in AppState so stop can reuse it.

    // Dummy implementations that panic if accidentally used by stop()
    struct StopOnlyCas;
    #[async_trait::async_trait]
    impl ContentStorage for StopOnlyCas {
        async fn store(&self, _: &[u8]) -> la_core::error::Result<String> {
            unreachable!("stop() does not use CAS")
        }
        async fn retrieve(&self, _: &str) -> la_core::error::Result<Vec<u8>> {
            unreachable!("stop() does not use CAS")
        }
        async fn exists(&self, _: &str) -> bool {
            unreachable!("stop() does not use CAS")
        }
    }

    struct StopOnlyMeta;
    #[async_trait::async_trait]
    impl MetadataStorage for StopOnlyMeta {
        async fn insert_file(
            &self,
            _: &la_core::storage_types::FileMetadata,
        ) -> la_core::error::Result<i64> {
            unreachable!("stop() does not use metadata")
        }
        async fn get_all_files(
            &self,
        ) -> la_core::error::Result<Vec<la_core::storage_types::FileMetadata>> {
            unreachable!("stop() does not use metadata")
        }
        async fn get_file_by_hash(
            &self,
            _: &str,
        ) -> la_core::error::Result<Option<la_core::storage_types::FileMetadata>> {
            unreachable!("stop() does not use metadata")
        }
    }

    struct StopOnlyEvents;
    #[async_trait::async_trait]
    impl EventPublisher for StopOnlyEvents {
        async fn emit_search_start(&self, _: &str) {}
        async fn emit_search_progress(&self, _: &str, _: usize) {}
        async fn emit_search_complete(&self, _: &str, _: SearchSummary) {}
        async fn emit_search_error(&self, _: &str, _: &str) {}
        async fn emit_search_cancelled(&self, _: &str) {}
        async fn emit_search_timeout(&self, _: &str) {}
        async fn emit_file_changed(&self, _: &str, _: &str, _: &str, _: i64) {}
        async fn emit_new_logs(&self, _: &str, _: &str) {}
    }

    let use_case = WatchUseCase::new(
        Arc::new(StopOnlyEvents),
        Arc::new(StopOnlyCas),
        Arc::new(StopOnlyMeta),
        Arc::clone(&state.watchers),
    );

    use_case
        .stop(&workspaceId)
        .await
        .map(|_result| ())
        .map_err(|e| e.to_string())
}
