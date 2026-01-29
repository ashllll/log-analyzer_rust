use super::messages::{CoordinatorMessage, ExtractorMessage, ProgressUpdate, TaskId, TaskStatus};
use crate::error::{AppError, Result};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Context for a running task
struct TaskContext {
    status: TaskStatus,
    // Channel to the extractor actor handling this task
    extractor_tx: mpsc::UnboundedSender<ExtractorMessage>,
}

/// The Coordinator Actor manages task distribution and monitoring
pub struct CoordinatorActor {
    receiver: mpsc::UnboundedReceiver<CoordinatorMessage>,
    tasks: Arc<DashMap<TaskId, TaskContext>>,
    extractors: Vec<ExtractorHandle>,
}

struct ExtractorHandle {
    id: String,
    sender: mpsc::UnboundedSender<ExtractorMessage>,
    active_tasks: Arc<std::sync::atomic::AtomicUsize>,
}

impl CoordinatorActor {
    /// Create and start a new Coordinator Actor
    pub fn spawn(
        receiver: mpsc::UnboundedReceiver<CoordinatorMessage>,
    ) -> tokio::task::JoinHandle<()> {
        let mut actor = Self {
            receiver,
            tasks: Arc::new(DashMap::new()),
            extractors: Vec::new(), // Initialized via start-up or configuration
        };

        tokio::spawn(async move {
            actor.run().await;
        })
    }

    async fn run(&mut self) {
        info!("Coordinator Actor started");
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                CoordinatorMessage::ExtractRequest {
                    archive_path,
                    workspace_id,
                    policy,
                    response,
                } => {
                    let res = self
                        .handle_extract_request(archive_path, workspace_id, policy)
                        .await;
                    let _ = response.send(res);
                }
                CoordinatorMessage::CancelTask { task_id, response } => {
                    let res = self.handle_cancel_task(task_id).await;
                    let _ = response.send(res);
                }
                CoordinatorMessage::QueryStatus { task_id, response } => {
                    let status = self.tasks.get(&task_id).map(|t| t.status.clone());
                    let _ = response.send(status);
                }
                CoordinatorMessage::TaskCompleted { task_id, result } => {
                    self.handle_task_completed(task_id, result).await;
                }
            }
        }
    }

    async fn handle_extract_request(
        &mut self,
        archive_path: PathBuf,
        _workspace_id: String,
        policy: crate::archive::ExtractionPolicy,
    ) -> Result<TaskId> {
        let task_id = Uuid::new_v4().to_string();

        // Load balancing: pick extractor with least connections
        let extractor = self
            .pick_extractor()
            .ok_or_else(|| AppError::archive_error("No available extractor actors", None))?;

        // Create target dir (placeholder logic, actual target dir should be determined by context)
        let target_dir = archive_path
            .parent()
            .unwrap_or(&archive_path)
            .join(&task_id);

        let (progress_tx, _progress_rx) = watch::channel(ProgressUpdate {
            task_id: task_id.clone(),
            ..Default::default()
        });

        // Initialize task context
        self.tasks.insert(
            task_id.clone(),
            TaskContext {
                status: TaskStatus::Pending,
                extractor_tx: extractor.sender.clone(),
            },
        );

        // Send start message to extractor
        extractor
            .sender
            .send(ExtractorMessage::StartExtraction {
                task_id: task_id.clone(),
                archive_path,
                target_dir,
                policy,
                progress_tx,
            })
            .map_err(|e| {
                AppError::archive_error(format!("Failed to send to extractor: {}", e), None)
            })?;

        extractor
            .active_tasks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        info!(task_id = %task_id, "Task assigned to extractor {}", extractor.id);
        Ok(task_id)
    }

    async fn handle_cancel_task(&mut self, task_id: TaskId) -> Result<()> {
        if let Some(task) = self.tasks.get(&task_id) {
            task.extractor_tx
                .send(ExtractorMessage::Abort)
                .map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to send abort to extractor: {}", e),
                        None,
                    )
                })?;
            debug!(task_id = %task_id, "Abort message sent to extractor");
            Ok(())
        } else {
            Err(AppError::not_found(format!("Task {} not found", task_id)))
        }
    }

    async fn handle_task_completed(&mut self, task_id: TaskId, result: Result<()>) {
        if let Some((_, mut task)) = self.tasks.remove(&task_id) {
            match result {
                Ok(_) => {
                    task.status = TaskStatus::Completed;
                    info!(task_id = %task_id, "Task completed successfully");
                }
                Err(e) => {
                    task.status = TaskStatus::Failed {
                        error: e.to_string(),
                    };
                    error!(task_id = %task_id, error = %e, "Task failed");
                }
            }
            // Decement active tasks for the extractor (need mapping back to handle)
            // For now, we assume handles are static and we'd need a way to find the right one.
        }
    }

    fn pick_extractor(&self) -> Option<&ExtractorHandle> {
        self.extractors
            .iter()
            .min_by_key(|e| e.active_tasks.load(std::sync::atomic::Ordering::SeqCst))
    }

    /// Register a new extractor actor
    pub fn add_extractor(&mut self, id: String, sender: mpsc::UnboundedSender<ExtractorMessage>) {
        self.extractors.push(ExtractorHandle {
            id,
            sender,
            active_tasks: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
    }
}
