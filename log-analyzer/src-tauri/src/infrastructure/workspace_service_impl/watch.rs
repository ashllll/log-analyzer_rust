use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use notify::Watcher;
use tracing::error;

use crate::application::watch::{WatchEvent, WatchEventKind};
use crate::application::workspace_service::WatchService;
use crate::infrastructure::watcher_runner::WatcherRunner;
use crate::services::file_watcher::WatcherState;
use la_core::error::{AppError, Result};

use super::WorkspaceServiceImpl;
#[async_trait]
impl WatchService for WorkspaceServiceImpl {
    async fn start_watch(&self, watch_path: &str) -> Result<()> {
        let watch_path_buf = PathBuf::from(watch_path);
        if !watch_path_buf.exists() {
            return Err(AppError::validation_error(format!(
                "Path does not exist: {watch_path}"
            )));
        }

        {
            let state = self.watcher_state.lock();
            if state.is_some() {
                return Err(AppError::validation_error(
                    "Workspace is already being watched".to_string(),
                ));
            }
        }

        let (tx, notify_rx) =
            crossbeam::channel::unbounded::<std::result::Result<notify::Event, notify::Error>>();
        let (watch_tx, rx) = crossbeam::channel::unbounded::<WatchEvent>();

        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|e| AppError::io_error(format!("Failed to create file watcher: {e}"), None))?;

        watcher
            .watch(&watch_path_buf, notify::RecursiveMode::Recursive)
            .map_err(|e| AppError::io_error(format!("Failed to start watching path: {e}"), None))?;

        std::thread::spawn(move || {
            for res in notify_rx {
                let event = match res {
                    Ok(e) => {
                        let kind = match e.kind {
                            notify::EventKind::Create(_) => WatchEventKind::Create,
                            notify::EventKind::Modify(_) => WatchEventKind::Modify,
                            notify::EventKind::Remove(_) => WatchEventKind::Remove,
                            _ => WatchEventKind::Other,
                        };
                        WatchEvent {
                            kind,
                            paths: e.paths,
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "NotifyWatcher: event error, skipping");
                        continue;
                    }
                };
                if watch_tx.send(event).is_err() {
                    break;
                }
            }
        });

        let runner = WatcherRunner::new(
            self.repo.cas().clone(),
            self.repo.metadata_store().clone(),
            Arc::clone(self.repo.search_engine()),
            watch_path_buf,
            self.workspace_id.clone(),
        );
        let handle = std::thread::spawn(move || runner.run(rx));

        *self.watcher_state.lock() = Some(WatcherState {
            workspace_id: self.workspace_id.clone(),
            watched_path: PathBuf::from(watch_path),
            file_offsets: HashMap::new(),
            line_counts: HashMap::new(),
            is_active: true,
            thread_handle: Arc::new(parking_lot::Mutex::new(Some(handle))),
            watcher: Arc::new(parking_lot::Mutex::new(Some(watcher))),
        });

        Ok(())
    }

    async fn stop_watch(&self) -> Result<()> {
        let mut state = self.watcher_state.lock();

        let Some(ref mut ws) = *state else {
            return Err(AppError::validation_error(
                "No active watcher found for this workspace".to_string(),
            ));
        };

        ws.is_active = false;
        let thread_handle = ws.thread_handle.lock().take();
        let watcher_opt = ws.watcher.lock().take();

        *state = None;
        drop(state);

        drop(watcher_opt);

        if let Some(handle) = thread_handle {
            if handle.join().is_err() {
                error!("Failed to join watcher thread");
            }
        }

        Ok(())
    }

    async fn is_watching(&self) -> Result<bool> {
        let guard = self.watcher_state.lock();
        Ok(guard.as_ref().map(|w| w.is_active).unwrap_or(false))
    }
}
