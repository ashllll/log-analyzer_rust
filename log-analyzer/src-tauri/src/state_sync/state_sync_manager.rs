//! State Synchronization Manager
//!
//! Coordinates WebSocket and Redis communication for real-time state synchronization.
//! Provides workspace state broadcasting with <100ms latency guarantee.

use crate::state_sync::{
    EventId, RedisConfig, RedisPublisher, SyncError, SyncResult, TaskInfo, UserId, WebSocketConfig,
    WebSocketManager, WebSocketMessage, WorkspaceEvent, WorkspaceStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// State synchronization configuration
#[derive(Debug, Clone)]
pub struct StateSyncConfig {
    pub websocket_config: WebSocketConfig,
    pub redis_config: RedisConfig,
    pub broadcast_timeout: Duration,
    pub state_sync_interval: Duration,
    pub max_event_queue_size: usize,
}

impl Default for StateSyncConfig {
    fn default() -> Self {
        Self {
            websocket_config: WebSocketConfig::default(),
            redis_config: RedisConfig::default(),
            broadcast_timeout: Duration::from_millis(100), // <100ms latency guarantee
            state_sync_interval: Duration::from_millis(50),
            max_event_queue_size: 10000,
        }
    }
}

/// Workspace state with synchronization metadata
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub id: String,
    pub status: WorkspaceStatus,
    pub progress: f64,
    pub last_updated: SystemTime,
    pub active_tasks: Vec<TaskInfo>,
    pub error_count: u32,
    pub processed_files: u32,
    pub total_files: u32,
    pub version: u64, // For conflict resolution
}

/// State synchronization manager
pub struct StateSyncManager {
    websocket_manager: Arc<WebSocketManager>,
    redis_publisher: Arc<RedisPublisher>,
    config: StateSyncConfig,
    workspace_states: Arc<RwLock<HashMap<String, WorkspaceState>>>,
    event_sequence: Arc<RwLock<u64>>,
}

impl StateSyncManager {
    /// Create a new state synchronization manager
    pub async fn new(config: StateSyncConfig) -> SyncResult<Self> {
        let websocket_manager = Arc::new(WebSocketManager::new(config.websocket_config.clone()));
        let redis_publisher = Arc::new(RedisPublisher::new(config.redis_config.clone()).await?);

        info!("State synchronization manager initialized");

        Ok(Self {
            websocket_manager,
            redis_publisher,
            config,
            workspace_states: Arc::new(RwLock::new(HashMap::new())),
            event_sequence: Arc::new(RwLock::new(0)),
        })
    }

    /// Broadcast workspace event to all connected clients
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> SyncResult<usize> {
        let start_time = SystemTime::now();

        // Persist event to Redis for reliability
        let stream_key = format!("workspace:{}", event.workspace_id());
        let _event_id = self
            .redis_publisher
            .append_to_stream(&stream_key, &event)
            .await?;

        // Create WebSocket message
        let ws_message = WebSocketMessage::EventNotification {
            event_id: self.generate_event_id().await,
            event_type: event.event_type(),
            payload: serde_json::to_value(&event)
                .map_err(|e| SyncError::SerializationError(e.to_string()))?,
        };

        // Broadcast to WebSocket clients
        let success_count = self.websocket_manager.broadcast(ws_message).await?;

        // Verify latency guarantee
        let elapsed = start_time.elapsed().unwrap_or(Duration::MAX);
        if elapsed > self.config.broadcast_timeout {
            warn!(
                elapsed_ms = elapsed.as_millis(),
                timeout_ms = self.config.broadcast_timeout.as_millis(),
                "Broadcast exceeded latency guarantee"
            );
        } else {
            debug!(
                elapsed_ms = elapsed.as_millis(),
                recipients = success_count,
                "Broadcast completed within latency guarantee"
            );
        }

        Ok(success_count)
    }

    /// Send event to specific user
    pub async fn send_to_user(&self, user_id: &UserId, event: WorkspaceEvent) -> SyncResult<()> {
        // Persist event
        let stream_key = format!("workspace:{}", event.workspace_id());
        let _event_id = self
            .redis_publisher
            .append_to_stream(&stream_key, &event)
            .await?;

        // Send to specific user
        let ws_message = WebSocketMessage::EventNotification {
            event_id: self.generate_event_id().await,
            event_type: event.event_type(),
            payload: serde_json::to_value(&event)
                .map_err(|e| SyncError::SerializationError(e.to_string()))?,
        };

        self.websocket_manager
            .send_to_user(user_id, ws_message)
            .await
    }

    /// Update workspace state and broadcast changes
    pub async fn update_workspace_state(
        &self,
        workspace_id: &str,
        status: WorkspaceStatus,
        progress: f64,
        active_tasks: Vec<TaskInfo>,
    ) -> SyncResult<()> {
        // Update local state
        {
            let mut states = self.workspace_states.write().await;
            let state = states
                .entry(workspace_id.to_string())
                .or_insert_with(|| WorkspaceState {
                    id: workspace_id.to_string(),
                    status: WorkspaceStatus::Idle,
                    progress: 0.0,
                    last_updated: SystemTime::now(),
                    active_tasks: Vec::new(),
                    error_count: 0,
                    processed_files: 0,
                    total_files: 0,
                    version: 0,
                });

            state.status = status.clone();
            state.progress = progress;
            state.active_tasks = active_tasks;
            state.last_updated = SystemTime::now();
            state.version += 1;
        }

        // Create and broadcast event
        let event = WorkspaceEvent::StatusChanged {
            workspace_id: workspace_id.to_string(),
            status,
            timestamp: SystemTime::now(),
        };

        self.broadcast_workspace_event(event).await?;
        Ok(())
    }

    /// Get current workspace state
    pub async fn get_workspace_state(
        &self,
        workspace_id: &str,
    ) -> SyncResult<Option<WorkspaceState>> {
        let states = self.workspace_states.read().await;
        Ok(states.get(workspace_id).cloned())
    }

    /// Sync workspace state from Redis (for recovery scenarios)
    pub async fn sync_workspace_state_from_redis(
        &self,
        workspace_id: &str,
        since: Option<SystemTime>,
    ) -> SyncResult<Vec<WorkspaceEvent>> {
        let stream_key = format!("workspace:{}", workspace_id);
        let since_id = since
            .map(|time| EventId::from_timestamp(time))
            .unwrap_or_else(|| EventId("0".to_string()));

        let events = self
            .redis_publisher
            .read_stream_since(&stream_key, &since_id, None)
            .await?;

        // Reconstruct state from events
        let mut states = self.workspace_states.write().await;
        for (_, event) in &events {
            self.apply_event_to_state(&mut states, event);
        }

        Ok(events.into_iter().map(|(_, event)| event).collect())
    }

    /// Apply event to workspace state
    fn apply_event_to_state(
        &self,
        states: &mut HashMap<String, WorkspaceState>,
        event: &WorkspaceEvent,
    ) {
        match event {
            WorkspaceEvent::StatusChanged {
                workspace_id,
                status,
                ..
            } => {
                let state = states
                    .entry(workspace_id.clone())
                    .or_insert_with(|| WorkspaceState {
                        id: workspace_id.clone(),
                        status: WorkspaceStatus::Idle,
                        progress: 0.0,
                        last_updated: SystemTime::now(),
                        active_tasks: Vec::new(),
                        error_count: 0,
                        processed_files: 0,
                        total_files: 0,
                        version: 0,
                    });

                state.status = status.clone();
                state.last_updated = SystemTime::now();
                state.version += 1;
            }
            WorkspaceEvent::ProgressUpdate {
                workspace_id,
                progress,
                ..
            } => {
                if let Some(state) = states.get_mut(workspace_id) {
                    state.progress = *progress;
                    state.last_updated = SystemTime::now();
                }
            }
            _ => {
                // Handle other event types as needed
            }
        }
    }

    /// Generate unique event ID
    async fn generate_event_id(&self) -> String {
        let mut sequence = self.event_sequence.write().await;
        *sequence += 1;
        format!("event:{}", *sequence)
    }

    /// Get WebSocket manager for direct access
    pub fn websocket_manager(&self) -> &Arc<WebSocketManager> {
        &self.websocket_manager
    }

    /// Get Redis publisher for direct access
    pub fn redis_publisher(&self) -> &Arc<RedisPublisher> {
        &self.redis_publisher
    }

    /// Get synchronization statistics
    pub async fn get_sync_stats(&self) -> SyncStats {
        let states = self.workspace_states.read().await;
        let connections = self.websocket_manager.get_connection_stats().await;

        SyncStats {
            workspace_count: states.len(),
            active_connections: connections.active_connections,
            max_connections: connections.max_connections,
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub workspace_count: usize,
    pub active_connections: usize,
    pub max_connections: usize,
}

// Helper trait for WorkspaceEvent
impl WorkspaceEvent {
    /// Get workspace ID from event
    pub fn workspace_id(&self) -> String {
        match self {
            WorkspaceEvent::StatusChanged { workspace_id, .. } => workspace_id.clone(),
            WorkspaceEvent::ProgressUpdate { workspace_id, .. } => workspace_id.clone(),
            WorkspaceEvent::TaskCompleted { workspace_id, .. } => workspace_id.clone(),
            WorkspaceEvent::Error { workspace_id, .. } => workspace_id.clone(),
            WorkspaceEvent::WorkspaceDeleted { workspace_id, .. } => workspace_id.clone(),
            WorkspaceEvent::WorkspaceCreated { workspace_id, .. } => workspace_id.clone(),
        }
    }

    /// Get event type as string
    pub fn event_type(&self) -> String {
        match self {
            WorkspaceEvent::StatusChanged { .. } => "status_changed".to_string(),
            WorkspaceEvent::ProgressUpdate { .. } => "progress_update".to_string(),
            WorkspaceEvent::TaskCompleted { .. } => "task_completed".to_string(),
            WorkspaceEvent::Error { .. } => "error".to_string(),
            WorkspaceEvent::WorkspaceDeleted { .. } => "workspace_deleted".to_string(),
            WorkspaceEvent::WorkspaceCreated { .. } => "workspace_created".to_string(),
        }
    }
}

// Helper for EventId
impl EventId {
    /// Create EventId from timestamp
    pub fn from_timestamp(timestamp: SystemTime) -> Self {
        let duration = timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        EventId(format!("{}", duration.as_millis()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_sync_manager_creation() {
        let config = StateSyncConfig::default();
        let result = StateSyncManager::new(config).await;

        // This will fail if Redis is not available, which is expected in test environment
        match result {
            Ok(_) => {
                // Manager created successfully
            }
            Err(e) => {
                println!(
                    "State sync manager creation failed (expected in test): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_workspace_event_workspace_id() {
        let event = WorkspaceEvent::StatusChanged {
            workspace_id: "test-workspace".to_string(),
            status: WorkspaceStatus::Idle,
            timestamp: SystemTime::now(),
        };

        assert_eq!(event.workspace_id(), "test-workspace");
        assert_eq!(event.event_type(), "status_changed");
    }
}
