use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::{ImportOptions, ImportResult, ImportService};
use crate::utils::encoding::decode_log_content;
use la_archive::processor::process_path_with_cas_cancellable;
use la_core::error::{AppError, ErrorCategory};
use la_core::traits::AppConfigProvider;
use la_core::utils::compute_file_stats;

use super::WorkspaceServiceImpl;

async fn acquire_import_permit(
    import_lock: Arc<tokio::sync::Mutex<()>>,
    cancellation_token: &CancellationToken,
    source_path: &std::path::Path,
) -> la_core::error::Result<tokio::sync::OwnedMutexGuard<()>> {
    tokio::select! {
        biased;
        _ = cancellation_token.cancelled() => Err(AppError::Archive {
            category: ErrorCategory::CancellationRequested,
            _message: "Import cancelled while waiting for the workspace".to_string(),
            _path: Some(source_path.to_path_buf()),
        }),
        permit = import_lock.lock_owned() => Ok(permit),
    }
}

async fn rebuild_search_index_inner(
    metadata_store: Arc<la_storage::MetadataStore>,
    cas: Arc<la_storage::ContentAddressableStorage>,
    search_manager: Arc<la_search::SearchEngineManager>,
    cancellation_token: CancellationToken,
) -> std::result::Result<usize, String> {
    ensure_index_rebuild_active(&cancellation_token)?;
    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to enumerate imported files for indexing: {e}"))?;
    ensure_index_rebuild_active(&cancellation_token)?;

    tokio::task::spawn_blocking(move || -> std::result::Result<usize, String> {
        ensure_index_rebuild_active(&cancellation_token)?;
        let mut rebuild = search_manager
            .begin_rebuild()
            .map_err(|e| format!("Failed to begin search index rebuild: {e}"))?;
        ensure_index_rebuild_active(&cancellation_token)?;

        let mut indexed_lines = 0usize;

        for file in files {
            ensure_index_rebuild_active(&cancellation_token)?;
            let content = cas.read_content_sync(&file.sha256_hash).map_err(|e| {
                format!("Failed to read CAS content for {}: {e}", file.virtual_path)
            })?;
            let actual_hash = la_storage::ContentAddressableStorage::compute_hash(&content);
            if actual_hash != file.sha256_hash {
                return Err(format!(
                    "CAS hash mismatch for {}: expected {}, got {}",
                    file.virtual_path, file.sha256_hash, actual_hash
                ));
            }
            let (content_str, _) = decode_log_content(&content);
            let real_path = format!("cas://{}", file.sha256_hash);

            let mut line_buffer = Vec::with_capacity(1024);
            let mut start_line_number = 1usize;

            for line in content_str.lines() {
                line_buffer.push(line.to_string());

                if line_buffer.len() >= 1024 {
                    ensure_index_rebuild_active(&cancellation_token)?;
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
                ensure_index_rebuild_active(&cancellation_token)?;
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

        ensure_index_rebuild_active(&cancellation_token)?;
        rebuild
            .commit()
            .map_err(|e| format!("Failed to finalize rebuilt search index: {e}"))?;

        Ok(indexed_lines)
    })
    .await
    .map_err(|e| format!("Search index rebuild task panicked: {e}"))?
}

fn ensure_index_rebuild_active(
    cancellation_token: &CancellationToken,
) -> std::result::Result<(), String> {
    if cancellation_token.is_cancelled() {
        Err("Search index rebuild cancelled".to_string())
    } else {
        Ok(())
    }
}

fn map_index_rebuild_error(
    error: String,
    source_path: &std::path::Path,
    cancellation_token: &CancellationToken,
) -> AppError {
    if cancellation_token.is_cancelled() {
        AppError::Archive {
            category: ErrorCategory::CancellationRequested,
            _message: "Import cancelled while rebuilding the search index".to_string(),
            _path: Some(source_path.to_path_buf()),
        }
    } else {
        AppError::index_error(error)
    }
}

async fn finish_import_indexing<F>(
    workspace_id: String,
    cancellation_token: CancellationToken,
    rebuild: F,
) -> std::result::Result<usize, String>
where
    F: Future<Output = std::result::Result<usize, String>>,
{
    if cancellation_token.is_cancelled() {
        return Err("Search index rebuild cancelled".to_string());
    }

    let indexed_lines = rebuild.await?;
    tracing::info!(
        workspace_id = %workspace_id,
        indexed_lines,
        "Tantivy index rebuild completed"
    );

    Ok(indexed_lines)
}

async fn finalize_pending_file_stats(
    metadata_store: Arc<la_storage::MetadataStore>,
    cas: Arc<la_storage::ContentAddressableStorage>,
    cancellation_token: &CancellationToken,
) -> la_core::error::Result<usize> {
    let files = metadata_store.get_all_files().await?;
    let mut updated = 0usize;

    for file in files {
        if cancellation_token.is_cancelled() {
            return Err(AppError::Archive {
                category: ErrorCategory::CancellationRequested,
                _message: "Import cancelled while finalizing file statistics".to_string(),
                _path: None,
            });
        }
        if file.analysis_status == la_core::storage_types::AnalysisStatus::Ready {
            continue;
        }

        let content = cas.read_content(&file.sha256_hash).await?;
        let (min_ts, max_ts, level_mask) =
            tokio::task::spawn_blocking(move || compute_file_stats(&content))
                .await
                .map_err(|error| {
                    AppError::internal_error(format!(
                        "File statistics worker failed while finalizing import: {error}"
                    ))
                })?;
        metadata_store
            .update_file_ready(&file.virtual_path, min_ts, max_ts, level_mask)
            .await?;
        updated += 1;
    }

    Ok(updated)
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
        let _import_permit = acquire_import_permit(
            Arc::clone(&self.import_lock),
            &cancellation_token,
            source_path,
        )
        .await?;

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

        let finalized_stats = finalize_pending_file_stats(
            self.repo.metadata_store().clone(),
            Arc::clone(self.repo.cas()),
            &cancellation_token,
        )
        .await?;
        if finalized_stats > 0 {
            tracing::info!(
                workspace_id = %self.workspace_id,
                stats_updated = finalized_stats,
                "Pending file statistics finalized before indexing"
            );
        }

        let metadata_store = self.repo.metadata_store().clone();
        let cas = Arc::clone(self.repo.cas());
        let search_engine = Arc::clone(self.repo.search_engine());
        let workspace_id_bg = self.workspace_id.clone();
        let ct_bg = cancellation_token.clone();
        let ct_rebuild = cancellation_token.clone();
        finish_import_indexing(workspace_id_bg, ct_bg, async move {
            rebuild_search_index_inner(metadata_store, cas, search_engine, ct_rebuild).await
        })
        .await
        .map_err(|error| map_index_rebuild_error(error, source_path, &cancellation_token))?;

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
    use std::time::Duration;
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
    async fn pending_file_stats_are_finalized_before_indexing() {
        let (_workspace, _index, metadata_store, cas, _search_manager) = test_dependencies().await;
        let content = b"2026-07-30 12:00:00 ERROR finalized stats\n";
        let hash = cas.store_content(content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(hash, content.len()))
            .await
            .unwrap();

        let updated = finalize_pending_file_stats(
            Arc::clone(&metadata_store),
            cas,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

        assert_eq!(updated, 1);
        let file = metadata_store
            .get_file_by_virtual_path("logs/app.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(file.analysis_status, AnalysisStatus::Ready);
        assert!(file.min_timestamp.is_some());
        assert_eq!(file.level_mask, Some(1));
    }

    #[tokio::test]
    async fn import_indexing_does_not_finish_before_the_rebuild() {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let mut task = tokio::spawn(finish_import_indexing(
            "workspace-1".to_string(),
            CancellationToken::new(),
            async move {
                let _ = started_tx.send(());
                let _ = release_rx.await;
                Ok(7)
            },
        ));

        started_rx.await.unwrap();
        let premature = tokio::time::timeout(Duration::from_millis(20), &mut task).await;
        let _ = release_tx.send(());

        let (completed_early, result) = match premature {
            Ok(join_result) => (true, join_result.unwrap()),
            Err(_) => (false, task.await.unwrap()),
        };

        assert!(
            !completed_early,
            "import completion was reported before the index rebuild finished"
        );
        assert_eq!(result.unwrap(), 7);
    }

    #[tokio::test]
    async fn a_second_import_waits_for_the_existing_workspace_import() {
        let import_lock = Arc::new(tokio::sync::Mutex::new(()));
        let existing_import = Arc::clone(&import_lock).lock_owned().await;
        let waiting_lock = Arc::clone(&import_lock);
        let mut waiting_import = tokio::spawn(async move {
            acquire_import_permit(
                waiting_lock,
                &CancellationToken::new(),
                std::path::Path::new("/tmp/source.log"),
            )
            .await
        });

        let premature = tokio::time::timeout(Duration::from_millis(20), &mut waiting_import).await;
        drop(existing_import);

        let (completed_early, result) = match premature {
            Ok(join_result) => (true, join_result.unwrap()),
            Err(_) => (false, waiting_import.await.unwrap()),
        };

        assert!(
            !completed_early,
            "a second import entered the workspace while another import held the permit"
        );
        result.unwrap();
    }

    #[tokio::test]
    async fn a_waiting_import_can_be_cancelled() {
        let import_lock = Arc::new(tokio::sync::Mutex::new(()));
        let _existing_import = Arc::clone(&import_lock).lock_owned().await;
        let cancellation_token = CancellationToken::new();
        let waiting_token = cancellation_token.clone();
        let mut waiting_import = tokio::spawn(async move {
            acquire_import_permit(
                import_lock,
                &waiting_token,
                std::path::Path::new("/tmp/source.log"),
            )
            .await
        });

        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting_import)
                .await
                .is_err()
        );
        cancellation_token.cancel();

        let result = tokio::time::timeout(Duration::from_secs(1), waiting_import)
            .await
            .expect("cancelled import did not wake")
            .unwrap();
        let error = match result {
            Ok(_) => panic!("cancelled import unexpectedly acquired the permit"),
            Err(error) => error,
        };
        assert_eq!(error.category(), ErrorCategory::CancellationRequested);
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
                CancellationToken::new(),
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
            rebuild_search_index_inner(
                metadata_store,
                cas,
                Arc::clone(&search_manager),
                CancellationToken::new(),
            )
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
            CancellationToken::new(),
        )
        .await
        .unwrap();

        metadata_store
            .insert_file(&file_metadata("missing-cas-object".to_string(), 1))
            .await
            .unwrap();

        let error = rebuild_search_index_inner(
            metadata_store,
            cas,
            Arc::clone(&search_manager),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert!(error.contains("Failed to read CAS content"));

        search_manager.commit().unwrap();
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }

    #[tokio::test]
    async fn corrupted_cas_object_aborts_rebuild_before_commit() {
        let (_workspace, _index, metadata_store, cas, search_manager) = test_dependencies().await;

        let content = b"2024-01-01 00:00:00 INFO durable content\n";
        let hash = cas.store_content(content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(hash.clone(), content.len()))
            .await
            .unwrap();
        rebuild_search_index_inner(
            Arc::clone(&metadata_store),
            Arc::clone(&cas),
            Arc::clone(&search_manager),
            CancellationToken::new(),
        )
        .await
        .unwrap();

        tokio::fs::write(
            cas.get_object_path(&hash),
            b"2025-01-01 00:00:00 ERROR corrupted content\n",
        )
        .await
        .unwrap();

        let error = rebuild_search_index_inner(
            metadata_store,
            cas,
            Arc::clone(&search_manager),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert!(error.contains("hash mismatch"));

        search_manager.commit().unwrap();
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }

    #[tokio::test]
    async fn cancelled_rebuild_preserves_the_last_committed_index() {
        let (_workspace, _index, metadata_store, cas, search_manager) = test_dependencies().await;

        let old_content = b"2024-01-01 00:00:00 INFO durable content\n";
        let old_hash = cas.store_content(old_content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(old_hash, old_content.len()))
            .await
            .unwrap();
        rebuild_search_index_inner(
            Arc::clone(&metadata_store),
            Arc::clone(&cas),
            Arc::clone(&search_manager),
            CancellationToken::new(),
        )
        .await
        .unwrap();

        let new_content = b"2025-01-01 00:00:00 ERROR replacement content\n";
        let new_hash = cas.store_content(new_content).await.unwrap();
        metadata_store
            .insert_file(&file_metadata(new_hash, new_content.len()))
            .await
            .unwrap();

        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();
        let error = rebuild_search_index_inner(
            metadata_store,
            cas,
            Arc::clone(&search_manager),
            cancellation_token,
        )
        .await
        .unwrap_err();

        assert_eq!(error, "Search index rebuild cancelled");
        assert_eq!(
            search_manager.get_time_range().unwrap(),
            (1_704_067_200, 1_704_067_200, 1)
        );
    }

    #[test]
    fn cancelled_indexing_is_reported_as_import_cancellation() {
        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();

        let error = map_index_rebuild_error(
            "Search index rebuild cancelled".to_string(),
            std::path::Path::new("/tmp/source.log"),
            &cancellation_token,
        );

        assert_eq!(error.category(), ErrorCategory::CancellationRequested);
    }
}
