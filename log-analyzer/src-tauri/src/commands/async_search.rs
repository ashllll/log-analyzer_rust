//! 异步搜索命令实现
//!
//! 提供支持取消和超时的异步搜索功能
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use std::time::Duration;
use tauri::{command, AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::commands::search::{load_search_runtime_config, resolve_search_query};
use crate::models::AppState;
use crate::services::{parse_metadata, ExecutionPlan, QueryExecutor};
use crate::utils::async_resource_manager::OperationType;
use crate::utils::workspace_paths::resolve_workspace_dir;
use la_core::models::search::SearchQuery;
use la_core::models::LogEntry;
use la_storage::{ContentAddressableStorage, MetadataStore};

/// 异步搜索日志
///
/// 支持取消和超时的异步搜索实现
#[command]
pub async fn async_search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] structuredQuery: Option<SearchQuery>,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    #[allow(non_snake_case)] maxResults: Option<usize>,
    #[allow(non_snake_case)] timeoutSeconds: Option<u64>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    let search_id = Uuid::new_v4().to_string();
    // C-H1 优化: 从 workspace_dirs 动态获取第一个可用的工作区，而非硬编码 "default"
    let workspace_id = {
        let dirs = state.workspace_dirs.lock();
        workspaceId
            .clone()
            .or_else(|| dirs.keys().next().cloned())
            .unwrap_or_else(|| "default".to_string())
    };
    let max_results = maxResults.unwrap_or(10000).clamp(1, 50000);
    // timeoutSeconds 最小值为 1，防止 Duration::from_secs(0) 导致立即超时
    let timeout = Duration::from_secs(timeoutSeconds.unwrap_or(30).max(1));

    // 注册异步操作
    let cancellation_token = state
        .register_async_operation(
            search_id.clone(),
            OperationType::Search,
            Some(workspace_id.clone()),
        )
        .await;

    let app_handle = app.clone();
    let app_handle_for_result = app.clone();
    let search_id_clone = search_id.clone();
    let query_clone = query.clone();
    let structured_query_clone = structuredQuery.clone();
    let workspace_id_clone = workspace_id.clone();

    // 启动异步搜索任务
    tauri::async_runtime::spawn(async move {
        let result = perform_async_search(
            app_handle,
            query_clone,
            structured_query_clone,
            workspace_id_clone,
            max_results,
            timeout,
            cancellation_token,
            search_id_clone.clone(),
        )
        .await;

        match result {
            Ok(count) => {
                let _ =
                    app_handle_for_result.emit("async-search-complete", (&search_id_clone, count));
            }
            Err(e) => {
                let _ = app_handle_for_result.emit("async-search-error", (&search_id_clone, &e));
            }
        }
    });

    Ok(search_id)
}

/// 取消异步搜索
#[command]
pub async fn cancel_async_search(
    #[allow(non_snake_case)] searchId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.cancel_async_operation(&searchId).await?;
    Ok(())
}

/// 获取活跃搜索数量
#[command]
pub async fn get_active_searches_count(state: State<'_, AppState>) -> Result<usize, String> {
    Ok(state.get_active_operations_count().await)
}

/// 执行异步搜索的核心逻辑
#[allow(clippy::too_many_arguments)]
async fn perform_async_search(
    app: AppHandle,
    query: String,
    structured_query: Option<SearchQuery>,
    workspace_id: String,
    max_results: usize,
    timeout: Duration,
    cancellation_token: CancellationToken,
    search_id: String,
) -> Result<usize, String> {
    let start_time = std::time::Instant::now();

    // 发送搜索开始事件
    let _ = app.emit("async-search-start", &search_id);

    let workspace_dir = resolve_workspace_dir(&app, &workspace_id)?;

    if !workspace_dir.exists() {
        return Err(format!("Workspace not found: {}", workspace_id));
    }

    // Initialize MetadataStore and CAS
    let metadata_store = MetadataStore::new(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to open metadata store: {}", e))?;

    let cas = ContentAddressableStorage::new(workspace_dir);

    // Get file list from MetadataStore
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to get file list: {}", e))?;

    let runtime_config = load_search_runtime_config(&app);
    let (_, search_query) = resolve_search_query(
        &query,
        structured_query,
        runtime_config.case_sensitive,
        "async_search_query",
    )
    .map_err(|error| error.to_string())?;
    let mut executor = QueryExecutor::new(runtime_config.regex_cache_size.max(1));
    let plan = executor
        .execute(&search_query)
        .map_err(|e| format!("Failed to build async search plan: {}", e))?;

    let mut results_count = 0;
    let mut batch_results: Vec<LogEntry> = Vec::new();
    // 优化：batch_size 从 500 增加到 2000，减少 IPC 调用次数 75%，提高吞吐量
    let batch_size = 2000;

    for (i, file) in files.iter().enumerate() {
        // 检查取消令牌
        if cancellation_token.is_cancelled() {
            tracing::info!(search_id = %search_id, "Search cancelled by user");
            return Err("Search cancelled".to_string());
        }

        // 检查超时
        if start_time.elapsed() > timeout {
            tracing::warn!(search_id = %search_id, "Search timed out");
            return Err("Search timed out".to_string());
        }

        // Read file content from CAS using SHA-256 hash
        let content = match cas.read_content(&file.sha256_hash).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(
                    search_id = %search_id,
                    file = %file.virtual_path,
                    hash = %file.sha256_hash,
                    error = %e,
                    "Failed to read file from CAS"
                );
                continue;
            }
        };

        // Convert bytes to string with encoding fallback (三层容错策略)
        use crate::utils::encoding::decode_log_content;

        let (content_str, encoding_info) = decode_log_content(&content);

        if encoding_info.had_errors {
            tracing::debug!(
                search_id = %search_id,
                file = %file.virtual_path,
                encoding = %encoding_info.encoding,
                fallback_used = encoding_info.fallback_used,
                "File content decoded with encoding fallback in async search"
            );
        }

        // Search content line by line
        match search_content_async(
            &content_str,
            &file.virtual_path,
            &executor,
            &plan,
            results_count,
        )
        .await
        {
            Ok(mut file_results) => {
                for entry in file_results.drain(..) {
                    if results_count >= max_results {
                        break;
                    }
                    batch_results.push(entry);
                    results_count += 1;

                    // 批量发送结果
                    if batch_results.len() >= batch_size {
                        let _ =
                            app.emit("async-search-results", &std::mem::take(&mut batch_results));
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    search_id = %search_id,
                    file = %file.virtual_path,
                    error = %e,
                    "Failed to search file content"
                );
            }
        }

        // 发送进度更新
        if i % 10 == 0 {
            let progress = (i as f64 / files.len() as f64 * 100.0) as u32;
            let _ = app.emit("async-search-progress", (&search_id, progress));
        }

        // 让出控制权，避免阻塞
        if i % 5 == 0 {
            tokio::task::yield_now().await;
        }
    }

    // 发送剩余结果
    if !batch_results.is_empty() {
        let _ = app.emit("async-search-results", &batch_results);
    }

    let duration = start_time.elapsed();
    tracing::info!(
        search_id = %search_id,
        results_count = results_count,
        duration_ms = duration.as_millis(),
        "Async search completed"
    );

    Ok(results_count)
}

/// 异步搜索文件内容
async fn search_content_async(
    content: &str,
    virtual_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Result<Vec<LogEntry>, String> {
    let mut results = Vec::new();
    let mut line_number = 0;

    for line in content.lines() {
        line_number += 1;

        let Some(match_details) = executor.match_with_details(plan, line) else {
            continue;
        };

        let matched_keywords = match_details
            .iter()
            .map(|detail| detail.term_value.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let (ts, lvl) = parse_metadata(line);
        results.push(LogEntry {
            id: global_offset.saturating_add(line_number),
            timestamp: ts.into(),
            level: lvl.into(),
            file: virtual_path.into(),
            real_path: virtual_path.into(),
            line: line_number,
            content: line.into(),
            tags: vec![],
            match_details: Some(match_details),
            matched_keywords: (!matched_keywords.is_empty()).then_some(matched_keywords),
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};

    #[tokio::test]
    async fn test_search_content_async() {
        let content =
            "2023-01-01 10:00:00 INFO Test log entry\n2023-01-01 10:01:00 ERROR Another entry\n";
        let (_, query) = resolve_search_query("Test", None, false, "async_search_query")
            .expect("Query should build");
        let mut executor = QueryExecutor::new(100);
        let plan = executor.execute(&query).expect("Plan should build");

        // Test search
        let results = search_content_async(content, "test.log", &executor, &plan, 0)
            .await
            .expect("Search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].content.as_ref(),
            "2023-01-01 10:00:00 INFO Test log entry"
        );
        assert_eq!(results[0].line, 1);
    }

    #[tokio::test]
    async fn test_search_content_async_preserves_not_matches_without_highlights() {
        let content =
            "2023-01-01 10:00:00 INFO service healthy\n2023-01-01 10:01:00 DEBUG noisy log\n";
        let query = SearchQuery {
            id: "async_search_query".to_string(),
            terms: vec![SearchTerm {
                id: "term_1".to_string(),
                value: "DEBUG".to_string(),
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
        let mut executor = QueryExecutor::new(100);
        let plan = executor.execute(&query).expect("Plan should build");

        let results = search_content_async(content, "test.log", &executor, &plan, 0)
            .await
            .expect("Search should succeed");

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .match_details
            .as_ref()
            .is_some_and(|details| details.is_empty()));
        assert!(results[0].matched_keywords.is_none());
        assert_eq!(results[0].line, 1);
    }

    #[test]
    fn test_async_search_uses_structured_query_when_provided() {
        let structured_query = SearchQuery {
            id: "saved-query".to_string(),
            terms: vec![SearchTerm {
                id: "term_1".to_string(),
                value: "error.*timeout".to_string(),
                operator: QueryOperator::Or,
                source: TermSource::Preset,
                preset_group_id: Some("preset-1".to_string()),
                is_regex: true,
                priority: 5,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 1,
                last_modified: 2,
                execution_count: 3,
                label: Some("saved".to_string()),
            },
        };

        let (_, resolved) = resolve_search_query(
            "error.*timeout",
            Some(structured_query),
            false,
            "async_search_query",
        )
        .expect("Query should resolve");

        assert_eq!(resolved.id, "async_search_query");
        assert!(resolved.terms[0].is_regex);
        assert_eq!(resolved.terms[0].source, TermSource::Preset);
        assert_eq!(
            resolved.terms[0].preset_group_id.as_deref(),
            Some("preset-1")
        );
        assert_eq!(resolved.metadata.execution_count, 0);
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }
}
