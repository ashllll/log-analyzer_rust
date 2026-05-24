//! Search command interface adapters.
//!
//! Thin Tauri command wrapper that constructs domain adapters from AppState
//! and delegates to `SearchUseCase::execute()`. Timeout handling and Tantivy
//! prefetch remain as infrastructure-level enhancements on top of the clean
//! domain core.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use la_core::error::CommandError;
use la_core::models::{SearchFilters, SearchQuery};

use crate::application::SearchUseCase;
use crate::commands::import::ensure_workspace_runtime_state;
use crate::commands::search::query::resolve_search_query;
use crate::commands::search::{
    load_search_runtime_config, resolve_workspace_id, validate_search_params,
};
use crate::infrastructure::{
    CasLogFileRepository, DiskResultStoreRepo, QueryEngineLogSearcher, TauriEventPublisher,
};
use crate::models::AppState;

#[tauri::command]
#[allow(non_snake_case)]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    structuredQuery: Option<SearchQuery>,
    workspaceId: Option<String>,
    maxResults: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    // ── 1. Validate ──
    validate_search_params(&query)?;

    // ── 2. Load config ──
    let rc = load_search_runtime_config(&app);

    // ── 3. Extract AppState fields ──
    let wd = Arc::clone(&state.workspace_dirs);
    let cts = Arc::clone(&state.search_cancellation_tokens);
    let ds = state.disk_result_store.read().clone().ok_or_else(|| {
        CommandError::new("NOT_INITIALIZED", "Disk result store not initialized")
            .with_help("App may be initializing")
    })?;
    let tp = Arc::clone(&state.search_thread_pool);

    // ── 4. Resolve params ──
    let mr = maxResults.unwrap_or(rc.default_max_results).min(100_000);
    let f = filters.unwrap_or_default();
    let (raw_terms, sq) = resolve_search_query(
        &query,
        structuredQuery,
        rc.case_sensitive,
        "search_logs",
    )?;
    let ws_id = resolve_workspace_id(workspaceId, &wd)?;

    // ── 5. Ensure workspace runtime state (CAS + MetadataStore) ──
    let workspace_dir = wd.lock().get(&ws_id).cloned().ok_or_else(|| {
        CommandError::new("NOT_FOUND", format!("Workspace {ws_id} not found"))
            .with_help("Try refreshing the workspace list")
    })?;

    let (cas, metadata_store, _search_mgr) =
        ensure_workspace_runtime_state(&app, &state, &ws_id, &workspace_dir)
            .await
            .map_err(|e| {
                CommandError::new("DATABASE_ERROR", format!("Failed to init workspace: {e}"))
                    .with_help("Try reloading the workspace")
            })?;

    // ── 6. Build domain adapters ──
    let log_files: Arc<CasLogFileRepository> = Arc::new(CasLogFileRepository {
        metadata: metadata_store.clone(),
        cas: cas.clone(),
    });
    let results: Arc<DiskResultStoreRepo> = Arc::new(DiskResultStoreRepo { store: ds.clone() });
    let events: Arc<TauriEventPublisher> =
        Arc::new(TauriEventPublisher { app_handle: app.clone() });
    let searcher: Arc<QueryEngineLogSearcher> =
        Arc::new(QueryEngineLogSearcher::new(rc.regex_cache_size.max(1)));

    // ── 7. Build SearchUseCase ──
    let use_case = Arc::new(SearchUseCase::new(
        log_files,
        results,
        events,
        searcher,
        tp,
    ));

    // ── 8. Cancellation token ──
    let sid = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    {
        cts.lock().insert(sid.clone(), token.clone());
    }

    // ── 9. Spawn search with timeout ──
    let uc = Arc::clone(&use_case);
    let sid_clone = sid.clone();
    let token_clone = token.clone();
    let cts_clone = Arc::clone(&cts);
    let ds_clone = Arc::clone(&ds);
    let app_clone = app.clone();
    let timeout_secs = rc.timeout_seconds;

    tokio::spawn(async move {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            uc.execute(
                &ws_id,
                &sq,
                raw_terms,
                &f,
                mr,
                sid_clone.clone(),
                token_clone,
            ),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                // Search completed normally — events already emitted by SearchUseCase
            }
            Ok(Err(e)) => {
                let _ = app_clone.emit(
                    "search-error",
                    serde_json::json!({ "search_id": sid_clone, "error": e.to_string() }),
                );
                ds_clone.remove_session(&sid_clone);
            }
            Err(_elapsed) => {
                // Timeout
                token.cancel(); // cancel the already-spawned token from step 8
                cts_clone.lock().remove(&sid_clone);
                ds_clone.remove_session(&sid_clone);
                let _ = app_clone.emit(
                    "search-timeout",
                    serde_json::json!({ "search_id": sid_clone }),
                );
            }
        }
    });

    Ok(sid)
}
