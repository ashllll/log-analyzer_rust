//! SearchExecutor — 可测试的搜索执行器（P7 提取自 SearchUseCase）。
//!
//! 将 execute_blocking 的搜索循环从 SearchUseCase 中分离，
//! 使批处理、截断、并行文件扫描逻辑可独立测试。

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::domain::{LogFileRepository, LogSearcher, SearchResultRepository};
use la_core::models::{LogEntry, SearchFilters, SearchQuery};
use la_core::storage_types::FileMetadata;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::utils::encoding::decode_log_content;

/// 搜索执行器的运行时上下文。
///
/// 封装所有依赖，使 `run()` 方法零参数。
pub(crate) struct SearchExecutor {
    log_files: Arc<dyn LogFileRepository>,
    pub(crate) results: Arc<dyn SearchResultRepository>,
    pub(crate) events: Arc<dyn EventPublisher>,
    searcher: Arc<dyn LogSearcher>,
    thread_pool: Arc<rayon::ThreadPool>,
}

const BATCH_SIZE: usize = 2000;
const FILE_CHUNK_SIZE: usize = 10;

/// 搜索执行结果。
#[derive(Debug)]
pub(crate) struct SearchOutcome {
    pub(crate) total_count: usize,
    pub(crate) duration_ms: u64,
    pub(crate) was_truncated: bool,
}

impl SearchExecutor {
    pub(crate) fn new(
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

    /// 执行搜索——阻塞运行，在 spawn_blocking 中调用。
    ///
    /// 返回 SearchOutcome 供调用者决定如何发射完成事件。
    pub(crate) fn run(
        &self,
        search_id: &str,
        query: &SearchQuery,
        filters: &SearchFilters,
        files: &[FileMetadata],
        max_results: usize,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> SearchOutcome {
        let start = std::time::Instant::now();

        // ── Build plan ──
        let plan = match self.searcher.build_plan(query) {
            Ok(p) => p,
            Err(e) => {
                let events = self.events.clone();
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
        let mut results_count = 0;
        let mut was_truncated = false;
        let mut batch: Vec<LogEntry> = Vec::new();

        let flush = |batch: &mut Vec<LogEntry>, count: usize| -> bool {
            if batch.is_empty() {
                return true;
            }
            if let Err(e) = self.results.append_entries(search_id, batch) {
                let events = self.events.clone();
                let sid = search_id.to_string();
                let msg = e.to_string();
                tokio::spawn(async move { events.emit_search_error(&sid, &msg).await });
                return false;
            }
            batch.clear();
            let events = self.events.clone();
            let sid = search_id.to_string();
            tokio::spawn(async move { events.emit_search_progress(&sid, count).await });
            true
        };

        'outer: for file_batch in files.chunks(FILE_CHUNK_SIZE) {
            if cancellation_token.is_cancelled() {
                break;
            }
            if results_count >= max_results {
                was_truncated = true;
                break;
            }

            let batch_results: Vec<Vec<LogEntry>> = self.thread_pool.install(|| {
                file_batch
                    .par_iter()
                    .map(|fm| {
                        if cancellation_token.is_cancelled() {
                            return Vec::new();
                        }
                        search_one_file(&self.log_files, &self.searcher, fm, &plan, filters)
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
                    if batch.len() >= BATCH_SIZE && !flush(&mut batch, results_count) {
                        break 'outer;
                    }
                }
            }
            if !flush(&mut batch, results_count) {
                break;
            }
        }

        SearchOutcome {
            total_count: results_count,
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
