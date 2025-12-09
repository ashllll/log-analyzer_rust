//! 搜索命令实现
//! 包含日志搜索及缓存逻辑，附带关键词统计与结果批量推送

use rayon::prelude::*;
use regex::Regex;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    sync::Arc,
    thread,
    time::Duration,
};
use tauri::{command, AppHandle, Emitter, State};

use crate::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use crate::models::search_statistics::SearchResultSummary;
use crate::models::{AppState, LogEntry, SearchCacheKey, SearchFilters, SearchQuery};
use crate::services::{calculate_keyword_statistics, parse_metadata, ExecutionPlan, QueryExecutor};

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
    let cache_key: SearchCacheKey = (
        query.clone(),
        workspace_id.clone(),
        filters.time_start.clone(),
        filters.time_end.clone(),
        filters.levels.clone(),
        filters.file_pattern.clone(),
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
            let guard = path_map.lock().expect("Failed to lock path_map");
            guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut all_results: Vec<LogEntry> = files
            .par_iter()
            .enumerate()
            .flat_map(|(idx, (real_path, virtual_path))| {
                search_single_file_with_details(
                    real_path,
                    virtual_path,
                    &executor,
                    &plan,
                    idx * 10000,
                )
            })
            .collect();

        if !filters.levels.is_empty()
            || filters.time_start.is_some()
            || filters.time_end.is_some()
            || filters.file_pattern.is_some()
        {
            all_results.retain(|entry| {
                if !filters.levels.is_empty() && !filters.levels.contains(&entry.level) {
                    return false;
                }
                if let Some(ref start) = filters.time_start {
                    if entry.timestamp < *start {
                        return false;
                    }
                }
                if let Some(ref end) = filters.time_end {
                    if entry.timestamp > *end {
                        return false;
                    }
                }
                if let Some(ref pattern) = filters.file_pattern {
                    if !entry.file.contains(pattern) && !entry.real_path.contains(pattern) {
                        return false;
                    }
                }
                true
            });
        }

        let results_truncated = all_results.len() > max_results;
        if results_truncated {
            all_results.truncate(max_results);
        }

        if !results_truncated && !all_results.is_empty() {
            if let Ok(mut cache_guard) = search_cache.try_lock() {
                cache_guard.put(cache_key.clone(), all_results.clone());
            }
        }

        for chunk in all_results.chunks(500) {
            let _ = app_handle.emit("search-results", chunk);
            thread::sleep(Duration::from_millis(2));
        }

        let duration = start_time.elapsed().as_millis() as u64;
        if let Ok(mut last_duration) = last_search_duration.lock() {
            *last_duration = duration;
        }

        let keyword_stats = calculate_keyword_statistics(&all_results, &raw_terms);
        let summary = SearchResultSummary::new(
            all_results.len(),
            keyword_stats,
            duration,
            results_truncated,
        );

        let _ = app_handle.emit("search-summary", &summary);
        let _ = app_handle.emit("search-complete", all_results.len());
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
