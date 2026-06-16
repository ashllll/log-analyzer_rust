use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::SearchService;
use crate::application::SearchUseCase;
use crate::infrastructure::{CasLogFileRepository, DiskResultStoreRepo, QueryEngineLogSearcher};
use la_core::error::{AppError, Result};
use la_core::models::{SearchFilters, SearchQuery};

use super::WorkspaceServiceImpl;

#[async_trait]
impl SearchService for WorkspaceServiceImpl {
    async fn search(
        &self,
        query: SearchQuery,
        _raw_terms: Vec<String>,
        filters: SearchFilters,
        max_results: usize,
    ) -> Result<String> {
        let search_id = uuid::Uuid::new_v4().to_string();
        let cancellation_token = CancellationToken::new();

        self.search_session_manager
            .create_session(&search_id)
            .map_err(|e| {
                AppError::io_error(format!("Failed to create search session: {e}"), None)
            })?;
        self.search_session_manager
            .register_token(&search_id, cancellation_token.clone());

        let log_files = Arc::new(CasLogFileRepository {
            metadata: self.repo.metadata_store().clone(),
            cas: self.repo.cas().clone(),
        });
        let results = Arc::new(DiskResultStoreRepo {
            store: self.repo.disk_result_store().clone(),
        });
        let searcher: Arc<QueryEngineLogSearcher> = Arc::clone(&self.searcher);

        let use_case = SearchUseCase::new(
            log_files,
            results,
            self.event_publisher.clone(),
            searcher,
            self.thread_pool.clone(),
        );

        let workspace_id = self.workspace_id.clone();
        let search_id_clone = search_id.clone();
        let session_manager = self.search_session_manager.clone();

        tokio::spawn(async move {
            let result = use_case
                .execute(
                    &workspace_id,
                    &query,
                    &filters,
                    max_results,
                    search_id_clone.clone(),
                    cancellation_token,
                )
                .await;

            session_manager.cleanup_token(&search_id_clone);

            if let Err(e) = result {
                tracing::warn!(
                    search_id = %search_id_clone,
                    error = %e,
                    "Search execution failed"
                );
            }
        });

        Ok(search_id)
    }

    async fn fetch_search_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<la_search::SearchPageResult> {
        self.search_session_manager
            .fetch_search_page(search_id, offset, limit)
    }

    async fn cancel_search(&self, search_id: &str) -> Result<()> {
        self.search_session_manager.cancel_search(search_id)
    }
}
