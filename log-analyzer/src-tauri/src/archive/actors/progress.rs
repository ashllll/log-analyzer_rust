use super::messages::ProgressUpdate;
use tokio::sync::watch;
use tracing::{error, info};

#[cfg(feature = "standalone")]
use tauri::{AppHandle, Emitter};

/// The Progress Actor aggregates progress from multiple tasks and emits Tauri events
#[cfg(feature = "standalone")]
pub struct ProgressActor {
    app_handle: AppHandle,
    refresh_interval_ms: u64,
}

#[cfg(feature = "standalone")]
impl ProgressActor {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            refresh_interval_ms: 100, // 100ms refresh cycle as per requirements
        }
    }

    /// Monitor a progress channel and emit events
    pub fn monitor_task(&self, mut rx: watch::Receiver<ProgressUpdate>) {
        let app = self.app_handle.clone();
        let interval = self.refresh_interval_ms;

        tokio::spawn(async move {
            let mut last_emit = std::time::Instant::now();

            while rx.changed().await.is_ok() {
                let progress = rx.borrow().clone();

                // Throttle emissions to avoid overwhelming the UI thread
                if last_emit.elapsed() >= std::time::Duration::from_millis(interval) {
                    if let Err(e) = app.emit("task-progress", &progress) {
                        error!("Failed to emit progress event: {}", e);
                    }
                    last_emit = std::time::Instant::now();
                }

                // If task is finished (current_file is Some("Completed")), emit final state
                if let Some(ref status) = progress.current_file {
                    if status == "Completed" || status.starts_with("Error") {
                        let _ = app.emit("task-progress", &progress);
                        break;
                    }
                }
            }
            info!("Progress monitoring finished for task");
        });
    }
}

/// FFI 模式的空实现
#[cfg(not(feature = "standalone"))]
pub struct ProgressActor;

#[cfg(not(feature = "standalone"))]
impl ProgressActor {
    pub fn new() -> Self {
        Self
    }

    pub fn monitor_task(&self, _rx: watch::Receiver<ProgressUpdate>) {
        // FFI 模式下不发送 Tauri 事件
    }
}
