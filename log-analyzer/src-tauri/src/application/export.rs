//! ExportUseCase — application-layer export orchestration.
//!
//! Encapsulates the export flow (CSV/JSON) using domain traits.

use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::models::LogEntry;

/// Result of an export operation.
#[derive(Debug, Clone)]
pub struct ExportResult {
    pub path: String,
    pub entries_exported: usize,
    pub format: String,
}

/// Application use case for exporting search results.
pub struct ExportUseCase<E>
where
    E: EventPublisher + 'static,
{
    _events: Arc<E>,
}

impl<E> ExportUseCase<E>
where
    E: EventPublisher,
{
    pub fn new(events: Arc<E>) -> Self {
        Self { _events: events }
    }

    /// Export results to CSV or JSON format.
    ///
    /// Tauri-specific I/O (path validation, file writing) remains in the
    /// command layer. This use case handles only the pure data transformation.
    pub fn transform_csv(&self, entries: &[LogEntry]) -> String {
        let mut output = String::from("\u{FEFF}ID,Timestamp,Level,File,Line,Content\n");
        for entry in entries {
            let content = entry.content.replace('\"', "\"\"");
            let file = entry.file.replace('\"', "\"\"");
            output.push_str(&format!(
                "{},\"{}\",{},\"{}\",{},\"{}\"\n",
                entry.id, entry.timestamp, entry.level, file, entry.line, content
            ));
        }
        output
    }

    pub fn transform_json(&self, entries: &[LogEntry]) -> String {
        let data = serde_json::json!({
            "metadata": {
                "exportTime": chrono::Utc::now().to_rfc3339(),
                "totalCount": entries.len(),
            },
            "results": entries,
        });
        serde_json::to_string_pretty(&data).unwrap_or_default()
    }
}
