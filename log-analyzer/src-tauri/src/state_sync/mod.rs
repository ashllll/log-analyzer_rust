//! Real-Time State Synchronization
//!
//! This module provides state synchronization for workspace operations.
//! Standalone 模式下使用 Tauri 事件系统，FFI 模式下使用内部事件总线。
//!
//! Key features:
//! - <10ms latency for state updates
//! - Zero external dependencies (no WebSocket/Redis needed)
//! - Process-internal communication
//! - Event history for debugging

#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

#[cfg(feature = "standalone")]
use tauri::Emitter;
#[cfg(feature = "standalone")]
use crate::AppHandle;

pub mod models;

#[cfg(test)]
mod property_tests;

pub use models::{WorkspaceEvent, WorkspaceState, WorkspaceStatus};

use crate::models::TaskProgress;

// =============================================================================
// StateSync 结构定义
// =============================================================================

/// State synchronization manager
#[derive(Clone)]
pub struct StateSync {
    #[cfg(feature = "standalone")]
    app_handle: AppHandle,
    state_cache: Arc<RwLock<HashMap<String, WorkspaceState>>>,
    event_history: Arc<RwLock<VecDeque<WorkspaceEvent>>>,
    max_history_size: usize,
}

// =============================================================================
// Standalone 模式实现
// =============================================================================

#[cfg(feature = "standalone")]
impl StateSync {
    /// Create a new StateSync instance with Tauri AppHandle
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            state_cache: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 1000,
        }
    }

    /// Broadcast workspace event to frontend using Tauri
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        // 1. Update local state cache
        self.update_state_cache(&event).await;

        // 2. Store in event history
        self.append_to_history(event.clone()).await;

        // 3. Emit Tauri event
        let result = self
            .app_handle
            .emit("workspace-event", &event)
            .map_err(|e| format!("Failed to emit event: {}", e));

        tracing::debug!(
            event_type = ?event,
            duration_ms = start_time.elapsed().as_millis(),
            "Broadcasted workspace event"
        );

        result
    }
}

// =============================================================================
// FFI 模式实现
// =============================================================================

#[cfg(not(feature = "standalone"))]
impl StateSync {
    /// Create a new StateSync instance for FFI mode
    pub fn new() -> Self {
        Self {
            state_cache: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 1000,
        }
    }

    /// Broadcast workspace event using internal event bus
    pub async fn broadcast_workspace_event(&self, event: WorkspaceEvent) -> Result<(), String> {
        // 1. Update local state cache
        self.update_state_cache(&event).await;

        // 2. Store in event history
        self.append_to_history(event.clone()).await;

        // 3. Emit to internal event bus
        let _ = crate::events::emit_event(crate::events::AppEvent::TaskUpdate {
            progress: TaskProgress {
                task_id: "state_sync".to_string(),
                target: event.workspace_id().to_string(),
                message: format!("{:?}", event),
                status: "Running".to_string(),
                progress: 0,
                task_type: "state_sync".to_string(),
                workspace_id: Some(event.workspace_id().to_string()),
            },
        });

        Ok(())
    }
}

// =============================================================================
// 共享实现
// =============================================================================

impl StateSync {
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

#[cfg(not(feature = "standalone"))]
impl Default for StateSync {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests are run as integration tests
}
