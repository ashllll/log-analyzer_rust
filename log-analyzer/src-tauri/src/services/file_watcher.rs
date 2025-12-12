//! 文件监听服务
//!
//! 提供实时文件监听和增量读取功能，支持：
//! - 从指定偏移量读取文件新增内容
//! - 日志行解析（提取时间戳和日志级别）
//! - 增量索引更新
//! - 实时事件推送到前端

use crate::error::{AppError, Result};
use crate::models::log_entry::LogEntry;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// 从指定偏移量读取文件新增内容（支持增量读取）
///
/// # Arguments
///
/// * `path` - 文件路径
/// * `offset` - 上次读取的偏移量（字节）
///
/// # Returns
///
/// 返回元组：(新增的行, 新的偏移量)
///
/// # Errors
///
/// - 文件打开失败
/// - 文件元数据读取失败
/// - 偏移量定位失败
///
/// # 特性
///
/// - **截断检测**：如果文件被截断（大小小于上次偏移量），自动从头读取
/// - **增量读取**：只读取新增内容，避免重复处理
/// - **错误容忍**：单行读取错误不中断整体流程
/// - **大文件优化**：使用大缓冲区提高读取效率
pub fn read_file_from_offset(path: &Path, offset: u64) -> Result<(Vec<String>, u64)> {
    use std::io::{Seek, SeekFrom};

    let mut file = File::open(path).map_err(AppError::Io)?;

    // 获取当前文件大小
    let file_size = file.metadata().map_err(AppError::Io)?.len();

    // 如果文件被截断（小于上次偏移量），从头开始读取
    let start_offset = if file_size < offset {
        eprintln!(
            "[WARNING] File truncated, reading from beginning: {}",
            path.display()
        );
        0
    } else {
        offset
    };

    // 如果没有新内容，直接返回
    if start_offset >= file_size {
        return Ok((Vec::new(), file_size));
    }

    // 移动到偏移量位置
    file.seek(SeekFrom::Start(start_offset))
        .map_err(AppError::Io)?;

    // 读取新增内容 - 使用大缓冲区（64KB）提高大文件读取效率
    let reader = BufReader::with_capacity(65536, file);
    let mut lines = Vec::new();

    for line_result in reader.lines() {
        match line_result {
            Ok(line) => lines.push(line),
            Err(e) => {
                eprintln!("[WARNING] Error reading line: {}", e);
                break;
            }
        }
    }

    eprintln!(
        "[DEBUG] Read {} new lines from {} (offset: {} -> {})",
        lines.len(),
        path.display(),
        start_offset,
        file_size
    );

    Ok((lines, file_size))
}

/// 解析日志行元数据（时间戳和日志级别）
///
/// # Arguments
///
/// * `line` - 日志行内容
///
/// # Returns
///
/// 返回元组：(时间戳, 日志级别)
///
/// # 提取规则
///
/// - **时间戳**：取前19个字符（通常为 ISO 8601 格式）
/// - **日志级别**：按优先级匹配 ERROR > WARN > INFO > DEBUG（默认）
pub fn parse_metadata(line: &str) -> (String, String) {
    let level = if line.contains("ERROR") {
        "ERROR"
    } else if line.contains("WARN") {
        "WARN"
    } else if line.contains("INFO") {
        "INFO"
    } else {
        "DEBUG"
    };
    let timestamp = if line.len() > 19 {
        line[0..19].to_string()
    } else {
        "".to_string()
    };
    (timestamp, level.to_string())
}

/// 解析日志行并创建 LogEntry
///
/// # Arguments
///
/// * `lines` - 日志行列表
/// * `file_path` - 文件虚拟路径（用于显示）
/// * `real_path` - 实际文件路径
/// * `start_id` - 起始 ID（递增）
/// * `start_line_number` - 起始行号
///
/// # Returns
///
/// 返回解析后的 LogEntry 列表
///
/// # 说明
///
/// - 每个日志行生成一个 LogEntry
/// - 无搜索情境下，match_details 字段为 None
/// - ID 和行号自动递增
pub fn parse_log_lines(
    lines: &[String],
    file_path: &str,
    real_path: &str,
    start_id: usize,
    start_line_number: usize,
) -> Vec<LogEntry> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let (timestamp, level) = parse_metadata(line);
            LogEntry {
                id: start_id + i,
                timestamp,
                level,
                file: file_path.to_string(),
                real_path: real_path.to_string(),
                line: start_line_number + i,
                content: line.clone(),
                tags: vec![],
                match_details: None,    // 无搜索情境下不包含匹配详情
                matched_keywords: None, // 无搜索情境下不包含匹配关键词
            }
        })
        .collect()
}

/// 将新日志条目添加到工作区索引（增量更新）
///
/// # Arguments
///
/// * `workspace_id` - 工作区 ID
/// * `new_entries` - 新的日志条目列表
/// * `app` - Tauri 应用句柄
/// * `_state` - 应用状态（为未来扩展保留，可用于持久化索引更新）
///
/// # Returns
///
/// - `Ok(())`: 成功
/// - `Err(String)`: 错误信息
///
/// # 行为
///
/// - 通过 Tauri 事件系统发送新日志到前端（事件名：`new-logs`）
/// - 当前实现不立即持久化索引（性能优化）
/// - 可选择性地批量更新或定期保存索引
pub fn append_to_workspace_index(
    workspace_id: &str,
    new_entries: &[LogEntry],
    app: &AppHandle,
    _state: &crate::models::state::AppState, // 为未来扩展保留（可用于持久化索引更新）
) -> Result<()> {
    if new_entries.is_empty() {
        return Ok(());
    }

    eprintln!(
        "[DEBUG] Appending {} new entries to workspace: {}",
        new_entries.len(),
        workspace_id
    );

    // 发送新日志到前端（实时更新）
    let _ = app.emit("new-logs", new_entries);

    // 这里可以选择性地更新持久化索引
    // 为了性能考虑，可以批量更新或定期保存
    // 当前实现：只发送到前端，不立即持久化

    eprintln!("[DEBUG] New entries sent to frontend");

    Ok(())
}

/// 获取文件元数据（用于增量判断）
///
/// # 功能
///
/// 提取文件的修改时间和大小，用于：
/// - 增量索引更新（判断文件是否变化）
/// - 索引持久化（保存元数据到磁盘）
///
/// # Returns
///
/// - `Ok(FileMetadata)`: 包含 `modified_time` (Unix 时间戳) 和 `size` (字节)
/// - `Err(String)`: 读取失败的错误信息
///
/// # 例子
///
/// ```ignore
/// // 此例子仅用于说明，不会执行
/// use std::path::Path;
/// let metadata = get_file_metadata(Path::new("file.txt"))?;
/// println!("Modified: {}, Size: {}", metadata.modified_time, metadata.size);
/// ```
///
/// # 使用场景
///
/// - ✅ 已集成: `process_path_recursive_inner_with_metadata` 中收集普通文件元数据
pub fn get_file_metadata(path: &Path) -> Result<crate::models::config::FileMetadata> {
    use std::time::SystemTime;

    let metadata = path.metadata().map_err(AppError::Io)?;

    let modified = metadata.modified().map_err(AppError::Io)?;

    let modified_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::validation_error(format!("Invalid timestamp: {}", e)))?
        .as_secs() as i64;

    Ok(crate::models::config::FileMetadata {
        modified_time,
        size: metadata.len(),
    })
}
