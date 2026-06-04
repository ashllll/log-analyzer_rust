//! SearchUseCase — application-layer search orchestration.
//!
//! Encapsulates the search flow using domain traits, keeping the Tauri
//! command handler thin.
//!
//! # P7 去泛型
//!
//! 原设计使用泛型参数 `<L, R, E, S>` 以支持理论上的多种实现替换。
//! 实践中始终只有一套生产实现，泛型参数增加了认知负担和编译时间。
//! P7 改为 trait 对象，保持测试可 mock 的同时简化类型系统。

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::domain::{LogFileRepository, LogSearcher, SearchResultRepository};
use la_core::error::Result;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::commands::search::filters::CompiledSearchFilters;
use crate::utils::encoding::decode_log_content;

/// The application use case for executing a log search.
///
/// P7: 去泛型，使用 trait 对象。所有依赖通过 `Arc<dyn Trait>` 注入，
/// 支持运行时替换实现（主要用于测试 mock）。
pub struct SearchUseCase {
    log_files: Arc<dyn LogFileRepository>,
    results: Arc<dyn SearchResultRepository>,
    events: Arc<dyn EventPublisher>,
    searcher: Arc<dyn LogSearcher>,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl SearchUseCase {
    pub fn new(
        log_files: Arc<dyn LogFileRepository>,
        results: Arc<dyn SearchResultRepository>,
        events: Arc<dyn EventPublisher>,
        searcher: Arc<dyn LogSearcher>,
        thread_pool: Arc<rayon::ThreadPool>,
    ) -> Self {
        Self {
            log_files,
            results,
            events,
            searcher,
            thread_pool,
        }
    }

    /// Execute a search query asynchronously.
    ///
    /// This method spawns the CPU-intensive work on `spawn_blocking` and
    /// returns immediately with a search_id. Progress is reported via
    /// the EventPublisher.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        workspace_id: &str,
        query: &SearchQuery,
        raw_terms: Vec<String>,
        filters: &SearchFilters,
        max_results: usize,
        search_id: String,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let compiled_filters = CompiledSearchFilters::compile(filters)
            .map_err(|e| la_core::error::AppError::Validation(e.message))?;

        // 1. Get candidate files from repository
        let files = self
            .log_files
            .get_files_with_filters(
                workspace_id,
                compiled_filters
                    .time_start
                    .map(|dt| dt.and_utc().timestamp()),
                compiled_filters.time_end.map(|dt| dt.and_utc().timestamp()),
                compiled_filters.level_mask,
                compiled_filters.database_file_pattern().as_deref(),
            )
            .await?;

        // 2. Create result session
        self.results.create_session(&search_id)?;

        // 3. Notify start
        self.events.emit_search_start(&search_id).await;

        // 4. Spawn blocking search — clone all captured data for 'static
        let log_files = Arc::clone(&self.log_files);
        let results = Arc::clone(&self.results);
        let events = Arc::clone(&self.events);
        let searcher = Arc::clone(&self.searcher);
        let thread_pool = Arc::clone(&self.thread_pool);
        let sid = search_id.clone();
        let query_owned = query.clone();
        let filters_owned = filters.clone();
        let files_owned = files.clone();
        let raw_terms_owned = raw_terms.clone();

        tokio::task::spawn_blocking(move || {
            Self::execute_blocking(
                log_files,
                results,
                events,
                searcher,
                &sid,
                &query_owned,
                &raw_terms_owned,
                &filters_owned,
                &files_owned,
                max_results,
                thread_pool,
                cancellation_token,
            );
        });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_blocking(
        log_files: Arc<dyn LogFileRepository>,
        results: Arc<dyn SearchResultRepository>,
        events: Arc<dyn EventPublisher>,
        searcher: Arc<dyn LogSearcher>,
        search_id: &str,
        query: &SearchQuery,
        _raw_terms: &[String],
        filters: &SearchFilters,
        files: &[la_core::storage_types::FileMetadata],
        max_results: usize,
        thread_pool: Arc<rayon::ThreadPool>,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) {
        let start = std::time::Instant::now();

        // ── Build execution plan ──
        let plan = match searcher.build_plan(query) {
            Ok(p) => p,
            Err(e) => {
                let events = events.clone();
                let sid = search_id.to_string();
                let msg = e.to_string();
                tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                results.remove_session(search_id);
                return;
            }
        };

        // ── Search loop ──
        let batch_size = 2000;
        let mut results_count = 0;
        let mut was_truncated = false;
        let mut batch: Vec<LogEntry> = Vec::new();

        // Flush helper: persist batch to disk, emit progress via spawn (no block_on)
        let flush = |batch: &mut Vec<LogEntry>, count: usize| -> bool {
            if batch.is_empty() {
                return true;
            }
            if let Err(e) = results.append_entries(search_id, batch) {
                let events = events.clone();
                let sid = search_id.to_string();
                let msg = e.to_string();
                tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                return false;
            }
            batch.clear();
            let events = events.clone();
            let sid = search_id.to_string();
            tokio::spawn(async move { events.emit_search_progress(&sid, count).await });
            true
        };

        'outer: for file_batch in files.chunks(10) {
            if cancellation_token.is_cancelled() {
                break;
            }
            if results_count >= max_results {
                was_truncated = true;
                break;
            }

            let batch_results: Vec<Vec<LogEntry>> = thread_pool.install(|| {
                file_batch
                    .par_iter()
                    .map(|fm| {
                        if cancellation_token.is_cancelled() {
                            return Vec::new();
                        }
                        search_one_file(&log_files, &searcher, fm, &plan, filters)
                    })
                    .collect()
            });

            for file_results in batch_results {
                for entry in file_results {
                    if results_count >= max_results {
                        if !flush(&mut batch, results_count) {
                            break 'outer;
                        }
                        was_truncated = true;
                        break 'outer;
                    }
                    batch.push(entry);
                    results_count += 1;
                    if batch.len() >= batch_size && !flush(&mut batch, results_count) {
                        break 'outer;
                    }
                }
            }
            if !flush(&mut batch, results_count) {
                break;
            }
        }

        let _ = results.complete_session(search_id);
        let duration = start.elapsed().as_millis() as u64;

        let events = events.clone();
        let sid = search_id.to_string();
        tokio::spawn(async move {
            events
                .emit_search_complete(
                    &sid,
                    la_core::domain::event::SearchSummary {
                        total_count: results_count,
                        duration_ms: duration,
                        was_truncated,
                    },
                )
                .await;
        });
    }
}

/// Search a single file using CAS content.
fn search_one_file(
    log_files: &Arc<dyn LogFileRepository>,
    searcher: &Arc<dyn LogSearcher>,
    fm: &la_core::storage_types::FileMetadata,
    plan: &la_core::domain::ExecutionPlan,
    filters: &SearchFilters,
) -> Vec<LogEntry> {
    let hash = &fm.sha256_hash;
    let content = match log_files.read_content_sync(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };

    let (text, _) = decode_log_content(&content);
    let mut entries = searcher.match_content(&text, &fm.virtual_path, plan, filters, 0);
    for entry in &mut entries {
        entry.real_path = format!("cas://{}", hash).into();
    }
    entries
}
