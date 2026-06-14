//! NotifyWatcher — production adapter implementing FileWatcher via `notify`.

use std::sync::Arc;

use crossbeam::channel;
use notify::{Event, EventKind, Watcher};

use crate::application::watch::{FileWatcher, WatchEvent, WatchEventKind};

/// Production adapter that wraps the `notify` crate.
#[derive(Clone)]
pub struct NotifyWatcher {
    _watcher: Arc<notify::RecommendedWatcher>,
    rx: channel::Receiver<std::result::Result<Event, notify::Error>>,
}

impl NotifyWatcher {
    /// Create a new watcher for the given path.
    pub fn new(path: &std::path::Path) -> Result<Self, String> {
        let (tx, rx) = channel::unbounded();
        let mut watcher =
            notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
                let _ = tx.send(res);
            })
            .map_err(|e| format!("Failed to create filesystem watcher: {e}"))?;

        watcher
            .watch(path, notify::RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path {}: {e}", path.display()))?;

        Ok(Self {
            _watcher: Arc::new(watcher),
            rx,
        })
    }
}

fn to_watch_event(res: std::result::Result<Event, notify::Error>) -> Result<WatchEvent, String> {
    let event = res.map_err(|e| format!("Watch error: {e}"))?;
    let kind = match event.kind {
        EventKind::Create(_) => WatchEventKind::Create,
        EventKind::Modify(_) => WatchEventKind::Modify,
        EventKind::Remove(_) => WatchEventKind::Remove,
        _ => WatchEventKind::Other,
    };
    Ok(WatchEvent {
        kind,
        paths: event.paths,
    })
}

impl FileWatcher for NotifyWatcher {
    fn watch(&self, _path: &std::path::Path) -> Result<channel::Receiver<WatchEvent>, String> {
        let (tx, rx) = channel::unbounded();
        let inner_rx = self.rx.clone();

        std::thread::spawn(move || {
            for res in inner_rx {
                match to_watch_event(res) {
                    Ok(event) => {
                        if tx.send(event).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "NotifyWatcher: failed to convert event");
                    }
                }
            }
        });

        Ok(rx)
    }
}
