//! FileWatcher domain trait + WatchEvent type.
//!
//! Abstracts file-system event notification behind a trait so that
//! WatcherRunner can be tested with synthetic events instead of depending
//! on the `notify` crate directly.

use std::path::PathBuf;

/// A file-system event, abstracted over `notify::Event`.
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub kind: WatchEventKind,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Create,
    Modify,
    Remove,
    Other,
}

/// Trait for watching file-system changes.
///
/// Production adapter: `NotifyWatcher` (wraps the `notify` crate).
/// Test adapter: `MockFileWatcher` (emits synthetic events).
pub trait FileWatcher: Send + Sync {
    /// Start watching a path. Returns a receiver that yields events until
    /// the watcher is dropped or stopped.
    fn watch(
        &self,
        path: &std::path::Path,
    ) -> Result<
        crossbeam::channel::Receiver<WatchEvent>,
        String,
    >;
}
