//! 搜索策略
//!
//! 定义搜索策略 trait 及三种实现：
//! - TantivySearchStrategy：基于 Tantivy 索引的快速搜索
//! - CasSearchStrategy：基于 CAS 存储的逐文件扫描（回退路径）
//! - HybridSearchStrategy：Tantivy + CAS 混合搜索，深度融合：Tantivy 提供候选文件，
//!   CAS 只扫描 Tantivy 未覆盖的文件做精确补充

use std::sync::Arc;

use rayon::prelude::*;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

use la_core::error::CommandError;
use la_core::models::{LogEntry, SearchQuery};
use la_storage::{ContentAddressableStorage, MetadataStore};

use crate::services::search_filters::{search_single_file_with_details, CompiledSearchFilters};
use crate::services::QueryExecutor;

/// 搜索策略 trait
#[async_trait::async_trait]
pub trait SearchStrategy {
    async fn execute(
        &self,
        query: &SearchQuery,
        workspace_id: &str,
        filters: &CompiledSearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<LogEntry>, CommandError>;
}

/// Tantivy 索引搜索策略
pub struct TantivySearchStrategy {
    pub engine_manager: Arc<la_search::SearchEngineManager>,
    pub raw_terms: Vec<String>,
}

#[async_trait::async_trait]
impl SearchStrategy for TantivySearchStrategy {
    async fn execute(
        &self,
        _query: &SearchQuery,
        _workspace_id: &str,
        filters: &CompiledSearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<LogEntry>, CommandError> {
        let tantivy_query_str = self.raw_terms.join(" OR ");
        let tantivy_start = std::time::Instant::now();

        // 从 CompiledSearchFilters 提取过滤条件
        let time_range = filters.time_start.and_then(|start| {
            filters
                .time_end
                .map(|end| (start.and_utc().timestamp(), end.and_utc().timestamp()))
        });
        let levels = filters.levels.as_ref();
        let file_pattern = filters.file_pattern.as_deref();

        // 构建完整过滤查询
        let query = match self.engine_manager.build_tantivy_query(
            &tantivy_query_str,
            time_range,
            levels,
            file_pattern,
        ) {
            Ok(q) => q,
            Err(e) => {
                warn!(error = %e, "Failed to build Tantivy query");
                return Err(CommandError::new(
                    "QUERY_ERROR",
                    format!("Tantivy query build failed: {}", e),
                ));
            }
        };

        match self
            .engine_manager
            .search_with_query(
                query,
                Some(max_results),
                Some(std::time::Duration::from_millis(500)),
                Some(cancellation_token),
            )
            .await
        {
            Ok(results) => {
                info!(
                    query = %tantivy_query_str,
                    hits = results.entries.len(),
                    total = results.total_count,
                    ms = tantivy_start.elapsed().as_millis(),
                    "Tantivy filtered search succeeded"
                );
                Ok(results.entries)
            }
            Err(la_search::SearchError::Timeout(_)) => {
                warn!("Tantivy search timed out");
                Err(CommandError::new(
                    "TIMEOUT_ERROR",
                    "Tantivy search timed out",
                ))
            }
            Err(e) => {
                warn!(error = %e, "Tantivy search failed");
                Err(CommandError::new(
                    "SEARCH_ERROR",
                    format!("Tantivy search failed: {}", e),
                ))
            }
        }
    }
}

/// CAS 扫描搜索策略（回退路径）
pub struct CasSearchStrategy {
    pub cas: Arc<ContentAddressableStorage>,
    pub metadata_store: Arc<MetadataStore>,
    pub regex_cache_size: usize,
}

impl CasSearchStrategy {
    /// 文件级 pruning：基于文件模式过滤
    fn prune_and_sort_files(
        &self,
        files: Vec<la_core::storage_types::FileMetadata>,
        filters: &CompiledSearchFilters,
    ) -> Vec<la_core::storage_types::FileMetadata> {
        files
            .into_iter()
            .filter(|file| {
                // 文件模式匹配
                filters.matches_file(&file.virtual_path, None)
            })
            .collect()
    }

    /// 对指定文件列表执行 CAS 精确扫描
    pub async fn scan_files(
        &self,
        query: &SearchQuery,
        filters: &CompiledSearchFilters,
        max_results: usize,
        files: Vec<la_core::storage_types::FileMetadata>,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<LogEntry>, CommandError> {
        let files_for_search = self.prune_and_sort_files(files, filters);

        let cas = Arc::clone(&self.cas);
        let filters = filters.clone();
        let query = query.clone();
        let regex_cache_size = self.regex_cache_size;

        let handle = tokio::task::spawn_blocking(move || {
            let mut executor = QueryExecutor::new(regex_cache_size);
            let plan = match executor.execute(&query) {
                Ok(p) => p,
                Err(e) => {
                    return Err(CommandError::new(
                        "QUERY_ERROR",
                        format!("Query execution error: {}", e),
                    ));
                }
            };

            debug!(
                total_files = files_for_search.len(),
                "Starting search across files using CAS"
            );

            let mut results_count = 0usize;
            let mut was_truncated = false;
            let mut all_results: Vec<LogEntry> = Vec::new();

            'outer: for file_batch in files_for_search.chunks(10) {
                if cancellation_token.is_cancelled() {
                    break;
                }

                if results_count >= max_results {
                    was_truncated = true;
                    break 'outer;
                }

                let batch: Vec<_> = file_batch
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
                            &executor,
                            &plan,
                            &filters,
                            idx * 10000,
                        )
                    })
                    .collect();

                if cancellation_token.is_cancelled() {
                    continue;
                }

                for file_results in batch {
                    if cancellation_token.is_cancelled() {
                        break 'outer;
                    }

                    for mut entry in file_results {
                        if cancellation_token.is_cancelled() {
                            break 'outer;
                        }

                        if results_count >= max_results {
                            was_truncated = true;
                            break 'outer;
                        }

                        entry.id = results_count;
                        all_results.push(entry);
                        results_count += 1;
                    }
                }
            }

            if cancellation_token.is_cancelled() {
                return Ok(Vec::new());
            }

            if was_truncated {
                trace!(total = results_count, "Search results truncated");
            }

            Ok(all_results)
        });

        match handle.await {
            Ok(Ok(entries)) => Ok(entries),
            Ok(Err(e)) => Err(e),
            Err(e) => {
                error!(error = %e, "Search task panicked");
                Err(
                    CommandError::new("INTERNAL_ERROR", format!("Search task panicked: {}", e))
                        .with_help(
                            "This is an unexpected error. Try simplifying your search query",
                        ),
                )
            }
        }
    }
}

#[async_trait::async_trait]
impl SearchStrategy for CasSearchStrategy {
    async fn execute(
        &self,
        query: &SearchQuery,
        _workspace_id: &str,
        filters: &CompiledSearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<LogEntry>, CommandError> {
        let time_start = filters.time_start.map(|dt| dt.and_utc().timestamp());
        let time_end = filters.time_end.map(|dt| dt.and_utc().timestamp());
        let file_pattern = filters.file_pattern.as_deref();

        let files = match self
            .metadata_store
            .get_files_with_pruning(time_start, time_end, filters.level_mask, file_pattern)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!(error = %e, "Failed to get files with pruning");
                return Err(CommandError::new(
                    "DATABASE_ERROR",
                    format!("Internal error occurred while accessing workspace: {}", e),
                )
                .with_help("The workspace database may be corrupted. Try refreshing or reimporting the workspace"));
            }
        };

        self.scan_files(query, filters, max_results, files, cancellation_token)
            .await
    }
}

/// 混合搜索策略：Tantivy 快速候选 + CAS 精确补充
///
/// 深度融合实现：
/// 1. Tantivy 搜索获取完整结果（已带过滤条件）
/// 2. 提取 Tantivy 已覆盖的文件路径集合
/// 3. CAS 只扫描 Tantivy **未覆盖**的文件做精确匹配
/// 4. 合并去重后返回
///
/// 这样避免了 CAS 全量扫描所有文件，只在 Tantivy 索引未命中的文件上回退。
pub struct HybridSearchStrategy {
    pub tantivy: TantivySearchStrategy,
    pub cas: CasSearchStrategy,
}

impl HybridSearchStrategy {
    /// 创建新的混合搜索策略实例
    pub fn new(tantivy: TantivySearchStrategy, cas: CasSearchStrategy) -> Self {
        Self { tantivy, cas }
    }
}

#[async_trait::async_trait]
impl SearchStrategy for HybridSearchStrategy {
    async fn execute(
        &self,
        query: &SearchQuery,
        workspace_id: &str,
        filters: &CompiledSearchFilters,
        max_results: usize,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<LogEntry>, CommandError> {
        let start = std::time::Instant::now();

        // Phase 1: Tantivy 快速搜索（已带过滤条件）
        let tantivy_results = self
            .tantivy
            .execute(
                query,
                workspace_id,
                filters,
                max_results,
                cancellation_token.clone(),
            )
            .await?;

        if cancellation_token.is_cancelled() {
            return Ok(Vec::new());
        }

        // 如果 Tantivy 结果已充足，直接返回
        if tantivy_results.len() >= max_results {
            info!(
                hits = tantivy_results.len(),
                ms = start.elapsed().as_millis(),
                "混合搜索：Tantivy 结果充足，直接返回"
            );
            return Ok(tantivy_results);
        }

        // 提取 Tantivy 已覆盖的文件路径
        let tantivy_files: std::collections::HashSet<String> =
            tantivy_results.iter().map(|e| e.file.to_string()).collect();

        info!(
            tantivy_hits = tantivy_results.len(),
            covered_files = tantivy_files.len(),
            "混合搜索：Tantivy 候选提取完成"
        );

        // Phase 2: 获取所有文件，筛选出 Tantivy 未覆盖的文件
        let all_files = match self.cas.metadata_store.get_all_files().await {
            Ok(files) => files,
            Err(e) => {
                error!(error = %e, "Failed to get all files for hybrid search");
                return Err(CommandError::new(
                    "DATABASE_ERROR",
                    format!("Internal error occurred while accessing workspace: {}", e),
                )
                .with_help("The workspace database may be corrupted. Try refreshing or reimporting the workspace"));
            }
        };

        let uncovered_files: Vec<la_core::storage_types::FileMetadata> = all_files
            .into_iter()
            .filter(|f| !tantivy_files.contains(&f.virtual_path))
            .collect();

        if uncovered_files.is_empty() {
            info!(
                hits = tantivy_results.len(),
                ms = start.elapsed().as_millis(),
                "混合搜索：无未覆盖文件，返回 Tantivy 结果"
            );
            return Ok(tantivy_results);
        }

        info!(
            uncovered_files = uncovered_files.len(),
            "混合搜索：CAS 扫描未覆盖文件"
        );

        // Phase 3: CAS 只扫描未覆盖文件
        let cas_entries = self
            .cas
            .scan_files(
                query,
                filters,
                max_results - tantivy_results.len(),
                uncovered_files,
                cancellation_token.clone(),
            )
            .await?;

        if cancellation_token.is_cancelled() {
            return Ok(Vec::new());
        }

        // Phase 4: 合并去重
        let mut results = tantivy_results;
        let existing_keys: std::collections::HashSet<(Arc<str>, usize, Arc<str>)> = results
            .iter()
            .map(|e| (e.file.clone(), e.line, e.content.clone()))
            .collect();

        let mut cas_added = 0usize;
        for entry in cas_entries {
            let key = (entry.file.clone(), entry.line, entry.content.clone());
            if !existing_keys.contains(&key) {
                results.push(entry);
                cas_added += 1;
                if results.len() >= max_results {
                    break;
                }
            }
        }

        info!(
            total = results.len(),
            tantivy = results.len() - cas_added,
            cas = cas_added,
            ms = start.elapsed().as_millis(),
            "混合搜索完成"
        );

        Ok(results)
    }
}
