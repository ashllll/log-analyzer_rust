use super::messages::{ExtractorMessage, ProgressUpdate, TaskId};
use crate::archive::{ArchiveManager, ExtractionPolicy};
use crate::error::{AppError, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

/// The Extractor Actor performs the actual extraction work
pub struct ExtractorActor {
    id: String,
    receiver: mpsc::UnboundedReceiver<ExtractorMessage>,
    archive_manager: Arc<ArchiveManager>,
}

impl ExtractorActor {
    /// Create and start a new Extractor Actor
    pub fn spawn(
        id: String,
        receiver: mpsc::UnboundedReceiver<ExtractorMessage>,
        archive_manager: Arc<ArchiveManager>,
    ) -> tokio::task::JoinHandle<()> {
        let mut actor = Self {
            id,
            receiver,
            archive_manager,
        };

        tokio::spawn(async move {
            actor.run().await;
        })
    }

    async fn run(&mut self) {
        info!("Extractor Actor {} started", self.id);
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ExtractorMessage::StartExtraction {
                    task_id,
                    archive_path,
                    target_dir,
                    policy,
                    progress_tx,
                } => {
                    info!(actor_id = %self.id, task_id = %task_id, "Starting extraction");
                    let res = self
                        .handle_extraction(task_id, &archive_path, &target_dir, policy, progress_tx)
                        .await;

                    if let Err(e) = res {
                        error!(actor_id = %self.id, error = %e, "Extraction failed");
                    }
                }
                ExtractorMessage::Abort => {
                    warn!(actor_id = %self.id, "Received abort message, but cancellation is not yet fully implemented in handlers");
                }
                ExtractorMessage::Ping { response } => {
                    let _ = response.send(());
                }
            }
        }
    }

    async fn handle_extraction(
        &self,
        task_id: TaskId,
        source: &Path,
        target_dir: &Path,
        policy: ExtractionPolicy,
        progress_tx: watch::Sender<ProgressUpdate>,
    ) -> Result<()> {
        // Find the appropriate handler
        let handler = self.archive_manager.find_handler(source).ok_or_else(|| {
            AppError::archive_error(
                format!("Unsupported archive format: {:?}", source.extension()),
                Some(source.to_path_buf()),
            )
        })?;

        // In a real streaming implementation, we'd use a more granular progress feedback.
        // For the "temporary integration", we call the existing async method.

        debug!(task_id = %task_id, "Calling archive handler");

        // Initial progress update
        let _ = progress_tx.send(ProgressUpdate {
            task_id: task_id.clone(),
            current_file: Some("Initializing...".to_string()),
            ..Default::default()
        });

        // We wrap the potentially blocking or long-running async call
        // Note: Existing extract_with_limits is async, but might block threads if it uses std::fs internally.
        let summary = handler
            .extract_with_limits(
                source,
                target_dir,
                policy.max_file_size,
                policy.max_total_size,
                1000000, // Large file count limit for stress testing
            )
            .await?;

        // Final progress update
        let _ = progress_tx.send(ProgressUpdate {
            task_id: task_id.clone(),
            files_processed: summary.files_extracted,
            bytes_processed: summary.total_size,
            current_file: Some("Completed".to_string()),
            ..Default::default()
        });

        info!(task_id = %task_id, files = summary.files_extracted, "Extraction summary generated");
        Ok(())
    }
}
