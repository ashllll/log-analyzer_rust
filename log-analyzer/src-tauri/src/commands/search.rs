//! 搜索命令实现
//! 包含日志搜索及缓存逻辑，附带关键词统计与结果批量推送

use parking_lot::Mutex;
use regex::Regex;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};
use tauri::{command, AppHandle, Emitter, State};
use tracing::{debug, error, info, warn};

use crate::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use crate::models::search_statistics::SearchResultSummary;
use crate::models::{AppState, LogEntry, SearchCacheKey, SearchFilters, SearchQuery};
use crate::search_engine::manager::{SearchConfig, SearchEngineManager};
use crate::services::{calculate_keyword_statistics, parse_metadata, ExecutionPlan, QueryExecutor};

/// 计算并打印缓存统计信息
fn log_cache_statistics(total_searches: &Arc<Mutex<u64>>, cache_hits: &Arc<Mutex<u64>>) {
    let total = total_searches.lock();
    let hits = cache_hits.lock();
    let hit_rate = if *total > 0 {
        (*hits as f64 / *total as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "[CACHE STATS] Total searches: {}, Cache hits: {}, Hit rate: {:.2}%",
        total, hits, hit_rate
    );
}

/// 获取或初始化 SearchEngineManager
///
/// 使用延迟初始化模式，首次调用时创建索引
fn get_or_init_search_engine(state: &AppState, workspace_id: &str) -> Result<(), String> {
    let mut engine_guard = state.search_engine.lock();

    if engine_guard.is_none() {
        // 创建索引目录路径
        let index_path = PathBuf::from(format!("./search_index/{}", workspace_id));

        // 配置搜索引擎
        let config = SearchConfig {
            default_timeout: Duration::from_millis(200),
            max_results: 50_000,
            index_path,
            writer_heap_size: 50_000_000, // 50MB
        };

        // 初始化搜索引擎
        match SearchEngineManager::new(config) {
            Ok(engine) => {
                println!(
                    "[SEARCH ENGINE] Initialized Tantivy search engine for workspace: {}",
                    workspace_id
                );
                *engine_guard = Some(engine);
                Ok(())
            }
            Err(e) => {
                eprintln!("[SEARCH ENGINE] Failed to initialize: {}", e);
                Err(format!("Failed to initialize search engine: {}", e))
            }
        }
    } else {
        Ok(())
    }
}

#[command]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    max_results: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }
    if query.len() > 1000 {
        return Err("Search query too long (max 1000 characters)".to_string());
    }

    let app_handle = app.clone();
    let path_map = Arc::clone(&state.path_map);
    let cache_manager = Arc::clone(&state.cache_manager);
    let total_searches = Arc::clone(&state.total_searches);
    let cache_hits = Arc::clone(&state.cache_hits);
    let last_search_duration = Arc::clone(&state.last_search_duration);
    let cancellation_tokens = Arc::clone(&state.search_cancellation_tokens);
    let metrics_collector = Arc::clone(&state.metrics_collector);

    let max_results = max_results.unwrap_or(50000).min(100_000);
    let filters = filters.unwrap_or_default();
    let workspace_id = workspaceId.unwrap_or_else(|| "default".to_string());

    // 生成唯一的搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();

    // 创建取消令牌
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = cancellation_tokens.lock();
        tokens.insert(search_id.clone(), cancellation_token.clone());
    }

    // 缓存键：基于查询参数生成，不包含时间戳以确保缓存可命中
    // 注意：当索引更新时，应清除相关缓存
    let cache_key: SearchCacheKey = (
        query.clone(),
        workspace_id.clone(),
        filters.time_start.clone(),
        filters.time_end.clone(),
        filters.levels.clone(),
        filters.file_pattern.clone(),
        false, // case_sensitive - 需要从查询中获取
        max_results,
        String::new(), // 移除时间戳版本号，避免缓存永远失效
    );

    {
        // 使用 CacheManager 的同步 get 方法
        let cache_result = state.cache_manager.get_sync(&cache_key);

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

            for chunk in cached_results.chunks(500) {
                let _ = app_handle.emit("search-results", chunk);
                thread::sleep(Duration::from_millis(2));
            }

            let raw_terms: Vec<String> = query
                .split('|')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect();

            let keyword_stats = calculate_keyword_statistics(&cached_results, &raw_terms);
            let summary = SearchResultSummary::new(cached_results.len(), keyword_stats, 0, false);

            let _ = app_handle.emit("search-summary", &summary);
            let _ = app_handle.emit("search-complete", cached_results.len());
            return Ok(search_id);
        }
    }

    {
        let mut searches = total_searches.lock();
        *searches += 1;
    }

    let search_id_clone = search_id.clone();
    thread::spawn(move || {
        let start_time = std::time::Instant::now();
        let parse_start = std::time::Instant::now();

        let raw_terms: Vec<String> = query
            .split('|')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();

        if raw_terms.is_empty() {
            let _ = app_handle.emit("search-error", "Search query is empty after processing");
            return;
        }

        let search_terms: Vec<SearchTerm> = raw_terms
            .iter()
            .enumerate()
            .map(|(i, term)| SearchTerm {
                id: format!("term_{}", i),
                value: term.clone(),
                operator: QueryOperator::Or,
                source: TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            })
            .collect();

        let structured_query = SearchQuery {
            id: "search_logs_query".to_string(),
            terms: search_terms,
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let parse_duration = parse_start.elapsed();

        let execution_start = std::time::Instant::now();
        let mut executor = QueryExecutor::new(100);
        let plan = match executor.execute(&structured_query) {
            Ok(p) => p,
            Err(e) => {
                let _ = app_handle.emit("search-error", format!("Query execution error: {}", e));
                return;
            }
        };

        let files: Vec<(String, String)> = {
            let guard = path_map.lock();
            guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        debug!(
            total_files = files.len(),
            "Starting search across files"
        );

        // 流式处理：分批发送结果，避免内存峰值
        let batch_size = 500;
        let mut total_processed = 0;
        let mut results_count = 0;
        // 流式统计：使用HashMap增量统计关键词，避免累积所有结果
        let mut keyword_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut was_truncated = false;
        let mut pending_batch: Vec<LogEntry> = Vec::new(); // 当前待发送批次
        let mut all_results: Vec<LogEntry> = Vec::new(); // 用于缓存的完整结果集

        // 先发送开始事件
        let _ = app_handle.emit("search-start", "Starting search...");

        'outer: for file_batch in files.chunks(10) {
            // 检查取消状态
            if cancellation_token.is_cancelled() {
                let _ = app_handle.emit("search-cancelled", search_id_clone.clone());
                // 清理取消令牌
                {
                    let mut tokens = cancellation_tokens.lock();
                    tokens.remove(&search_id_clone);
                }
                return;
            }

            // 检查是否已达到max_results限制
            if results_count >= max_results {
                was_truncated = true;
                break 'outer;
            }

            // 每批处理10个文件
            let mut batch_results: Vec<LogEntry> = Vec::new();

            // 并行处理当前批次
            let batch: Vec<_> = file_batch
                .iter()
                .enumerate()
                .map(|(idx, (real_path, virtual_path))| {
                    search_single_file_with_details(
                        real_path,
                        virtual_path,
                        None, // CAS not yet integrated, using legacy path-based access
                        &executor,
                        &plan,
                        total_processed + idx * 10000,
                    )
                })
                .collect();

            // 收集当前批次的结果
            for file_results in batch {
                for entry in file_results {
                    // 检查是否已达到max_results限制
                    if results_count >= max_results {
                        was_truncated = true;
                        break 'outer;
                    }

                    // 应用过滤器
                    let mut include = true;

                    if !filters.levels.is_empty() && !filters.levels.contains(&entry.level) {
                        include = false;
                    }
                    if include && filters.time_start.is_some() {
                        if let Some(ref start) = filters.time_start {
                            if entry.timestamp < *start {
                                include = false;
                            }
                        }
                    }
                    if include && filters.time_end.is_some() {
                        if let Some(ref end) = filters.time_end {
                            if entry.timestamp > *end {
                                include = false;
                            }
                        }
                    }
                    if include && filters.file_pattern.is_some() {
                        if let Some(ref pattern) = filters.file_pattern {
                            if !entry.file.contains(pattern) && !entry.real_path.contains(pattern) {
                                include = false;
                            }
                        }
                    }

                    if include {
                        // 流式统计：增量更新关键词计数
                        if let Some(ref keywords) = entry.matched_keywords {
                            for kw in keywords {
                                *keyword_counts.entry(kw.clone()).or_insert(0) += 1;
                            }
                        }

                        // 保存到完整结果集用于缓存
                        all_results.push(entry.clone());
                        batch_results.push(entry);
                        results_count += 1;

                        // 当批次满时发送
                        if batch_results.len() >= batch_size {
                            let _ = app_handle.emit("search-results", &batch_results);
                            batch_results.clear();
                        }
                    }
                }
            }

            // 保存未发送的结果
            if !batch_results.is_empty() {
                pending_batch = batch_results;
            }

            total_processed += file_batch.len();

            // 发送进度更新
            let progress = (total_processed as f64 / files.len() as f64 * 100.0) as i32;
            let _ = app_handle.emit("search-progress", progress);

            // 避免阻塞：定期暂停
            if total_processed % 50 == 0 {
                thread::sleep(Duration::from_millis(1));
            }
        }

        // 发送剩余结果
        if !pending_batch.is_empty() {
            let _ = app_handle.emit("search-results", &pending_batch);
        }

        // 计算搜索统计信息
        let duration = start_time.elapsed().as_millis() as u64;
        {
            let mut last_duration = last_search_duration.lock();
            *last_duration = duration;
        }

        let execution_duration = execution_start.elapsed();
        let format_duration = start_time.elapsed() - parse_duration - execution_duration;

        // 记录性能指标
        use crate::monitoring::metrics_collector::SearchPhase;
        let phase_timings = vec![
            (SearchPhase::Parsing, parse_duration),
            (SearchPhase::Execution, execution_duration),
            (SearchPhase::Formatting, format_duration),
        ];

        metrics_collector.record_search_operation(
            &query,
            results_count,
            start_time.elapsed(),
            phase_timings,
            !was_truncated && !cancellation_token.is_cancelled(),
        );

        // 使用流式统计结果构建关键词统计
        let keyword_stats: Vec<crate::models::search_statistics::KeywordStatistics> = raw_terms
            .iter()
            .map(|term| {
                let count = keyword_counts.get(term).copied().unwrap_or(0);
                crate::models::search_statistics::KeywordStatistics::new(
                    term.clone(),
                    count,
                    results_count,
                )
            })
            .collect();

        let summary = SearchResultSummary::new(
            results_count,
            keyword_stats,
            duration,
            was_truncated, // 标记是否因达到限制而截断
        );

        // 将结果插入缓存(仅在未截断且未取消时缓存)
        if !was_truncated && !cancellation_token.is_cancelled() {
            cache_manager.insert_sync(cache_key, all_results);
        }

        let _ = app_handle.emit("search-summary", &summary);
        let _ = app_handle.emit("search-complete", results_count);

        // 清理取消令牌
        {
            let mut tokens = cancellation_tokens.lock();
            tokens.remove(&search_id_clone);
        }
    });

    Ok(search_id)
}

/// Legacy search function using regex (kept for backward compatibility)
///
/// This function is marked as dead_code but kept for potential future use.
/// New code should use `search_single_file_with_details` instead.
#[allow(dead_code)]
fn search_single_file(
    sha256_hash: &str,
    virtual_path: &str,
    cas: &crate::storage::ContentAddressableStorage,
    re: &Regex,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    // Read content from CAS using hash
    let content = match tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(cas.read_content(sha256_hash))
    }) {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!(
                hash = %sha256_hash,
                virtual_path = %virtual_path,
                error = %e,
                "Failed to read content from CAS"
            );
            return results;
        }
    };

    // Convert bytes to string
    let content_str = match String::from_utf8(content) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                hash = %sha256_hash,
                virtual_path = %virtual_path,
                error = %e,
                "File content is not valid UTF-8, skipping"
            );
            return results;
        }
    };

    // Process lines
    for (i, line) in content_str.lines().enumerate() {
        if re.is_match(line) {
            let (ts, lvl) = parse_metadata(line);
            results.push(LogEntry {
                id: global_offset + i,
                timestamp: ts,
                level: lvl,
                file: virtual_path.to_string(),
                real_path: format!("cas://{}", sha256_hash),
                line: i + 1,
                content: line.to_string(),
                tags: vec![],
                match_details: None,
                matched_keywords: None,
            });
        }
    }

    results
}

/// 取消正在进行的搜索
#[command]
pub async fn cancel_search(
    #[allow(non_snake_case)] searchId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cancellation_tokens = Arc::clone(&state.search_cancellation_tokens);

    let token = {
        let tokens = cancellation_tokens.lock();
        tokens.get(&searchId).cloned()
    };

    if let Some(token) = token {
        token.cancel();
        Ok(())
    } else {
        Err(format!(
            "Search with ID {} not found or already completed",
            searchId
        ))
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
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    // Determine if this is CAS-based or path-based access
    if file_identifier.starts_with("cas://") {
        // Hash-based access via CAS
        let sha256_hash = &file_identifier[6..]; // Remove "cas://" prefix

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

        // Read content from CAS using hash
        let content = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(cas.read_content(sha256_hash))
        }) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(
                    hash = %sha256_hash,
                    virtual_path = %virtual_path,
                    error = %e,
                    "Failed to read content from CAS, skipping file"
                );
                return results;
            }
        };

        // Convert bytes to string
        let content_str = match String::from_utf8(content) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    hash = %sha256_hash,
                    virtual_path = %virtual_path,
                    error = %e,
                    "File content is not valid UTF-8, skipping"
                );
                return results;
            }
        };

        // Process lines
        for (i, line) in content_str.lines().enumerate() {
            if executor.matches_line(plan, line) {
                let (ts, lvl) = parse_metadata(line);
                let match_details = executor.match_with_details(plan, line);
                let matched_keywords = match_details.as_ref().map(|details| {
                    details
                        .iter()
                        .map(|detail| detail.term_value.clone())
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                });

                results.push(LogEntry {
                    id: global_offset + i,
                    timestamp: ts,
                    level: lvl,
                    file: virtual_path.to_string(),
                    real_path: file_identifier.to_string(),
                    line: i + 1,
                    content: line.to_string(),
                    tags: vec![],
                    match_details,
                    matched_keywords: matched_keywords.filter(|v| !v.is_empty()),
                });
            }
        }

        debug!(
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

                for (i, line_res) in reader.lines().enumerate() {
                    if let Ok(line) = line_res {
                        if executor.matches_line(plan, &line) {
                            let (ts, lvl) = parse_metadata(&line);
                            let match_details = executor.match_with_details(plan, &line);
                            let matched_keywords = match_details.as_ref().map(|details| {
                                details
                                    .iter()
                                    .map(|detail| detail.term_value.clone())
                                    .collect::<HashSet<_>>()
                                    .into_iter()
                                    .collect::<Vec<_>>()
                            });

                            results.push(LogEntry {
                                id: global_offset + i,
                                timestamp: ts,
                                level: lvl,
                                file: virtual_path.to_string(),
                                real_path: real_path.to_string(),
                                line: i + 1,
                                content: line,
                                tags: vec![],
                                match_details,
                                matched_keywords: matched_keywords.filter(|v| !v.is_empty()),
                            });
                        }
                    }
                }

                debug!(
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
