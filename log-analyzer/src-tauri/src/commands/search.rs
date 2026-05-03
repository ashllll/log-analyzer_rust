//! 搜索命令实现
//! 包含日志搜索及缓存逻辑，附带关键词统计与结果批量推送
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, trace, warn};

use crate::commands::import::ensure_workspace_runtime_state;
use crate::models::AppState;
use la_core::error::{AppError, CommandError};
use la_core::models::config::AppConfigLoader;
use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use la_core::models::search_statistics::SearchResultSummary;
use la_core::models::{LogEntry, SearchCacheKey, SearchFilters, SearchQuery};
use rayon::prelude::*;
use regex::Regex;

// MessagePack 序列化支持
use serde::{Deserialize, Serialize};

/// 二进制搜索结果结构（用于 MessagePack 传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchResult {
    pub search_id: String,
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub duration_ms: u64,
    pub was_truncated: bool,
}

/// 二进制搜索请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    pub filters: Option<SearchFilters>,
}

// 导入移除: SearchEngineManager 相关类型未使用
use crate::services::file_watcher::TimestampParser;
use crate::services::{calculate_keyword_statistics, parse_metadata, ExecutionPlan, QueryExecutor};
use crate::utils::encoding::decode_log_content;
use crate::utils::workspace_paths::resolve_workspace_dir;

const SEARCH_SEGMENT_LINE_COUNT: usize = 256;

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
            let config = loader.get_config();
            SearchRuntimeConfig {
                default_max_results: config.search.max_results,
                timeout_seconds: config.search.timeout_seconds,
                regex_cache_size: config.cache.regex_cache_size.max(1),
                case_sensitive: config.search.case_sensitive,
            }
        })
        .unwrap_or_default()
}

/// Smart split of a query string by `|` (pipe).
///
/// Rules:
/// - `|` at the top level (not inside any bracket pair) is a term separator.
/// - `\|` is treated as a literal `|` and does NOT split.
/// - `|` inside `()`, `[]`, or `{}` is protected and does NOT split.
pub(crate) fn split_query_by_pipe(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    let mut escaped = false;

    for ch in query.chars() {
        if escaped {
            if ch == '|' {
                current.push('|');
            } else {
                current.push('\\');
                current.push(ch);
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '(' || ch == '[' || ch == '{' {
            depth += 1;
            current.push(ch);
            continue;
        }

        if ch == ')' || ch == ']' || ch == '}' {
            if depth > 0 {
                depth -= 1;
            }
            current.push(ch);
            continue;
        }

        if ch == '|' && depth == 0 {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                terms.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }

        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        terms.push(trimmed.to_string());
    }

    terms
}

pub(crate) fn build_structured_search_query(
    query: &str,
    case_sensitive: bool,
    query_id: &str,
) -> Result<(Vec<String>, SearchQuery), CommandError> {
    let raw_terms: Vec<String> = split_query_by_pipe(query);

    if raw_terms.is_empty() {
        return Err(
            CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                .with_help("Please enter at least one search term"),
        );
    }

    let terms = raw_terms
        .iter()
        .enumerate()
        .map(|(index, value)| SearchTerm {
            id: format!("term_{}", index),
            value: value.clone(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive,
        })
        .collect();

    Ok((
        raw_terms,
        SearchQuery {
            id: query_id.to_string(),
            terms,
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        },
    ))
}

pub(crate) fn resolve_search_query(
    query: &str,
    structured_query: Option<SearchQuery>,
    case_sensitive: bool,
    query_id: &str,
) -> Result<(Vec<String>, SearchQuery), CommandError> {
    if let Some(mut structured_query) = structured_query {
        let raw_terms: Vec<String> = structured_query
            .terms
            .iter()
            .filter(|term| term.enabled)
            .map(|term| term.value.trim())
            .filter(|term| !term.is_empty())
            .map(|term| term.to_string())
            .collect();

        if raw_terms.is_empty() {
            return Err(
                CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                    .with_help("Please enter at least one search term"),
            );
        }

        structured_query.id = query_id.to_string();
        structured_query.filters = None;
        structured_query.metadata = QueryMetadata {
            created_at: 0,
            last_modified: 0,
            execution_count: 0,
            label: None,
        };
        return Ok((raw_terms, structured_query));
    }

    build_structured_search_query(query, case_sensitive, query_id)
}

/// 计算并打印缓存统计信息
fn log_cache_statistics(total_searches: &Arc<Mutex<u64>>, cache_hits: &Arc<Mutex<u64>>) {
    let total = total_searches.lock();
    let hits = cache_hits.lock();
    let hit_rate = if *total > 0 {
        (*hits as f64 / *total as f64) * 100.0
    } else {
        0.0
    };
    info!(
        total = *total,
        hits = *hits,
        hit_rate = hit_rate,
        "Cache statistics"
    );
}

/// 计算查询内容的哈希版本号（用于缓存键区分）
///
/// 使用 SHA-256 哈希算法生成查询的版本标识符，确保不同查询内容
/// 使用不同的缓存键，避免缓存污染。
fn compute_query_version(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 将搜索结果写入 DiskResultStore，统一 Tantivy 和 CAS 路径的写入逻辑
fn write_results_to_disk(
    disk_store: &crate::search_engine::disk_result_store::DiskResultStore,
    search_id: &str,
    entries: &[LogEntry],
    context: &str,
) {
    if !disk_store.has_session(search_id) {
        if let Err(e) = disk_store.create_session(search_id) {
            warn!(error = %e, "{}: 无法创建磁盘搜索会话", context);
            return;
        }
    }
    for chunk in entries.chunks(2000) {
        if let Err(e) = disk_store.append_entries(search_id, chunk) {
            warn!(error = %e, "{}: 磁盘写入失败", context);
            break;
        }
    }
    if let Err(e) = disk_store.complete_session(search_id) {
        warn!(error = %e, "{}: 完成磁盘会话失败", context);
    }
}

/// 计算关键词统计信息
fn compute_keyword_stats(
    entries: &[LogEntry],
    raw_terms: &[String],
) -> Vec<la_core::models::search_statistics::KeywordStatistics> {
    raw_terms
        .iter()
        .map(|term| {
            let count = entries.iter().filter(|e| e.content.contains(term)).count();
            la_core::models::search_statistics::KeywordStatistics::new(
                term.clone(),
                count,
                entries.len(),
            )
        })
        .collect()
}

/// 发送搜索完成事件（search-start / progress / summary / complete）
fn emit_search_complete(
    app_handle: &AppHandle,
    results_count: usize,
    keyword_stats: Vec<la_core::models::search_statistics::KeywordStatistics>,
    duration_ms: u64,
    was_truncated: bool,
) {
    let summary = SearchResultSummary::new(
        results_count,
        keyword_stats,
        duration_ms,
        was_truncated,
    );

    let _ = app_handle.emit("search-start", ());
    let _ = app_handle.emit("search-progress", results_count);
    let _ = app_handle.emit("search-summary", &summary);
    let _ = app_handle.emit("search-complete", results_count);
}

/// 缓存搜索结果（限制大小避免内存爆炸）
fn cache_search_results(
    cache_manager: &Arc<Mutex<crate::utils::cache_manager::CacheManager>>,
    cache_key: SearchCacheKey,
    entries: Vec<LogEntry>,
) {
    if entries.len() < 100_000 {
        cache_manager.lock().insert_sync(cache_key, entries);
    }
}

/// 清理搜索取消令牌
fn remove_cancellation_token(
    cancellation_tokens: &Arc<Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>>,
    search_id: &str,
) {
    let mut tokens = cancellation_tokens.lock();
    tokens.remove(search_id);
}

#[derive(Debug, Clone)]
struct CompiledSearchFilters {
    levels: Option<HashSet<String>>,
    level_mask: Option<u8>,
    time_start: Option<chrono::NaiveDateTime>,
    time_end: Option<chrono::NaiveDateTime>,
    file_matcher: Option<FilePatternMatcher>,
}

#[derive(Debug, Clone)]
struct ParsedLineMetadata {
    timestamp: String,
    level: String,
    level_normalized: String,
    datetime: Option<chrono::NaiveDateTime>,
    level_mask: u8,
}

#[derive(Debug, Clone, Default)]
struct SearchSegmentSummary {
    min_datetime: Option<chrono::NaiveDateTime>,
    max_datetime: Option<chrono::NaiveDateTime>,
    level_mask: u8,
}

#[derive(Debug, Clone)]
struct SearchLineCandidate<'a> {
    index: usize,
    line: Cow<'a, str>,
    metadata: ParsedLineMetadata,
}

#[derive(Debug, Clone)]
enum FilePatternMatcher {
    Substring(String),
    Wildcard(Regex),
}

impl FilePatternMatcher {
    fn compile(raw: &str) -> Result<Self, CommandError> {
        let trimmed = raw.trim();
        if trimmed.contains('*') || trimmed.contains('?') {
            let escaped = regex::escape(trimmed);
            let regex_pattern = format!("^{}$", escaped.replace(r"\*", ".*").replace(r"\?", "."));
            let regex = Regex::new(&regex_pattern).map_err(|e| {
                CommandError::new(
                    "VALIDATION_ERROR",
                    format!("Invalid file pattern '{}': {}", trimmed, e),
                )
                .with_help("Use a valid file pattern such as '*.log' or 'service-error.log'")
            })?;
            Ok(Self::Wildcard(regex))
        } else {
            Ok(Self::Substring(trimmed.to_string()))
        }
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Substring(pattern) => value.contains(pattern),
            Self::Wildcard(regex) => regex.is_match(value),
        }
    }
}

fn level_to_mask(level: &str) -> u8 {
    match level.trim().to_ascii_lowercase().as_str() {
        "error" => 1 << 0,
        "warn" | "warning" => 1 << 1,
        "info" => 1 << 2,
        "debug" => 1 << 3,
        _ => 0,
    }
}

impl ParsedLineMetadata {
    fn parse(line: &str, needs_datetime: bool) -> Self {
        let (timestamp, level) = parse_metadata(line);
        let level_normalized = level.to_ascii_lowercase();
        let datetime = if needs_datetime {
            TimestampParser::parse_naive_datetime(&timestamp)
        } else {
            None
        };

        Self {
            timestamp,
            level,
            level_normalized: level_normalized.clone(),
            datetime,
            level_mask: level_to_mask(&level_normalized),
        }
    }
}

impl SearchSegmentSummary {
    fn record(&mut self, metadata: &ParsedLineMetadata) {
        self.level_mask |= metadata.level_mask;

        if let Some(datetime) = metadata.datetime {
            self.min_datetime = Some(
                self.min_datetime
                    .map(|current| current.min(datetime))
                    .unwrap_or(datetime),
            );
            self.max_datetime = Some(
                self.max_datetime
                    .map(|current| current.max(datetime))
                    .unwrap_or(datetime),
            );
        }
    }
}

impl CompiledSearchFilters {
    fn compile(filters: &SearchFilters) -> Result<Self, CommandError> {
        let levels = if filters.levels.is_empty() {
            None
        } else {
            Some(
                filters
                    .levels
                    .iter()
                    .map(|level| level.trim())
                    .filter(|level| !level.is_empty())
                    .map(|level| level.to_ascii_lowercase())
                    .collect::<HashSet<_>>(),
            )
            .filter(|levels| !levels.is_empty())
        };
        let level_mask = levels.as_ref().map(|levels| {
            levels
                .iter()
                .fold(0u8, |mask, level| mask | level_to_mask(level))
        });

        let time_start = Self::parse_filter_datetime(filters.time_start.as_deref(), "start time")?;
        let time_end = Self::parse_filter_datetime(filters.time_end.as_deref(), "end time")?;

        if let (Some(start), Some(end)) = (time_start, time_end) {
            if start > end {
                return Err(CommandError::new(
                    "VALIDATION_ERROR",
                    "Search filter start time cannot be later than end time",
                )
                .with_help("Adjust the selected time range and try again"));
            }
        }

        let file_matcher = filters
            .file_pattern
            .as_deref()
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .map(FilePatternMatcher::compile)
            .transpose()?;

        Ok(Self {
            levels,
            level_mask,
            time_start,
            time_end,
            file_matcher,
        })
    }

    fn parse_filter_datetime(
        value: Option<&str>,
        label: &str,
    ) -> Result<Option<chrono::NaiveDateTime>, CommandError> {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(None);
        };

        TimestampParser::parse_naive_datetime(value)
            .ok_or_else(|| {
                CommandError::new(
                "VALIDATION_ERROR",
                format!("Invalid {} '{}'", label, value),
            )
            .with_help(
                "Use a valid datetime value such as '2024-01-15T10:30' or '2024-01-15 10:30:45'",
            )
            })
            .map(Some)
    }

    fn matches_file(&self, virtual_path: &str, real_path: Option<&str>) -> bool {
        let Some(matcher) = &self.file_matcher else {
            return true;
        };

        matcher.matches(virtual_path) || real_path.is_some_and(|path| matcher.matches(path))
    }

    #[cfg(test)]
    fn matches_line_metadata(&self, timestamp: &str, level: &str) -> bool {
        let metadata = ParsedLineMetadata {
            timestamp: timestamp.to_string(),
            level: level.to_string(),
            level_normalized: level.to_ascii_lowercase(),
            datetime: if self.has_time_filter() {
                TimestampParser::parse_naive_datetime(timestamp)
            } else {
                None
            },
            level_mask: level_to_mask(level),
        };

        self.matches_parsed_line_metadata(&metadata)
    }

    fn matches_parsed_line_metadata(&self, metadata: &ParsedLineMetadata) -> bool {
        if let Some(levels) = &self.levels {
            if !levels.contains(&metadata.level_normalized) {
                return false;
            }
        }

        if !self.has_time_filter() {
            return true;
        }

        let Some(entry_dt) = metadata.datetime else {
            return false;
        };

        if let Some(start) = self.time_start {
            if entry_dt < start {
                return false;
            }
        }

        if let Some(end) = self.time_end {
            if entry_dt > end {
                return false;
            }
        }

        true
    }

    fn has_time_filter(&self) -> bool {
        self.time_start.is_some() || self.time_end.is_some()
    }

    fn needs_segment_pruning(&self) -> bool {
        self.levels.is_some() || self.has_time_filter()
    }

    fn segment_may_match(&self, summary: &SearchSegmentSummary) -> bool {
        if let Some(levels) = &self.levels {
            if self.level_mask.unwrap_or(0) == 0 && !levels.is_empty() {
                return false;
            }

            if summary.level_mask & self.level_mask.unwrap_or(0) == 0 {
                return false;
            }
        }

        if !self.has_time_filter() {
            return true;
        }

        // 如果段内没有可解析的时间戳，不跳过整个段。
        // 这些行可能仍然匹配关键词搜索，应由逐行过滤处理。
        let (Some(min_datetime), Some(max_datetime)) = (summary.min_datetime, summary.max_datetime)
        else {
            return true;
        };

        if let Some(start) = self.time_start {
            if max_datetime < start {
                return false;
            }
        }

        if let Some(end) = self.time_end {
            if min_datetime > end {
                return false;
            }
        }

        true
    }
}

#[command]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] structuredQuery: Option<SearchQuery>,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    #[allow(non_snake_case)] maxResults: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
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

    let runtime_config = load_search_runtime_config(&app);

    let app_handle = app.clone();
    let workspace_dirs: Arc<Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>> =
        Arc::clone(&state.workspace_dirs);
    let cache_manager: Arc<Mutex<crate::utils::cache_manager::CacheManager>> =
        Arc::clone(&state.cache_manager);
    let total_searches: Arc<Mutex<u64>> = Arc::clone(&state.total_searches);
    let cache_hits: Arc<Mutex<u64>> = Arc::clone(&state.cache_hits);
    let last_search_duration: Arc<Mutex<std::time::Duration>> =
        Arc::clone(&state.last_search_duration);
    let cancellation_tokens: Arc<
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    > = Arc::clone(&state.search_cancellation_tokens);
    // 磁盘搜索结果存储：新架构的核心，替代 search-results IPC 事件
    let disk_result_store: Arc<crate::search_engine::disk_result_store::DiskResultStore> =
        state.disk_result_store.read().clone();

    let max_results = maxResults
        .unwrap_or(runtime_config.default_max_results)
        .min(100_000);
    let filters = filters.unwrap_or_default();
    let compiled_filters = CompiledSearchFilters::compile(&filters)?;
    let case_sensitive = runtime_config.case_sensitive;
    let search_timeout_secs = runtime_config.timeout_seconds;
    let regex_cache_size = runtime_config.regex_cache_size.max(1);
    let (raw_terms, structured_query) =
        resolve_search_query(&query, structuredQuery, case_sensitive, "search_logs_query")?;

    // 修复工作区ID处理：当没有提供workspaceId时，使用第一个可用的工作区而不是硬编码的"default"
    let workspace_id = if let Some(ref id) = workspaceId {
        id.clone()
    } else {
        // 当没有提供工作区ID时，获取第一个可用的工作区
        let dirs = workspace_dirs.lock();
        if let Some(first_workspace_id) = dirs.keys().next() {
            debug!(
                workspace_id = %first_workspace_id,
                available_workspaces = ?dirs.keys().collect::<Vec<_>>(),
                "Using first available workspace as default"
            );
            first_workspace_id.clone()
        } else {
            // 如果没有可用的工作区，返回明确的错误
            let _ = app.emit(
                "search-error",
                "No workspaces available. Please create a workspace first.",
            );
            return Err(CommandError::new("NOT_FOUND", "No workspaces available")
                .with_help("Please create a workspace first using the import feature"));
        }
    };

    // 生成唯一的搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();

    // 创建取消令牌
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = cancellation_tokens.lock();
        if let Some(old_token) = tokens.get(&search_id) {
            tracing::warn!(
                search_id = %search_id,
                "Search ID already exists in cancellation tokens, cancelling old token"
            );
            old_token.cancel();
        }
        tokens.insert(search_id.clone(), cancellation_token.clone());
    }

    // 缓存键：基于查询参数生成，使用查询内容的哈希作为版本号
    // 使用 SHA-256 哈希确保不同查询使用不同缓存键，避免缓存污染
    let query_version = compute_query_version(
        &serde_json::to_string(&structured_query).unwrap_or_else(|_| query.clone()),
    );
    let cache_key: SearchCacheKey = (
        query.clone(),
        workspace_id.clone(),
        filters.time_start.clone(),
        filters.time_end.clone(),
        filters.levels.clone(),
        filters.file_pattern.clone(),
        false, // 全局 case_sensitive 占位（per-term 大小写敏感已包含在 query.terms 中，此维度暂保留以维持缓存键类型兼容）
        max_results,
        query_version, // 使用 SHA-256 哈希作为版本号
    );

    {
        // 使用 CacheManager 的同步 get 方法
        let cache = state.cache_manager.lock();
        let cache_result = cache.get_sync(&cache_key);

        if let Some(cached_results) = cache_result {
            {
                let mut hits = cache_hits.lock();
                *hits += 1;
            }
            {
                let mut searches = total_searches.lock();
                *searches += 1;
            }

            // 记录缓存统计
            log_cache_statistics(&total_searches, &cache_hits);

            // 缓存命中时也需发送 search-start，使前端清空旧结果、重置滚动位置
            let _ = app_handle.emit("search-start", ());

            // 将缓存结果写入磁盘（新架构：不通过 IPC 发送原始数据，前端按需分页读取）
            if let Err(e) = disk_result_store.create_session(&search_id) {
                warn!(error = %e, "缓存命中：无法创建磁盘搜索会话");
            } else {
                for chunk in cached_results.chunks(2000) {
                    let chunk: &[la_core::models::LogEntry] = chunk;
                    if let Err(e) = disk_result_store.append_entries(&search_id, chunk) {
                        warn!(error = %e, "缓存命中：磁盘写入失败");
                        break;
                    }
                }
                if let Err(e) = disk_result_store.complete_session(&search_id) {
                    warn!(error = %e, "缓存命中：完成磁盘会话失败");
                }
            }
            // 仅发送实时计数事件，前端通过 fetch_search_page 按需读取实际数据
            let _ = app_handle.emit("search-progress", cached_results.len());

            let raw_term_refs: Vec<&str> = raw_terms.iter().map(String::as_str).collect();
            let keyword_stats = calculate_keyword_statistics(&cached_results, &raw_term_refs);
            let summary = SearchResultSummary::new(cached_results.len(), keyword_stats, 0, false);

            let _ = app_handle.emit("search-summary", &summary);
            let _ = app_handle.emit("search-complete", cached_results.len());
            // 缓存命中路径：清理之前创建的 cancellation token（缓存路径无需保留）
            {
                let mut tokens = cancellation_tokens.lock();
                tokens.remove(&search_id);
            }
            return Ok(search_id);
        }
    }

    {
        let mut searches = total_searches.lock();
        *searches += 1;
    }

    let workspace_dir = {
        let existing = {
            let dirs = workspace_dirs.lock();
            dirs.get(&workspace_id).cloned()
        };

        if let Some(dir) = existing {
            dir
        } else {
            resolve_workspace_dir(&app_handle, &workspace_id).map_err(|e| {
                let _ = app_handle.emit("search-error", &e);
                CommandError::new("NOT_FOUND", e).with_help(
                    "The workspace may have been deleted. Try refreshing the workspace list",
                )
            })?
        }
    };

    let (cas, metadata_store, search_engine_manager) =
        ensure_workspace_runtime_state(&app_handle, &state, &workspace_id, &workspace_dir)
            .await
            .map_err(|e| {
                CommandError::new(
                    "DATABASE_ERROR",
                    format!("Failed to initialize workspace runtime state: {}", e),
                )
                .with_help("Try reloading the workspace before searching again")
            })?;

    // ========== Tantivy 优先路径 ==========
    // 如果索引非空，优先使用 Tantivy 查询（亚秒级 vs CAS 扫描的秒级）
    let tantivy_has_docs = match search_engine_manager.get_time_range() {
        Ok((_, _, total_count)) => total_count > 0,
        Err(_) => false,
    };

    if tantivy_has_docs {
        let tantivy_query = raw_terms.join(" OR ");
        let tantivy_start = std::time::Instant::now();

        match search_engine_manager
            .search_with_timeout(
                &tantivy_query,
                Some(max_results),
                Some(std::time::Duration::from_millis(500)),
                Some(cancellation_token.clone()),
            )
            .await
        {
            Ok(tantivy_results) => {
                info!(
                    query = %tantivy_query,
                    hits = tantivy_results.entries.len(),
                    total = tantivy_results.total_count,
                    ms = tantivy_start.elapsed().as_millis(),
                    "Tantivy search succeeded"
                );

                let entries = tantivy_results.entries;
                let results_count = entries.len();

                write_results_to_disk(&disk_result_store, &search_id, &entries, "Tantivy");

                let keyword_stats = compute_keyword_stats(&entries, &raw_terms);
                emit_search_complete(
                    &app_handle,
                    results_count,
                    keyword_stats,
                    tantivy_start.elapsed().as_millis() as u64,
                    false,
                );

                cache_search_results(&cache_manager, cache_key, entries);
                remove_cancellation_token(&cancellation_tokens, &search_id);

                return Ok(search_id);
            }
            Err(crate::search_engine::SearchError::Timeout(_)) => {
                warn!("Tantivy search timed out, falling back to CAS scan");
            }
            Err(e) => {
                warn!(error = %e, "Tantivy search failed, falling back to CAS scan");
            }
        }
    }

    // Get all files from MetadataStore BEFORE spawn_blocking
    let files = match metadata_store.get_all_files().await {
        Ok(result) => result,
        Err(e) => {
            error!(
                workspace_id = %workspace_id,
                error = %e,
                "Failed to get files from metadata store"
            );
            let _ = app_handle.emit(
                "search-error",
                format!(
                    "Internal error occurred while accessing workspace: {}",
                    workspace_id
                ),
            );
            return Err(CommandError::new("DATABASE_ERROR", format!("Internal error occurred while accessing workspace: {}", workspace_id))
                .with_help("The workspace database may be corrupted. Try refreshing or reimporting the workspace"));
        }
    };

    let search_id_clone = search_id.clone();
    let files_for_search: Vec<_> = files
        .into_iter()
        .filter(|file| compiled_filters.matches_file(&file.virtual_path, None))
        .collect();

    // 创建磁盘搜索会话（仅在非缓存命中路径下创建，缓存命中路径已创建）
    if !disk_result_store.has_session(&search_id) {
        if let Err(e) = disk_result_store.create_session(&search_id) {
            warn!(error = %e, search_id = %search_id, "无法创建磁盘搜索会话，分页读取功能将不可用");
        }
    }
    let disk_store_spawn = Arc::clone(&disk_result_store);
    let compiled_filters_for_search = compiled_filters.clone();

    // 为超时处理克隆必要的变量
    let app_handle_for_timeout = app_handle.clone();
    let cancellation_token_for_timeout = cancellation_token.clone();
    let cancellation_tokens_for_timeout = Arc::clone(&cancellation_tokens);
    let disk_store_for_timeout = Arc::clone(&disk_result_store);
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_timeout = Arc::clone(&timed_out);
    let timed_out_for_search = Arc::clone(&timed_out);

    let handle = tokio::task::spawn_blocking(move || {
        let start_time = std::time::Instant::now();

        let mut executor = QueryExecutor::new(regex_cache_size);
        let plan = match executor.execute(&structured_query) {
            Ok(p) => p,
            Err(e) => {
                let _ = app_handle.emit("search-error", format!("Query execution error: {}", e));
                return;
            }
        };

        // Note: workspace_dir, metadata_store, cas, and files are now obtained
        // BEFORE spawn_blocking to avoid nested runtime blocking issues

        debug!(
            total_files = files_for_search.len(),
            workspace_id = %workspace_id,
            "Starting search across files using CAS"
        );

        // 流式处理：分批发送结果，避免内存峰值
        // 优化：batch_size 从 500 增加到 2000，减少 IPC 调用次数 75%，提高吞吐量
        let batch_size = 2000;
        let mut total_processed = 0;
        let mut results_count = 0;
        // 流式统计：使用HashMap增量统计关键词，避免累积所有结果
        let mut keyword_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut was_truncated = false;
        let mut all_results: Vec<LogEntry> = Vec::new(); // 用于缓存的完整结果集
        const MAX_CACHE_SIZE: usize = 100_000; // 限制缓存中的结果数量

        let flush_batch = |batch: &mut Vec<LogEntry>, progress_count: usize| {
            if batch.is_empty() {
                return;
            }

            if cancellation_token.is_cancelled() {
                batch.clear();
                return;
            }

            // 修复：限制重试次数为 3 次，超过后丢弃批次，避免无限重试和重复数据
            const MAX_RETRIES: usize = 3;
            let mut last_err = None;
            for _ in 0..MAX_RETRIES {
                match disk_store_spawn.append_entries(&search_id_clone, batch) {
                    Ok(_) => {
                        batch.clear();
                        if !timed_out_for_search.load(Ordering::SeqCst) {
                            let _ = app_handle.emit("search-progress", progress_count);
                        }
                        return;
                    }
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
            if let Some(e) = last_err {
                error!(
                    error = %e,
                    retries = MAX_RETRIES,
                    "磁盘写入搜索结果批次失败，丢弃批次以避免重复数据"
                );
            }
            batch.clear(); // 丢弃批次，防止下次 flush 重复输出
        };

        // 先发送开始事件
        let _ = app_handle.emit("search-start", "Starting search...");

        'outer: for file_batch in files_for_search.chunks(10) {
            // 检查取消状态
            if cancellation_token.is_cancelled() {
                if !timed_out_for_search.load(Ordering::SeqCst) {
                    let _ = app_handle.emit("search-cancelled", search_id_clone.clone());
                }
                // 清理取消令牌
                {
                    let mut tokens = cancellation_tokens.lock();
                    tokens.remove(&search_id_clone);
                }
                // 清理磁盘会话（.ndjson 和 .idx 文件），避免文件泄漏和会话槽位占用
                disk_store_spawn.remove_session(&search_id_clone);
                return;
            }

            // 检查是否已达到max_results限制
            if results_count >= max_results {
                was_truncated = true;
                break 'outer;
            }

            // 每批处理10个文件
            let mut batch_results: Vec<LogEntry> = Vec::new();

            // 并行处理当前批次 (Requirements 2.3: 使用 CAS 读取内容)
            let batch: Vec<_> = file_batch
                .par_iter()
                .enumerate()
                .map(|(idx, file_metadata)| {
                    // 如果已经取消，尽早退出单个文件的搜索（虽然是同步的，但检查可以减少无效工作）
                    if cancellation_token.is_cancelled() {
                        return Vec::new();
                    }

                    // Use CAS-based access with hash
                    let file_identifier = format!("cas://{}", file_metadata.sha256_hash);
                    search_single_file_with_details(
                        &file_identifier,
                        &file_metadata.virtual_path,
                        Some(&*cas), // Pass CAS instance for hash-based access
                        &executor,
                        &plan,
                        &compiled_filters_for_search,
                        total_processed + idx * 10000,
                    )
                })
                .collect();

            // 如果批次处理过程中取消了，直接退出
            if cancellation_token.is_cancelled() {
                continue 'outer; // 下次循环首部会处理取消逻辑
            }

            // 收集当前批次的结果
            for file_results in batch {
                if cancellation_token.is_cancelled() {
                    batch_results.clear();
                    continue 'outer;
                }

                for mut entry in file_results {
                    if cancellation_token.is_cancelled() {
                        batch_results.clear();
                        continue 'outer;
                    }

                    // 检查是否已达到max_results限制
                    if results_count >= max_results {
                        flush_batch(&mut batch_results, results_count);
                        was_truncated = true;
                        break 'outer;
                    }

                    entry.id = results_count;

                    // 流式统计：增量更新关键词计数
                    if let Some(ref keywords) = entry.matched_keywords {
                        for kw in keywords {
                            *keyword_counts.entry(kw.clone()).or_insert(0) += 1;
                        }
                    }

                    // 保存到完整结果集用于缓存（限制大小）
                    if all_results.len() < MAX_CACHE_SIZE {
                        all_results.push(entry.clone());
                    } else if all_results.len() == MAX_CACHE_SIZE {
                        // 首次达到限制时记录警告
                        tracing::warn!(
                            "Cache size limit reached ({}), additional results will not be cached",
                            MAX_CACHE_SIZE
                        );
                    }
                    batch_results.push(entry);
                    results_count += 1;

                    // 批次满时写入磁盘并发送实时计数（不再发送原始数据到前端）
                    if batch_results.len() >= batch_size {
                        flush_batch(&mut batch_results, results_count);
                    }
                }
            }

            // 将当前文件批次尚未发送的结果立即写盘，避免下一轮截断时丢失尾批次。
            flush_batch(&mut batch_results, results_count);

            total_processed += file_batch.len();
        }

        if cancellation_token.is_cancelled() {
            {
                let mut tokens = cancellation_tokens.lock();
                tokens.remove(&search_id_clone);
            }
            disk_store_spawn.remove_session(&search_id_clone);
            return;
        }

        // 完成磁盘会话，确保所有写入对读者可见
        if let Err(e) = disk_store_spawn.complete_session(&search_id_clone) {
            warn!(error = %e, "完成磁盘搜索会话失败");
        }

        // 计算搜索统计信息
        let duration = start_time.elapsed().as_millis() as u64;
        {
            let mut last_duration = last_search_duration.lock();
            *last_duration = std::time::Duration::from_millis(duration);
        }

        // 使用流式统计结果构建关键词统计
        let keyword_stats: Vec<la_core::models::search_statistics::KeywordStatistics> = raw_terms
            .iter()
            .map(|term| {
                let count = keyword_counts.get(term).copied().unwrap_or(0);
                la_core::models::search_statistics::KeywordStatistics::new(
                    term.clone(),
                    count,
                    results_count,
                )
            })
            .collect();

        // 将结果插入缓存(仅在未截断且未取消时缓存)
        if !was_truncated && !cancellation_token.is_cancelled() {
            cache_manager.lock().insert_sync(cache_key, all_results);
        }

        if !timed_out_for_search.load(Ordering::SeqCst) {
            let _ = app_handle.emit("search-summary", &SearchResultSummary::new(
                results_count,
                keyword_stats,
                duration,
                was_truncated,
            ));
            let _ = app_handle.emit("search-complete", results_count);
        }

        remove_cancellation_token(&cancellation_tokens, &search_id_clone);
    });

    // 添加超时控制，等待搜索任务完成
    match tokio::time::timeout(std::time::Duration::from_secs(search_timeout_secs), handle).await {
        Ok(Ok(())) => Ok(search_id),
        Ok(Err(e)) => {
            error!(error = %e, search_id = %search_id, "Search task panicked");
            Err(
                CommandError::new("INTERNAL_ERROR", format!("Search task panicked: {}", e))
                    .with_help("This is an unexpected error. Try simplifying your search query"),
            )
        }
        Err(_) => {
            warn!(search_id = %search_id, "Search timed out after {} seconds", search_timeout_secs);
            timed_out_for_timeout.store(true, Ordering::SeqCst);
            cancellation_token_for_timeout.cancel();
            {
                let mut tokens = cancellation_tokens_for_timeout.lock();
                tokens.remove(&search_id);
            }
            // 清理磁盘会话，避免文件泄漏和会话槽位占用
            disk_store_for_timeout.remove_session(&search_id);
            // 发送超时事件
            let _ = app_handle_for_timeout.emit("search-timeout", &search_id);
            Err(CommandError::new(
                "TIMEOUT_ERROR",
                format!("Search timed out after {} seconds", search_timeout_secs),
            )
            .with_help("Try using more specific search terms to reduce processing time"))
        }
    }
}

/// 取消正在进行的搜索
#[command]
pub async fn cancel_search(
    #[allow(non_snake_case)] searchId: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let cancellation_tokens: Arc<
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    > = Arc::clone(&state.search_cancellation_tokens);

    let token = {
        let tokens = cancellation_tokens.lock();
        tokens.get(&searchId).cloned()
    };

    if let Some(token) = token {
        token.cancel();
        Ok(())
    } else {
        Err(CommandError::new(
            "NOT_FOUND",
            format!("Search with ID {} not found or already completed", searchId),
        )
        .with_help("The search may have already finished or been cancelled"))
    }
}

fn search_lines_with_details<'a, I>(
    lines: I,
    virtual_path: &str,
    real_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    global_offset: usize,
) -> Vec<LogEntry>
where
    I: IntoIterator<Item = (usize, Cow<'a, str>)>,
{
    if filters.needs_segment_pruning() {
        search_lines_with_segment_pruning(
            lines,
            virtual_path,
            real_path,
            executor,
            plan,
            filters,
            global_offset,
        )
    } else {
        search_lines_direct(
            lines,
            virtual_path,
            real_path,
            executor,
            plan,
            global_offset,
        )
    }
}

fn search_lines_direct<'a, I>(
    lines: I,
    virtual_path: &str,
    real_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Vec<LogEntry>
where
    I: IntoIterator<Item = (usize, Cow<'a, str>)>,
{
    let mut results = Vec::new();

    for (index, line) in lines {
        let line_ref = line.as_ref();
        let Some(match_details) = executor.match_with_details(plan, line_ref) else {
            continue;
        };

        let metadata = ParsedLineMetadata::parse(line_ref, false);
        results.push(build_log_entry(
            global_offset + index,
            index + 1,
            virtual_path,
            real_path,
            line,
            metadata,
            Some(match_details),
        ));
    }

    results
}

fn search_lines_with_segment_pruning<'a, I>(
    lines: I,
    virtual_path: &str,
    real_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    global_offset: usize,
) -> Vec<LogEntry>
where
    I: IntoIterator<Item = (usize, Cow<'a, str>)>,
{
    let mut results = Vec::new();
    let mut segment = Vec::with_capacity(SEARCH_SEGMENT_LINE_COUNT);
    let mut summary = SearchSegmentSummary::default();
    let needs_datetime = filters.has_time_filter();

    for (index, line) in lines {
        let metadata = ParsedLineMetadata::parse(line.as_ref(), needs_datetime);
        summary.record(&metadata);
        segment.push(SearchLineCandidate {
            index,
            line,
            metadata,
        });

        if segment.len() >= SEARCH_SEGMENT_LINE_COUNT {
            flush_search_segment(
                &mut segment,
                &mut summary,
                &mut results,
                virtual_path,
                real_path,
                executor,
                plan,
                filters,
                global_offset,
            );
        }
    }

    if !segment.is_empty() {
        flush_search_segment(
            &mut segment,
            &mut summary,
            &mut results,
            virtual_path,
            real_path,
            executor,
            plan,
            filters,
            global_offset,
        );
    }

    results
}

#[allow(clippy::too_many_arguments)]
fn flush_search_segment(
    segment: &mut Vec<SearchLineCandidate<'_>>,
    summary: &mut SearchSegmentSummary,
    results: &mut Vec<LogEntry>,
    virtual_path: &str,
    real_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    global_offset: usize,
) {
    if segment.is_empty() {
        return;
    }

    if filters.segment_may_match(summary) {
        for candidate in segment.drain(..) {
            if !filters.matches_parsed_line_metadata(&candidate.metadata) {
                continue;
            }

            let Some(match_details) = executor.match_with_details(plan, candidate.line.as_ref())
            else {
                continue;
            };

            results.push(build_log_entry(
                global_offset + candidate.index,
                candidate.index + 1,
                virtual_path,
                real_path,
                candidate.line,
                candidate.metadata,
                Some(match_details),
            ));
        }
    } else {
        segment.clear();
    }

    *summary = SearchSegmentSummary::default();
}

fn build_log_entry(
    id: usize,
    line_number: usize,
    virtual_path: &str,
    real_path: &str,
    line: Cow<'_, str>,
    metadata: ParsedLineMetadata,
    match_details: Option<Vec<crate::services::query_executor::MatchDetail>>,
) -> LogEntry {
    let matched_keywords = match_details.as_ref().map(|details| {
        details
            .iter()
            .map(|detail| detail.term_value.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    });

    LogEntry {
        id,
        timestamp: metadata.timestamp.into(),
        level: metadata.level.into(),
        file: virtual_path.into(),
        real_path: real_path.into(),
        line: line_number,
        content: line.into_owned().into(),
        tags: vec![],
        match_details,
        matched_keywords: matched_keywords.filter(|keywords| !keywords.is_empty()),
    }
}

/// Search a single file with support for both path-based and hash-based access
///
/// This function supports two modes:
/// 1. Hash-based (CAS): When `file_identifier` starts with "cas://", reads from CAS
/// 2. Path-based (legacy): When `file_identifier` is a file path, reads from filesystem
///
/// # Arguments
///
/// * `file_identifier` - Either "cas://<hash>" for CAS access or a file path for legacy access
/// * `virtual_path` - Virtual path for display purposes
/// * `cas_opt` - Optional Content-Addressable Storage instance (required for CAS mode)
/// * `executor` - Query executor for matching
/// * `plan` - Execution plan for the query
/// * `global_offset` - Offset for line numbering
///
/// # Returns
///
/// Vector of matching log entries
fn search_single_file_with_details(
    file_identifier: &str,
    virtual_path: &str,
    cas_opt: Option<&crate::storage::ContentAddressableStorage>,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    // Determine if this is CAS-based or path-based access
    if let Some(sha256_hash) = file_identifier.strip_prefix("cas://") {
        if !filters.matches_file(virtual_path, None) {
            return results;
        }

        // Hash-based access via CAS

        let cas = match cas_opt {
            Some(c) => c,
            None => {
                error!(
                    hash = %sha256_hash,
                    virtual_path = %virtual_path,
                    "CAS instance not provided for hash-based access"
                );
                return results;
            }
        };

        // Verify hash exists in CAS before reading (Requirements 8.1, 8.3)
        if !cas.exists(sha256_hash) {
            warn!(
                hash = %sha256_hash,
                virtual_path = %virtual_path,
                "Hash does not exist in CAS, skipping file"
            );
            return results;
        }

        let object_size = cas.object_size_sync(sha256_hash);

        if object_size > 1024 * 1024 {
            // 大文件（>1MB）：mmap 路径，避免 Vec<u8> 全量加载
            match cas.read_content_mmap_sync(sha256_hash) {
                Ok(mmap) => {
                    // 优先零拷贝 UTF-8 解码（日志文件 95%+ 是纯 UTF-8）
                    // 使用 simdutf8 加速 UTF-8 验证，比 std::str::from_utf8 快 5-20x
                    match simdutf8::compat::from_utf8(&mmap) {
                        Ok(text) => {
                            results = search_lines_with_details(
                                text.lines()
                                    .enumerate()
                                    .map(|(index, line)| (index, Cow::Borrowed(line))),
                                virtual_path,
                                file_identifier,
                                executor,
                                plan,
                                filters,
                                global_offset,
                            );
                        }
                        Err(_) => {
                            // 非 UTF-8，回退到容错解码（需复制到堆）
                            let (content_str, encoding_info) = decode_log_content(&mmap);
                            if encoding_info.had_errors {
                                debug!(
                                    hash = %sha256_hash,
                                    virtual_path = %virtual_path,
                                    encoding = %encoding_info.encoding,
                                    fallback_used = encoding_info.fallback_used,
                                    "Large file decoded with encoding fallback via mmap"
                                );
                            }
                            results = search_lines_with_details(
                                content_str
                                    .lines()
                                    .enumerate()
                                    .map(|(index, line)| (index, Cow::Borrowed(line))),
                                virtual_path,
                                file_identifier,
                                executor,
                                plan,
                                filters,
                                global_offset,
                            );
                        }
                    }

                    trace!(
                        hash = %sha256_hash,
                        virtual_path = %virtual_path,
                        size = object_size,
                        matches = results.len(),
                        "Searched large file via mmap"
                    );
                    return results;
                }
                Err(e) => {
                    warn!(
                        hash = %sha256_hash,
                        virtual_path = %virtual_path,
                        error = %e,
                        "Failed to mmap content from CAS, falling back to standard read"
                    );
                    // 不返回错误，继续执行下方的标准读取路径作为回退
                }
            }
        }

        // 小文件（<=1MB）或 mmap 失败回退：标准读取路径，避免 mmap 固定开销
        let content = match cas.read_content_sync(sha256_hash) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(
                    hash = %sha256_hash,
                    virtual_path = %virtual_path,
                    error = %e,
                    "Failed to read content from CAS, skipping file"
                );
                results.push(LogEntry {
                    id: global_offset,
                    timestamp: "0".into(),
                    level: "ERROR".into(),
                    file: virtual_path.into(),
                    real_path: file_identifier.into(),
                    line: 0,
                    content: format!("[搜索系统: 无法读取文件内容 - {e}]").into(),
                    tags: vec![],
                    match_details: None,
                    matched_keywords: None,
                });
                return results;
            }
        };

        let (content_str, encoding_info) = decode_log_content(&content);
        drop(content);

        if encoding_info.had_errors {
            debug!(
                hash = %sha256_hash,
                virtual_path = %virtual_path,
                encoding = %encoding_info.encoding,
                fallback_used = encoding_info.fallback_used,
                "File content decoded with encoding fallback in structured search"
            );
        }

        results = search_lines_with_details(
            content_str
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            virtual_path,
            file_identifier,
            executor,
            plan,
            filters,
            global_offset,
        );

        trace!(
            hash = %sha256_hash,
            virtual_path = %virtual_path,
            matches = results.len(),
            "Searched file via CAS"
        );
    } else {
        // Legacy path-based access
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use std::path::Path;

        let real_path = file_identifier;
        if !filters.matches_file(virtual_path, Some(real_path)) {
            return results;
        }
        let path = Path::new(real_path);

        if !path.exists() {
            warn!(
                file = %real_path,
                "Skipping non-existent file"
            );
            return results;
        }

        match File::open(real_path) {
            Ok(file) => {
                let reader = BufReader::with_capacity(8192, file);
                results = search_lines_with_details(
                    reader
                        .lines()
                        .enumerate()
                        .filter_map(|(index, line_res)| match line_res {
                            Ok(line) => Some((index, Cow::Owned(line))),
                            Err(e) => {
                                warn!(
                                    file = %real_path,
                                    line_index = index + 1,
                                    error = %e,
                                    "Failed to read line during search, skipping line"
                                );
                                None
                            }
                        }),
                    virtual_path,
                    real_path,
                    executor,
                    plan,
                    filters,
                    global_offset,
                );

                // 高频循环中使用 trace 级别
                trace!(
                    path = %real_path,
                    virtual_path = %virtual_path,
                    matches = results.len(),
                    "Searched file via filesystem"
                );
            }
            Err(e) => {
                error!(
                    file = %real_path,
                    error = %e,
                    "Failed to open file for search"
                );
            }
        }
    }

    results
}

// ============================================================================
// 分页搜索功能
// ============================================================================

/// 获取搜索结果的指定分页
///
/// 优先从磁盘结果缓存读取分页；VirtualSearchManager 仅作为兼容降级路径。
///
/// # 参数
/// - `state`: 应用状态，包含 VirtualSearchManager
/// - `search_id`: 搜索会话 ID
/// - `offset`: 起始偏移量
/// - `limit`: 返回条目数限制
///
/// # 返回
/// 指定范围的日志条目列表
#[command]
pub async fn fetch_search_page(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
    offset: usize,
    limit: usize,
) -> Result<crate::search_engine::disk_result_store::SearchPageResult, CommandError> {
    use la_search::SearchPageResult;

    // 限制每页最大数量，防止内存问题
    let limit = limit.min(10000);

    let disk_store_arc = state.disk_result_store.read();
    let disk_store = &*disk_store_arc;

    // 优先从磁盘存储读取（新架构：Notepad++ 式磁盘直写，前端按需分页）
    if disk_store.has_session(&searchId) {
        let result: crate::search_engine::disk_result_store::SearchPageResult = disk_store
            .read_page(&searchId, offset, limit)
            .map_err(|e: std::io::Error| {
                CommandError::from(AppError::io_error(
                    format!("Failed to read search page: {e}"),
                    None,
                ))
                .with_help("The search results may have been cleared. Try running the search again")
            })?;

        debug!(
            search_id = %searchId,
            offset = offset,
            limit = limit,
            returned = result.entries.len(),
            total = result.total_count,
            is_complete = result.is_complete,
            "从磁盘读取搜索分页"
        );

        return Ok(result);
    }

    // 降级：从 VirtualSearchManager 内存缓存读取（向后兼容旧架构）
    let manager = &state.virtual_search_manager;
    if manager.has_session(&searchId) {
        let results = manager.get_range(&searchId, offset, limit);
        let total = manager.get_total_count(&searchId);
        let next_offset = if offset + results.len() < total {
            Some(offset + results.len())
        } else {
            None
        };
        debug!(
            search_id = %searchId,
            offset = offset,
            returned = results.len(),
            "从 VirtualSearchManager 降级读取搜索分页"
        );
        return Ok(SearchPageResult {
            entries: results,
            total_count: total,
            is_complete: true,
            has_more: next_offset.is_some(),
            next_offset,
        });
    }

    Err(CommandError::new(
        "NOT_FOUND",
        format!("Search session '{}' not found or expired", searchId),
    )
    .with_help("The search results may have been cleared. Try running the search again"))
}

/// 注册搜索会话到 VirtualSearchManager
///
/// 用于将搜索结果缓存到 VirtualSearchManager，供后续分页查询使用。
/// 通常在完成搜索后调用，将完整结果存入管理器。
///
/// # 参数
/// - `state`: 应用状态
/// - `search_id`: 搜索会话 ID
/// - `query`: 搜索查询字符串
/// - `entries`: 搜索结果条目列表
///
/// # 返回
/// 注册的 search_id
#[command]
pub async fn register_search_session(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
    query: String,
    entries: Vec<LogEntry>,
) -> Result<String, CommandError> {
    let manager = &state.virtual_search_manager;

    let registered_id = manager.register_session(searchId, query, entries);

    info!(
        search_id = %registered_id,
        "Search session registered in VirtualSearchManager"
    );

    Ok(registered_id)
}

/// 获取搜索会话信息
///
/// # 参数
/// - `state`: 应用状态
/// - `search_id`: 搜索会话 ID
///
/// # 返回
/// 会话信息，包括总条目数、创建时间等
#[command]
pub async fn get_search_session_info(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
) -> Result<Option<serde_json::Value>, CommandError> {
    let manager = &state.virtual_search_manager;

    if let Some(session) = manager.get_session_info(&searchId) {
        Ok(Some(serde_json::json!({
            "searchId": session.search_id,
            "query": session.query,
            "total_count": session.total_count,
            "created_at": session.created_at.elapsed().as_secs(),
        })))
    } else {
        Ok(None)
    }
}

/// 获取搜索会话总条目数
///
/// # 参数
/// - `state`: 应用状态
/// - `search_id`: 搜索会话 ID
///
/// # 返回
/// 总条目数，如果会话不存在返回 0
#[command]
pub async fn get_search_total_count(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
) -> Result<usize, CommandError> {
    // 新架构：优先从 DiskResultStore 读取
    if let Some(status) = state.disk_result_store.read().get_status(&searchId) {
        return Ok(status.0);
    }
    // 降级：从 VirtualSearchManager 读取
    Ok(state.virtual_search_manager.get_total_count(&searchId))
}

/// 移除搜索会话
///
/// 清理不再需要的搜索会话，释放内存。
///
/// # 参数
/// - `state`: 应用状态
/// - `search_id`: 搜索会话 ID
///
/// # 返回
/// 是否成功移除
#[command]
pub async fn remove_search_session(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
) -> Result<bool, CommandError> {
    let manager = &state.virtual_search_manager;
    Ok(manager.remove_session(&searchId))
}

/// 清理过期的搜索会话
///
/// # 参数
/// - `state`: 应用状态
/// - `max_age_secs`: 最大存活时间（秒），默认 3600
///
/// # 返回
/// 清理的会话数量
#[command]
pub async fn cleanup_expired_search_sessions(
    state: State<'_, AppState>,
    _max_age_secs: Option<u64>,
) -> Result<usize, CommandError> {
    let manager = &state.virtual_search_manager;

    // 注意：VirtualSearchManager 内部有 TTL 机制
    // 这里调用 cleanup_expired_sessions 清理过期会话
    // _max_age_secs 保留用于 API 兼容性，实际使用 VirtualSearchManager 内部配置的 TTL
    let cleaned = manager.cleanup_expired_sessions();

    info!(cleaned = cleaned, "Expired search sessions cleaned up");

    Ok(cleaned)
}

/// 获取 VirtualSearchManager 统计信息
///
/// # 返回
/// 活跃会话数、总缓存条目数等统计信息
#[command]
pub async fn get_virtual_search_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, CommandError> {
    let manager = &state.virtual_search_manager;
    let stats = manager.get_statistics();

    Ok(serde_json::json!({
        "active_sessions": stats.active_sessions,
        "total_cached_entries": stats.total_cached_entries,
        "max_sessions": stats.max_sessions,
        "max_entries_per_session": stats.max_entries_per_session,
        "session_ttl_seconds": stats.session_ttl_seconds,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};

    fn build_filters(
        time_start: Option<&str>,
        time_end: Option<&str>,
        levels: &[&str],
        file_pattern: Option<&str>,
    ) -> SearchFilters {
        SearchFilters {
            time_start: time_start.map(str::to_string),
            time_end: time_end.map(str::to_string),
            levels: levels.iter().map(|level| (*level).to_string()).collect(),
            file_pattern: file_pattern.map(str::to_string),
        }
    }

    fn build_executor_and_plan(query: &str) -> (QueryExecutor, ExecutionPlan) {
        let mut executor = QueryExecutor::new(64);
        let terms = split_query_by_pipe(query)
            .into_iter()
            .enumerate()
            .map(|(index, value)| SearchTerm {
                id: format!("term_{}", index),
                value: value.trim().to_string(),
                operator: QueryOperator::Or,
                source: TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            })
            .collect();
        let query = SearchQuery {
            id: "test_query".to_string(),
            terms,
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };
        let plan = executor.execute(&query).unwrap();
        (executor, plan)
    }

    fn build_not_executor_and_plan(query: &str) -> (QueryExecutor, ExecutionPlan) {
        let mut executor = QueryExecutor::new(64);
        let query = SearchQuery {
            id: "test_not_query".to_string(),
            terms: vec![SearchTerm {
                id: "term_not".to_string(),
                value: query.to_string(),
                operator: QueryOperator::Not,
                source: TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: QueryOperator::Not,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };
        let plan = executor.execute(&query).unwrap();
        (executor, plan)
    }

    #[test]
    fn resolve_search_query_uses_structured_query_when_provided() {
        let structured_query = SearchQuery {
            id: "saved-query".to_string(),
            terms: vec![SearchTerm {
                id: "term-1".to_string(),
                value: "error.*timeout".to_string(),
                operator: QueryOperator::Or,
                source: TermSource::Preset,
                preset_group_id: Some("preset-1".to_string()),
                is_regex: true,
                priority: 10,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let (raw_terms, resolved) = resolve_search_query(
            "error.*timeout",
            Some(structured_query),
            false,
            "search_logs_query",
        )
        .unwrap();

        assert_eq!(raw_terms, vec!["error.*timeout".to_string()]);
        assert_eq!(resolved.id, "search_logs_query");
        assert!(resolved.terms[0].is_regex);
        assert_eq!(resolved.terms[0].source, TermSource::Preset);
        assert_eq!(
            resolved.terms[0].preset_group_id.as_deref(),
            Some("preset-1")
        );
    }

    #[test]
    fn resolve_search_query_falls_back_to_literal_query_parsing() {
        let (raw_terms, resolved) =
            resolve_search_query("error|timeout", None, false, "search_logs_query").unwrap();

        assert_eq!(raw_terms, vec!["error".to_string(), "timeout".to_string()]);
        assert_eq!(resolved.global_operator, QueryOperator::Or);
        assert!(!resolved.terms[0].is_regex);
        assert_eq!(resolved.terms[0].operator, QueryOperator::Or);
    }

    #[test]
    fn compiled_filters_support_datetime_local_range() {
        let filters = build_filters(
            Some("2024-01-15T10:00"),
            Some("2024-01-15T11:00"),
            &["ERROR"],
            None,
        );

        let compiled = CompiledSearchFilters::compile(&filters).unwrap();

        assert!(compiled.matches_line_metadata("2024-01-15 10:30:45", "ERROR"));
        assert!(!compiled.matches_line_metadata("2024-01-15 09:59:59", "ERROR"));
        assert!(!compiled.matches_line_metadata("", "ERROR"));
        assert!(!compiled.matches_line_metadata("2024-01-15 10:30:45", "INFO"));
    }

    #[test]
    fn compiled_filters_reject_invalid_time_range() {
        let filters = build_filters(
            Some("2024-01-15T12:00"),
            Some("2024-01-15T11:00"),
            &[],
            None,
        );

        let error = CompiledSearchFilters::compile(&filters).unwrap_err();
        assert!(error.to_string().contains("start time"));
    }

    #[test]
    fn file_pattern_supports_wildcards() {
        let filters = build_filters(None, None, &[], Some("logs/*.log"));
        let compiled = CompiledSearchFilters::compile(&filters).unwrap();

        assert!(compiled.matches_file("logs/app.log", None));
        assert!(!compiled.matches_file("logs/app.txt", None));
    }

    #[test]
    fn file_pattern_without_wildcard_uses_substring_match() {
        let filters = build_filters(None, None, &[], Some("service-error"));
        let compiled = CompiledSearchFilters::compile(&filters).unwrap();

        assert!(compiled.matches_file("prod/service-error.log", None));
        assert!(!compiled.matches_file("prod/service-info.log", None));
    }

    #[test]
    fn segment_pruning_keeps_in_range_matches_only() {
        let filters = build_filters(
            Some("2024-01-15T10:00"),
            Some("2024-01-15T11:00"),
            &["ERROR"],
            None,
        );
        let compiled = CompiledSearchFilters::compile(&filters).unwrap();
        let (executor, plan) = build_executor_and_plan("panic");
        let content = "2024-01-15 09:30:00 ERROR panic before window\n\
2024-01-15 10:15:00 INFO panic wrong level\n\
2024-01-15 10:30:00 ERROR panic in window\n";

        let results = search_lines_with_details(
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            "logs/app.log",
            "cas://hash",
            &executor,
            &plan,
            &compiled,
            0,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(&*results[0].timestamp, "2024-01-15 10:30:00");
        assert_eq!(&*results[0].level, "ERROR");
    }

    #[test]
    fn segment_pruning_skips_segments_without_matching_levels() {
        let filters = build_filters(None, None, &["ERROR"], None);
        let compiled = CompiledSearchFilters::compile(&filters).unwrap();
        let (executor, plan) = build_executor_and_plan("keyword");
        let content = "2024-01-15 10:00:00 INFO keyword info only\n\
2024-01-15 10:01:00 INFO keyword still info\n";

        let results = search_lines_with_details(
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            "logs/app.log",
            "cas://hash",
            &executor,
            &plan,
            &compiled,
            0,
        );

        assert!(results.is_empty());
    }

    #[test]
    fn search_lines_with_details_keeps_not_matches_without_highlights() {
        let compiled = CompiledSearchFilters::compile(&build_filters(None, None, &[], None))
            .expect("Filters should compile");
        let (executor, plan) = build_not_executor_and_plan("debug");
        let content = "2024-01-15 10:00:00 INFO service healthy\n\
2024-01-15 10:01:00 DEBUG should be excluded\n";

        let results = search_lines_with_details(
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            "logs/app.log",
            "cas://hash",
            &executor,
            &plan,
            &compiled,
            0,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(
            &*results[0].content,
            "2024-01-15 10:00:00 INFO service healthy"
        );
        assert!(results[0]
            .match_details
            .as_ref()
            .is_some_and(|details| details.is_empty()));
        assert!(results[0].matched_keywords.is_none());
    }

    #[test]
    fn split_query_by_pipe_basic_separation() {
        assert_eq!(
            split_query_by_pipe("error | timeout"),
            vec!["error", "timeout"]
        );
        assert_eq!(split_query_by_pipe("a|b|c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn split_query_by_pipe_protects_brackets() {
        assert_eq!(
            split_query_by_pipe("(error|timeout)"),
            vec!["(error|timeout)"]
        );
        assert_eq!(
            split_query_by_pipe("a | (b|c) | d"),
            vec!["a", "(b|c)", "d"]
        );
        assert_eq!(split_query_by_pipe("[foo|bar]"), vec!["[foo|bar]"]);
    }

    #[test]
    fn split_query_by_pipe_escapes_literal_pipe() {
        assert_eq!(split_query_by_pipe("foo\\|bar"), vec!["foo|bar"]);
        assert_eq!(split_query_by_pipe("a | b\\|c | d"), vec!["a", "b|c", "d"]);
    }

    #[test]
    fn split_query_by_pipe_handles_empty_and_whitespace() {
        assert!(split_query_by_pipe("").is_empty());
        assert!(split_query_by_pipe("   ").is_empty());
        assert_eq!(
            split_query_by_pipe("  error  |  timeout  "),
            vec!["error", "timeout"]
        );
    }
}
