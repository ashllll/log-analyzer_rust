//! Real-Time State Synchronization using Tauri Events
//!
//! This module provides state synchronization for workspace operations using
//! Tauri's built-in event system, which is the recommended approach for desktop applications.
//!
//! Key features:
//! - <10ms latency for state updates
//! - Zero external dependencies (no WebSocket/Redis needed)
//! - Process-internal communication
//! - Event history for debugging

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

pub mod models;

#[cfg(test)]
mod property_tests;

pub use models::{WorkspaceEvent, WorkspaceState, WorkspaceStatus};

/// State synchronization manager using Tauri Events
#[derive(Clone)]
pub struct StateSync {
    app_handle: AppHandle,
    state_cache: Arc<RwLock<HashMap<String, WorkspaceState>>>,
    event_history: Arc<RwLock<VecDeque<WorkspaceEvent>>>,
    max_history_size: usize,
}

impl StateSync {
    /// Create a new StateSync instance
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            state_cache: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 1000, // Keep last 1000 events
        }
    }

    /// Create a dummy StateSync instance for testing
    #[cfg(feature = "test")]
    pub fn new_dummy() -> Self {
        use tauri::test::MockRuntime;
        let mock_app = tauri::Builder::<tauri::Wry>::new("dummy")
            .build(tauri::test::mock_context())
            .unwrap();
        Self::new(mock_app.handle())
    }

    /// Create a dummy StateSync instance for testing (when test feature is not available)
    #[cfg(not(feature = "test"))]
    pub fn new_dummy() -> Self {
        // Fallback: panic with a helpful message
        panic!(
            "new_dummy() requires the 'test' feature. Enable it with: cargo test --features test"
        );
    }

    /// Broadcast workspace event to frontend
    ///
    /// Uses Tauri's event system for <10ms latency
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        // 1. Update local state cache
        self.update_state_cache(&event).await;

        // 2. Store in event history for debugging
        self.append_to_history(event.clone()).await;

        // 3. Emit Tauri event to frontend (<10ms latency)
        let emit_result = self
            .app_handle
            .emit("workspace-event", &event)
            .map_err(|e| format!("Failed to emit event: {}", e));

        let total_duration = start_time.elapsed();

        tracing::debug!(
            event_type = ?event,
            duration_ms = total_duration.as_millis(),
            "Broadcasted workspace event"
        );

        emit_result
    }

    /// Update workspace state in cache
    async fn update_state_cache(&self, event: &WorkspaceEvent) {
        let mut cache = self.state_cache.write().await;

        match event {
            WorkspaceEvent::StatusChanged {
                workspace_id,
                status,
            } => {
                cache
                    .entry(workspace_id.clone())
                    .and_modify(|state| {
                        state.status = status.clone();
                        state.last_updated = SystemTime::now();
                    })
                    .or_insert_with(|| WorkspaceState {
                        id: workspace_id.clone(),
                        status: status.clone(),
                        progress: 0.0,
                        last_updated: SystemTime::now(),
                        active_tasks: vec![],
                        error_count: 0,
                        processed_files: 0,
                        total_files: 0,
                    });
            }
            WorkspaceEvent::ProgressUpdate {
                workspace_id,
                progress,
            } => {
                cache.entry(workspace_id.clone()).and_modify(|state| {
                    state.progress = *progress;
                    state.last_updated = SystemTime::now();
                });
            }
            WorkspaceEvent::TaskCompleted {
                workspace_id,
                task_id: _,
            } => {
                cache.entry(workspace_id.clone()).and_modify(|state| {
                    state.last_updated = SystemTime::now();
                });
            }
            WorkspaceEvent::Error {
                workspace_id,
                error: _,
            } => {
                cache.entry(workspace_id.clone()).and_modify(|state| {
                    state.error_count += 1;
                    state.last_updated = SystemTime::now();
                });
            }
        }
    }

    /// Append event to history
    async fn append_to_history(&self, event: WorkspaceEvent) {
        let mut history = self.event_history.write().await;

        history.push_back(event);

        // Maintain max history size
        while history.len() > self.max_history_size {
            history.pop_front();
        }
    }

    /// Get current workspace state
    pub async fn get_workspace_state(&self, workspace_id: &str) -> Option<WorkspaceState> {
        let cache = self.state_cache.read().await;
        cache.get(workspace_id).cloned()
    }

    /// Get event history for a workspace
    pub async fn get_event_history(&self, workspace_id: &str, limit: usize) -> Vec<WorkspaceEvent> {
        let history = self.event_history.read().await;

        history
            .iter()
            .filter(|e| e.workspace_id() == workspace_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get all event history
    pub async fn get_all_event_history(&self, limit: usize) -> Vec<WorkspaceEvent> {
        let history = self.event_history.read().await;

        history.iter().rev().take(limit).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    // Note: Tauri tests require a running Tauri application context
    // These tests are placeholders and should be run as integration tests
}
