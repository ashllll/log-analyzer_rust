//! 搜索过滤与辅助类型
//!
//! 包含编译后的搜索过滤器、文件模式匹配器、行元数据解析、
//! 日志条目构建以及基于 CAS/文件系统的单文件搜索实现。

use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};
use tauri::{AppHandle, Manager};
use tracing::{debug, error, info, trace, warn};

use la_core::error::CommandError;
use la_core::models::config::AppConfigLoader;
use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use regex::Regex;

use crate::services::file_watcher::TimestampParser;
use crate::services::{parse_metadata, ExecutionPlan, QueryExecutor};
use crate::utils::encoding::decode_log_content;

pub const SEARCH_SEGMENT_LINE_COUNT: usize = 256;

#[derive(Debug, Clone)]
pub struct SearchRuntimeConfig {
    pub default_max_results: usize,
    pub timeout_seconds: u64,
    pub regex_cache_size: usize,
    pub case_sensitive: bool,
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

pub fn load_search_runtime_config(app: &AppHandle) -> SearchRuntimeConfig {
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

pub fn split_query_by_pipe(query: &str) -> Vec<String> {
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

pub fn build_structured_search_query(
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

pub fn resolve_search_query(
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

pub fn log_cache_statistics(total_searches: &AtomicU64, cache_hits: &AtomicU64) {
    let total = total_searches.load(Ordering::Relaxed);
    let hits = cache_hits.load(Ordering::Relaxed);
    let hit_rate = if total > 0 {
        (hits as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    info!(
        total = total,
        hits = hits,
        hit_rate = hit_rate,
        "Cache statistics"
    );
}

pub fn compute_query_version(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
pub struct CompiledSearchFilters {
    pub levels: Option<HashSet<String>>,
    pub level_mask: Option<u8>,
    pub time_start: Option<chrono::NaiveDateTime>,
    pub time_end: Option<chrono::NaiveDateTime>,
    pub file_matcher: Option<FilePatternMatcher>,
    pub file_pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedLineMetadata {
    pub timestamp: String,
    pub level: &'static str,
    pub level_normalized: &'static str,
    pub datetime: Option<chrono::NaiveDateTime>,
    pub level_mask: u8,
}

#[derive(Debug, Clone, Default)]
pub struct SearchSegmentSummary {
    pub min_datetime: Option<chrono::NaiveDateTime>,
    pub max_datetime: Option<chrono::NaiveDateTime>,
    pub level_mask: u8,
}

#[derive(Debug, Clone)]
pub struct SearchLineCandidate<'a> {
    pub index: usize,
    pub line: Cow<'a, str>,
    pub metadata: ParsedLineMetadata,
}

#[derive(Debug, Clone)]
pub enum FilePatternMatcher {
    Substring(String),
    Wildcard(Regex),
}

impl FilePatternMatcher {
    pub fn compile(raw: &str) -> Result<Self, CommandError> {
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

    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Substring(pattern) => value.contains(pattern),
            Self::Wildcard(regex) => regex.is_match(value),
        }
    }
}

pub fn level_to_mask(level: &str) -> u8 {
    match level.trim().to_ascii_lowercase().as_str() {
        "error" => 1 << 0,
        "warn" | "warning" => 1 << 1,
        "info" => 1 << 2,
        "debug" => 1 << 3,
        _ => 0,
    }
}

impl ParsedLineMetadata {
    pub fn parse(line: &str, needs_datetime: bool) -> Self {
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
            level_mask: level_to_mask(level),
        }
    }
}

impl SearchSegmentSummary {
    pub fn record(&mut self, metadata: &ParsedLineMetadata) {
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

pub fn compile_filters(filters: &SearchFilters) -> Result<CompiledSearchFilters, CommandError> {
    CompiledSearchFilters::compile(filters)
}

impl CompiledSearchFilters {
    pub fn compile(filters: &SearchFilters) -> Result<Self, CommandError> {
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
            file_pattern: filters
                .file_pattern
                .as_deref()
                .map(str::trim)
                .filter(|pattern| !pattern.is_empty())
                .map(|s| s.to_string()),
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

    pub fn matches_file(&self, virtual_path: &str, real_path: Option<&str>) -> bool {
        let Some(matcher) = &self.file_matcher else {
            return true;
        };

        matcher.matches(virtual_path) || real_path.is_some_and(|path| matcher.matches(path))
    }

    #[cfg(test)]
    pub fn matches_line_metadata(&self, timestamp: &str, level: &str) -> bool {
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
            level_mask: level_to_mask(level),
        };

        self.matches_parsed_line_metadata(&metadata)
    }

    pub fn matches_parsed_line_metadata(&self, metadata: &ParsedLineMetadata) -> bool {
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

    pub fn has_time_filter(&self) -> bool {
        self.time_start.is_some() || self.time_end.is_some()
    }

    pub fn is_complex(&self) -> bool {
        self.levels.is_some() || self.has_time_filter() || self.file_matcher.is_some()
    }

    pub fn needs_segment_pruning(&self) -> bool {
        self.levels.is_some() || self.has_time_filter()
    }

    pub fn segment_may_match(&self, summary: &SearchSegmentSummary) -> bool {
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

pub fn search_lines_with_details<'a, I>(
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

pub fn search_lines_direct<'a, I>(
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
    let mut results = Vec::with_capacity(64);

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

pub fn search_lines_with_segment_pruning<'a, I>(
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
pub fn flush_search_segment(
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

pub fn build_log_entry(
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

/// 搜索单个文件，支持 CAS 哈希访问和路径访问
pub fn search_single_file_with_details(
    file_identifier: &str,
    virtual_path: &str,
    cas_opt: Option<&crate::storage::ContentAddressableStorage>,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    if let Some(sha256_hash) = file_identifier.strip_prefix("cas://") {
        if !filters.matches_file(virtual_path, None) {
            return results;
        }

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
            match cas.read_content_mmap_sync(sha256_hash) {
                Ok(mmap) => {
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
                }
            }
        }

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
