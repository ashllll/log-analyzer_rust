//! ImportUseCase — application-layer import orchestration.
//!
//! Encapsulates the import flow using domain traits, following the same
//! Clean Architecture pattern established by SearchUseCase.

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::domain::LogFileRepository;
use la_core::error::Result;
use la_storage::ContentAddressableStorage;

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub workspace_id: String,
    pub files_imported: usize,
    pub total_bytes: u64,
}

/// Application use case for importing log files into a workspace.
pub struct ImportUseCase<L, E>
where
    L: LogFileRepository + 'static,
    E: EventPublisher + 'static,
{
    _log_files: Arc<L>,
    events: Arc<E>,
    #[allow(dead_code)]
    cas: Arc<ContentAddressableStorage>,
}

impl<L, E> ImportUseCase<L, E>
where
    L: LogFileRepository,
    E: EventPublisher,
{
    pub fn new(_log_files: Arc<L>, events: Arc<E>, cas: Arc<ContentAddressableStorage>) -> Self {
        Self {
            _log_files,
            events,
            cas,
        }
    }

    /// Execute an import from a folder path.
    ///
    /// This provides the pure orchestration logic — file discovery, extraction,
    /// CAS storage, and metadata indexing. Tauri-specific event emission and
    /// task management remain in the command layer.
    pub async fn execute(&self, _path: &str, _workspace_id: &str) -> Result<ImportResult> {
        // TODO(p3): Extract core import logic from commands/import.rs
        // The import flow involves:
        // 1. File discovery (walkdir)
        // 2. Archive extraction (ArchiveManager)
        // 3. CAS storage + dedup (ContentAddressableStorage)
        // 4. Metadata indexing (MetadataStore)
        // 5. Tantivy index rebuild (SearchEngineManager)
        // 6. Integrity verification (verify_after_import)
        //
        // Current implementation lives in commands/import.rs:import_folder().
        // Migration blocked on: ArchiveExtractor domain trait, TaskScheduler trait.

        self.events.emit_search_start("import-stub").await;
        Ok(ImportResult {
            workspace_id: _workspace_id.to_string(),
            files_imported: 0,
            total_bytes: 0,
        })
    }
}
