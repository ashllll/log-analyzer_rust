use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::{ImportOptions, ImportResult, ImportService};
use crate::utils::encoding::decode_log_content;
use la_archive::processor::process_path_with_cas_cancellable;
use la_core::error::{AppError, ErrorCategory};
use la_core::traits::AppConfigProvider;
use la_core::utils::compute_file_stats;

use super::WorkspaceServiceImpl;
const FALLBACK_STATS_DELAY_SECS: u64 = 5;

async fn rebuild_search_index_inner(
    metadata_store: Arc<la_storage::MetadataStore>,
    cas: Arc<la_storage::ContentAddressableStorage>,
    search_manager: Arc<la_search::SearchEngineManager>,
) -> std::result::Result<usize, String> {
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to enumerate imported files for indexing: {e}"))?;

    tokio::task::spawn_blocking(move || -> std::result::Result<usize, String> {
        let mut rebuild = search_manager
            .begin_rebuild()
            .map_err(|e| format!("Failed to begin search index rebuild: {e}"))?;

        let mut indexed_lines = 0usize;

        for file in files {
            let content = cas.read_content_sync(&file.sha256_hash).map_err(|e| {
                format!("Failed to read CAS content for {}: {e}", file.virtual_path)
            })?;
            let (content_str, _) = decode_log_content(&content);
            let real_path = format!("cas://{}", file.sha256_hash);

            let mut line_buffer = Vec::with_capacity(1024);
            let mut start_line_number = 1usize;

            for line in content_str.lines() {
                line_buffer.push(line.to_string());

                if line_buffer.len() >= 1024 {
                    let entries = la_core::utils::parse_log_lines(
                        &line_buffer,
                        &file.virtual_path,
                        &real_path,
                        indexed_lines,
                        start_line_number,
                    );
                    rebuild
                        .add_documents(&entries)
                        .map_err(|e| format!("Failed to add indexed documents: {e}"))?;
                    indexed_lines += entries.len();
                    start_line_number += line_buffer.len();
                    line_buffer.clear();
                }
            }

            if !line_buffer.is_empty() {
                let entries = la_core::utils::parse_log_lines(
                    &line_buffer,
                    &file.virtual_path,
                    &real_path,
                    indexed_lines,
                    start_line_number,
                );
                rebuild
                    .add_documents(&entries)
                    .map_err(|e| format!("Failed to add indexed documents: {e}"))?;
                indexed_lines += entries.len();
            }
        }

        rebuild
            .commit()
            .map_err(|e| format!("Failed to finalize rebuilt search index: {e}"))?;

        Ok(indexed_lines)
    })
    .await
    .map_err(|e| format!("Search index rebuild task panicked: {e}"))?
}

#[async_trait]
impl ImportService for WorkspaceServiceImpl {
    async fn import_file(
        &self,
        source_path: &std::path::Path,
        _options: ImportOptions,
        config_provider: &dyn AppConfigProvider,
        task_id: &str,
        cancellation_token: CancellationToken,
    ) -> la_core::error::Result<ImportResult> {
        let root_name = source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        process_path_with_cas_cancellable(
            source_path,
            &root_name,
            &self.workspace_dir,
            self.repo.cas(),
            self.repo.metadata_store().clone(),
            config_provider,
            task_id,
            &self.workspace_id,
            None,
            0,
            cancellation_token.clone(),
        )
        .await
        .map_err(|e| {
            if e.category() == ErrorCategory::CancellationRequested {
                e
            } else {
                AppError::archive_error(
                    format!("Import failed: {e}"),
                    Some(source_path.to_path_buf()),
                )
            }
        })?;

        let metadata_store = self.repo.metadata_store().clone();
        let cas = Arc::clone(self.repo.cas());
        let workspace_id = self.workspace_id.clone();
        let ct = cancellation_token.clone();
        tokio::spawn(async move {
            if ct.is_cancelled() {
                return;
            }
            tokio::time::sleep(Duration::from_secs(FALLBACK_STATS_DELAY_SECS)).await;
            if ct.is_cancelled() {
                return;
            }

            match metadata_store.get_all_files().await {
                Ok(files) => {
                    let mut updated = 0usize;
                    let mut failed = 0usize;

                    for file in files {
                        if file.analysis_status == la_core::storage_types::AnalysisStatus::Ready {
                            continue;
                        }

                        match cas.read_content(&file.sha256_hash).await {
                            Ok(content) => {
                                let (min_ts, max_ts, level_mask) = compute_file_stats(&content);
                                if let Err(e) = metadata_store
                                    .update_file_ready(
                                        &file.virtual_path,
                                        min_ts,
                                        max_ts,
                                        level_mask,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        virtual_path = %file.virtual_path,
                                        error = %e,
                                        "Failed to update file ready status in fallback"
                                    );
                                    failed += 1;
                                } else {
                                    updated += 1;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    virtual_path = %file.virtual_path,
                                    hash = %file.sha256_hash,
                                    error = %e,
                                    "Failed to read file content for fallback stats"
                                );
                                failed += 1;
                            }
                        }
                    }

                    if updated > 0 || failed > 0 {
                        tracing::info!(
                            workspace_id = %workspace_id,
                            stats_updated = updated,
                            stats_failed = failed,
                            "Fallback file stats computation completed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        "Failed to get files for fallback stats computation"
                    );
                }
            }
        });

        let metadata_store = self.repo.metadata_store().clone();
        let cas = Arc::clone(self.repo.cas());
        let search_engine = Arc::clone(self.repo.search_engine());
        let workspace_id_bg = self.workspace_id.clone();
        let ct_bg = cancellation_token.clone();
        tokio::spawn(async move {
            if ct_bg.is_cancelled() {
                return;
            }
            if let Err(e) = rebuild_search_index_inner(metadata_store, cas, search_engine).await {
                tracing::warn!(
                    workspace_id = %workspace_id_bg,
                    error = %e,
                    "Background Tantivy index rebuild failed; get_time_range may be stale"
                );
            } else {
                tracing::info!(
                    workspace_id = %workspace_id_bg,
                    "Background Tantivy index rebuild completed"
                );
            }
        });

        let files_imported = self
            .repo
            .metadata_store()
            .count_files()
            .await
            .map_err(|e| {
                tracing::warn!(
                    workspace_id = %self.workspace_id,
                    error = %e,
                    "Failed to count files after import, database may be corrupted"
                );
                AppError::database_error(format!("Failed to count files: {e}"))
            })? as usize;

        Ok(ImportResult {
            root_name,
            files_imported,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::storage_types::{AnalysisStatus, FileMetadata};
    use la_search::manager::SearchConfig;
    use tempfile::TempDir;

    fn file_metadata(hash: String, content_len: usize) -> FileMetadata {
        FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: "logs/app.log".to_string(),
            original_name: "app.log".to_string(),
            size: content_len as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        }
    }

    async fn test_dependencies() -> (
        TempDir,
        TempDir,
        Arc<la_storage::MetadataStore>,
        Arc<la_storage::ContentAddressableStorage>,
        Arc<la_search::SearchEngineManager>,
    ) {
        let workspace = TempDir::new().unwrap();
        let index = TempDir::new().unwrap();
        let metadata_store = Arc::new(
            la_storage::MetadataStore::new(workspace.path())
                .await
                .unwrap(),
        );
        let cas = Arc::new(la_storage::ContentAddressableStorage::new(
            workspace.path().to_path_buf(),
        ));
        let search_manager = Arc::new(
            la_search::SearchEngineManager::new(SearchConfig {
                index_path: index.path().to_path_buf(),
                ..Default::default()
            })
            .unwrap(),
        );

        (workspace, index, metadata_store, cas, search_manager)
    }

    #[tokio::test]
    async fn rebuild_replaces_documents_when_a_virtual_path_changes() {
        let (_workspace, _index, metadata_store, cas, search_manager) = test_dependencies().await;

        let old_content = b"2024-01-01 00:00:00 INFO old content\n";
        let old_hash = cas.store_content(old_content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(old_hash, old_content.len()))
            .await
            .unwrap();

        assert_eq!(
            rebuild_search_index_inner(
                Arc::clone(&metadata_store),
                Arc::clone(&cas),
                Arc::clone(&search_manager),
            )
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );

        let new_content = b"2025-01-01 00:00:00 ERROR new content\n";
        let new_hash = cas.store_content(new_content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(new_hash, new_content.len()))
            .await
            .unwrap();

        assert_eq!(
            rebuild_search_index_inner(metadata_store, cas, Arc::clone(&search_manager))
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_735_689_600, 1_735_689_600, 1)
        );
    }

    #[tokio::test]
    async fn failed_rebuild_preserves_the_last_committed_index() {
        let (_workspace, _index, metadata_store, cas, search_manager) = test_dependencies().await;

        let content = b"2024-01-01 00:00:00 INFO durable content\n";
        let hash = cas.store_content(content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(hash, content.len()))
            .await
            .unwrap();
        rebuild_search_index_inner(
            Arc::clone(&metadata_store),
            Arc::clone(&cas),
            Arc::clone(&search_manager),
        )
        .await
        .unwrap();

        metadata_store
            .insert_file(&file_metadata("missing-cas-object".to_string(), 1))
            .await
            .unwrap();

        let error = rebuild_search_index_inner(metadata_store, cas, Arc::clone(&search_manager))
            .await
            .unwrap_err();
        assert!(error.contains("Failed to read CAS content"));

        search_manager.commit().unwrap();
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }
}
