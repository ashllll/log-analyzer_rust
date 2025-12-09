//! 导出命令实现（CSV / JSON）

use std::fs;
use std::io::{BufWriter, Write};

use serde_json::json;
use tauri::command;

use crate::models::LogEntry;

#[command]
pub async fn export_results(
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, String> {
    match format.as_str() {
        "csv" => export_to_csv(&results, &savePath),
        "json" => export_to_json(&results, &savePath),
        _ => Err(format!("Unsupported export format: {}", format)),
    }
}

fn export_to_csv(results: &[LogEntry], path: &str) -> Result<String, String> {
    let file =
        std::fs::File::create(path).map_err(|e| format!("Failed to create CSV file: {}", e))?;
    let mut writer = BufWriter::new(file);

    writer
        .write_all(b"\xEF\xBB\xBF")
        .map_err(|e| format!("Failed to write BOM: {}", e))?;

    writeln!(writer, "ID,Timestamp,Level,File,Line,Content")
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    for entry in results {
        let content_escaped = entry
            .content
            .replace('\"', "\"\"")
            .replace('\n', " ")
            .replace('\r', "");
        let file_escaped = entry.file.replace('\"', "\"\"");

        writeln!(
            writer,
            "{},\"{}\",{},\"{}\",{},\"{}\"",
            entry.id, entry.timestamp, entry.level, file_escaped, entry.line, content_escaped
        )
        .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;

    Ok(path.to_string())
}

fn export_to_json(results: &[LogEntry], path: &str) -> Result<String, String> {
    let export_data = json!({
        "metadata": {
            "exportTime": chrono::Utc::now().to_rfc3339(),
            "totalCount": results.len(),
        },
        "results": results,
    });

    let json_string = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    fs::write(path, json_string).map_err(|e| format!("Failed to write JSON file: {}", e))?;

    Ok(path.to_string())
}
