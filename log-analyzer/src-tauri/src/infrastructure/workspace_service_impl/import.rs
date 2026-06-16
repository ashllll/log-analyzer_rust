use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::application::workspace_service::{ImportOptions, ImportResult, ImportService};
use crate::utils::encoding::decode_log_content;
use la_archive::processor::process_path_with_cas;
use la_core::error::AppError;
use la_core::traits::AppConfigProvider;

use super::WorkspaceServiceImpl;
const SEARCH_INDEX_COMMIT_EVERY_FILES: usize = 25;
const FALLBACK_STATS_DELAY_SECS: u64 = 5;

pub(super) fn compute_file_stats(content: &[u8]) -> (Option<i64>, Option<i64>, Option<u8>) {
    crate::utils::log_stats::compute_file_stats(content)
}

async fn rebuild_search_index_inner(
    metadata_store: Arc<la_storage::MetadataStore>,
    cas: Arc<la_storage::ContentAddressableStorage>,
    search_manager: Arc<la_search::SearchEngineManager>,
) -> std::result::Result<usize, String> {
    let index_empty = match search_manager.get_time_range() {
        Ok((_, _, count)) => count == 0,
        Err(_) => true,
    };

    if !index_empty {
        tracing::info!("Skipping index rebuild: Tantivy index already has documents");
        return Ok(0);
    }

    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to enumerate imported files for indexing: {e}"))?;

    tokio::task::spawn_blocking(move || -> std::result::Result<usize, String> {
        search_manager
            .clear_index()
            .map_err(|e| format!("Failed to clear search index before rebuild: {e}"))?;

        let mut indexed_lines = 0usize;

        for (file_index, file) in files.into_iter().enumerate() {
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
                    for entry in &entries {
                        search_manager
                            .add_document(entry)
                            .map_err(|e| format!("Failed to add indexed document: {e}"))?;
                    }
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
                for entry in &entries {
                    search_manager
                        .add_document(entry)
                        .map_err(|e| format!("Failed to add indexed document: {e}"))?;
                }
                indexed_lines += entries.len();
            }

            if (file_index + 1) % SEARCH_INDEX_COMMIT_EVERY_FILES == 0 {
                search_manager
                    .commit()
                    .map_err(|e| format!("Failed to commit rebuilt search index: {e}"))?;
            }
        }

        search_manager
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

        process_path_with_cas(
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
        )
        .await
        .map_err(|e| {
            AppError::archive_error(
                format!("Import failed: {e}"),
                Some(source_path.to_path_buf()),
            )
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

        let files_imported = self.repo.metadata_store().count_files().await.unwrap_or(0) as usize;

        Ok(ImportResult {
            root_name,
            files_imported,
        })
    }
}
