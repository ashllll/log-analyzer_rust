//! State Synchronization Module
//!
//! Real-time state synchronization system with WebSocket and Redis integration.
//! Provides reliable event publishing, state management, and network resilience.

pub mod network_resilience;
pub mod property_tests;
pub mod redis_publisher;
pub mod redis_publisher_property_tests;
pub mod state_sync_manager;
pub mod websocket_manager;

// Re-export main types
pub use network_resilience::{NetworkResilienceConfig, NetworkResilienceManager, ResilienceStats};
pub use redis_publisher::{RedisConfig, RedisPublisher};
pub use state_sync_manager::{StateSyncConfig, StateSyncManager, SyncStats, WorkspaceState};
pub use websocket_manager::{
    AuthRequest, AuthResponse, AuthToken, AuthValidator, ConnectionStats, DefaultAuthValidator,
    WebSocketConfig, WebSocketManager, WebSocketMessage,
};

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// User ID for WebSocket connections
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UserId(pub String);

/// Event ID for ordering and gap detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventId(pub String);

/// Workspace status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceStatus {
    Idle,
    Processing {
        started_at: SystemTime,
    },
    Completed {
        duration: std::time::Duration,
    },
    Failed {
        error: String,
        failed_at: SystemTime,
    },
    Cancelled {
        cancelled_at: SystemTime,
    },
}

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub progress: f64,
    pub started_at: SystemTime,
    pub estimated_completion: Option<SystemTime>,
}

/// Workspace events for state synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    StatusChanged {
        workspace_id: String,
        status: WorkspaceStatus,
        timestamp: SystemTime,
    },
    ProgressUpdate {
        workspace_id: String,
        progress: f64,
        timestamp: SystemTime,
    },
    TaskCompleted {
        workspace_id: String,
        task_id: String,
        timestamp: SystemTime,
    },
    Error {
        workspace_id: String,
        error: String,
        timestamp: SystemTime,
    },
    WorkspaceDeleted {
        workspace_id: String,
        timestamp: SystemTime,
    },
    WorkspaceCreated {
        workspace_id: String,
        timestamp: SystemTime,
    },
}

/// Synchronization errors
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),
}

/// Synchronization result type
pub type SyncResult<T> = Result<T, SyncError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let user_id = UserId("test-user".to_string());
        assert_eq!(user_id.0, "test-user");
    }

    #[test]
    fn test_event_id_creation() {
        let event_id = EventId("event-123".to_string());
        assert_eq!(event_id.0, "event-123");
    }

    #[test]
    fn test_workspace_status_serialization() {
        let status = WorkspaceStatus::Idle;
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: WorkspaceStatus = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            WorkspaceStatus::Idle => {
                // Success
            }
            _ => panic!("Deserialization failed"),
        }
    }
}
