//! 导出命令实现（CSV / JSON）
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use std::fs;
use std::io::{BufWriter, Write};

use la_core::error::AppError;
use la_core::models::LogEntry;
use serde_json::json;
use tauri::command;

#[command]
pub async fn export_results(
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, String> {
    // 验证导出路径不包含路径遍历（CMD-NEW-H3 修复）
    let save_path = std::path::Path::new(&savePath);
    if save_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err("导出路径包含非法路径遍历 (..)".to_string());
    }
    // 验证父目录存在且可写
    if let Some(parent) = save_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!("导出目录不存在: {}", parent.display()));
        }
    }

    tokio::task::spawn_blocking(move || match format.as_str() {
        "csv" => export_to_csv(&results, &savePath),
        "json" => export_to_json(&results, &savePath),
        _ => Err(format!("Unsupported export format: {}", format)),
    })
    .await
    .map_err(|e| format!("Export task panicked: {}", e))?
}

fn export_to_csv(results: &[LogEntry], path: &str) -> Result<String, String> {
    let file_path = std::path::PathBuf::from(path);
    let file = std::fs::File::create(path).map_err(|e| {
        AppError::io_error(
            format!("Failed to create CSV file: {e}"),
            Some(file_path.clone()),
        )
        .to_string()
    })?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"\xEF\xBB\xBF").map_err(|e| {
        AppError::io_error(format!("Failed to write BOM: {e}"), Some(file_path.clone())).to_string()
    })?;

    writeln!(writer, "ID,Timestamp,Level,File,Line,Content").map_err(|e| {
        AppError::io_error(
            format!("Failed to write CSV header: {e}"),
            Some(file_path.clone()),
        )
        .to_string()
    })?;

    for entry in results {
        // 按 RFC 4180 规范：含换行的字段用双引号包裹，内部换行保留（不替换为空格）
        // 内部双引号转义为 ""，\r 移除（CSV 行分隔符为 \r\n，内容中的 \r 无意义）
        let content_escaped = entry.content.replace('\"', "\"\"").replace('\r', "");
        let file_escaped = entry.file.replace('\"', "\"\"");

        writeln!(
            writer,
            "{},\"{}\",{},\"{}\",{},\"{}\"",
            entry.id, entry.timestamp, entry.level, file_escaped, entry.line, content_escaped
        )
        .map_err(|e| {
            AppError::io_error(
                format!("Failed to write CSV row: {e}"),
                Some(file_path.clone()),
            )
            .to_string()
        })?;
    }

    writer.flush().map_err(|e| {
        AppError::io_error(format!("Failed to flush CSV file: {e}"), Some(file_path)).to_string()
    })?;

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

    let file_path = std::path::PathBuf::from(path);
    fs::write(path, json_string).map_err(|e| {
        AppError::io_error(format!("Failed to write JSON file: {e}"), Some(file_path)).to_string()
    })?;

    Ok(path.to_string())
}
