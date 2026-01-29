use crate::archive::ExtractionPolicy;
use crate::error::Result;
use std::path::PathBuf;
use tokio::sync::{oneshot, watch};

/// Task ID for identifying extraction jobs
pub type TaskId = String;

/// Message types for the Coordinator Actor
#[derive(Debug)]
pub enum CoordinatorMessage {
    /// Request to extract an archive
    ExtractRequest {
        archive_path: PathBuf,
        workspace_id: String,
        policy: ExtractionPolicy,
        response: oneshot::Sender<Result<TaskId>>,
    },
    /// Request to cancel a running task
    CancelTask {
        task_id: TaskId,
        response: oneshot::Sender<Result<()>>,
    },
    /// Request the current status of a task
    QueryStatus {
        task_id: TaskId,
        response: oneshot::Sender<Option<TaskStatus>>,
    },
    /// Internal: Notify coordinator that a task has finished
    TaskCompleted { task_id: TaskId, result: Result<()> },
}

/// Message types for Extractor Actors
#[derive(Debug)]
pub enum ExtractorMessage {
    /// Start extraction task
    StartExtraction {
        task_id: TaskId,
        archive_path: PathBuf,
        target_dir: PathBuf,
        policy: ExtractionPolicy,
        progress_tx: watch::Sender<ProgressUpdate>,
    },
    /// Stop current extraction
    Abort,
    /// Heartbeat for health monitoring
    Ping { response: oneshot::Sender<()> },
}

/// Task status information
#[derive(Debug, Clone, serde::Serialize)]
pub enum TaskStatus {
    Pending,
    Processing { progress: ProgressUpdate },
    Completed,
    Failed { error: String },
    Cancelled,
}

/// Real-time progress update
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ProgressUpdate {
    pub task_id: TaskId,
    pub files_processed: usize,
    pub total_files_estimated: Option<usize>,
    pub bytes_processed: u64,
    pub total_bytes_estimated: Option<u64>,
    pub current_file: Option<String>,
    pub depth_level: usize,
}

/// Supervisor messages
#[derive(Debug)]
pub enum SupervisorMessage {
    /// Monitor a new actor
    WatchActor {
        actor_id: String,
        // In a real system, we might pass a handle or channel here
    },
    /// Report an actor failure
    ActorPanicked { actor_id: String, reason: String },
}
