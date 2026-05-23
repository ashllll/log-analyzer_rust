//! WatchUseCase — application-layer file watch orchestration.
//!
//! Encapsulates the file watching flow: start watching a workspace directory
//! for file changes and stop when no longer needed.

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::error::Result;

/// Result type for start-watch operations.
#[derive(Debug, Clone)]
pub struct WatchStartResult {
    pub workspace_id: String,
    pub watched_path: String,
}

/// Result type for stop-watch operations.
#[derive(Debug, Clone)]
pub struct WatchStopResult {
    pub workspace_id: String,
}

/// Application use case for workspace file watching.
pub struct WatchUseCase<E>
where
    E: EventPublisher + 'static,
{
    events: Arc<E>,
}

impl<E> WatchUseCase<E>
where
    E: EventPublisher,
{
    pub fn new(events: Arc<E>) -> Self {
        Self { events }
    }

    /// Start watching a workspace directory for file changes.
    ///
    /// TODO(p3): Extract core watch logic from commands/watch.rs.
    /// The watch flow involves:
    /// 1. Validate workspace and path
    /// 2. Create notify watcher with event channel
    /// 3. Spawn background thread to process events
    /// 4. Track file offsets and line counts for incremental reads
    /// 5. Append new log entries to workspace index via CAS
    ///
    /// Current implementation lives in commands/watch.rs:start_watch_impl().
    /// Migration blocked on: extracting Tauri-specific concerns (AppHandle emit,
    /// CAS write, metadata store) behind domain traits.
    pub async fn start(&self, _workspace_id: &str, _path: &str) -> Result<WatchStartResult> {
        self.events.emit_search_start("watch-start-stub").await;
        Ok(WatchStartResult {
            workspace_id: _workspace_id.to_string(),
            watched_path: _path.to_string(),
        })
    }

    /// Stop watching a workspace.
    pub async fn stop(&self, _workspace_id: &str) -> Result<WatchStopResult> {
        Ok(WatchStopResult {
            workspace_id: _workspace_id.to_string(),
        })
    }

    /// Check if a workspace is currently being watched.
    pub fn is_watching(&self, _workspace_id: &str) -> bool {
        // TODO(p3): Query watcher state from repository
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;

    struct StubEvents {
        last_event: Mutex<String>,
    }

    #[async_trait]
    impl EventPublisher for StubEvents {
        async fn emit_search_start(&self, id: &str) {
            *self.last_event.lock() = format!("search_start:{}", id);
        }
        async fn emit_search_progress(&self, _id: &str, _c: usize) {}
        async fn emit_search_complete(
            &self,
            _id: &str,
            _s: la_core::domain::event::SearchSummary,
        ) {
        }
        async fn emit_search_error(&self, _id: &str, _e: &str) {}
        async fn emit_search_cancelled(&self, _id: &str) {}
        async fn emit_search_timeout(&self, _id: &str) {}
    }

    #[tokio::test]
    async fn watch_use_case_start_stop() {
        let events = Arc::new(StubEvents {
            last_event: Mutex::new(String::new()),
        });
        let use_case = WatchUseCase::new(events);

        let result = use_case.start("ws-1", "/tmp/logs").await.unwrap();
        assert_eq!(result.workspace_id, "ws-1");
        assert_eq!(result.watched_path, "/tmp/logs");

        let result = use_case.stop("ws-1").await.unwrap();
        assert_eq!(result.workspace_id, "ws-1");

        assert!(!use_case.is_watching("ws-1"));
    }
}
