//! SearchUseCase — application-layer search orchestration.
//!
//! # P7 重构
//!
//! - 去泛型：使用 trait 对象（`Arc<dyn Trait>`），简化类型系统
//! - 提取 SearchExecutor：搜索循环 → 独立可测试结构体
//! - 消除 block_on：进度事件通过 `tokio::spawn` 发射

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::domain::{LogFileRepository, LogSearcher, SearchResultRepository};
use la_core::error::Result;

use crate::application::search_executor::SearchExecutor;
use crate::services::search_filters::CompiledSearchFilters;

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

        // 4. Spawn blocking search via SearchExecutor
        let executor = SearchExecutor::new(
            Arc::clone(&self.log_files),
            Arc::clone(&self.results),
            Arc::clone(&self.events),
            Arc::clone(&self.searcher),
            Arc::clone(&self.thread_pool),
        );
        let sid = search_id.clone();
        let query_owned = query.clone();
        let filters_owned = filters.clone();
        let files_owned = files.clone();

        tokio::task::spawn_blocking(move || {
            let outcome = executor.run(
                &sid,
                &query_owned,
                &filters_owned,
                &files_owned,
                max_results,
                cancellation_token,
            );

            let _ = executor.results.complete_session(&sid);
            tokio::spawn(async move {
                executor.events
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
}
