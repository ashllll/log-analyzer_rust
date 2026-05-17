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
use tauri::{command, AppHandle, Manager};

#[command]
pub async fn export_results(
    app: AppHandle,
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, String> {
    // FIX(CR-02): 强制 savePath 为相对路径，拒绝绝对路径，并限制只能写入下载目录
    let save_path = std::path::Path::new(&savePath);

    // 拒绝绝对路径（包括 RootDir、Prefix 等）
    for component in save_path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("导出路径必须是相对路径，不允许使用绝对路径".to_string());
            }
            std::path::Component::ParentDir => {
                return Err("导出路径包含非法路径遍历 (..)".to_string());
            }
        }
    }

    // 额外的路径遍历防护（覆盖 URL 编码等绕过手段）
    let safe_path = crate::utils::validation::prevent_path_traversal(&savePath)
        .map_err(|e| format!("导出路径不安全: {}", e))?;

    let download_dir = app
        .path()
        .download_dir()
        .map_err(|e| format!("无法获取下载目录: {}", e))?;
    let final_path = download_dir.join(&safe_path);

    // 验证最终路径没有通过符号链接等方式逃逸出下载目录
    if let Ok(canonical_final) = dunce::canonicalize(&final_path) {
        let canonical_download = dunce::canonicalize(&download_dir).unwrap_or(download_dir);
        if !canonical_final.starts_with(&canonical_download) {
            return Err("导出路径不允许超出下载目录范围".to_string());
        }
    }

    tokio::task::spawn_blocking(move || match format.as_str() {
        "csv" => export_to_csv(&results, final_path.to_string_lossy().as_ref()),
        "json" => export_to_json(&results, final_path.to_string_lossy().as_ref()),
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
