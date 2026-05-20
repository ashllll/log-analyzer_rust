//! 搜索命令实现
//!
//! # 架构
//! - `events`: 事件类型与 emit 函数
//! - `query`: 查询解析 (split_query_by_pipe / build_structured_search_query)
//! - `filters`: 过滤器类型与匹配逻辑 (CompiledSearchFilters / FilePatternMatcher)
//! - `mod.rs` (本文件): Tauri 命令入口 + 执行引擎 + 文件搜索

pub(crate) mod events;
pub(crate) mod query;
pub(crate) mod filters;

use std::{borrow::Cow, collections::{HashMap, HashSet}, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use parking_lot::Mutex;
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{error, warn};
use serde::{Deserialize, Serialize};

use la_core::error::{AppError, CommandError};
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use la_core::models::config::AppConfigLoader;
use la_core::models::search_statistics::SearchResultSummary;
use la_storage::ContentAddressableStorage;
use rayon::prelude::*;

use crate::commands::import::ensure_workspace_runtime_state;
use crate::commands::search::events::{emit_search_id_event, emit_search_error, SearchProgressEvent, SearchResultBatchEvent, SearchSummaryEvent, SearchCompleteEvent};
use crate::commands::search::filters::{CompiledSearchFilters, ParsedLineMetadata, SearchSegmentSummary, SearchLineCandidate};
use crate::commands::search::query::resolve_search_query;
use crate::models::state::SearchMetrics;
use crate::models::AppState;
use crate::services::{ExecutionPlan, QueryPlanBuilder};
use crate::utils::encoding::decode_log_content;
use crate::utils::workspace_paths::resolve_workspace_dir;

const SEARCH_SEGMENT_LINE_COUNT: usize = 256;

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
    fn default() -> Self { Self { default_max_results: 100_000, timeout_seconds: 10, regex_cache_size: 1000, case_sensitive: false } }
}

pub(crate) fn load_search_runtime_config(app: &AppHandle) -> SearchRuntimeConfig {
    let config_path = match app.path().app_config_dir() { Ok(dir) => dir.join("config.json"), Err(_) => return SearchRuntimeConfig::default() };
    if !config_path.exists() { return SearchRuntimeConfig::default(); }
    AppConfigLoader::load(Some(config_path)).ok().map(|loader| {
        let c = loader.get_config();
        SearchRuntimeConfig { default_max_results: c.search.max_results, timeout_seconds: c.search.timeout_seconds, regex_cache_size: c.search.regex_cache_size.max(1), case_sensitive: c.search.case_sensitive }
    }).unwrap_or_default()
}

// ============================================================================
// 辅助函数
// ============================================================================

pub(crate) fn remove_cancellation_token(tokens: &Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>, search_id: &str) {
    tokens.lock().remove(search_id);
}

fn validate_search_params(query: &str) -> Result<(), CommandError> {
    if query.is_empty() { return Err(CommandError::new("VALIDATION_ERROR", "Search query cannot be empty").with_help("Please enter at least one search term")); }
    if query.len() > 1000 { return Err(CommandError::new("VALIDATION_ERROR", "Search query too long (max 1000 characters)").with_help("Try reducing the number of search terms")); }
    Ok(())
}

fn resolve_workspace_id(id_arg: Option<String>, dirs: &Arc<Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>>) -> Result<String, CommandError> {
    if let Some(id) = id_arg { return Ok(id); }
    let d = dirs.lock();
    if let Some(first) = d.keys().next() { Ok(first.clone()) } else { Err(CommandError::new("NOT_FOUND", "No workspaces available").with_help("Create a workspace first")) }
}

// ============================================================================
// 搜索环境准备与超时编排
// ============================================================================

async fn prepare_search_environment(
    app_handle: &AppHandle, state: &AppState, workspace_id: &str,
    workspace_dirs: &Arc<Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>>,
    compiled_filters: &CompiledSearchFilters,
    disk_result_store: &Arc<la_search::DiskResultStore>, search_id: &str,
) -> Result<(Vec<la_storage::FileMetadata>, Arc<ContentAddressableStorage>), CommandError> {
    let workspace_dir = {
        if let Some(dir) = workspace_dirs.lock().get(workspace_id).cloned() { dir }
        else { resolve_workspace_dir(app_handle, workspace_id).map_err(|e| { emit_search_error(app_handle, search_id, &e); CommandError::new("NOT_FOUND", e).with_help("Workspace may have been deleted. Try refreshing") })? }
    };
    let (cas, metadata_store, _) = ensure_workspace_runtime_state(app_handle, state, workspace_id, &workspace_dir).await.map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to init workspace: {}", e)).with_help("Try reloading the workspace"))?;
    let files = metadata_store.get_files_with_pruning(
        compiled_filters.time_start.map(|dt| dt.and_utc().timestamp()),
        compiled_filters.time_end.map(|dt| dt.and_utc().timestamp()),
        compiled_filters.level_mask,
        compiled_filters.database_file_pattern().as_deref(),
    ).await.map_err(|e| { error!(%workspace_id, %e, "Failed to get pruned files"); CommandError::new("DATABASE_ERROR", "Failed to access workspace files").with_help("The database may be corrupted. Try reimporting") })?;
    if !disk_result_store.has_session(search_id) {
        if let Err(e) = disk_result_store.create_session(search_id) { emit_search_error(app_handle, search_id, format!("Failed to create session: {e}")); return Err(CommandError::new("IO_ERROR", format!("Failed to create session: {e}")).with_help("Check app data dir is writable")); }
    }
    Ok((files, cas))
}

struct SearchExecutionRequest {
    app_handle: AppHandle, search_id: String, files_for_search: Vec<la_storage::FileMetadata>,
    cas: Arc<ContentAddressableStorage>, structured_query: SearchQuery, compiled_filters: CompiledSearchFilters,
    max_results: usize, regex_cache_size: usize, raw_terms: Vec<String>,
    search_metrics: Arc<Mutex<SearchMetrics>>, disk_result_store: Arc<la_search::DiskResultStore>,
    cancellation_token: tokio_util::sync::CancellationToken,
    cancellation_tokens: Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    search_timeout_secs: u64, search_thread_pool: Arc<rayon::ThreadPool>,
    /// Optional: hybrid search via Tantivy index
    search_engine_manager: Option<Arc<la_search::SearchEngineManager>>,
    /// Pre-fetched Tantivy results from async pre-search
    tantivy_prefetch: Option<Vec<LogEntry>>,
}

async fn run_search_with_timeout(req: SearchExecutionRequest) -> Result<String, CommandError> {
    let SearchExecutionRequest { app_handle, search_id, files_for_search, cas, structured_query, compiled_filters, max_results, regex_cache_size, raw_terms, search_metrics, disk_result_store, cancellation_token, cancellation_tokens, search_timeout_secs, search_thread_pool, search_engine_manager, tantivy_prefetch } = req;
    let app_h = app_handle.clone(); let ct = cancellation_token.clone(); let cts = Arc::clone(&cancellation_tokens);
    let ds = Arc::clone(&disk_result_store); let timed_out = Arc::new(AtomicBool::new(false));
    let tfb = Arc::clone(&timed_out); let sid = search_id.clone();

    let handle = tokio::task::spawn_blocking(move || {
        execute_file_search(app_handle, sid, files_for_search, cas, structured_query, compiled_filters, max_results, regex_cache_size, raw_terms, search_metrics, disk_result_store, cancellation_token, cancellation_tokens, search_thread_pool, tfb, search_engine_manager, tantivy_prefetch);
    });

    match tokio::time::timeout(std::time::Duration::from_secs(search_timeout_secs), handle).await {
        Ok(Ok(())) => Ok(search_id),
        Ok(Err(e)) => { error!(%e, %search_id, "Search panicked"); { cts.lock().remove(&search_id); } ds.remove_session(&search_id); Err(CommandError::new("INTERNAL_ERROR", format!("Search panicked: {e}")).with_help("Try simplifying your query")) }
        Err(_) => { warn!(%search_id, "Search timed out after {}s", search_timeout_secs); timed_out.store(true, Ordering::SeqCst); ct.cancel(); { cts.lock().remove(&search_id); } ds.remove_session(&search_id); emit_search_id_event(&app_h, "search-timeout", &search_id); Err(CommandError::new("TIMEOUT_ERROR", format!("Search timed out after {}s", search_timeout_secs)).with_help("Use more specific terms")) }
    }
}

// ============================================================================
// Tauri 命令
// ============================================================================

#[command]
pub async fn search_logs(app: AppHandle, query: String, #[allow(non_snake_case)] structuredQuery: Option<SearchQuery>, #[allow(non_snake_case)] workspaceId: Option<String>, #[allow(non_snake_case)] maxResults: Option<usize>, filters: Option<SearchFilters>, state: State<'_, AppState>) -> Result<String, CommandError> {
    validate_search_params(&query)?;
    let rc = load_search_runtime_config(&app); let ah = app.clone();
    let wd = Arc::clone(&state.workspace_dirs); let sm = Arc::clone(&state.search_metrics);
    let cts = Arc::clone(&state.search_cancellation_tokens);
    let ds = state.disk_result_store.read().clone().ok_or_else(|| CommandError::new("NOT_INITIALIZED", "Disk result store not initialized").with_help("App may be initializing"))?;
    let tp = Arc::clone(&state.search_thread_pool);
    let mr = maxResults.unwrap_or(rc.default_max_results).min(100_000);
    let f = filters.unwrap_or_default();
    let cf = CompiledSearchFilters::compile(&f)?;
    let (raw, sq) = resolve_search_query(&query, structuredQuery, rc.case_sensitive, "search_logs_query")?;
    let ws_id = resolve_workspace_id(workspaceId, &wd)?;
    let sid = uuid::Uuid::new_v4().to_string();
    let token = tokio_util::sync::CancellationToken::new();
    { let mut t = cts.lock(); if let Some(old) = t.get(&sid) { old.cancel(); } t.insert(sid.clone(), token.clone()); }
    { sm.lock().total_searches += 1; }
    let (files, cas) = prepare_search_environment(&ah, &state, &ws_id, &wd, &cf, &ds, &sid).await?;
    let sem = { state.search_engine_managers.lock().get(&ws_id).cloned() };
    // ── Hybrid: Tantivy pre-search before CAS scan ──
    let tantivy_prefetch = if let Some(ref mgr) = sem {
        let require_all = sq.global_operator == la_core::models::search::QueryOperator::And;
        match mgr.search_multi_keyword(&raw, require_all, Some(mr), None, None).await {
            Ok(r) => Some(r.entries),
            Err(_) => None,
        }
    } else { None };
    let bid = sid.clone();
    tokio::spawn(async move {
        let r = run_search_with_timeout(SearchExecutionRequest { app_handle: ah.clone(), search_id: bid.clone(), files_for_search: files, cas, structured_query: sq, compiled_filters: cf, max_results: mr, regex_cache_size: rc.regex_cache_size.max(1), raw_terms: raw, search_metrics: sm, disk_result_store: ds, cancellation_token: token, cancellation_tokens: cts, search_timeout_secs: rc.timeout_seconds, search_thread_pool: tp, search_engine_manager: sem, tantivy_prefetch }).await;
        if let Err(e) = r { if e.code != "TIMEOUT_ERROR" { emit_search_error(&ah, &bid, e.message); } }
    });
    Ok(sid)
}

#[command]
pub async fn cancel_search(#[allow(non_snake_case)] searchId: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let cts = Arc::clone(&state.search_cancellation_tokens);
    let token = { cts.lock().get(&searchId).cloned() };
    if let Some(t) = token { t.cancel(); cts.lock().remove(&searchId); Ok(()) }
    else { Err(CommandError::new("NOT_FOUND", format!("Search {} not found", searchId)).with_help("Search may have already finished")) }
}

#[command]
pub async fn fetch_search_page(state: State<'_, AppState>, #[allow(non_snake_case)] searchId: String, offset: usize, limit: usize) -> Result<la_search::SearchPageResult, CommandError> {
    let limit = limit.min(10000);
    let ds_opt = state.disk_result_store.read();
    let ds = ds_opt.as_ref().ok_or_else(|| CommandError::new("NOT_INITIALIZED", "Disk result store not initialized").with_help("App may be initializing"))?;
    if ds.has_session(&searchId) {
        return ds.read_page(&searchId, offset, limit).map_err(|e| CommandError::from(AppError::io_error(format!("Failed to read page: {e}"), None)).with_help("Results may have been cleared. Try searching again"));
    }
    Err(CommandError::new("NOT_FOUND", format!("Session '{}' not found", searchId)).with_help("Results may have been cleared. Try searching again"))
}

// ============================================================================
// 执行引擎
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn execute_file_search(app_handle: AppHandle, search_id: String, files_for_search: Vec<la_storage::FileMetadata>, cas: Arc<ContentAddressableStorage>, structured_query: SearchQuery, compiled_filters: CompiledSearchFilters, max_results: usize, regex_cache_size: usize, raw_terms: Vec<String>, search_metrics: Arc<Mutex<SearchMetrics>>, disk_store: Arc<la_search::DiskResultStore>, cancellation_token: tokio_util::sync::CancellationToken, cancellation_token_map: Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>, search_thread_pool: Arc<rayon::ThreadPool>, timed_out: Arc<AtomicBool>, _search_engine_manager: Option<Arc<la_search::SearchEngineManager>>, tantivy_prefetch: Option<Vec<LogEntry>>) {
    let start = std::time::Instant::now(); let mut builder = QueryPlanBuilder::new(regex_cache_size);
    let plan = match builder.build(&structured_query) { Ok(p) => p, Err(e) => { emit_search_error(&app_handle, &search_id, format!("Query error: {e}")); disk_store.remove_session(&search_id); remove_cancellation_token(&cancellation_token_map, &search_id); return; } };
    let batch_size = 2000; let mut results_count = 0; let mut was_truncated = false;
    let mut keyword_counts: HashMap<String, usize> = HashMap::new();
    let to = Arc::clone(&timed_out);

    let flush = |batch: &mut Vec<LogEntry>, count: usize| -> bool {
        if batch.is_empty() { return true; }
        if cancellation_token.is_cancelled() { batch.clear(); return true; }
        for _ in 0..3 {
            match disk_store.append_entries(&search_id, batch) { Ok(_) => { if !to.load(Ordering::SeqCst) { let _ = app_handle.emit("search-progress", SearchProgressEvent { search_id: search_id.clone(), count }); let _ = app_handle.emit("search-result-batch", SearchResultBatchEvent { search_id: search_id.clone(), entries: batch.clone(), offset: count - batch.len(), is_final: false }); } batch.clear(); return true; } Err(_e) => { if to.load(Ordering::SeqCst) || cancellation_token.is_cancelled() { batch.clear(); return true; } } }
        }
        batch.clear(); false
    };

    emit_search_id_event(&app_handle, "search-start", &search_id);

    // ── Consume Tantivy pre-fetch ──
    let mut batch_results: Vec<LogEntry> = Vec::new();
    if let Some(prefetch) = tantivy_prefetch {
        for entry in prefetch {
            if results_count >= max_results { was_truncated = true; break; }
            if let Some(ref kw) = entry.matched_keywords { for k in kw { *keyword_counts.entry(k.clone()).or_insert(0) += 1; } }
            let mut e = entry; e.id = results_count; batch_results.push(e); results_count += 1;
            if batch_results.len() >= batch_size && !flush(&mut batch_results, results_count) { break; }
        }
        tracing::info!(tantivy_count = results_count, "Tantivy pre-fetch consumed");
    }

    'outer: for file_batch in files_for_search.chunks(10) {
        if cancellation_token.is_cancelled() { if !timed_out.load(Ordering::SeqCst) { emit_search_id_event(&app_handle, "search-cancelled", &search_id); } remove_cancellation_token(&cancellation_token_map, &search_id); disk_store.remove_session(&search_id); return; }
        if results_count >= max_results { was_truncated = true; break; }
        let batch: Vec<_> = search_thread_pool.install(|| file_batch.par_iter().map(|fm| { if cancellation_token.is_cancelled() { return Vec::new(); } search_single_file_with_details(&format!("cas://{}", fm.sha256_hash), &fm.virtual_path, Some(&*cas), &builder, &plan, &compiled_filters, 0) }).collect());
        for file_results in batch {
            for mut entry in file_results {
                if results_count >= max_results { let _ = flush(&mut batch_results, results_count); was_truncated = true; break 'outer; }
                entry.id = results_count;
                if let Some(ref kw) = entry.matched_keywords { for k in kw { *keyword_counts.entry(k.clone()).or_insert(0) += 1; } }
                batch_results.push(entry); results_count += 1;
                if batch_results.len() >= batch_size && !flush(&mut batch_results, results_count) { break 'outer; }
            }
        }
        if !flush(&mut batch_results, results_count) { break; }
    }
    if !cancellation_token.is_cancelled() {
        let _ = disk_store.complete_session(&search_id);
        let dur = start.elapsed().as_millis() as u64;
        { search_metrics.lock().last_search_duration = std::time::Duration::from_millis(dur); }
        if !timed_out.load(Ordering::SeqCst) {
            let ks: Vec<la_core::models::search_statistics::KeywordStatistics> = raw_terms.iter().map(|t| la_core::models::search_statistics::KeywordStatistics::new(t.clone(), keyword_counts.get(t).copied().unwrap_or(0), results_count)).collect();
            let _ = app_handle.emit("search-summary", SearchSummaryEvent { search_id: search_id.clone(), summary: SearchResultSummary::new(results_count, ks, dur, was_truncated) });
            let _ = app_handle.emit("search-complete", SearchCompleteEvent { search_id: search_id.clone(), total_count: results_count });
            // Streaming: emit final empty batch to signal completion
            let _ = app_handle.emit("search-result-batch", SearchResultBatchEvent { search_id: search_id.clone(), entries: vec![], offset: results_count, is_final: true });
        }
    }
    remove_cancellation_token(&cancellation_token_map, &search_id);
}

// ============================================================================
// 行搜索 + 单文件搜索
// ============================================================================

fn build_log_entry(id: usize, line_number: usize, vpath: &str, rpath: &str, line: Cow<'_, str>, metadata: ParsedLineMetadata, match_details: Option<Vec<crate::services::query_executor::MatchDetail>>) -> LogEntry {
    let keywords = match_details.as_ref().map(|d| d.iter().map(|m| m.term_value.clone()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>());
    LogEntry { id, timestamp: metadata.timestamp.into(), level: metadata.level.into(), file: vpath.into(), real_path: rpath.into(), line: line_number, content: line.into_owned().into(), tags: vec![], match_details, matched_keywords: keywords.filter(|k| !k.is_empty()) }
}

fn search_lines_direct<'a, I: IntoIterator<Item = (usize, Cow<'a, str>)>>(lines: I, vpath: &str, rpath: &str, builder: &QueryPlanBuilder, plan: &ExecutionPlan, global_offset: usize) -> Vec<LogEntry> {
    let mut r = Vec::new();
    for (i, line) in lines { if let Some(dt) = builder.match_with_details(plan, line.as_ref()) { let m = ParsedLineMetadata::parse(line.as_ref(), false); r.push(build_log_entry(global_offset + i, i + 1, vpath, rpath, line, m, Some(dt))); } }
    r
}

#[allow(clippy::too_many_arguments)]
fn flush_search_segment(segment: &mut Vec<SearchLineCandidate<'_>>, summary: &mut SearchSegmentSummary, results: &mut Vec<LogEntry>, vpath: &str, rpath: &str, builder: &QueryPlanBuilder, plan: &ExecutionPlan, filters: &CompiledSearchFilters, global_offset: usize) {
    if segment.is_empty() { return; }
    if filters.segment_may_match(summary) {
        for c in segment.drain(..) { if !filters.matches_parsed_line_metadata(&c.metadata) { continue; } else if let Some(dt) = builder.match_with_details(plan, c.line.as_ref()) { results.push(build_log_entry(global_offset + c.index, c.index + 1, vpath, rpath, c.line, c.metadata, Some(dt))); } }
    } else { segment.clear(); }
    *summary = SearchSegmentSummary::default();
}

fn search_lines_with_segment_pruning<'a, I: IntoIterator<Item = (usize, Cow<'a, str>)>>(lines: I, vpath: &str, rpath: &str, builder: &QueryPlanBuilder, plan: &ExecutionPlan, filters: &CompiledSearchFilters, global_offset: usize) -> Vec<LogEntry> {
    let mut r = Vec::new(); let mut seg = Vec::with_capacity(SEARCH_SEGMENT_LINE_COUNT); let mut sum = SearchSegmentSummary::default(); let nd = filters.has_time_filter();
    for (i, line) in lines { let m = ParsedLineMetadata::parse(line.as_ref(), nd); sum.record(&m); seg.push(SearchLineCandidate { index: i, line, metadata: m }); if seg.len() >= SEARCH_SEGMENT_LINE_COUNT { flush_search_segment(&mut seg, &mut sum, &mut r, vpath, rpath, builder, plan, filters, global_offset); } }
    if !seg.is_empty() { flush_search_segment(&mut seg, &mut sum, &mut r, vpath, rpath, builder, plan, filters, global_offset); }
    r
}

fn search_lines_with_details<'a, I: IntoIterator<Item = (usize, Cow<'a, str>)>>(lines: I, vpath: &str, rpath: &str, builder: &QueryPlanBuilder, plan: &ExecutionPlan, filters: &CompiledSearchFilters, global_offset: usize) -> Vec<LogEntry> {
    if filters.needs_segment_pruning() { search_lines_with_segment_pruning(lines, vpath, rpath, builder, plan, filters, global_offset) } else { search_lines_direct(lines, vpath, rpath, builder, plan, global_offset) }
}

fn search_single_file_with_details(file_identifier: &str, virtual_path: &str, cas_opt: Option<&la_storage::ContentAddressableStorage>, builder: &QueryPlanBuilder, plan: &ExecutionPlan, filters: &CompiledSearchFilters, global_offset: usize) -> Vec<LogEntry> {
    let mut results = Vec::new();
    if let Some(hash) = file_identifier.strip_prefix("cas://") {
        if !filters.matches_file(virtual_path, None) { return results; }
        let cas = match cas_opt { Some(c) => c, None => { error!(%hash, %virtual_path, "CAS not provided"); return results; } };
        if !cas.exists(hash) { warn!(%hash, %virtual_path, "Hash not in CAS"); return results; }
        let content = match cas.read_content_sync(hash) { Ok(b) => b, Err(e) => { warn!(%hash, %virtual_path, %e, "Failed to read CAS"); return vec![LogEntry { id: global_offset, timestamp: "0".into(), level: "ERROR".into(), file: virtual_path.into(), real_path: file_identifier.into(), line: 0, content: format!("[Search system: cannot read file - {e}]").into(), tags: vec![], match_details: None, matched_keywords: None }]; } };
        let (text, _) = decode_log_content(&content);
        results = search_lines_with_details(text.lines().enumerate().map(|(i, l)| (i, Cow::Borrowed(l))), virtual_path, file_identifier, builder, plan, filters, global_offset);
    } else {
        use std::fs::File; use std::io::{BufRead, BufReader}; use std::path::Path;
        if !filters.matches_file(virtual_path, Some(file_identifier)) { return results; }
        let path = Path::new(file_identifier);
        if !path.exists() { return results; }
        if let Ok(file) = File::open(file_identifier) {
            let reader = BufReader::with_capacity(8192, file);
            results = search_lines_with_details(reader.lines().enumerate().filter_map(|(i, lr)| match lr { Ok(l) => Some((i, Cow::Owned(l))), Err(e) => { warn!(file = %file_identifier, line = i + 1, %e, "Failed to read line"); None } }), virtual_path, file_identifier, builder, plan, filters, global_offset);
        }
    }
    results
}
