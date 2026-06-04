//! Application layer — use cases + service traits.
//!
//! # Architecture
//!
//! - **Use cases** (config/export/search): domain-trait-based orchestrators
//! - **WorkspaceService** trait family: per-workspace service composition (SearchService, ImportService, WatchService)
//! - Dead use cases removed in P6: ImportUseCase (replaced by ImportService), WorkspaceUseCase + RuntimeWorkspaceRepository (never wired)

pub mod config;
pub mod export;
pub mod search;
pub mod search_executor;
pub mod search_filters;
pub mod workspace_service;

pub use config::ConfigUseCase;
pub use export::ExportUseCase;
pub use search::SearchUseCase;
pub use workspace_service::{
    ImportOptions, ImportResult, ImportService, SearchService, WatchService, WorkspaceService,
    WorkspaceServiceRef,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use la_core::domain::event::EventPublisher;
    use la_core::domain::{LogFileRepository, SearchResultPage, SearchResultRepository};
    use la_core::error::Result;
    use la_core::storage_types::FileMetadata;
    use std::sync::Arc;

    // ── Test doubles ──

    struct StubLogFiles;
    #[async_trait]
    impl LogFileRepository for StubLogFiles {
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
                size: 100,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
                min_timestamp: None,
                max_timestamp: None,
                level_mask: Some(0),
                analysis_status: la_core::storage_types::AnalysisStatus::Ready,
            }])
        }
        fn read_content_sync(&self, _hash: &str) -> Result<Vec<u8>> {
            Ok(b"error: test\ndebug: foo\n".to_vec())
        }
        fn file_exists_sync(&self, _hash: &str) -> bool {
            true
        }
    }

    struct StubResults {
        entries: std::sync::Mutex<Vec<la_core::models::LogEntry>>,
    }
    impl SearchResultRepository for StubResults {
        fn create_session(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        fn append_entries(&self, _id: &str, entries: &[la_core::models::LogEntry]) -> Result<()> {
            self.entries.lock().unwrap().extend_from_slice(entries);
            Ok(())
        }
        fn read_page(&self, _id: &str, _off: usize, _lim: usize) -> Result<SearchResultPage> {
            Ok(SearchResultPage {
                entries: vec![],
                total_count: 0,
                is_complete: true,
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

    struct StubEvents;
    #[async_trait]
    impl EventPublisher for StubEvents {
        async fn emit_search_start(&self, _id: &str) {}
        async fn emit_search_progress(&self, _id: &str, _c: usize) {}
        async fn emit_search_complete(&self, _id: &str, _s: la_core::domain::event::SearchSummary) {
        }
        async fn emit_search_error(&self, _id: &str, _e: &str) {}
        async fn emit_search_cancelled(&self, _id: &str) {}
        async fn emit_search_timeout(&self, _id: &str) {}
        async fn emit_file_changed(
            &self,
            _workspace_id: &str,
            _event_type: &str,
            _file_path: &str,
            _timestamp: i64,
        ) {
        }
        async fn emit_new_logs(&self, _workspace_id: &str, _entries_json: &str) {}
    }

    #[tokio::test]
    async fn search_use_case_integration() {
        let use_case = SearchUseCase::new(
            Arc::new(StubLogFiles),
            Arc::new(StubResults {
                entries: std::sync::Mutex::new(vec![]),
            }),
            Arc::new(StubEvents),
            Arc::new(crate::infrastructure::QueryEngineLogSearcher::new(100)),
            Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(1)
                    .build()
                    .unwrap(),
            ),
        );

        let query = la_core::models::SearchQuery {
            id: "test".into(),
            terms: vec![la_core::models::search::SearchTerm {
                id: "t0".into(),
                value: "error".into(),
                operator: la_core::models::search::QueryOperator::Or,
                source: la_core::models::search::TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: la_core::models::search::QueryOperator::Or,
            filters: None,
            metadata: la_core::models::search::QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let token = tokio_util::sync::CancellationToken::new();
        let result = use_case
            .execute(
                "ws1",
                &query,
                vec!["error".into()],
                &Default::default(),
                100,
                "test-search".into(),
                token,
            )
            .await;

        // The search runs in spawn_blocking, so it completes asynchronously.
        // Just verify no immediate error.
        assert!(
            result.is_ok(),
            "SearchUseCase::execute should not error: {:?}",
            result.err()
        );
    }
}
