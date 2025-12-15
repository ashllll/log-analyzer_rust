//! 搜索命令实现
//! 包含日志搜索及缓存逻辑，附带关键词统计与结果批量推送

use regex::Regex;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tauri::{command, AppHandle, Emitter, State};

use crate::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use crate::models::search_statistics::SearchResultSummary;
use crate::models::{AppState, LogEntry, SearchCacheKey, SearchFilters, SearchQuery};
use crate::services::{calculate_keyword_statistics, parse_metadata, ExecutionPlan, QueryExecutor};

/// 计算并打印缓存统计信息
fn log_cache_statistics(total_searches: &Arc<Mutex<u64>>, cache_hits: &Arc<Mutex<u64>>) {
    if let (Ok(total), Ok(hits)) = (total_searches.lock(), cache_hits.lock()) {
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
}

#[command]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    max_results: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }
    if query.len() > 1000 {
        return Err("Search query too long (max 1000 characters)".to_string());
    }

    let app_handle = app.clone();
    let path_map = Arc::clone(&state.path_map);
    let search_cache = Arc::clone(&state.search_cache);
    let total_searches = Arc::clone(&state.total_searches);
    let cache_hits = Arc::clone(&state.cache_hits);
    let last_search_duration = Arc::clone(&state.last_search_duration);

    let max_results = max_results.unwrap_or(50000).min(100_000);
    let filters = filters.unwrap_or_default();
    let workspace_id = workspaceId.unwrap_or_else(|| "default".to_string());

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
        let cache_result = {
            let mut cache_guard = search_cache
                .lock()
                .map_err(|e| format!("Failed to lock search_cache: {}", e))?;
            cache_guard.get(&cache_key).cloned()
        };

        if let Some(cached_results) = cache_result {
            if let Ok(mut hits) = cache_hits.lock() {
                *hits += 1;
            }
            if let Ok(mut searches) = total_searches.lock() {
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
            return Ok(());
        }
    }

    if let Ok(mut searches) = total_searches.lock() {
        *searches += 1;
    }

    thread::spawn(move || {
        let start_time = std::time::Instant::now();

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

        let mut executor = QueryExecutor::new(100);
        let plan = match executor.execute(&structured_query) {
            Ok(p) => p,
            Err(e) => {
                let _ = app_handle.emit("search-error", format!("Query execution error: {}", e));
                return;
            }
        };

        let files: Vec<(String, String)> = {
            match path_map.lock() {
                Ok(guard) => guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                Err(e) => {
                    let _ = app_handle.emit(
                        "search-error",
                        format!("Failed to access file index: {}", e),
                    );
                    return;
                }
            }
        };

        // 流式处理：分批发送结果，避免内存峰值
        let batch_size = 500;
        let mut total_processed = 0;
        let mut results_count = 0;
        // 流式统计：使用HashMap增量统计关键词，避免累积所有结果
        let mut keyword_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut was_truncated = false;
        let mut pending_batch: Vec<LogEntry> = Vec::new(); // 当前待发送批次

        // 先发送开始事件
        let _ = app_handle.emit("search-start", "Starting search...");

        'outer: for file_batch in files.chunks(10) {
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
        if let Ok(mut last_duration) = last_search_duration.lock() {
            *last_duration = duration;
        }

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

        let _ = app_handle.emit("search-summary", &summary);
        let _ = app_handle.emit("search-complete", results_count);
    });

    Ok(())
}

#[allow(dead_code)]
fn search_single_file(
    real_path: &str,
    virtual_path: &str,
    re: &Regex,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    if let Ok(file) = File::open(real_path) {
        let reader = BufReader::with_capacity(8192, file);

        for (i, line_res) in reader.lines().enumerate() {
            if let Ok(line) = line_res {
                if re.is_match(&line) {
                    let (ts, lvl) = parse_metadata(&line);
                    results.push(LogEntry {
                        id: global_offset + i,
                        timestamp: ts,
                        level: lvl,
                        file: virtual_path.to_string(),
                        real_path: real_path.to_string(),
                        line: i + 1,
                        content: line,
                        tags: vec![],
                        match_details: None,
                        matched_keywords: None,
                    });
                }
            }
        }
    }

    results
}

fn search_single_file_with_details(
    real_path: &str,
    virtual_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    if let Ok(file) = File::open(real_path) {
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
    }

    results
}
