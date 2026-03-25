//! 搜索命令实现
//! 包含日志搜索及缓存逻辑，附带关键词统计与结果批量推送
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, sync::Arc};
use tauri::{command, AppHandle, Emitter, State};
use tracing::{debug, error, info, warn};

use crate::models::search::{
    PagedSearchResult, QueryMetadata, QueryOperator, SearchTerm, TermSource,
};
use crate::models::search_statistics::SearchResultSummary;
use crate::models::{AppState, LogEntry, SearchCacheKey, SearchFilters, SearchQuery};

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
use crate::services::{calculate_keyword_statistics, parse_metadata, ExecutionPlan, QueryExecutor};
use crate::utils::encoding::decode_log_content;

// 分页搜索缓存配置
const MAX_CACHED_SEARCHES: usize = 100;
const MAX_RESULTS_PER_SEARCH: usize = 1_000_000;
const DEFAULT_PAGE_SIZE: usize = 1000;

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
    let workspace_dirs = Arc::clone(&state.workspace_dirs);
    let cas_instances = Arc::clone(&state.cas_instances);
    let metadata_stores = Arc::clone(&state.metadata_stores);
    let cache_manager = Arc::clone(&state.cache_manager);
    let total_searches = Arc::clone(&state.total_searches);
    let cache_hits = Arc::clone(&state.cache_hits);
    let last_search_duration = Arc::clone(&state.last_search_duration);
    let cancellation_tokens = Arc::clone(&state.search_cancellation_tokens);
    // 磁盘搜索结果存储：新架构的核心，替代 search-results IPC 事件
    let disk_result_store = Arc::clone(&state.disk_result_store);

    let max_results = max_results.unwrap_or(50000).min(100_000);
    let filters = filters.unwrap_or_default();

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
            return Err("No workspaces available".to_string());
        }
    };

    // 生成唯一的搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();

    // 创建取消令牌
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = cancellation_tokens.lock();
        // 检查是否已存在相同 ID 的令牌，避免覆盖
        if tokens
            .insert(search_id.clone(), cancellation_token.clone())
            .is_some()
        {
            tracing::warn!(
                "Search ID {} already exists in cancellation tokens, overwriting",
                search_id
            );
        }
    }

    // 缓存键：基于查询参数生成，使用查询内容的哈希作为版本号
    // 使用 SHA-256 哈希确保不同查询使用不同缓存键，避免缓存污染
    let query_version = compute_query_version(&query);
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

            let raw_terms: Vec<String> = query
                .split('|')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect();

            #[allow(clippy::needless_borrow)]
            let keyword_stats = calculate_keyword_statistics(&cached_results, &raw_terms);
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

    // Get workspace directory BEFORE spawn_blocking to avoid holding lock during async operations
    let workspace_dir = {
        let dirs = workspace_dirs.lock();
        match dirs.get(&workspace_id) {
            Some(dir) => dir.clone(),
            None => {
                let _ = app_handle.emit(
                    "search-error",
                    format!("Workspace not found: {}", workspace_id),
                );
                return Err("Workspace not found".to_string());
            }
        }
    };

    // Get or create MetadataStore BEFORE spawn_blocking
    // This avoids calling block_on inside spawn_blocking
    // Note: We must not hold a lock across an await point
    let metadata_store = {
        // 单次加锁完成检查与克隆，避免两步操作之间的竞态条件
        let existing = {
            let stores = metadata_stores.lock();
            stores.get(&workspace_id).map(Arc::clone)
        };

        if let Some(store) = existing {
            store
        } else {
            // Store doesn't exist, create it (no lock held during async creation)
            let store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
                .await
                .map_err(|e| format!("Failed to open metadata store: {}", e))?;
            let store_arc = Arc::new(store);
            // Re-acquire lock to store
            let mut stores = metadata_stores.lock();
            stores.insert(workspace_id.clone(), Arc::clone(&store_arc));
            store_arc
        }
    };

    // Get or create CAS (sync operation, but we need to handle it properly)
    let cas = {
        let mut instances = cas_instances.lock();
        if let Some(cas) = instances.get(&workspace_id) {
            Arc::clone(cas)
        } else {
            let cas_arc = Arc::new(crate::storage::ContentAddressableStorage::new(
                workspace_dir.clone(),
            ));
            instances.insert(workspace_id.clone(), Arc::clone(&cas_arc));
            cas_arc
        }
    };

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
            return Err(format!(
                "Internal error occurred while accessing workspace: {}",
                workspace_id
            ));
        }
    };

    let search_id_clone = search_id.clone();
    let files_for_search = files.clone();

    // 创建磁盘搜索会话（仅在非缓存命中路径下创建，缓存命中路径已创建）
    if !disk_result_store.has_session(&search_id) {
        if let Err(e) = disk_result_store.create_session(&search_id) {
            warn!(error = %e, search_id = %search_id, "无法创建磁盘搜索会话，分页读取功能将不可用");
        }
    }
    let disk_store_spawn = Arc::clone(&disk_result_store);

    // 老王备注：修复线程泄漏！使用tokio::task::spawn_blocking代替std::thread::spawn
    // 这样tokio运行时会管理线程生命周期，避免资源泄漏
    // 注意：现在 MetadataStore 和文件获取已在 spawn_blocking 外部完成，避免了嵌套 runtime 阻塞问题
    let _handle = tokio::task::spawn_blocking(move || {
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

        // ============================================================        // 高级搜索特性集成点        // ============================================================        // FilterEngine: 位图索引加速过滤（10K文档 < 10ms）        // RegexSearchEngine: LRU缓存正则搜索（加速50x+）        // TimePartitionedIndex: 时间分区索引（时序查询优化）        // AutocompleteEngine: Trie树自动补全（< 100ms响应）        //         // 使用方式：        // 1. 从 AppState 获取高级特性实例（已初始化）        // 2. 在搜索前使用 FilterEngine 预过滤候选文档        // 3. 在过滤时使用 RegexSearchEngine 加速正则匹配        // 4. 在时间范围查询时使用 TimePartitionedIndex        //         // 配置开关：config.json -> advanced_features.enable_*        tracing::info!("🔍 高级搜索特性已就绪（可通过配置启用）");

        let mut executor = QueryExecutor::new(100);
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
            total_files = files.len(),
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
        let mut pending_batch: Vec<LogEntry> = Vec::new(); // 当前待发送批次
        let mut all_results: Vec<LogEntry> = Vec::new(); // 用于缓存的完整结果集
        const MAX_CACHE_SIZE: usize = 100_000; // 限制缓存中的结果数量

        // 先发送开始事件
        let _ = app_handle.emit("search-start", "Starting search...");

        'outer: for file_batch in files_for_search.chunks(10) {
            // 检查取消状态
            if cancellation_token.is_cancelled() {
                let _ = app_handle.emit("search-cancelled", search_id_clone.clone());
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
                .iter()
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
                for entry in file_results {
                    // 检查是否已达到max_results限制
                    if results_count >= max_results {
                        was_truncated = true;
                        break 'outer;
                    }

                    // 应用过滤器
                    let mut include = true;

                    let entry_level_lower = entry.level.to_string().to_lowercase();
                    if !filters.levels.is_empty()
                        && !filters
                            .levels
                            .iter()
                            .any(|l| l.to_lowercase() == entry_level_lower)
                    {
                        include = false;
                    }
                    if include && filters.time_start.is_some() {
                        if let Some(ref start) = filters.time_start {
                            if let Ok(entry_dt) =
                                chrono::DateTime::parse_from_rfc3339(entry.timestamp.as_ref())
                            {
                                if let Ok(start_dt) =
                                    chrono::DateTime::parse_from_rfc3339(start.as_str())
                                {
                                    if entry_dt < start_dt {
                                        include = false;
                                    }
                                }
                            }
                        }
                    }
                    if include && filters.time_end.is_some() {
                        if let Some(ref end) = filters.time_end {
                            if let Ok(entry_dt) =
                                chrono::DateTime::parse_from_rfc3339(entry.timestamp.as_ref())
                            {
                                if let Ok(end_dt) =
                                    chrono::DateTime::parse_from_rfc3339(end.as_str())
                                {
                                    if entry_dt > end_dt {
                                        include = false;
                                    }
                                }
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
                            if let Err(e) =
                                disk_store_spawn.append_entries(&search_id_clone, &batch_results)
                            {
                                warn!(error = %e, "磁盘写入搜索结果批次失败");
                            }
                            batch_results.clear();
                            // 发送实时条目计数（前端用于调整虚拟列表大小，不含实际数据）
                            let _ = app_handle.emit("search-progress", results_count);
                        }
                    }
                }
            }

            // 保存未发送的结果
            if !batch_results.is_empty() {
                pending_batch = batch_results;
            }

            total_processed += file_batch.len();
        }

        // 将剩余结果写入磁盘并完成会话
        if !pending_batch.is_empty() {
            if let Err(e) = disk_store_spawn.append_entries(&search_id_clone, &pending_batch) {
                warn!(error = %e, "磁盘写入剩余搜索结果失败");
            }
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
            cache_manager.lock().insert_sync(cache_key, all_results);
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
    if let Some(sha256_hash) = file_identifier.strip_prefix("cas://") {
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

        // Read content from CAS using hash (sync version for spawn_blocking context)
        let content = match cas.read_content_sync(sha256_hash) {
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

        // Convert bytes to string with encoding fallback (三层容错策略)
        let (content_str, encoding_info) = decode_log_content(&content);
        // Explicitly drop content bytes as early as possible to free memory and avoid holding potentially large buffers
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
                    timestamp: ts.into(),
                    level: lvl.into(),
                    file: virtual_path.into(),
                    real_path: file_identifier.into(),
                    line: i + 1,
                    content: line.into(),
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
                                timestamp: ts.into(),
                                level: lvl.into(),
                                file: virtual_path.into(),
                                real_path: real_path.into(),
                                line: i + 1,
                                content: line.into(),
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

// ============================================================================
// 分页搜索功能
// ============================================================================

use std::collections::VecDeque;

/// 分页搜索结果缓存项
#[derive(Clone)]
#[allow(dead_code)]
struct PagedSearchCacheEntry {
    search_id: String,
    query: String,
    workspace_id: String,
    results: Vec<LogEntry>,
    summary: SearchResultSummary,
    cached_at: std::time::Instant,
}

/// 分页搜索缓存
pub struct PagedSearchCache {
    entries: VecDeque<PagedSearchCacheEntry>,
    max_size: usize,
}

impl PagedSearchCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn get(&self, search_id: &str) -> Option<&PagedSearchCacheEntry> {
        self.entries.iter().find(|e| e.search_id == search_id)
    }

    fn insert(&mut self, entry: PagedSearchCacheEntry) {
        // 移除相同 search_id 的旧缓存
        self.entries.retain(|e| e.search_id != entry.search_id);

        // 如果超出容量，移除最旧的
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);
    }

    #[allow(dead_code)]
    fn cleanup_expired(&mut self, max_age_secs: u64) {
        let now = std::time::Instant::now();
        self.entries
            .retain(|e| now.duration_since(e.cached_at).as_secs() < max_age_secs);
    }
}

// 全局分页搜索缓存（使用 parking_lot::Mutex 避免 poison 问题）
static PAGED_SEARCH_CACHE: std::sync::OnceLock<parking_lot::Mutex<PagedSearchCache>> =
    std::sync::OnceLock::new();

fn get_paged_search_cache() -> &'static parking_lot::Mutex<PagedSearchCache> {
    PAGED_SEARCH_CACHE
        .get_or_init(|| parking_lot::Mutex::new(PagedSearchCache::new(MAX_CACHED_SEARCHES)))
}

/// 分页搜索日志
///
/// # 参数
/// - `query`: 搜索查询字符串
/// - `page_size`: 每页大小
/// - `page_index`: 页索引，-1 表示执行新搜索并缓存
/// - `state`: 应用状态
///
/// # 返回
/// 分页搜索结果，包含当前页数据和元数据
#[command]
pub async fn search_logs_paged(
    query: String,
    page_size: Option<usize>,
    page_index: i32,
    #[allow(non_snake_case)] searchId: Option<String>,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<PagedSearchResult, String> {
    // 参数验证
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }
    if query.len() > 1000 {
        return Err("Search query too long (max 1000 characters)".to_string());
    }

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(10000);

    // 处理已有缓存的搜索（page_index >= 0 且有 searchId）
    if page_index >= 0 {
        if let Some(ref sid) = searchId {
            let cache = get_paged_search_cache().lock();
            if let Some(entry) = cache.get(sid) {
                let page = page_index as usize;
                let start = page * page_size;

                if start >= entry.results.len() {
                    // 返回空页
                    return Ok(PagedSearchResult::new(
                        vec![],
                        entry.results.len(),
                        page_index,
                        page_size,
                        entry.summary.clone(),
                        query,
                        sid.clone(),
                    ));
                }

                let end = (start + page_size).min(entry.results.len());
                let page_results = entry.results[start..end].to_vec();

                return Ok(PagedSearchResult::new(
                    page_results,
                    entry.results.len(),
                    page_index,
                    page_size,
                    entry.summary.clone(),
                    query,
                    sid.clone(),
                ));
            }
        }
        // 缓存未找到，需要重新搜索
        return Err(
            "Search not found in cache. Please start a new search with page_index = -1".to_string(),
        );
    }

    // 执行新搜索（page_index == -1）
    let workspace_dirs = Arc::clone(&state.workspace_dirs);
    let cas_instances = Arc::clone(&state.cas_instances);
    let metadata_stores = Arc::clone(&state.metadata_stores);

    let filters = filters.unwrap_or_default();

    // 确定工作区ID
    let workspace_id = if let Some(ref id) = workspaceId {
        id.clone()
    } else {
        let dirs = workspace_dirs.lock();
        dirs.keys().next().cloned().ok_or_else(|| {
            "No workspaces available. Please create a workspace first.".to_string()
        })?
    };

    // Get workspace directory BEFORE spawn_blocking
    let workspace_dir = {
        let dirs = workspace_dirs.lock();
        dirs.get(&workspace_id)
            .cloned()
            .ok_or_else(|| format!("Workspace directory not found for: {}", workspace_id))?
    };

    // Get or create MetadataStore BEFORE spawn_blocking
    // Note: We must not hold a lock across an await point
    let metadata_store = {
        // 单次加锁完成检查与克隆，避免两步操作之间的竞态条件
        let existing = {
            let stores = metadata_stores.lock();
            stores.get(&workspace_id).map(Arc::clone)
        };

        if let Some(store) = existing {
            store
        } else {
            // Store doesn't exist, create it (no lock held during async creation)
            let store = crate::storage::metadata_store::MetadataStore::new(&workspace_dir)
                .await
                .map_err(|e| format!("Failed to open metadata store: {}", e))?;
            let store_arc = Arc::new(store);
            // Re-acquire lock to store
            let mut stores = metadata_stores.lock();
            stores.insert(workspace_id.clone(), Arc::clone(&store_arc));
            store_arc
        }
    };

    // Get CAS (sync)
    let cas = {
        let mut instances = cas_instances.lock();
        if let Some(cas) = instances.get(&workspace_id) {
            Arc::clone(cas)
        } else {
            let cas_arc = Arc::new(crate::storage::ContentAddressableStorage::new(
                workspace_dir.clone(),
            ));
            instances.insert(workspace_id.clone(), Arc::clone(&cas_arc));
            cas_arc
        }
    };

    // Get all files BEFORE spawn_blocking
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to get files: {}", e))?;

    // 生成搜索ID
    let search_id = uuid::Uuid::new_v4().to_string();
    let search_id_for_spawn = search_id.clone();
    let files_for_search = files.clone();

    // 克隆需要在闭包中使用的变量
    let query_for_spawn = query.clone();

    // 在阻塞任务中执行搜索
    let results = tokio::task::spawn_blocking(move || -> Result<Vec<LogEntry>, String> {
        let raw_terms: Vec<String> = query_for_spawn
            .split('|')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();

        if raw_terms.is_empty() {
            return Err("Search query is empty after processing".to_string());
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
            id: "paged_search_query".to_string(),
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
        let plan = executor
            .execute(&structured_query)
            .map_err(|e| format!("Query execution error: {}", e))?;

        // Note: workspace_dir, metadata_store, cas, and files are now obtained
        // BEFORE spawn_blocking to avoid nested runtime blocking issues

        // 执行搜索
        let mut all_results: Vec<LogEntry> = Vec::new();
        let mut keyword_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for file_batch in files_for_search.chunks(10) {
            let batch: Vec<_> = file_batch
                .iter()
                .enumerate()
                .map(|(idx, file_metadata)| {
                    let file_identifier = format!("cas://{}", file_metadata.sha256_hash);
                    search_single_file_with_details(
                        &file_identifier,
                        &file_metadata.virtual_path,
                        Some(&*cas),
                        &executor,
                        &plan,
                        idx * 10000,
                    )
                })
                .collect();

            for file_results in batch {
                for entry in file_results {
                    // 应用过滤器
                    let mut include = true;

                    let entry_level_lower = entry.level.to_string().to_lowercase();
                    if !filters.levels.is_empty()
                        && !filters
                            .levels
                            .iter()
                            .any(|l| l.to_lowercase() == entry_level_lower)
                    {
                        include = false;
                    }
                    if include && filters.time_start.is_some() {
                        if let Some(ref start) = filters.time_start {
                            if let Ok(entry_dt) =
                                chrono::DateTime::parse_from_rfc3339(entry.timestamp.as_ref())
                            {
                                if let Ok(start_dt) =
                                    chrono::DateTime::parse_from_rfc3339(start.as_str())
                                {
                                    if entry_dt < start_dt {
                                        include = false;
                                    }
                                }
                            }
                        }
                    }
                    if include && filters.time_end.is_some() {
                        if let Some(ref end) = filters.time_end {
                            if let Ok(entry_dt) =
                                chrono::DateTime::parse_from_rfc3339(entry.timestamp.as_ref())
                            {
                                if let Ok(end_dt) =
                                    chrono::DateTime::parse_from_rfc3339(end.as_str())
                                {
                                    if entry_dt > end_dt {
                                        include = false;
                                    }
                                }
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
                        // 流式统计
                        if let Some(ref keywords) = entry.matched_keywords {
                            for kw in keywords {
                                *keyword_counts.entry(kw.clone()).or_insert(0) += 1;
                            }
                        }

                        all_results.push(entry);

                        // 限制最大结果数
                        if all_results.len() >= MAX_RESULTS_PER_SEARCH {
                            break;
                        }
                    }
                }
            }
        }

        Ok(all_results)
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))?;

    let results = results?;

    // 计算关键词统计
    let raw_terms: Vec<String> = query
        .split('|')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    let keyword_stats: Vec<crate::models::search_statistics::KeywordStatistics> = raw_terms
        .iter()
        .map(|term| {
            let count = results.iter().filter(|e| e.content.contains(term)).count();
            crate::models::search_statistics::KeywordStatistics::new(
                term.clone(),
                count,
                results.len(),
            )
        })
        .collect();

    let summary = SearchResultSummary::new(
        results.len(),
        keyword_stats,
        0, // 简化处理，不计算实际耗时
        results.len() >= MAX_RESULTS_PER_SEARCH,
    );

    // 缓存结果
    {
        let mut cache = get_paged_search_cache().lock();
        cache.insert(PagedSearchCacheEntry {
            search_id: search_id_for_spawn.clone(),
            query: query.clone(),
            workspace_id: workspace_id.clone(),
            results: results.clone(),
            summary: summary.clone(),
            cached_at: std::time::Instant::now(),
        });
    }

    // 返回第一页
    let page_results = if results.len() > page_size {
        results[..page_size].to_vec()
    } else {
        results.clone()
    };

    Ok(PagedSearchResult::new(
        page_results,
        results.len(),
        0,
        page_size,
        summary,
        query,
        search_id_for_spawn,
    ))
}

/// 清理过期的分页搜索缓存
#[command]
pub async fn cleanup_paged_search_cache(max_age_secs: Option<u64>) -> Result<usize, String> {
    let max_age = max_age_secs.unwrap_or(3600); // 默认1小时
    let mut cache = get_paged_search_cache().lock();
    let before_count = cache.entries.len();
    cache.cleanup_expired(max_age);
    let after_count = cache.entries.len();
    Ok(before_count - after_count)
}

/// 获取分页搜索缓存统计
#[command]
pub async fn get_paged_search_cache_stats() -> Result<serde_json::Value, String> {
    let cache = get_paged_search_cache().lock();
    let total_entries = cache.entries.len();
    let total_results: usize = cache.entries.iter().map(|e| e.results.len()).sum();

    Ok(serde_json::json!({
        "cached_searches": total_entries,
        "total_cached_results": total_results,
        "max_cache_size": MAX_CACHED_SEARCHES,
        "max_results_per_search": MAX_RESULTS_PER_SEARCH,
    }))
}

// ============================================================================
// 流式搜索分页功能 (VirtualSearchManager 集成)
// ============================================================================

/// 获取搜索结果的指定分页
///
/// 通过 VirtualSearchManager 获取已缓存的搜索结果分页，
/// 支持前端使用 useInfiniteQuery 实现流式加载。
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
    search_id: String,
    offset: usize,
    limit: usize,
) -> Result<crate::search_engine::disk_result_store::SearchPageResult, String> {
    use crate::search_engine::disk_result_store::SearchPageResult;

    // 限制每页最大数量，防止内存问题
    let limit = limit.min(10000);

    let disk_store = &state.disk_result_store;

    // 优先从磁盘存储读取（新架构：Notepad++ 式磁盘直写，前端按需分页）
    if disk_store.has_session(&search_id) {
        let result = disk_store
            .read_page(&search_id, offset, limit)
            .map_err(|e| e.to_string())?;

        debug!(
            search_id = %search_id,
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
    if manager.has_session(&search_id) {
        let results = manager.get_range(&search_id, offset, limit);
        let total = manager.get_total_count(&search_id);
        let next_offset = if offset + results.len() < total {
            Some(offset + results.len())
        } else {
            None
        };
        debug!(
            search_id = %search_id,
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

    Err(format!(
        "Search session '{}' not found or expired",
        search_id
    ))
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
    search_id: String,
    query: String,
    entries: Vec<LogEntry>,
) -> Result<String, String> {
    let manager = &state.virtual_search_manager;

    let registered_id = manager.register_session(search_id, query, entries);

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
    search_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let manager = &state.virtual_search_manager;

    if let Some(session) = manager.get_session_info(&search_id) {
        Ok(Some(serde_json::json!({
            "search_id": session.search_id,
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
    search_id: String,
) -> Result<usize, String> {
    // 新架构：优先从 DiskResultStore 读取
    if let Some(status) = state.disk_result_store.get_status(&search_id) {
        return Ok(status.0);
    }
    // 降级：从 VirtualSearchManager 读取
    Ok(state.virtual_search_manager.get_total_count(&search_id))
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
    search_id: String,
) -> Result<bool, String> {
    let manager = &state.virtual_search_manager;
    Ok(manager.remove_session(&search_id))
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
) -> Result<usize, String> {
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
) -> Result<serde_json::Value, String> {
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
