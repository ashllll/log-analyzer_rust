//! Concrete file-processing strategies used by the search orchestrator.
//!
//! The two strategies deliberately share no trait: there is one application
//! consumer, and both paths use the same orchestrator-owned `SearchBatch`.

use std::sync::Arc;

use la_core::domain::filter::CompiledSearchFilter;
use la_core::domain::{ExecutionPlan, LogFileRepository, LogSearcher, SearchResultRepository};
use la_core::error::Result;
use la_core::models::LogEntry;
use la_core::storage_types::FileMetadata;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio_util::sync::CancellationToken;

use crate::application::search_batch::{BatchAction, SearchBatch};
use crate::utils::encoding::decode_log_content;

/// Files at or above this size use the line-streaming path.
pub(crate) const LARGE_FILE_STREAM_THRESHOLD_BYTES: i64 = 64 * 1024;

/// Number of lines supplied to a large-file match at a time.
const SEARCH_LINE_CHUNK_SIZE: usize = 512;

pub(crate) fn is_large_file(size: i64) -> bool {
    size >= LARGE_FILE_STREAM_THRESHOLD_BYTES
}

/// References shared by either concrete file-processing strategy.
///
/// The context is private to this module so domain consumers cannot depend on
/// the application execution shape. The orchestrator still owns the single
/// mutable `SearchBatch` passed to each strategy invocation.
struct SearchExecutionContext<'a> {
    log_files: &'a Arc<dyn LogFileRepository>,
    results: &'a Arc<dyn SearchResultRepository>,
    progress: &'a tokio::sync::mpsc::UnboundedSender<usize>,
    searcher: &'a Arc<dyn LogSearcher>,
    plan: &'a ExecutionPlan,
    compiled_filter: &'a dyn CompiledSearchFilter,
    search_id: &'a str,
    max_results: usize,
    cancellation_token: &'a CancellationToken,
}

impl<'a> SearchExecutionContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        log_files: &'a Arc<dyn LogFileRepository>,
        results: &'a Arc<dyn SearchResultRepository>,
        progress: &'a tokio::sync::mpsc::UnboundedSender<usize>,
        searcher: &'a Arc<dyn LogSearcher>,
        plan: &'a ExecutionPlan,
        compiled_filter: &'a dyn CompiledSearchFilter,
        search_id: &'a str,
        max_results: usize,
        cancellation_token: &'a CancellationToken,
    ) -> Self {
        Self {
            log_files,
            results,
            progress,
            searcher,
            plan,
            compiled_filter,
            search_id,
            max_results,
            cancellation_token,
        }
    }
}

/// Processes a contiguous segment of small files with Rayon.
pub(crate) struct ParallelSmallFileStrategy<'a> {
    context: SearchExecutionContext<'a>,
    thread_pool: &'a rayon::ThreadPool,
}

impl<'a> ParallelSmallFileStrategy<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        log_files: &'a Arc<dyn LogFileRepository>,
        results: &'a Arc<dyn SearchResultRepository>,
        progress: &'a tokio::sync::mpsc::UnboundedSender<usize>,
        searcher: &'a Arc<dyn LogSearcher>,
        plan: &'a ExecutionPlan,
        compiled_filter: &'a dyn CompiledSearchFilter,
        search_id: &'a str,
        max_results: usize,
        cancellation_token: &'a CancellationToken,
        thread_pool: &'a rayon::ThreadPool,
    ) -> Self {
        Self {
            context: SearchExecutionContext::new(
                log_files,
                results,
                progress,
                searcher,
                plan,
                compiled_filter,
                search_id,
                max_results,
                cancellation_token,
            ),
            thread_pool,
        }
    }

    /// Process small files in parallel and consume their results in input order.
    pub(crate) fn process(
        &self,
        files: &[&FileMetadata],
        batch: &mut SearchBatch,
        was_truncated: &mut bool,
        skipped_files: &mut usize,
    ) -> Result<bool> {
        if self.context.cancellation_token.is_cancelled() {
            return Ok(false);
        }
        if files.is_empty() {
            return Ok(true);
        }

        let chunk_results: Vec<Result<Vec<LogEntry>>> = self.thread_pool.install(|| {
            files
                .par_iter()
                .map(|fm| {
                    if self.context.cancellation_token.is_cancelled() {
                        return Ok(Vec::new());
                    }
                    search_one_file(&self.context, fm)
                })
                .collect()
        });

        for file_result in chunk_results {
            match file_result {
                Ok(file_results) => {
                    if !consume_search_entries(&self.context, file_results, batch, was_truncated)? {
                        return Ok(false);
                    }
                }
                Err(_) => {
                    *skipped_files += 1;
                }
            }
        }

        Ok(true)
    }
}

/// Processes one large file a 512 lines at a time.
pub(crate) struct StreamingLargeFileStrategy<'a> {
    context: SearchExecutionContext<'a>,
}

impl<'a> StreamingLargeFileStrategy<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        log_files: &'a Arc<dyn LogFileRepository>,
        results: &'a Arc<dyn SearchResultRepository>,
        progress: &'a tokio::sync::mpsc::UnboundedSender<usize>,
        searcher: &'a Arc<dyn LogSearcher>,
        plan: &'a ExecutionPlan,
        compiled_filter: &'a dyn CompiledSearchFilter,
        search_id: &'a str,
        max_results: usize,
        cancellation_token: &'a CancellationToken,
    ) -> Self {
        Self {
            context: SearchExecutionContext::new(
                log_files,
                results,
                progress,
                searcher,
                plan,
                compiled_filter,
                search_id,
                max_results,
                cancellation_token,
            ),
        }
    }

    /// Stream one large file, stopping on cancellation or global truncation.
    pub(crate) fn process(
        &self,
        fm: &FileMetadata,
        batch: &mut SearchBatch,
        was_truncated: &mut bool,
        skipped_files: &mut usize,
    ) -> Result<bool> {
        if self.context.cancellation_token.is_cancelled() {
            return Ok(false);
        }

        let hash = &fm.sha256_hash;
        let real_path = format!("cas://{hash}");
        let mut keep_searching = true;
        let mut processing_failed = false;
        let mut visitor = |chunk_lines: Vec<String>, chunk_start_line: usize| {
            if self.context.cancellation_token.is_cancelled() {
                keep_searching = false;
                return Ok(false);
            }

            keep_searching = search_line_chunk(
                &self.context,
                &fm.virtual_path,
                &real_path,
                &chunk_lines,
                chunk_start_line,
                batch,
                was_truncated,
            )
            .inspect_err(|_| {
                // The visitor can fail while writing results. Preserve its
                // error instead of treating it as a skippable source read.
                processing_failed = true;
            })?;
            Ok(keep_searching)
        };

        match self.context.log_files.read_line_chunks_sync(
            hash,
            SEARCH_LINE_CHUNK_SIZE,
            &mut visitor,
        ) {
            Ok(()) => Ok(keep_searching),
            Err(error) if processing_failed => Err(error),
            Err(e) => {
                *skipped_files += 1;
                tracing::error!(
                    hash = %hash,
                    virtual_path = %fm.virtual_path,
                    error = %e,
                    "Failed to read line chunks for large file, skipping (results will be incomplete)"
                );
                Ok(true)
            }
        }
    }
}

fn search_one_file(
    context: &SearchExecutionContext<'_>,
    fm: &FileMetadata,
) -> Result<Vec<LogEntry>> {
    let hash = &fm.sha256_hash;
    let content = match context.log_files.read_content_sync(hash) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(
                hash = %hash,
                virtual_path = %fm.virtual_path,
                error = %e,
                "Failed to read file content from CAS, skipping (results will be incomplete)"
            );
            return Err(e);
        }
    };

    let (text, _) = decode_log_content(&content);
    let mut entries = context.searcher.match_content(
        &text,
        &fm.virtual_path,
        context.plan,
        context.compiled_filter,
        0,
    );
    for entry in &mut entries {
        entry.real_path = format!("cas://{hash}").into();
    }
    Ok(entries)
}

fn search_line_chunk(
    context: &SearchExecutionContext<'_>,
    virtual_path: &str,
    real_path: &str,
    lines: &[String],
    start_line: usize,
    batch: &mut SearchBatch,
    was_truncated: &mut bool,
) -> Result<bool> {
    let text = lines.join("\n");
    let mut entries = context.searcher.match_content(
        &text,
        virtual_path,
        context.plan,
        context.compiled_filter,
        batch.total(),
    );
    let line_offset = start_line.saturating_sub(1);
    for entry in &mut entries {
        entry.line += line_offset;
        entry.real_path = real_path.to_string().into();
    }
    consume_search_entries(context, entries, batch, was_truncated)
}

fn consume_search_entries(
    context: &SearchExecutionContext<'_>,
    mut entries: Vec<LogEntry>,
    batch: &mut SearchBatch,
    was_truncated: &mut bool,
) -> Result<bool> {
    let next_id = batch.total();
    for (offset, entry) in entries.iter_mut().enumerate() {
        entry.id = next_id + offset;
    }

    match batch.accumulate(entries, context.max_results) {
        BatchAction::Continue => Ok(true),
        BatchAction::Flush => {
            flush_search_batch(context.results, context.progress, context.search_id, batch)?;
            Ok(true)
        }
        BatchAction::Truncate(_) => {
            if !batch.is_empty() {
                flush_search_batch(context.results, context.progress, context.search_id, batch)?;
            }
            *was_truncated = true;
            Ok(false)
        }
    }
}

/// Flush one shared batch and enqueue its progress in search order.
pub(crate) fn flush_search_batch(
    results: &Arc<dyn SearchResultRepository>,
    progress: &tokio::sync::mpsc::UnboundedSender<usize>,
    search_id: &str,
    batch: &mut SearchBatch,
) -> Result<()> {
    results.append_entries(search_id, &batch.take())?;
    let _ = progress.send(batch.total());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use la_core::domain::filter::{CompiledSearchFilter, LineMetadata};
    use la_core::domain::{
        ExecutionPlan, LogFileRepository, LogSearcher, SearchResultPage, SearchResultRepository,
    };
    use la_core::error::{AppError, Result};
    use la_core::models::SearchQuery;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    struct StrategyLogFiles {
        lines: HashMap<String, Vec<String>>,
        read_errors: HashSet<String>,
        content_reads: Arc<AtomicUsize>,
        chunk_sizes: Arc<Mutex<Vec<usize>>>,
        chunks_visited: Arc<AtomicUsize>,
        saw_append_before_eof: Arc<AtomicBool>,
        append_count: Arc<AtomicUsize>,
        cancel_after_first_chunk: Option<CancellationToken>,
    }

    impl StrategyLogFiles {
        fn new(lines: HashMap<String, Vec<String>>) -> Self {
            Self {
                lines,
                read_errors: HashSet::new(),
                content_reads: Arc::new(AtomicUsize::new(0)),
                chunk_sizes: Arc::new(Mutex::new(Vec::new())),
                chunks_visited: Arc::new(AtomicUsize::new(0)),
                saw_append_before_eof: Arc::new(AtomicBool::new(false)),
                append_count: Arc::new(AtomicUsize::new(0)),
                cancel_after_first_chunk: None,
            }
        }
    }

    #[async_trait]
    impl LogFileRepository for StrategyLogFiles {
        async fn get_files_with_filters(
            &self,
            _workspace_id: &str,
            _time_start: Option<i64>,
            _time_end: Option<i64>,
            _level_mask: Option<u8>,
            _file_pattern: Option<&str>,
        ) -> Result<Vec<FileMetadata>> {
            Ok(Vec::new())
        }

        fn read_content_sync(&self, hash: &str) -> Result<Vec<u8>> {
            self.content_reads.fetch_add(1, Ordering::SeqCst);
            if self.read_errors.contains(hash) {
                return Err(AppError::search_error("simulated content read failure"));
            }
            self.lines
                .get(hash)
                .map(|lines| lines.join("\n").into_bytes())
                .ok_or_else(|| AppError::search_error("missing test content"))
        }

        fn read_line_chunks_sync(
            &self,
            hash: &str,
            chunk_size: usize,
            visitor: &mut dyn FnMut(Vec<String>, usize) -> Result<bool>,
        ) -> Result<()> {
            if self.read_errors.contains(hash) {
                return Err(AppError::search_error("simulated chunk read failure"));
            }
            let lines = self
                .lines
                .get(hash)
                .ok_or_else(|| AppError::search_error("missing test content"))?;
            self.chunk_sizes.lock().unwrap().push(chunk_size);

            let mut start = 0usize;
            while start < lines.len() {
                let end = (start + chunk_size).min(lines.len());
                let keep_reading = visitor(lines[start..end].to_vec(), start + 1)?;
                self.chunks_visited.fetch_add(1, Ordering::SeqCst);
                if self.append_count.load(Ordering::SeqCst) > 0 && end < lines.len() {
                    self.saw_append_before_eof.store(true, Ordering::SeqCst);
                }
                if start == 0 {
                    if let Some(token) = &self.cancel_after_first_chunk {
                        token.cancel();
                    }
                }
                if !keep_reading {
                    return Ok(());
                }
                start = end;
            }

            Ok(())
        }

        fn file_exists_sync(&self, _hash: &str) -> bool {
            true
        }
    }

    struct StrategySearcher;

    impl LogSearcher for StrategySearcher {
        fn build_plan(&self, _query: &SearchQuery) -> Result<ExecutionPlan> {
            Ok(test_plan())
        }

        fn match_content(
            &self,
            text: &str,
            virtual_path: &str,
            _plan: &ExecutionPlan,
            _compiled_filter: &dyn CompiledSearchFilter,
            start_id: usize,
        ) -> Vec<LogEntry> {
            text.lines()
                .enumerate()
                .map(|(line_index, line)| LogEntry {
                    id: start_id + line_index,
                    timestamp: Arc::from(""),
                    level: Arc::from(""),
                    file: Arc::from(virtual_path),
                    line: line_index + 1,
                    content: Arc::from(line),
                    real_path: Arc::from(""),
                    tags: vec![],
                    match_details: None,
                    matched_keywords: None,
                })
                .collect()
        }
    }

    struct AcceptAllFilter;

    impl CompiledSearchFilter for AcceptAllFilter {
        fn matches_file(&self, _virtual_path: &str, _real_path: Option<&str>) -> bool {
            true
        }

        fn matches_line(&self, _metadata: &LineMetadata) -> bool {
            true
        }

        fn has_time_filter(&self) -> bool {
            false
        }
    }

    struct CapturingResults {
        entries: Mutex<Vec<LogEntry>>,
        append_count: Arc<AtomicUsize>,
    }

    impl SearchResultRepository for CapturingResults {
        fn create_session(&self, _search_id: &str) -> Result<()> {
            Ok(())
        }

        fn append_entries(&self, _search_id: &str, entries: &[LogEntry]) -> Result<()> {
            self.append_count.fetch_add(1, Ordering::SeqCst);
            self.entries.lock().unwrap().extend_from_slice(entries);
            Ok(())
        }

        fn read_page(
            &self,
            _search_id: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<SearchResultPage> {
            Ok(SearchResultPage {
                entries: vec![],
                total_count: 0,
                is_complete: false,
                has_more: false,
                next_offset: None,
            })
        }

        fn complete_session(&self, _search_id: &str) -> Result<()> {
            Ok(())
        }

        fn remove_session(&self, _search_id: &str) {}

        fn has_session(&self, _search_id: &str) -> bool {
            true
        }
    }

    struct TestHarness {
        log_files: Arc<dyn LogFileRepository>,
        log_files_probe: Arc<StrategyLogFiles>,
        results: Arc<dyn SearchResultRepository>,
        results_probe: Arc<CapturingResults>,
        progress: tokio::sync::mpsc::UnboundedSender<usize>,
        searcher: Arc<dyn LogSearcher>,
        plan: ExecutionPlan,
        filter: AcceptAllFilter,
        search_id: String,
        cancellation_token: CancellationToken,
    }

    impl TestHarness {
        fn new(mut log_files: StrategyLogFiles, cancellation_token: CancellationToken) -> Self {
            let results_probe = Arc::new(CapturingResults {
                entries: Mutex::new(Vec::new()),
                append_count: Arc::new(AtomicUsize::new(0)),
            });
            log_files.append_count = Arc::clone(&results_probe.append_count);
            let log_files_probe = Arc::new(log_files);
            let (progress, _progress_rx) = tokio::sync::mpsc::unbounded_channel();
            Self {
                log_files: log_files_probe.clone(),
                log_files_probe,
                results: results_probe.clone(),
                results_probe,
                progress,
                searcher: Arc::new(StrategySearcher),
                plan: test_plan(),
                filter: AcceptAllFilter,
                search_id: "strategy-test".into(),
                cancellation_token,
            }
        }

        fn small_strategy<'a>(
            &'a self,
            thread_pool: &'a rayon::ThreadPool,
            max_results: usize,
        ) -> ParallelSmallFileStrategy<'a> {
            ParallelSmallFileStrategy::new(
                &self.log_files,
                &self.results,
                &self.progress,
                &self.searcher,
                &self.plan,
                &self.filter,
                &self.search_id,
                max_results,
                &self.cancellation_token,
                thread_pool,
            )
        }

        fn streaming_strategy<'a>(&'a self, max_results: usize) -> StreamingLargeFileStrategy<'a> {
            StreamingLargeFileStrategy::new(
                &self.log_files,
                &self.results,
                &self.progress,
                &self.searcher,
                &self.plan,
                &self.filter,
                &self.search_id,
                max_results,
                &self.cancellation_token,
            )
        }

        fn entries(&self) -> Vec<LogEntry> {
            self.results_probe.entries.lock().unwrap().clone()
        }
    }

    fn test_plan() -> ExecutionPlan {
        ExecutionPlan {
            id: 1,
            engine_count: 1,
            steps: vec!["test".into()],
            plan: None,
        }
    }

    fn lines(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("line-{i}")).collect()
    }

    fn file(hash: &str, path: &str, size: i64) -> FileMetadata {
        FileMetadata {
            id: 1,
            sha256_hash: hash.into(),
            virtual_path: path.into(),
            original_name: path.into(),
            size,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: la_core::storage_types::AnalysisStatus::Ready,
        }
    }

    fn harness_with_lines(entries: &[(&str, usize)]) -> TestHarness {
        let mut content = HashMap::new();
        for (hash, count) in entries {
            content.insert((*hash).into(), lines(*count));
        }
        TestHarness::new(StrategyLogFiles::new(content), CancellationToken::new())
    }

    fn flush(harness: &TestHarness, batch: &mut SearchBatch) {
        flush_search_batch(
            &harness.results,
            &harness.progress,
            &harness.search_id,
            batch,
        )
        .unwrap();
    }

    #[test]
    fn parallel_small_files_are_consumed_in_input_order() {
        let harness = harness_with_lines(&[("first", 1), ("second", 1)]);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let strategy = harness.small_strategy(&thread_pool, 100);
        let first = file("first", "first.log", 1);
        let second = file("second", "second.log", 1);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(strategy
            .process(
                &[&first, &second],
                &mut batch,
                &mut was_truncated,
                &mut skipped
            )
            .unwrap());
        flush(&harness, &mut batch);

        let entries = harness.entries();
        let paths: Vec<&str> = entries.iter().map(|entry| entry.file.as_ref()).collect();
        assert_eq!(paths, vec!["first.log", "second.log"]);
    }

    #[test]
    fn streaming_uses_512_line_chunks_and_flushes_before_eof() {
        let harness = harness_with_lines(&[("large", 600)]);
        let strategy = harness.streaming_strategy(2_000);
        let large = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(strategy
            .process(&large, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(harness
            .log_files_probe
            .saw_append_before_eof
            .load(Ordering::SeqCst));
        assert_eq!(
            harness
                .log_files_probe
                .chunk_sizes
                .lock()
                .unwrap()
                .as_slice(),
            &[SEARCH_LINE_CHUNK_SIZE]
        );
        flush(&harness, &mut batch);
        assert_eq!(harness.entries().len(), 600);
        assert!(!was_truncated);
    }

    #[test]
    fn threshold_selects_small_below_64_kib_and_streaming_at_64_kib() {
        assert!(!is_large_file(LARGE_FILE_STREAM_THRESHOLD_BYTES - 1));
        assert!(is_large_file(LARGE_FILE_STREAM_THRESHOLD_BYTES));

        let harness = harness_with_lines(&[("below", 1), ("at", 1)]);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let small = harness.small_strategy(&thread_pool, 100);
        let large = harness.streaming_strategy(100);
        let below = file("below", "below.log", LARGE_FILE_STREAM_THRESHOLD_BYTES - 1);
        let at = file("at", "at.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(small
            .process(&[&below], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(large
            .process(&at, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        flush(&harness, &mut batch);
        assert_eq!(
            harness.log_files_probe.content_reads.load(Ordering::SeqCst),
            1
        );
        assert_eq!(harness.log_files_probe.chunk_sizes.lock().unwrap().len(), 1);
    }

    #[test]
    fn alternating_small_large_small_keeps_order_and_continuous_ids() {
        let harness = harness_with_lines(&[("small-1", 1), ("large", 1), ("small-2", 1)]);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let small = harness.small_strategy(&thread_pool, 100);
        let large = harness.streaming_strategy(100);
        let small_1 = file("small-1", "small-1.log", 1);
        let large_file = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let small_2 = file("small-2", "small-2.log", 1);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(small
            .process(&[&small_1], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(large
            .process(&large_file, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(small
            .process(&[&small_2], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        flush(&harness, &mut batch);

        let entries = harness.entries();
        let paths: Vec<&str> = entries.iter().map(|entry| entry.file.as_ref()).collect();
        let ids: Vec<usize> = entries.iter().map(|entry| entry.id).collect();
        assert_eq!(paths, vec!["small-1.log", "large.log", "small-2.log"]);
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn cancellation_before_processing_stops_both_paths_without_reads() {
        let token = CancellationToken::new();
        token.cancel();
        let harness = TestHarness::new(
            StrategyLogFiles::new(HashMap::from([
                ("small".into(), lines(1)),
                ("large".into(), lines(1)),
            ])),
            token,
        );
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let small = harness.small_strategy(&thread_pool, 100);
        let large = harness.streaming_strategy(100);
        let small_file = file("small", "small.log", 1);
        let large_file = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(!small
            .process(&[&small_file], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(!large
            .process(&large_file, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert_eq!(batch.total(), 0);
        assert_eq!(
            harness.log_files_probe.content_reads.load(Ordering::SeqCst),
            0
        );
        assert!(harness
            .log_files_probe
            .chunk_sizes
            .lock()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn cancellation_during_streaming_stops_after_the_current_chunk() {
        let token = CancellationToken::new();
        let mut log_files = StrategyLogFiles::new(HashMap::from([("large".into(), lines(1024))]));
        log_files.cancel_after_first_chunk = Some(token.clone());
        let harness = TestHarness::new(log_files, token);
        let strategy = harness.streaming_strategy(2_000);
        let large = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(!strategy
            .process(&large, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        flush(&harness, &mut batch);
        assert_eq!(
            harness
                .log_files_probe
                .chunks_visited
                .load(Ordering::SeqCst),
            2
        );
        assert_eq!(harness.entries().len(), 512);
    }

    #[test]
    fn max_results_is_global_across_small_and_large_strategies() {
        let harness = harness_with_lines(&[("small", 3), ("large", 3)]);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let small = harness.small_strategy(&thread_pool, 4);
        let large = harness.streaming_strategy(4);
        let small_file = file("small", "small.log", 1);
        let large_file = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(small
            .process(&[&small_file], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(!large
            .process(&large_file, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(was_truncated);
        assert_eq!(batch.total(), 4);
        assert!(batch.is_empty());
        let entries = harness.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(
            entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn streaming_line_offsets_are_separate_from_result_ids() {
        let harness = harness_with_lines(&[("large", 600)]);
        let strategy = harness.streaming_strategy(2_000);
        let large = file("large", "large.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(strategy
            .process(&large, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        flush(&harness, &mut batch);

        let entries = harness.entries();
        assert_eq!(entries[512].line, 513);
        assert_eq!(entries[512].id, 512);
        assert_eq!(entries[599].line, 600);
        assert_eq!(entries[599].id, 599);
    }

    #[test]
    fn large_file_read_failure_skips_and_allows_following_file() {
        let mut log_files = StrategyLogFiles::new(HashMap::from([
            ("bad".into(), lines(2)),
            ("good".into(), lines(1)),
        ]));
        log_files.read_errors.insert("bad".into());
        let harness = TestHarness::new(log_files, CancellationToken::new());
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let small = harness.small_strategy(&thread_pool, 100);
        let large = harness.streaming_strategy(100);
        let bad = file("bad", "bad.log", LARGE_FILE_STREAM_THRESHOLD_BYTES);
        let good = file("good", "good.log", 1);
        let mut batch = SearchBatch::new(256);
        let mut was_truncated = false;
        let mut skipped = 0;

        assert!(large
            .process(&bad, &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        assert!(small
            .process(&[&good], &mut batch, &mut was_truncated, &mut skipped)
            .unwrap());
        flush(&harness, &mut batch);

        let entries = harness.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file.as_ref(), "good.log");
        assert_eq!(entries[0].id, 0);
        assert_eq!(
            skipped, 1,
            "read failure must be counted so callers can signal incomplete results"
        );
    }
}
