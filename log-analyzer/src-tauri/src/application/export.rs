//! ExportUseCase — application-layer export data transformation.
//!
//! Pure functions that convert search results to CSV/JSON strings.
//! File I/O and path validation remain in the command layer.

use la_core::models::LogEntry;

/// Convert search results to CSV format (UTF-8 BOM + quoted fields).
pub fn transform_csv(entries: &[LogEntry]) -> String {
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

/// Convert search results to pretty-printed JSON format.
pub fn transform_json(entries: &[LogEntry]) -> String {
    let data = serde_json::json!({
        "metadata": {
            "exportTime": chrono::Utc::now().to_rfc3339(),
            "totalCount": entries.len(),
        },
        "results": entries,
    });
    serde_json::to_string_pretty(&data).unwrap_or_default()
}
