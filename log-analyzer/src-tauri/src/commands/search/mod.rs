//! 搜索命令实现 — Clean Architecture 路径（SearchUseCase）
//!
//! # 架构
//! - `events`: 事件类型与 emit 函数
//! - `query`: 查询解析
//! - `filters`: 过滤器类型与匹配逻辑
//! - `mod.rs` (本文件): Tauri 命令入口，所有搜索委托给 SearchUseCase

pub(crate) mod filters;
pub(crate) mod query;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use la_core::error::{AppError, CommandError};
use la_core::models::config::AppConfigLoader;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};

use crate::application::SearchUseCase;
use crate::commands::import::ensure_workspace_runtime_state;
use crate::commands::search::query::resolve_search_query;
use crate::infrastructure::{
    CasLogFileRepository, DiskResultStoreRepo, QueryEngineLogSearcher, TauriEventPublisher,
};
use crate::models::AppState;

// ============================================================================
// 公共类型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchResult {
    pub search_id: String,
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub duration_ms: u64,
    pub was_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    pub filters: Option<SearchFilters>,
}

// ============================================================================
// 运行时配置
// ============================================================================

#[derive(Debug, Clone)]
pub(crate) struct SearchRuntimeConfig {
    pub(crate) default_max_results: usize,
    pub(crate) timeout_seconds: u64,
    pub(crate) regex_cache_size: usize,
    pub(crate) case_sensitive: bool,
}

impl Default for SearchRuntimeConfig {
    fn default() -> Self {
        Self {
            default_max_results: 100_000,
            timeout_seconds: 10,
            regex_cache_size: 1000,
            case_sensitive: false,
        }
    }
}

pub(crate) fn load_search_runtime_config(app: &AppHandle) -> SearchRuntimeConfig {
    let config_path = match app.path().app_config_dir() {
        Ok(dir) => dir.join("config.json"),
        Err(_) => return SearchRuntimeConfig::default(),
    };
    if !config_path.exists() {
        return SearchRuntimeConfig::default();
    }
    AppConfigLoader::load(Some(config_path))
        .ok()
        .map(|loader| {
            let c = loader.get_config();
            SearchRuntimeConfig {
                default_max_results: c.search.max_results,
                timeout_seconds: c.search.timeout_seconds,
                regex_cache_size: c.search.regex_cache_size.max(1),
                case_sensitive: c.search.case_sensitive,
            }
        })
        .unwrap_or_default()
}

// ============================================================================
// 辅助函数
// ============================================================================

pub(crate) fn validate_search_params(query: &str) -> Result<(), CommandError> {
    if query.is_empty() {
        return Err(
            CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                .with_help("Please enter at least one search term"),
        );
    }
    if query.len() > 1000 {
        return Err(CommandError::new(
            "VALIDATION_ERROR",
            "Search query too long (max 1000 characters)",
        )
        .with_help("Try reducing the number of search terms"));
    }
    Ok(())
}

pub(crate) fn resolve_workspace_id(
    id_arg: Option<String>,
    dirs: &Arc<Mutex<BTreeMap<String, PathBuf>>>,
) -> Result<String, CommandError> {
    if let Some(id) = id_arg {
        return Ok(id);
    }
    let d = dirs.lock();
    if let Some(first) = d.keys().next() {
        Ok(first.clone())
    } else {
        Err(CommandError::new("NOT_FOUND", "No workspaces available")
            .with_help("Create a workspace first"))
    }
}

// ============================================================================
// Tauri 命令 — 搜索管理
// ============================================================================

#[command]
pub async fn cancel_search(
    #[allow(non_snake_case)] searchId: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let cts = Arc::clone(&state.search_cancellation_tokens);
    let token = { cts.lock().get(&searchId).cloned() };
    if let Some(t) = token {
        t.cancel();
        cts.lock().remove(&searchId);
        Ok(())
    } else {
        Err(
            CommandError::new("NOT_FOUND", format!("Search {} not found", searchId))
                .with_help("Search may have already finished"),
        )
    }
}

#[command]
pub async fn fetch_search_page(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
    offset: usize,
    limit: usize,
) -> Result<la_search::SearchPageResult, CommandError> {
    let limit = limit.min(10000);
    let ds_opt = state.disk_result_store.read();
    let ds = ds_opt.as_ref().ok_or_else(|| {
        CommandError::new("NOT_INITIALIZED", "Disk result store not initialized")
            .with_help("App may be initializing")
    })?;
    if ds.has_session(&searchId) {
        return ds.read_page(&searchId, offset, limit).map_err(|e| {
            CommandError::from(AppError::io_error(
                format!("Failed to read page: {e}"),
                None,
            ))
            .with_help("Results may have been cleared. Try searching again")
        });
    }
    Err(
        CommandError::new("NOT_FOUND", format!("Session '{}' not found", searchId))
            .with_help("Results may have been cleared. Try searching again"),
    )
}

// ============================================================================
// 搜索命令入口 — 使用 SearchUseCase（Clean Architecture 路径）
// ============================================================================

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
    let (raw_terms, sq) =
        resolve_search_query(&query, structuredQuery, rc.case_sensitive, "search_logs")?;
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
    let events: Arc<TauriEventPublisher> = Arc::new(TauriEventPublisher {
        app_handle: app.clone(),
    });
    let searcher: Arc<QueryEngineLogSearcher> =
        Arc::new(QueryEngineLogSearcher::new(rc.regex_cache_size.max(1)));

    // ── 7. Build SearchUseCase ──
    let use_case = Arc::new(SearchUseCase::new(log_files, results, events, searcher, tp));

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
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let _ = app_clone.emit(
                    "search-error",
                    serde_json::json!({ "search_id": sid_clone, "error": e.to_string() }),
                );
                ds_clone.remove_session(&sid_clone);
            }
            Err(_elapsed) => {
                token.cancel();
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
