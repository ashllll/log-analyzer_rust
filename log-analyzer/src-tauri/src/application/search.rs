//! SearchUseCase — application-layer search orchestration.
//!
//! # P7 重构
//!
//! - 去泛型：使用 trait 对象（`Arc<dyn Trait>`），简化类型系统
//! - 提取 SearchExecutor：搜索循环 → 独立可测试结构体
//! - 消除 block_on：进度事件通过 `tokio::spawn` 发射
//!
//! # P7-续（#56）
//!
//! - 将 SearchExecutor 循环折叠回 SearchUseCase
//! - 纯批量逻辑提取为 SearchBatch 模块
//! - 删除 application/search_executor.rs

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::domain::{ExecutionPlan, LogFileRepository, LogSearcher, SearchResultRepository};
use la_core::error::Result;
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use la_core::storage_types::FileMetadata;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::application::search_batch::{BatchAction, SearchBatch};
use crate::services::search_filters::CompiledSearchFilters;
use crate::utils::encoding::decode_log_content;

const BATCH_SIZE: usize = 2000;
const FILE_CHUNK_SIZE: usize = 10;

/// 搜索执行结果。
#[derive(Debug)]
pub(crate) struct SearchOutcome {
    pub(crate) total_count: usize,
    pub(crate) duration_ms: u64,
    pub(crate) was_truncated: bool,
}

/// The application use case for executing a log search.
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
    /// CPU-intensive work runs on `spawn_blocking`; this method returns
    /// immediately. Progress and results are reported via EventPublisher.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        workspace_id: &str,
        query: &la_core::models::SearchQuery,
        filters: &la_core::models::SearchFilters,
        max_results: usize,
        search_id: String,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        let compiled_filters = CompiledSearchFilters::compile(filters)
            .map_err(|e| la_core::error::AppError::validation_error(e.message))?;

        // 1. Get candidate files
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

        // 4. Spawn blocking search
        let sid = search_id.clone();
        let query_owned = query.clone();
        let filters_owned = filters.clone();
        let files_owned = files.clone();
        let log_files = Arc::clone(&self.log_files);
        let results = Arc::clone(&self.results);
        let events = Arc::clone(&self.events);
        let searcher = Arc::clone(&self.searcher);
        let thread_pool = Arc::clone(&self.thread_pool);

        tokio::task::spawn_blocking(move || {
            let outcome = Self::run_blocking(
                &log_files,
                &results,
                &events,
                &searcher,
                &thread_pool,
                &sid,
                &query_owned,
                &filters_owned,
                &files_owned,
                max_results,
                cancellation_token,
            );

            let _ = results.complete_session(&sid);
            tokio::spawn(async move {
                events
                    .emit_search_complete(
                        &sid,
                        la_core::domain::event::SearchSummary {
                            total_count: outcome.total_count,
                            duration_ms: outcome.duration_ms,
                            was_truncated: outcome.was_truncated,
                        },
                    )
                    .await;
            });
        });

        Ok(())
    }

    /// 阻塞搜索循环 —— 在 spawn_blocking 中调用。
    fn run_blocking(
        log_files: &Arc<dyn LogFileRepository>,
        results: &Arc<dyn SearchResultRepository>,
        events: &Arc<dyn EventPublisher>,
        searcher: &Arc<dyn LogSearcher>,
        thread_pool: &Arc<rayon::ThreadPool>,
        search_id: &str,
        query: &SearchQuery,
        filters: &SearchFilters,
        files: &[FileMetadata],
        max_results: usize,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> SearchOutcome {
        let start = std::time::Instant::now();

        // ── Build plan ──
        let plan = match searcher.build_plan(query) {
            Ok(p) => p,
            Err(e) => {
                let events = events.clone();
                let sid = search_id.to_string();
                let msg = e.to_string();
                tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                return SearchOutcome {
                    total_count: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    was_truncated: false,
                };
            }
        };

        // ── Search loop ──
        let mut batch = SearchBatch::new(BATCH_SIZE);
        let mut was_truncated = false;

        'outer: for file_batch in files.chunks(FILE_CHUNK_SIZE) {
            if cancellation_token.is_cancelled() {
                break;
            }

            let chunk_results: Vec<Vec<LogEntry>> = thread_pool.install(|| {
                file_batch
                    .par_iter()
                    .map(|fm| {
                        if cancellation_token.is_cancelled() {
                            return Vec::new();
                        }
                        search_one_file(log_files, searcher, fm, &plan, filters)
                    })
                    .collect()
            });

            for file_results in chunk_results {
                match batch.accumulate(file_results, max_results) {
                    BatchAction::Continue => {}
                    BatchAction::Flush => {
                        if let Err(e) = results.append_entries(search_id, &batch.take()) {
                            let events = events.clone();
                            let sid = search_id.to_string();
                            let msg = e.to_string();
                            tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                            break 'outer;
                        }
                        let events = events.clone();
                        let sid = search_id.to_string();
                        let count = batch.total();
                        tokio::spawn(async move { events.emit_search_progress(&sid, count).await });
                    }
                    BatchAction::Truncate(_) => {
                        if !batch.is_empty() {
                            if let Err(e) = results.append_entries(search_id, &batch.take()) {
                                let events = events.clone();
                                let sid = search_id.to_string();
                                let msg = e.to_string();
                                tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                                break 'outer;
                            }
                            let events = events.clone();
                            let sid = search_id.to_string();
                            let count = batch.total();
                            tokio::spawn(async move { events.emit_search_progress(&sid, count).await });
                        }
                        was_truncated = true;
                        break 'outer;
                    }
                }
            }
        }

        // Final flush
        if !batch.is_empty() {
            let _ = results.append_entries(search_id, &batch.take());
        }

        SearchOutcome {
            total_count: batch.total(),
            duration_ms: start.elapsed().as_millis() as u64,
            was_truncated,
        }
    }
}

/// 搜索单个文件的日志条目。
fn search_one_file(
    log_files: &Arc<dyn LogFileRepository>,
    searcher: &Arc<dyn LogSearcher>,
    fm: &FileMetadata,
    plan: &ExecutionPlan,
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
        entry.real_path = format!("cas://{hash}").into();
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use la_core::domain::event::EventPublisher;
    use la_core::domain::{
        ExecutionPlan, LogFileRepository, LogSearcher, MatchPlan, SearchResultPage,
        SearchResultRepository,
    };
    use la_core::error::Result;
    use la_core::models::match_detail::MatchDetail;
    use la_core::models::{LogEntry, SearchFilters, SearchQuery};
    use la_core::storage_types::FileMetadata;
    use std::sync::{Arc, Mutex};

    // ── Stub MatchPlan: returns N matches per line ──

    struct StubMatchPlan {
        matches_per_line: usize,
    }

    impl MatchPlan for StubMatchPlan {
        fn match_line(&self, line: &str) -> Option<Vec<MatchDetail>> {
            if line.is_empty() {
                return None;
            }
            let details: Vec<MatchDetail> = (0..self.matches_per_line)
                .map(|i| MatchDetail {
                    term_id: format!("t{i}"),
                    term_value: line.to_string(),
                    priority: 1,
                    match_position: Some((0, line.len())),
                })
                .collect();
            Some(details)
        }
    }

    // ── Stub LogSearcher ──

    struct StubSearcher {
        plan_id: u64,
        matches_per_line: usize,
    }

    impl LogSearcher for StubSearcher {
        fn build_plan(&self, _query: &SearchQuery) -> Result<ExecutionPlan> {
            Ok(ExecutionPlan {
                id: self.plan_id,
                engine_count: 1,
                steps: vec!["stub".into()],
                plan: Some(Arc::new(StubMatchPlan {
                    matches_per_line: self.matches_per_line,
                })),
            })
        }

        fn match_content(
            &self,
            text: &str,
            virtual_path: &str,
            plan: &ExecutionPlan,
            _filters: &SearchFilters,
            start_id: usize,
        ) -> Vec<LogEntry> {
            let mut entries = Vec::new();
            for (i, line) in text.lines().enumerate() {
                if let Some(match_plan) = &plan.plan {
                    if let Some(_details) = match_plan.match_line(line) {
                        entries.push(LogEntry {
                            id: start_id + i,
                            timestamp: Arc::from(""),
                            level: Arc::from(""),
                            file: Arc::from(virtual_path),
                            line: i + 1,
                            content: Arc::from(line),
                            real_path: Arc::from(""),
                            tags: vec![],
                            match_details: None,
                            matched_keywords: None,
                        });
                    }
                }
            }
            entries
        }
    }

    // ── Stub LogFileRepository with controllable content ──

    struct ContentStubLogFiles {
        content: Vec<u8>,
    }

    #[async_trait]
    impl LogFileRepository for ContentStubLogFiles {
        async fn get_files_with_filters(
            &self,
            _ws: &str,
            _ts: Option<i64>,
            _te: Option<i64>,
            _lm: Option<u8>,
            _fp: Option<&str>,
        ) -> Result<Vec<FileMetadata>> {
            Ok(vec![FileMetadata {
                id: 1,
                sha256_hash: "abc123".into(),
                virtual_path: "test.log".into(),
                original_name: "test.log".into(),
                size: self.content.len() as i64,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
                min_timestamp: None,
                max_timestamp: None,
                level_mask: None,
                analysis_status: la_core::storage_types::AnalysisStatus::Ready,
            }])
        }

        fn read_content_sync(&self, _hash: &str) -> Result<Vec<u8>> {
            Ok(self.content.clone())
        }

        fn file_exists_sync(&self, _hash: &str) -> bool {
            true
        }
    }

    // ── Stub SearchResultRepository that captures appends ──

    struct CapturingResults {
        entries: Mutex<Vec<LogEntry>>,
        append_count: Mutex<usize>,
    }

    impl CapturingResults {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
                append_count: Mutex::new(0),
            }
        }
    }

    impl SearchResultRepository for CapturingResults {
        fn create_session(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        fn append_entries(&self, _id: &str, entries: &[LogEntry]) -> Result<()> {
            *self.append_count.lock().unwrap() += 1;
            self.entries.lock().unwrap().extend_from_slice(entries);
            Ok(())
        }

        fn read_page(&self, _id: &str, _off: usize, _lim: usize) -> Result<SearchResultPage> {
            Ok(SearchResultPage {
                entries: vec![],
                total_count: 0,
                is_complete: false,
                has_more: false,
                next_offset: None,
            })
        }

        fn complete_session(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        fn remove_session(&self, _id: &str) {}

        fn has_session(&self, _id: &str) -> bool {
            true
        }
    }

    // ── Stub EventPublisher that captures emitted events ──

    struct CapturingEvents {
        error_count: Mutex<usize>,
        progress_count: Mutex<usize>,
    }

    impl CapturingEvents {
        fn new() -> Self {
            Self {
                error_count: Mutex::new(0),
                progress_count: Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl EventPublisher for CapturingEvents {
        async fn emit_search_start(&self, _id: &str) {}
        async fn emit_search_progress(&self, _id: &str, _c: usize) {
            *self.progress_count.lock().unwrap() += 1;
        }
        async fn emit_search_complete(&self, _id: &str, _s: la_core::domain::event::SearchSummary) {
        }
        async fn emit_search_error(&self, _id: &str, _e: &str) {
            *self.error_count.lock().unwrap() += 1;
        }
        async fn emit_search_cancelled(&self, _id: &str) {}
        async fn emit_search_timeout(&self, _id: &str) {}
        async fn emit_file_changed(&self, _ws: &str, _et: &str, _fp: &str, _ts: i64) {}
        async fn emit_new_logs(&self, _ws: &str, _ej: &str) {}
    }

    // ── Test helpers ──

    fn make_test_use_case(
        content: &str,
        matches_per_line: usize,
    ) -> (
        SearchUseCase,
        Arc<CapturingResults>,
        Arc<CapturingEvents>,
    ) {
        let log_files: Arc<dyn LogFileRepository> = Arc::new(ContentStubLogFiles {
            content: content.as_bytes().to_vec(),
        });
        let results = Arc::new(CapturingResults::new());
        let events = Arc::new(CapturingEvents::new());
        let searcher: Arc<dyn LogSearcher> = Arc::new(StubSearcher {
            plan_id: 42,
            matches_per_line,
        });
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .build()
                .unwrap(),
        );

        let use_case = SearchUseCase::new(
            log_files,
            results.clone() as Arc<dyn SearchResultRepository>,
            events.clone() as Arc<dyn EventPublisher>,
            searcher,
            thread_pool,
        );

        (use_case, results, events)
    }

    fn make_test_files() -> Vec<FileMetadata> {
        vec![FileMetadata {
            id: 1,
            sha256_hash: "abc123".into(),
            virtual_path: "test.log".into(),
            original_name: "test.log".into(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: la_core::storage_types::AnalysisStatus::Ready,
        }]
    }

    fn make_query() -> SearchQuery {
        use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
        SearchQuery {
            id: "test".into(),
            terms: vec![SearchTerm {
                id: "t0".into(),
                value: ".*".into(),
                operator: QueryOperator::Or,
                source: TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        }
    }

    // ===================================================================
    // Test 1: Empty file set → zero results, no errors
    // ===================================================================

    #[test]
    fn empty_file_set_returns_zero_results() {
        let (use_case, results, events) = make_test_use_case("line1\nline2\n", 1);
        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-1",
            &make_query(),
            &SearchFilters::default(),
            &[],
            1000,
            tokio_util::sync::CancellationToken::new(),
        );

        assert_eq!(outcome.total_count, 0);
        assert!(!outcome.was_truncated);
        assert_eq!(*results.append_count.lock().unwrap(), 0);
        assert_eq!(*events.error_count.lock().unwrap(), 0);
    }

    // ===================================================================
    // Test 2: Single file, below batch size → entries flushed
    // ===================================================================

    #[test]
    fn single_file_below_batch_size_flushes_results() {
        let (use_case, results, _events) = make_test_use_case("line1\nline2\nline3\n", 1);
        let files = make_test_files();

        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-2",
            &make_query(),
            &SearchFilters::default(),
            &files,
            1000,
            tokio_util::sync::CancellationToken::new(),
        );

        assert_eq!(outcome.total_count, 3);
        assert!(!outcome.was_truncated);
        assert_eq!(results.entries.lock().unwrap().len(), 3);
    }

    // ===================================================================
    // Test 3: Batch flush boundary
    // ===================================================================

    #[tokio::test]
    async fn batch_flush_fires_at_boundary() {
        let lines: Vec<String> = (0..2500).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let (use_case, results, _events) = make_test_use_case(&content, 1);
        let files = make_test_files();

        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-3",
            &make_query(),
            &SearchFilters::default(),
            &files,
            10000,
            tokio_util::sync::CancellationToken::new(),
        );

        assert_eq!(outcome.total_count, 2500);
        assert!(!outcome.was_truncated);
        assert_eq!(results.entries.lock().unwrap().len(), 2500);
        // With chunk-based accumulation, the 2500-entry chunk triggers a single
        // Flush because the buffer exceeds the 2000-entry batch_size.
        let appends = *results.append_count.lock().unwrap();
        assert!(appends >= 1, "Expected >= 1 flush calls, got {appends}");
    }

    // ===================================================================
    // Test 4: Truncation at max_results
    // ===================================================================

    #[tokio::test]
    async fn truncation_at_max_results() {
        let (use_case, results, _events) = make_test_use_case("a\nb\nc\nd\ne\n", 1);
        let files = make_test_files();

        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-4",
            &make_query(),
            &SearchFilters::default(),
            &files,
            3,
            tokio_util::sync::CancellationToken::new(),
        );

        assert_eq!(outcome.total_count, 3);
        assert!(outcome.was_truncated);
        assert_eq!(results.entries.lock().unwrap().len(), 3);
    }

    // ===================================================================
    // Test 5: Cancellation mid-scan
    // ===================================================================

    #[test]
    fn cancellation_stops_search_early() {
        let lines: Vec<String> = (0..500).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let (use_case, results, _events) = make_test_use_case(&content, 1);
        let files = make_test_files();
        let token = tokio_util::sync::CancellationToken::new();
        token.cancel();

        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-5",
            &make_query(),
            &SearchFilters::default(),
            &files,
            10000,
            token,
        );

        assert_eq!(outcome.total_count, 0);
        assert!(!outcome.was_truncated);
        assert_eq!(results.entries.lock().unwrap().len(), 0);
    }

    // ===================================================================
    // Test 6: LogSearcher error during build_plan
    // ===================================================================

    /// A searcher that always fails to build a plan.
    struct FailingSearcher;

    impl LogSearcher for FailingSearcher {
        fn build_plan(&self, _query: &SearchQuery) -> Result<ExecutionPlan> {
            Err(la_core::error::AppError::search_error(
                "simulated build_plan failure".to_string(),
            ))
        }

        fn match_content(
            &self,
            _text: &str,
            _virtual_path: &str,
            _plan: &ExecutionPlan,
            _filters: &SearchFilters,
            _start_id: usize,
        ) -> Vec<LogEntry> {
            vec![]
        }
    }

    #[tokio::test]
    async fn build_plan_error_emits_error_event() {
        let log_files: Arc<dyn LogFileRepository> = Arc::new(ContentStubLogFiles {
            content: b"irrelevant".to_vec(),
        });
        let results = Arc::new(CapturingResults::new());
        let events = Arc::new(CapturingEvents::new());
        let searcher: Arc<dyn LogSearcher> = Arc::new(FailingSearcher);
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .build()
                .unwrap(),
        );

        let use_case = SearchUseCase::new(
            log_files,
            results.clone() as Arc<dyn SearchResultRepository>,
            events.clone() as Arc<dyn EventPublisher>,
            searcher,
            thread_pool,
        );

        let files = make_test_files();
        let outcome = SearchUseCase::run_blocking(
            &use_case.log_files,
            &use_case.results,
            &use_case.events,
            &use_case.searcher,
            &use_case.thread_pool,
            "search-6",
            &make_query(),
            &SearchFilters::default(),
            &files,
            1000,
            tokio_util::sync::CancellationToken::new(),
        );

        assert_eq!(outcome.total_count, 0);
        assert!(!outcome.was_truncated);
        // Error event is emitted via tokio::spawn — yield to let it fire
        for _ in 0..5 {
            tokio::task::yield_now().await;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(*events.error_count.lock().unwrap(), 1);
    }
}
