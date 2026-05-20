//! SearchUseCase — application-layer search orchestration.
//!
//! Encapsulates the search flow using domain traits, keeping the Tauri
//! command handler thin.

use std::sync::Arc;

use la_core::domain::{LogFileRepository, SearchResultRepository};
use la_core::domain::event::EventPublisher;
use la_core::error::Result;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use la_storage::ContentAddressableStorage;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::commands::search::filters::CompiledSearchFilters;
use crate::commands::search::filters::ParsedLineMetadata;
use crate::services::{ExecutionPlan, QueryPlanBuilder};
use crate::utils::encoding::decode_log_content;

/// The application use case for executing a log search.
pub struct SearchUseCase<L, R, E>
where
    L: LogFileRepository + 'static,
    R: SearchResultRepository + 'static,
    E: EventPublisher + 'static,
{
    log_files: Arc<L>,
    results: Arc<R>,
    events: Arc<E>,
    cas: Arc<ContentAddressableStorage>,
    regex_cache_size: usize,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl<L, R, E> SearchUseCase<L, R, E>
where
    L: LogFileRepository,
    R: SearchResultRepository,
    E: EventPublisher,
{
    pub fn new(
        log_files: Arc<L>,
        results: Arc<R>,
        events: Arc<E>,
        cas: Arc<ContentAddressableStorage>,
        regex_cache_size: usize,
        thread_pool: Arc<rayon::ThreadPool>,
    ) -> Self {
        Self {
            log_files,
            results,
            events,
            cas,
            regex_cache_size,
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
                compiled_filters.time_start.map(|dt| dt.and_utc().timestamp()),
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
        let cas = Arc::clone(&self.cas);
        let thread_pool = Arc::clone(&self.thread_pool);
        let regex_cache_size = self.regex_cache_size;
        let sid = search_id.clone();
        let query_owned = query.clone();
        let filters_owned = compiled_filters.clone();
        let files_owned = files.clone();
        let raw_terms_owned = raw_terms.clone();

        tokio::task::spawn_blocking(move || {
            Self::execute_blocking(
                log_files,
                results,
                events,
                cas,
                &sid,
                &query_owned,
                &raw_terms_owned,
                &filters_owned,
                &files_owned,
                max_results,
                regex_cache_size,
                thread_pool,
                cancellation_token,
            );
        });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_blocking(
        log_files: Arc<L>,
        results: Arc<R>,
        events: Arc<E>,
        cas: Arc<ContentAddressableStorage>,
        search_id: &str,
        query: &SearchQuery,
        _raw_terms: &[String],
        filters: &CompiledSearchFilters,
        files: &[la_core::storage_types::FileMetadata],
        max_results: usize,
        regex_cache_size: usize,
        thread_pool: Arc<rayon::ThreadPool>,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) {
        use std::sync::atomic::{AtomicBool, Ordering};

        let start = std::time::Instant::now();
        let mut builder = QueryPlanBuilder::new(regex_cache_size);

        let plan = match builder.build(query) {
            Ok(p) => p,
            Err(e) => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(events.emit_search_error(search_id, &e.to_string()));
                results.remove_session(search_id);
                return;
            }
        };

        let batch_size = 2000;
        let mut results_count = 0;
        let mut was_truncated = false;
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_clone = Arc::clone(&timed_out);

        let mut batch: Vec<LogEntry> = Vec::new();
        let flush = |batch: &mut Vec<LogEntry>, count: usize| -> bool {
            if batch.is_empty() {
                return true;
            }
            if let Err(e) = results.append_entries(search_id, batch) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(events.emit_search_error(search_id, &e.to_string()));
                return false;
            }
            batch.clear();
            if !timed_out_clone.load(Ordering::SeqCst) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(events.emit_search_progress(search_id, count));
            }
            true
        };

        'outer: for file_batch in files.chunks(10) {
            if cancellation_token.is_cancelled() || timed_out.load(Ordering::SeqCst) {
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
                        search_one_file(
                            &cas,
                            fm,
                            &builder,
                            &plan,
                            filters,
                            &log_files,
                        )
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

        if !timed_out.load(Ordering::SeqCst) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(events.emit_search_complete(
                search_id,
                la_core::domain::event::SearchSummary {
                    total_count: results_count,
                    duration_ms: duration,
                    was_truncated,
                },
            ));
        }
    }
}

/// Search a single file using CAS content.
fn search_one_file<L: LogFileRepository>(
    cas: &ContentAddressableStorage,
    fm: &la_core::storage_types::FileMetadata,
    builder: &QueryPlanBuilder,
    plan: &ExecutionPlan,
    filters: &CompiledSearchFilters,
    _log_files: &Arc<L>,
) -> Vec<LogEntry> {
    // Filter compilation returns CommandError, map to AppError
    let hash = &fm.sha256_hash;
    if !filters.matches_file(&fm.virtual_path, None) {
        return Vec::new();
    }

    let content = match cas.read_content_sync(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };

    let (text, _) = decode_log_content(&content);

    let mut results = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let metadata = ParsedLineMetadata::parse(line, filters.has_time_filter());
        if !filters.matches_parsed_line_metadata(&metadata) {
            continue;
        }

        if let Some(details) = builder.match_with_details(plan, line) {
            let keywords: Vec<String> = details
                .iter()
                .map(|d| d.term_value.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            results.push(LogEntry {
                id: 0, // Will be assigned by the outer loop
                timestamp: metadata.timestamp.into(),
                level: metadata.level.into(),
                file: fm.virtual_path.clone().into(),
                real_path: format!("cas://{}", hash).into(),
                line: idx + 1,
                content: line.to_string().into(),
                tags: vec![],
                match_details: Some(details),
                matched_keywords: if keywords.is_empty() {
                    None
                } else {
                    Some(keywords)
                },
            });
        }
    }
    results
}
