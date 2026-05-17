//! 搜索命令实现
//! 包含日志搜索、磁盘分页结果存储、关键词统计与结果批量推送
//!
//! # 架构说明
//!
//! 搜索命令已按单一职责原则拆分为三个协作单元：
//! - `SearchOrchestrator`: 参数验证、配置加载、流程编排（本模块命令层）
//! - `SearchResultStore`: 结果批量写入与分页读取（disk_result_store）
//! - `SearchExecutionEngine`: 并行文件搜索、批次处理、统计计算（execute_file_search）
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。
//!
//! # 搜索结果存储弃用路线图 (TODO-P1-5)
//!
//! 当前主搜索结果统一写入 `disk_result_store: DiskResultStore`，由前端分页读取。
//!
//! ## 问题
//! - 稳定查询指纹与前端搜索会话 ID 的职责边界仍需统一
//!
//! ## 目标架构（单一层级存储）
//! ```
//! 搜索执行 → 批量结果缓冲
//!     ↓
//! DiskResultStore (唯一持久缓存，按搜索会话分页)
//!     ↓
//! 前端分页读取 (get_search_result_page)
//! ```
//!
//! ## 迁移步骤
//! 1. [x] 移除主搜索入口对旧搜索结果缓存的写入
//! 2. [ ] `execute_file_search` 改为流式写入 `DiskResultStore`，不再返回 Vec
//! 3. [x] 移除主搜索入口的 `try_get_cached_results` 旁路，统一由 `DiskResultStore` 承载当前分页会话
//! 4. [x] 删除主搜索入口的 `all_results` 本地缓冲和 `MAX_CACHE_SIZE` 限制
//! 5. [ ] 统一缓存键：明确稳定查询指纹与前端搜索会话 ID 的职责边界
//!
//! ## 兼容性
//! - `get_search_result_page` 命令接口保持不变
//! - `search_logs` 的完成事件仍返回当前搜索会话累计的 `results_count`

use parking_lot::Mutex;
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{debug, error, trace, warn};

use crate::commands::import::ensure_workspace_runtime_state;
use crate::models::state::SearchMetrics;
use crate::models::AppState;
use la_core::error::{AppError, CommandError};
use la_core::models::config::AppConfigLoader;
use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use la_core::models::search_statistics::SearchResultSummary;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use la_storage::ContentAddressableStorage;
use rayon::prelude::*;
use regex::Regex;

// MessagePack 序列化支持
use serde::{Deserialize, Serialize};

// ============================================================================
// 搜索流程辅助函数
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct SearchIdEvent {
    search_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct SearchProgressEvent {
    search_id: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SearchSummaryEvent {
    search_id: String,
    summary: SearchResultSummary,
}

#[derive(Debug, Clone, Serialize)]
struct SearchCompleteEvent {
    search_id: String,
    total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SearchErrorEvent {
    search_id: String,
    error: String,
}

fn emit_search_id_event(app_handle: &AppHandle, event_name: &str, search_id: &str) {
    let _ = app_handle.emit(
        event_name,
        SearchIdEvent {
            search_id: search_id.to_string(),
        },
    );
}

fn emit_search_error(app_handle: &AppHandle, search_id: &str, error: impl Into<String>) {
    let _ = app_handle.emit(
        "search-error",
        SearchErrorEvent {
            search_id: search_id.to_string(),
            error: error.into(),
        },
    );
}

/// 准备搜索：解析工作区目录、获取运行时状态、获取文件列表、创建磁盘会话
async fn prepare_search_environment(
    app_handle: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    workspace_dirs: &Arc<Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>>,
    compiled_filters: &CompiledSearchFilters,
    disk_result_store: &Arc<crate::search_engine::disk_result_store::DiskResultStore>,
    search_id: &str,
) -> Result<
    (
        Vec<la_storage::FileMetadata>,
        Arc<ContentAddressableStorage>,
    ),
    CommandError,
> {
    // 解析工作区目录
    let workspace_dir = {
        let existing = {
            let dirs = workspace_dirs.lock();
            dirs.get(workspace_id).cloned()
        };

        if let Some(dir) = existing {
            dir
        } else {
            resolve_workspace_dir(app_handle, workspace_id).map_err(|e| {
                emit_search_error(app_handle, search_id, &e);
                CommandError::new("NOT_FOUND", e).with_help(
                    "The workspace may have been deleted. Try refreshing the workspace list",
                )
            })?
        }
    };

    // 获取工作区运行时状态
    let (cas, metadata_store, _) =
        ensure_workspace_runtime_state(app_handle, state, workspace_id, &workspace_dir)
            .await
            .map_err(|e| {
                CommandError::new(
                    "DATABASE_ERROR",
                    format!("Failed to initialize workspace: {}", e),
                )
                .with_help("Try reloading the workspace before searching again")
            })?;

    // 获取文件列表
    let files = metadata_store.get_all_files().await.map_err(|e| {
        error!(workspace_id = %workspace_id, error = %e, "Failed to get files from metadata store");
        CommandError::new("DATABASE_ERROR", "Failed to access workspace files")
            .with_help("The workspace database may be corrupted. Try refreshing or reimporting")
    })?;

    let files_for_search: Vec<_> = files
        .into_iter()
        .filter(|file| compiled_filters.matches_file(&file.virtual_path, None))
        .collect();

    // 创建磁盘搜索会话
    if !disk_result_store.has_session(search_id) {
        if let Err(e) = disk_result_store.create_session(search_id) {
            emit_search_error(
                app_handle,
                search_id,
                format!("Failed to create search session: {e}"),
            );
            warn!(error = %e, search_id = %search_id, "无法创建磁盘搜索会话，分页读取功能将不可用");
            return Err(CommandError::new(
                "IO_ERROR",
                format!("Failed to create search session: {e}"),
            )
            .with_help("Check that the application data directory is writable"));
        }
    }

    Ok((files_for_search, cas))
}

struct SearchExecutionRequest {
    app_handle: AppHandle,
    search_id: String,
    files_for_search: Vec<la_storage::FileMetadata>,
    cas: Arc<ContentAddressableStorage>,
    structured_query: SearchQuery,
    compiled_filters: CompiledSearchFilters,
    max_results: usize,
    regex_cache_size: usize,
    raw_terms: Vec<String>,
    search_metrics: Arc<Mutex<SearchMetrics>>,
    disk_result_store: Arc<crate::search_engine::disk_result_store::DiskResultStore>,
    cancellation_token: tokio_util::sync::CancellationToken,
    cancellation_tokens:
        Arc<Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>>,
    search_timeout_secs: u64,
    // FIX(HI-01): 复用 AppState 中缓存的 ThreadPool
    search_thread_pool: Arc<rayon::ThreadPool>,
}

/// 执行搜索并处理超时
async fn run_search_with_timeout(request: SearchExecutionRequest) -> Result<String, CommandError> {
    let SearchExecutionRequest {
        app_handle,
        search_id,
        files_for_search,
        cas,
        structured_query,
        compiled_filters,
        max_results,
        regex_cache_size,
        raw_terms,
        search_metrics,
        disk_result_store,
        cancellation_token,
        cancellation_tokens,
        search_timeout_secs,
        search_thread_pool,
    } = request;

    let app_handle_for_timeout = app_handle.clone();
    let cancellation_token_for_timeout = cancellation_token.clone();
    let cancellation_tokens_for_timeout = Arc::clone(&cancellation_tokens);
    let disk_store_for_timeout = Arc::clone(&disk_result_store);
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_timeout = Arc::clone(&timed_out);

    let search_id_for_blocking = search_id.clone();
    let handle = tokio::task::spawn_blocking(move || {
        execute_file_search(
            app_handle,
            search_id_for_blocking,
            files_for_search,
            cas,
            structured_query,
            compiled_filters,
            max_results,
            regex_cache_size,
            raw_terms,
            search_metrics,
            disk_result_store,
            cancellation_token,
            cancellation_tokens,
            search_thread_pool,
        );
    });

    match tokio::time::timeout(std::time::Duration::from_secs(search_timeout_secs), handle).await {
        Ok(Ok(())) => Ok(search_id),
        Ok(Err(e)) => {
            error!(error = %e, search_id = %search_id, "Search task panicked");
            // FIX(HI-09): 搜索线程 panic 时也需要清理 CancellationToken
            {
                let mut tokens = cancellation_tokens_for_timeout.lock();
                tokens.remove(&search_id);
            }
            disk_store_for_timeout.remove_session(&search_id);
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
            disk_store_for_timeout.remove_session(&search_id);
            emit_search_id_event(&app_handle_for_timeout, "search-timeout", &search_id);
            Err(CommandError::new(
                "TIMEOUT_ERROR",
                format!("Search timed out after {} seconds", search_timeout_secs),
            )
            .with_help("Try using more specific search terms to reduce processing time"))
        }
    }
}

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

use crate::services::file_watcher::TimestampParser;
use crate::services::{looks_like_regex_pattern, parse_metadata, ExecutionPlan, QueryPlanBuilder};
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
                regex_cache_size: config.search.regex_cache_size.max(1),
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
            is_regex: looks_like_regex_pattern(value),
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

/// 清理搜索取消令牌
fn remove_cancellation_token(
    cancellation_tokens: &Arc<
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
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
    level: &'static str,
    level_normalized: &'static str,
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

impl ParsedLineMetadata {
    fn parse(line: &str, needs_datetime: bool) -> Self {
        let (timestamp, level) = parse_metadata(line);
        // level 已是已知的静态小写字符串，level_normalized 与之相同（零分配）
        let datetime = if needs_datetime {
            TimestampParser::parse_naive_datetime(&timestamp)
        } else {
            None
        };

        Self {
            timestamp,
            level,
            level_normalized: level,
            datetime,
            level_mask: crate::commands::level_to_mask(level),
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
                .fold(0u8, |mask, level| mask | crate::commands::level_to_mask(level))
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
        // 将动态 level 参数映射为静态字符串（与 parse_metadata 一致）
        let static_level: &'static str = match level.trim().to_ascii_lowercase().as_str() {
            "error" => "error",
            "warn" | "warning" => "warn",
            "info" => "info",
            _ => "debug",
        };
        let metadata = ParsedLineMetadata {
            timestamp: timestamp.to_string(),
            level: static_level,
            level_normalized: static_level,
            datetime: if self.has_time_filter() {
                TimestampParser::parse_naive_datetime(timestamp)
            } else {
                None
            },
            level_mask: crate::commands::level_to_mask(level),
        };

        self.matches_parsed_line_metadata(&metadata)
    }

    fn matches_parsed_line_metadata(&self, metadata: &ParsedLineMetadata) -> bool {
        if let Some(levels) = &self.levels {
            if !levels.contains(metadata.level_normalized) {
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
/// 验证搜索参数
fn validate_search_params(query: &str) -> Result<(), CommandError> {
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

/// 解析工作区ID，未提供时使用第一个可用工作区
fn resolve_workspace_id(
    workspace_id_arg: Option<String>,
    workspace_dirs: &Arc<Mutex<std::collections::BTreeMap<String, std::path::PathBuf>>>,
) -> Result<String, CommandError> {
    if let Some(id) = workspace_id_arg {
        return Ok(id);
    }

    let dirs = workspace_dirs.lock();
    if let Some(first_id) = dirs.keys().next() {
        debug!(
            workspace_id = %first_id,
            available_workspaces = ?dirs.keys().collect::<Vec<_>>(),
            "Using first available workspace as default"
        );
        Ok(first_id.clone())
    } else {
        Err(CommandError::new("NOT_FOUND", "No workspaces available")
            .with_help("Please create a workspace first using the import feature"))
    }
}

/// 在 spawn_blocking 中执行实际的文件搜索
#[allow(clippy::too_many_arguments)]
fn execute_file_search(
    app_handle: AppHandle,
    search_id: String,
    files_for_search: Vec<la_storage::FileMetadata>,
    cas: Arc<ContentAddressableStorage>,
    structured_query: SearchQuery,
    compiled_filters: CompiledSearchFilters,
    max_results: usize,
    regex_cache_size: usize,
    raw_terms: Vec<String>,
    search_metrics: Arc<Mutex<SearchMetrics>>,
    disk_store: Arc<crate::search_engine::disk_result_store::DiskResultStore>,
    cancellation_token: tokio_util::sync::CancellationToken,
    cancellation_token_map: Arc<
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
    // FIX(HI-01): 使用 AppState 中缓存的 ThreadPool，避免每次搜索新建
    search_thread_pool: Arc<rayon::ThreadPool>,
) {
    let start_time = std::time::Instant::now();

    let mut builder = QueryPlanBuilder::new(regex_cache_size);
    let plan = match builder.build(&structured_query) {
        Ok(p) => p,
        Err(e) => {
            emit_search_error(
                &app_handle,
                &search_id,
                format!("Query execution error: {}", e),
            );
            disk_store.remove_session(&search_id);
            // FIX(HI-09): 清理 CancellationToken，防止 HashMap 只增不减
            remove_cancellation_token(&cancellation_token_map, &search_id);
            return;
        }
    };

    debug!(
        total_files = files_for_search.len(),
        "Starting search across files using CAS"
    );

    let batch_size = 2000;
    let mut total_processed = 0;
    let mut results_count = 0;
    let mut keyword_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut was_truncated = false;

    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_search = Arc::clone(&timed_out);

    // FIX(HI-02): flush_batch 现在返回 bool，失败时 emit search-error 而不是静默丢弃
    let flush_batch = |batch: &mut Vec<LogEntry>, progress_count: usize| -> bool {
        if batch.is_empty() {
            return true;
        }

        if cancellation_token.is_cancelled() {
            batch.clear();
            return true;
        }

        const MAX_RETRIES: usize = 3;
        let mut last_err = None;
        for _ in 0..MAX_RETRIES {
            match disk_store.append_entries(&search_id, batch) {
                Ok(_) => {
                    batch.clear();
                    if !timed_out_for_search.load(Ordering::SeqCst) {
                        let _ = app_handle.emit(
                            "search-progress",
                            SearchProgressEvent {
                                search_id: search_id.clone(),
                                count: progress_count,
                            },
                        );
                    }
                    return true;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }
        if let Some(e) = last_err {
            error!(error = %e, retries = MAX_RETRIES, "磁盘写入搜索结果批次失败");
            emit_search_error(
                &app_handle,
                &search_id,
                format!("Disk write failed after {} retries: {}", MAX_RETRIES, e),
            );
        }
        batch.clear();
        false
    };

    emit_search_id_event(&app_handle, "search-start", &search_id);

    let mut disk_write_failed = false;

    'outer: for file_batch in files_for_search.chunks(10) {
        if cancellation_token.is_cancelled() {
            if !timed_out_for_search.load(Ordering::SeqCst) {
                emit_search_id_event(&app_handle, "search-cancelled", &search_id);
            }
            {
                let mut tokens = cancellation_token_map.lock();
                tokens.remove(&search_id);
            }
            disk_store.remove_session(&search_id);
            return;
        }

        if results_count >= max_results {
            was_truncated = true;
            break 'outer;
        }

        let mut batch_results: Vec<LogEntry> = Vec::new();

        let batch: Vec<_> = search_thread_pool.install(|| {
            file_batch
                .par_iter()
                .enumerate()
                .map(|(idx, file_metadata)| {
                    if cancellation_token.is_cancelled() {
                        return Vec::new();
                    }

                    let file_identifier = format!("cas://{}", file_metadata.sha256_hash);
                    search_single_file_with_details(
                        &file_identifier,
                        &file_metadata.virtual_path,
                        Some(&*cas),
                        &builder,
                        &plan,
                        &compiled_filters,
                        total_processed + idx * 10000,
                    )
                })
                .collect()
        });

        if cancellation_token.is_cancelled() {
            continue 'outer;
        }

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

                if results_count >= max_results {
                    let _ = flush_batch(&mut batch_results, results_count);
                    was_truncated = true;
                    break 'outer;
                }

                entry.id = results_count;

                if let Some(ref keywords) = entry.matched_keywords {
                    for kw in keywords {
                        *keyword_counts.entry(kw.clone()).or_insert(0) += 1;
                    }
                }

                batch_results.push(entry);
                results_count += 1;

                if batch_results.len() >= batch_size {
                    if !flush_batch(&mut batch_results, results_count) {
                        disk_write_failed = true;
                        break 'outer;
                    }
                }
            }
        }

        if disk_write_failed {
            break 'outer;
        }
        if !flush_batch(&mut batch_results, results_count) {
            disk_write_failed = true;
            break 'outer;
        }
        total_processed += file_batch.len();
    }

    if disk_write_failed {
        emit_search_error(
            &app_handle,
            &search_id,
            "Disk write failed: search results may be incomplete",
        );
        disk_store.remove_session(&search_id);
        remove_cancellation_token(&cancellation_token_map, &search_id);
        return;
    }

    if cancellation_token.is_cancelled() {
        {
            let mut tokens = cancellation_token_map.lock();
            tokens.remove(&search_id);
        }
        disk_store.remove_session(&search_id);
        return;
    }

    if let Err(e) = disk_store.complete_session(&search_id) {
        warn!(error = %e, "完成磁盘搜索会话失败");
    }

    let duration = start_time.elapsed().as_millis() as u64;
    {
        let mut metrics = search_metrics.lock();
        metrics.last_search_duration = std::time::Duration::from_millis(duration);
    }

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

    if !timed_out_for_search.load(Ordering::SeqCst) {
        let _ = app_handle.emit(
            "search-summary",
            SearchSummaryEvent {
                search_id: search_id.clone(),
                summary: SearchResultSummary::new(
                    results_count,
                    keyword_stats,
                    duration,
                    was_truncated,
                ),
            },
        );
        let _ = app_handle.emit(
            "search-complete",
            SearchCompleteEvent {
                search_id: search_id.clone(),
                total_count: results_count,
            },
        );
    }

    remove_cancellation_token(&cancellation_token_map, &search_id);
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
    // Step 1: 参数验证
    validate_search_params(&query)?;

    let runtime_config = load_search_runtime_config(&app);
    let app_handle = app.clone();
    let workspace_dirs = Arc::clone(&state.workspace_dirs);
    let search_metrics = Arc::clone(&state.search_metrics);
    let cancellation_tokens = Arc::clone(&state.search_cancellation_tokens);
    // FIX(CR-06): disk_result_store is Option<Arc<DiskResultStore>> to avoid panic in default()
    let disk_result_store = state.disk_result_store.read().clone().ok_or_else(|| {
        CommandError::new("NOT_INITIALIZED", "Disk result store not initialized")
            .with_help("The application may still be initializing. Please try again")
    })?;
    let search_thread_pool = Arc::clone(&state.search_thread_pool);

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

    // Step 2: 解析工作区ID
    let workspace_id = resolve_workspace_id(workspaceId, &workspace_dirs)?;

    // Step 3: 生成搜索ID和取消令牌
    let search_id = uuid::Uuid::new_v4().to_string();
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = cancellation_tokens.lock();
        if let Some(old_token) = tokens.get(&search_id) {
            tracing::warn!(search_id = %search_id, "Cancelling old search token");
            old_token.cancel();
        }
        tokens.insert(search_id.clone(), cancellation_token.clone());
    }

    {
        let mut metrics = search_metrics.lock();
        metrics.total_searches += 1;
    }

    // Step 5-8: 获取文件列表并创建磁盘会话
    let (files_for_search, cas) = prepare_search_environment(
        &app_handle,
        &state,
        &workspace_id,
        &workspace_dirs,
        &compiled_filters,
        &disk_result_store,
        &search_id,
    )
    .await?;

    let background_search_id = search_id.clone();
    tokio::spawn(async move {
        let result = run_search_with_timeout(SearchExecutionRequest {
            app_handle: app_handle.clone(),
            search_id: background_search_id.clone(),
            files_for_search,
            cas,
            structured_query,
            compiled_filters,
            max_results,
            regex_cache_size,
            raw_terms,
            search_metrics,
            disk_result_store,
            cancellation_token,
            cancellation_tokens,
            search_timeout_secs,
            // FIX(HI-01): 从 AppState 复用已初始化的 ThreadPool
            search_thread_pool,
        })
        .await;

        if let Err(error) = result {
            if error.code != "TIMEOUT_ERROR" {
                emit_search_error(&app_handle, &background_search_id, error.message);
            }
        }
    });

    Ok(search_id)
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
        // FIX(HI-09): 显式移除已取消的 token，防止 search_cancellation_tokens 只增不减
        let mut tokens = cancellation_tokens.lock();
        tokens.remove(&searchId);
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
    builder: &QueryPlanBuilder,
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
            builder,
            plan,
            filters,
            global_offset,
        )
    } else {
        search_lines_direct(lines, virtual_path, real_path, builder, plan, global_offset)
    }
}

fn search_lines_direct<'a, I>(
    lines: I,
    virtual_path: &str,
    real_path: &str,
    builder: &QueryPlanBuilder,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Vec<LogEntry>
where
    I: IntoIterator<Item = (usize, Cow<'a, str>)>,
{
    let mut results = Vec::new();

    for (index, line) in lines {
        let line_ref = line.as_ref();
        let Some(match_details) = builder.match_with_details(plan, line_ref) else {
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
    builder: &QueryPlanBuilder,
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
                builder,
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
            builder,
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
    builder: &QueryPlanBuilder,
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

            let Some(match_details) = builder.match_with_details(plan, candidate.line.as_ref())
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
    builder: &QueryPlanBuilder,
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
                                builder,
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
                                builder,
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
            builder,
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
                    builder,
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
/// 从 DiskResultStore 磁盘结果缓存读取分页。
///
/// # 参数
/// - `state`: 应用状态，包含 DiskResultStore
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
    // 限制每页最大数量，防止内存问题
    let limit = limit.min(10000);

    let disk_store_opt = state.disk_result_store.read();
    let disk_store = disk_store_opt.as_ref().ok_or_else(|| {
        CommandError::new("NOT_INITIALIZED", "Disk result store not initialized")
            .with_help("The application may still be initializing. Please try again")
    })?;

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

    Err(CommandError::new(
        "NOT_FOUND",
        format!("Search session '{}' not found or expired", searchId),
    )
    .with_help("The search results may have been cleared. Try running the search again"))
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

    fn build_builder_and_plan(query: &str) -> (QueryPlanBuilder, ExecutionPlan) {
        let mut builder = QueryPlanBuilder::new(64);
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
        let plan = builder.build(&query).unwrap();
        (builder, plan)
    }

    fn build_not_builder_and_plan(query: &str) -> (QueryPlanBuilder, ExecutionPlan) {
        let mut builder = QueryPlanBuilder::new(64);
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
        let plan = builder.build(&query).unwrap();
        (builder, plan)
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
        let (builder, plan) = build_builder_and_plan("panic");
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
            &builder,
            &plan,
            &compiled,
            0,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(&*results[0].timestamp, "2024-01-15 10:30:00");
        assert_eq!(&*results[0].level, "error");
    }

    #[test]
    fn segment_pruning_skips_segments_without_matching_levels() {
        let filters = build_filters(None, None, &["ERROR"], None);
        let compiled = CompiledSearchFilters::compile(&filters).unwrap();
        let (builder, plan) = build_builder_and_plan("keyword");
        let content = "2024-01-15 10:00:00 INFO keyword info only\n\
2024-01-15 10:01:00 INFO keyword still info\n";

        let results = search_lines_with_details(
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            "logs/app.log",
            "cas://hash",
            &builder,
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
        let (builder, plan) = build_not_builder_and_plan("debug");
        let content = "2024-01-15 10:00:00 INFO service healthy\n\
2024-01-15 10:01:00 DEBUG should be excluded\n";

        let results = search_lines_with_details(
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, Cow::Borrowed(line))),
            "logs/app.log",
            "cas://hash",
            &builder,
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
