//! 搜索流水线编排
//!
//! 统一缓存检查、策略选择、结果写入、事件发送与缓存回写。

use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use la_core::error::CommandError;
use la_core::models::search_statistics::SearchResultSummary;
use la_core::models::{LogEntry, SearchCacheKey, SearchFilters, SearchQuery};
use la_search::DiskResultStore;

use crate::commands::import::ensure_workspace_runtime_state;
use crate::models::AppState;
use crate::services::search_events::SearchEventBus;
use crate::services::search_filters::{compile_filters, resolve_search_query, SearchRuntimeConfig};
use crate::services::search_statistics::calculate_keyword_statistics;
use crate::services::search_strategies::{
    CasSearchStrategy, HybridSearchStrategy, SearchStrategy, TantivySearchStrategy,
};
use crate::utils::cache_manager::CacheManager;
use crate::utils::workspace_paths::resolve_workspace_dir;

/// 搜索请求参数
#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub query: String,
    pub structured_query: Option<SearchQuery>,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    pub filters: Option<SearchFilters>,
    pub runtime_config: SearchRuntimeConfig,
}

/// 搜索流水线
#[derive(Clone)]
pub struct SearchPipeline {
    pub cache: Arc<CacheManager>,
    pub disk_store: Arc<DiskResultStore>,
    pub app_handle: AppHandle,
}

impl SearchPipeline {
    pub async fn execute(
        &self,
        request: SearchRequest,
        state: &AppState,
        cancellation_token: CancellationToken,
    ) -> Result<String, CommandError> {
        let event_bus = SearchEventBus::new(self.app_handle.clone());

        let max_results = request
            .max_results
            .unwrap_or(request.runtime_config.default_max_results)
            .min(100_000);
        let filters = request.filters.unwrap_or_default();
        let compiled_filters = compile_filters(&filters)?;
        let case_sensitive = request.runtime_config.case_sensitive;
        let search_timeout_secs = request.runtime_config.timeout_seconds;
        let regex_cache_size = request.runtime_config.regex_cache_size.max(1);
        let (raw_terms, structured_query) = resolve_search_query(
            &request.query,
            request.structured_query,
            case_sensitive,
            "search_logs_query",
        )?;

        // 解析工作区ID
        let workspace_id =
            resolve_workspace_id(request.workspace_id, &state.workspace_dirs, &event_bus)?;

        let search_id = uuid::Uuid::new_v4().to_string();

        // 注册取消令牌
        register_cancellation_token(
            &search_id,
            &state.search_cancellation_tokens,
            cancellation_token.clone(),
        );

        // 构建缓存键
        let query_version = compute_query_version(
            &serde_json::to_string(&structured_query).unwrap_or_else(|_| request.query.clone()),
        );
        let cache_key: SearchCacheKey = (
            request.query.clone(),
            workspace_id.clone(),
            filters.time_start.clone(),
            filters.time_end.clone(),
            filters.levels.clone(),
            filters.file_pattern.clone(),
            false,
            max_results,
            query_version,
        );

        // 1. 检查缓存
        if let Some(cached_results) = self.cache.get_sync(&cache_key) {
            *state.cache_hits.lock() += 1;
            *state.total_searches.lock() += 1;
            {
                let total = *state.total_searches.lock();
                let hits = *state.cache_hits.lock();
                let hit_rate = if total > 0 {
                    (hits as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                info!(total = total, hits = hits, hit_rate = hit_rate, "缓存统计");
            }

            event_bus.emit_start(&search_id);
            write_entries_to_disk(&self.disk_store, &search_id, &cached_results);
            event_bus.emit_progress(&search_id, cached_results.len());

            let raw_term_refs: Vec<&str> = raw_terms.iter().map(String::as_str).collect();
            let keyword_stats = calculate_keyword_statistics(&cached_results, &raw_term_refs);
            let summary = SearchResultSummary::new(cached_results.len(), keyword_stats, 0, false);
            event_bus.emit_summary(&search_id, &summary);
            event_bus.emit_complete(&search_id, cached_results.len());

            cleanup_cancellation_token(&search_id, &state.search_cancellation_tokens);
            return Ok(search_id);
        }

        *state.total_searches.lock() += 1;

        // 2. 获取工作区目录与运行时状态
        let workspace_dir = resolve_workspace_dir_path(
            &self.app_handle,
            &workspace_id,
            &state.workspace_dirs,
            &event_bus,
        )?;

        let (cas, metadata_store, search_engine_manager) =
            ensure_workspace_runtime_state(&self.app_handle, state, &workspace_id, &workspace_dir)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "DATABASE_ERROR",
                        format!("Failed to initialize workspace runtime state: {}", e),
                    )
                    .with_help("Try reloading the workspace before searching again")
                })?;

        // 3. 创建磁盘会话
        if !self.disk_store.has_session(&search_id) {
            if let Err(e) = self.disk_store.create_session(&search_id) {
                warn!(error = %e, search_id = %search_id, "无法创建磁盘搜索会话");
            }
        }

        // 4. 选择并执行策略
        let entries = self
            .run_strategy(
                &structured_query,
                &workspace_id,
                &compiled_filters,
                max_results,
                search_timeout_secs,
                regex_cache_size,
                &search_engine_manager,
                &cas,
                &metadata_store,
                &raw_terms,
                &event_bus,
                &search_id,
                cancellation_token.clone(),
                state,
            )
            .await?;

        // 5. 完成结果写入
        write_entries_to_disk(&self.disk_store, &search_id, &entries);
        if let Err(e) = self.disk_store.complete_session(&search_id) {
            warn!(error = %e, "完成磁盘搜索会话失败");
        }

        // 6. 关键词统计与事件发送
        let raw_term_refs: Vec<&str> = raw_terms.iter().map(String::as_str).collect();
        let keyword_stats = calculate_keyword_statistics(&entries, &raw_term_refs);
        let duration_ms = state.last_search_duration.lock().as_millis() as u64;
        let was_truncated = entries.len() >= max_results;
        let summary =
            SearchResultSummary::new(entries.len(), keyword_stats, duration_ms, was_truncated);

        event_bus.emit_start(&search_id);
        event_bus.emit_progress(&search_id, entries.len());
        event_bus.emit_summary(&search_id, &summary);
        event_bus.emit_complete(&search_id, entries.len());

        // 7. 缓存结果
        if entries.len() < 100_000 && !cancellation_token.is_cancelled() {
            self.cache.insert_sync(cache_key, entries);
        }

        cleanup_cancellation_token(&search_id, &state.search_cancellation_tokens);
        Ok(search_id)
    }

    fn select_strategy(
        &self,
        _filters: &crate::services::search_filters::CompiledSearchFilters,
        regex_cache_size: usize,
        search_engine_manager: &Arc<la_search::SearchEngineManager>,
        cas: &Arc<la_storage::ContentAddressableStorage>,
        metadata_store: &Arc<la_storage::MetadataStore>,
        raw_terms: &[String],
    ) -> Box<dyn SearchStrategy> {
        let tantivy_has_docs = match search_engine_manager.get_time_range() {
            Ok((_, _, total_count)) => total_count > 0,
            Err(_) => false,
        };

        // 当 Tantivy 索引非空时，默认使用混合策略（深度融合，高效回退）
        if tantivy_has_docs {
            info!("使用混合搜索策略（Tantivy + CAS）");
            Box::new(HybridSearchStrategy::new(
                TantivySearchStrategy {
                    engine_manager: Arc::clone(search_engine_manager),
                    raw_terms: raw_terms.to_vec(),
                },
                CasSearchStrategy {
                    cas: Arc::clone(cas),
                    metadata_store: Arc::clone(metadata_store),
                    regex_cache_size,
                },
            ))
        } else {
            info!("使用 CAS 搜索策略");
            Box::new(CasSearchStrategy {
                cas: Arc::clone(cas),
                metadata_store: Arc::clone(metadata_store),
                regex_cache_size,
            })
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_strategy(
        &self,
        query: &SearchQuery,
        workspace_id: &str,
        filters: &crate::services::search_filters::CompiledSearchFilters,
        max_results: usize,
        search_timeout_secs: u64,
        regex_cache_size: usize,
        search_engine_manager: &Arc<la_search::SearchEngineManager>,
        cas: &Arc<la_storage::ContentAddressableStorage>,
        metadata_store: &Arc<la_storage::MetadataStore>,
        raw_terms: &[String],
        event_bus: &SearchEventBus,
        search_id: &str,
        cancellation_token: CancellationToken,
        state: &AppState,
    ) -> Result<Vec<LogEntry>, CommandError> {
        let strategy = self.select_strategy(
            filters,
            regex_cache_size,
            search_engine_manager,
            cas,
            metadata_store,
            raw_terms,
        );

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(search_timeout_secs),
            strategy.execute(
                query,
                workspace_id,
                filters,
                max_results,
                cancellation_token.clone(),
            ),
        )
        .await;

        match result {
            Ok(Ok(entries)) => {
                let duration = start.elapsed();
                *state.last_search_duration.lock() = duration;
                Ok(entries)
            }
            Ok(Err(e)) => {
                error!(error = %e, "Search strategy failed");
                event_bus.emit_error(search_id, &e.to_string());
                Err(e)
            }
            Err(_) => {
                warn!(search_id = %search_id, "Search timed out after {} seconds", search_timeout_secs);
                cancellation_token.cancel();
                cleanup_cancellation_token(search_id, &state.search_cancellation_tokens);
                event_bus.emit_timeout(search_id);
                Err(CommandError::new(
                    "TIMEOUT_ERROR",
                    format!("Search timed out after {} seconds", search_timeout_secs),
                )
                .with_help("Try using more specific search terms to reduce processing time"))
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn compute_query_version(query: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn resolve_workspace_id(
    workspace_id: Option<String>,
    workspace_dirs: &Arc<
        parking_lot::Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>,
    >,
    event_bus: &SearchEventBus,
) -> Result<String, CommandError> {
    if let Some(id) = workspace_id {
        return Ok(id);
    }

    let dirs = workspace_dirs.lock();
    if let Some(first_id) = dirs.keys().next() {
        debug!(
            workspace_id = %first_id,
            available = ?dirs.keys().collect::<Vec<_>>(),
            "Using first available workspace as default"
        );
        Ok(first_id.clone())
    } else {
        let _ = event_bus.app_handle().emit(
            "search-error",
            "No workspaces available. Please create a workspace first.",
        );
        Err(CommandError::new("NOT_FOUND", "No workspaces available")
            .with_help("Please create a workspace first using the import feature"))
    }
}

fn resolve_workspace_dir_path(
    app: &AppHandle,
    workspace_id: &str,
    workspace_dirs: &Arc<
        parking_lot::Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>,
    >,
    event_bus: &SearchEventBus,
) -> Result<std::path::PathBuf, CommandError> {
    let existing = {
        let dirs = workspace_dirs.lock();
        dirs.get(workspace_id).cloned()
    };

    if let Some(dir) = existing {
        Ok(dir)
    } else {
        resolve_workspace_dir(app, workspace_id).map_err(|e| {
            let _ = event_bus.app_handle().emit("search-error", &e);
            CommandError::new("NOT_FOUND", e)
                .with_help("The workspace may have been deleted. Try refreshing the workspace list")
        })
    }
}

fn register_cancellation_token(
    search_id: &str,
    tokens_map: &Arc<parking_lot::Mutex<std::collections::HashMap<String, CancellationToken>>>,
    token: CancellationToken,
) {
    let mut map = tokens_map.lock();
    if let Some(old) = map.get(search_id) {
        tracing::warn!(search_id = %search_id, "Search ID already exists in cancellation tokens, cancelling old token");
        old.cancel();
    }
    map.insert(search_id.to_string(), token);
}

fn cleanup_cancellation_token(
    search_id: &str,
    tokens_map: &Arc<parking_lot::Mutex<std::collections::HashMap<String, CancellationToken>>>,
) {
    tokens_map.lock().remove(search_id);
}

fn write_entries_to_disk(disk_store: &DiskResultStore, search_id: &str, entries: &[LogEntry]) {
    if !disk_store.has_session(search_id) {
        if let Err(e) = disk_store.create_session(search_id) {
            warn!(error = %e, "无法创建磁盘搜索会话");
            return;
        }
    }
    for chunk in entries.chunks(2000) {
        if let Err(e) = disk_store.append_entries(search_id, chunk) {
            warn!(error = %e, "磁盘写入失败");
            break;
        }
    }
}
