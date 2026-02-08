//! 异步搜索命令实现
//!
//! 提供支持取消和超时的异步搜索功能

use std::time::Duration;
use tauri::{command, AppHandle, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::events::bridge::emit;
use crate::models::{AppState, LogEntry};
use crate::services::parse_metadata;
use crate::storage::{ContentAddressableStorage, MetadataStore};
use crate::utils::async_resource_manager::OperationType;

/// 异步搜索日志
///
/// 支持取消和超时的异步搜索实现
#[command]
pub async fn async_search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    max_results: Option<usize>,
    timeout_seconds: Option<u64>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    let search_id = Uuid::new_v4().to_string();
    let workspace_id = workspaceId.unwrap_or_else(|| "default".to_string());
    let max_results = max_results.unwrap_or(10000).min(50000);
    let timeout = Duration::from_secs(timeout_seconds.unwrap_or(30));

    // 注册异步操作
    let cancellation_token = state
        .register_async_operation(
            search_id.clone(),
            OperationType::Search,
            Some(workspace_id.clone()),
        )
        .await;

    let app_handle = app.clone();
    let search_id_clone = search_id.clone();
    let query_clone = query.clone();
    let workspace_id_clone = workspace_id.clone();

    // 启动异步搜索任务
    tauri::async_runtime::spawn(async move {
        let result = perform_async_search(
            app_handle,
            query_clone,
            workspace_id_clone,
            max_results,
            timeout,
            cancellation_token,
            search_id_clone.clone(),
        )
        .await;

        match result {
            Ok(count) => {
                let _ = emit::async_search_complete(search_id_clone, count);
            }
            Err(e) => {
                let _ = emit::async_search_error(search_id_clone, e);
            }
        }
    });

    Ok(search_id)
}

/// 取消异步搜索
#[command]
pub async fn cancel_async_search(
    search_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.cancel_async_operation(&search_id).await?;
    Ok(())
}

/// 获取活跃搜索数量
#[command]
pub async fn get_active_searches_count(state: State<'_, AppState>) -> Result<usize, String> {
    Ok(state.get_active_operations_count().await)
}

/// 执行异步搜索的核心逻辑
async fn perform_async_search(
    app: AppHandle,
    query: String,
    workspace_id: String,
    max_results: usize,
    timeout: Duration,
    cancellation_token: CancellationToken,
    search_id: String,
) -> Result<usize, String> {
    let start_time = std::time::Instant::now();

    // 发送搜索开始事件
    let _ = emit::async_search_start(&search_id);

    // Get workspace directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let workspace_dir = app_data_dir.join("extracted").join(&workspace_id);

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

    let mut results_count = 0;
    let mut batch_results: Vec<LogEntry> = Vec::new();
    let batch_size = 500;

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
        match search_content_async(&content_str, &file.virtual_path, &query, results_count).await {
            Ok(mut file_results) => {
                for entry in file_results.drain(..) {
                    if results_count >= max_results {
                        break;
                    }
                    batch_results.push(entry);
                    results_count += 1;

                    // 批量发送结果
                    if batch_results.len() >= batch_size {
                        let _ = emit::async_search_results(batch_results.clone());
                        batch_results.clear();
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
            let _ = emit::async_search_progress(search_id.clone(), progress);
        }

        // 让出控制权，避免阻塞
        if i % 5 == 0 {
            tokio::task::yield_now().await;
        }
    }

    // 发送剩余结果
    if !batch_results.is_empty() {
        let _ = emit::async_search_results(batch_results);
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
    query: &str,
    global_offset: usize,
) -> Result<Vec<LogEntry>, String> {
    let mut results = Vec::new();
    let mut line_number = 0;

    for line in content.lines() {
        line_number += 1;

        if line.contains(query) {
            let (ts, lvl) = parse_metadata(line);
            results.push(LogEntry {
                id: global_offset + line_number,
                timestamp: ts.into(),
                level: lvl.into(),
                file: virtual_path.into(),
                real_path: virtual_path.into(),
                line: line_number,
                content: line.into(),
                tags: vec![],
                match_details: None,
                // 使用 Option 类型，只有当有匹配关键词时才包装为 Some
                matched_keywords: if !query.is_empty() {
                    Some(vec![query.to_string()])
                } else {
                    None
                },
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_content_async() {
        let content =
            "2023-01-01 10:00:00 INFO Test log entry\n2023-01-01 10:01:00 ERROR Another entry\n";

        // Test search
        let results = search_content_async(content, "test.log", "Test", 0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].content.as_ref(),
            "2023-01-01 10:00:00 INFO Test log entry"
        );
        assert_eq!(results[0].line, 1);
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }
}
