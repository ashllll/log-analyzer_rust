//! State synchronization data models

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Workspace event types
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum WorkspaceEvent {
    StatusChanged {
        workspace_id: String,
        status: WorkspaceStatus,
    },
    ProgressUpdate {
        workspace_id: String,
        progress: f64,
    },
    TaskCompleted {
        workspace_id: String,
        task_id: String,
    },
    Error {
        workspace_id: String,
        error: String,
    },
}

impl WorkspaceEvent {
    /// Get workspace ID from event
    pub fn workspace_id(&self) -> &str {
        match self {
            WorkspaceEvent::StatusChanged { workspace_id, .. } => workspace_id,
            WorkspaceEvent::ProgressUpdate { workspace_id, .. } => workspace_id,
            WorkspaceEvent::TaskCompleted { workspace_id, .. } => workspace_id,
            WorkspaceEvent::Error { workspace_id, .. } => workspace_id,
        }
    }
}

/// Workspace status
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "status")]
pub enum WorkspaceStatus {
    Idle,
    Processing {
        #[serde(with = "system_time_serde")]
        started_at: SystemTime,
    },
    Completed {
        #[serde(with = "duration_serde")]
        duration: Duration,
    },
    Failed {
        error: String,
        #[serde(with = "system_time_serde")]
        failed_at: SystemTime,
    },
    Cancelled {
        #[serde(with = "system_time_serde")]
        cancelled_at: SystemTime,
    },
}

/// Workspace state
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceState {
    pub id: String,
    pub status: WorkspaceStatus,
    pub progress: f64,
    #[serde(with = "system_time_serde")]
    pub last_updated: SystemTime,
    pub active_tasks: Vec<TaskInfo>,
    pub error_count: u32,
    pub processed_files: u32,
    pub total_files: u32,
}

/// Task information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskInfo {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub progress: f64,
    #[serde(with = "system_time_serde")]
    pub started_at: SystemTime,
    #[serde(with = "option_system_time_serde")]
    pub estimated_completion: Option<SystemTime>,
}

/// Task type
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskType {
    Indexing,
    Searching,
    Extraction,
    Analysis,
}

/// Task status
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// Serde helpers for SystemTime
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

mod option_system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let duration = t
                    .duration_since(UNIX_EPOCH)
                    .map_err(serde::ser::Error::custom)?;
                Some(duration.as_secs()).serialize(serializer)
            }
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs_opt = Option::<u64>::deserialize(deserializer)?;
        Ok(secs_opt.map(|secs| UNIX_EPOCH + std::time::Duration::from_secs(secs)))
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
