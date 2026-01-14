//! 文件监听服务
//!
//! 提供实时文件监听和增量读取功能，支持：
//! - 从指定偏移量读取文件新增内容
//! - 日志行解析（提取时间戳和日志级别）
//! - 增量索引更新
//! - 实时事件推送到前端

use crate::error::{AppError, Result};
use crate::models::log_entry::LogEntry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// 文件监听器状态
#[derive(Debug, Clone)]
pub struct WatcherState {
    pub workspace_id: String,
    pub watched_path: std::path::PathBuf,
    pub file_offsets: HashMap<String, u64>,
    pub is_active: bool,
}
use tracing::{debug, warn};

/// 时间戳解析器
pub struct TimestampParser;

impl TimestampParser {
    /// 支持的时间戳格式
    const FORMATS: &'static [&'static str] = &[
        "%Y-%m-%dT%H:%M:%S%.f", // ISO 8601 with fractional seconds
        "%Y-%m-%dT%H:%M:%S",    // ISO 8601
        "%Y-%m-%d %H:%M:%S%.f", // Common format with fractional seconds
        "%Y-%m-%d %H:%M:%S",    // Common format
        "%d/%m/%Y %H:%M:%S%.f", // European format with fractional seconds
        "%d/%m/%Y %H:%M:%S",    // European format
        "%m/%d/%Y %H:%M:%S%.f", // US format with fractional seconds
        "%m/%d/%Y %H:%M:%S",    // US format
        "%Y/%m/%d %H:%M:%S%.f", // Asian format with fractional seconds
        "%Y/%m/%d %H:%M:%S",    // Asian format
        "%d-%m-%Y %H:%M:%S",    // Additional formats
        "%m-%d-%Y %H:%M:%S",
        "%Y%m%d %H:%M:%S",
        "%b %d %H:%M:%S",    // Syslog format
        "%d/%b/%Y:%H:%M:%S", // Apache format
    ];

    /// 解析时间戳
    pub fn parse_timestamp(line: &str) -> Option<String> {
        use once_cell::sync::Lazy;
        use regex::Regex;

        // 使用 Lazy 静态初始化正则表达式，避免重复编译
        static TIMESTAMP_PATTERNS: Lazy<Vec<(Regex, usize)>> = Lazy::new(|| {
            vec![
                (
                    Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}").unwrap(),
                    23,
                ), // ISO 8601 with ms
                (
                    Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}").unwrap(),
                    19,
                ), // ISO 8601
                (
                    Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}").unwrap(),
                    23,
                ), // Common with ms
                (
                    Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap(),
                    19,
                ), // Common
                (
                    Regex::new(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}\.\d{3}").unwrap(),
                    23,
                ), // US with ms
                (
                    Regex::new(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}").unwrap(),
                    19,
                ), // US
                (
                    Regex::new(r"\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}\.\d{3}").unwrap(),
                    23,
                ), // Asian with ms
                (
                    Regex::new(r"\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}").unwrap(),
                    19,
                ), // Asian
            ]
        });

        for (pattern, _length) in TIMESTAMP_PATTERNS.iter() {
            if let Some(mat) = pattern.find(line) {
                let timestamp_str = mat.as_str();

                // 验证时间戳格式
                for format in Self::FORMATS {
                    if let Ok(_dt) = chrono::NaiveDateTime::parse_from_str(timestamp_str, format) {
                        return Some(timestamp_str.to_string());
                    }
                }
            }
        }

        None
    }
}

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
/// - **资源安全**：使用作用域确保文件句柄正确关闭
pub fn read_file_from_offset(path: &Path, offset: u64) -> Result<(Vec<String>, u64)> {
    use std::io::{Seek, SeekFrom};

    // 使用作用域确保文件句柄自动关闭，防止资源泄漏
    let (lines, file_size, start_offset) = {
        let mut file = File::open(path).map_err(AppError::Io)?;

        // 获取当前文件大小
        let file_size = file.metadata().map_err(AppError::Io)?.len();

        // If file was truncated (smaller than last offset), read from beginning
        let start_offset = if file_size < offset {
            warn!(
                file = %path.display(),
                "File truncated, reading from beginning"
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

        // Read new content - use large buffer (64KB) for better performance with large files
        let reader = BufReader::with_capacity(65536, file);
        let mut lines = Vec::new();

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => lines.push(line),
                Err(e) => {
                    warn!(
                        error = %e,
                        "Error reading line, continuing with next line"
                    );
                    // Continue reading instead of breaking to avoid losing subsequent valid lines
                    continue;
                }
            }
        }

        (lines, file_size, start_offset)
    }; // File handle automatically closed here, preventing resource leaks

    debug!(
        lines_read = lines.len(),
        file = %path.display(),
        offset_start = start_offset,
        offset_end = file_size,
        "Read new lines from file"
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
/// - **时间戳**：使用改进的时间戳解析器
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

    // 使用改进的时间戳解析器
    let timestamp = TimestampParser::parse_timestamp(line).unwrap_or_default();

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

    debug!(
        entries_count = new_entries.len(),
        workspace_id = %workspace_id,
        "Appending new entries to workspace"
    );

    // Send new logs to frontend (real-time update)
    let _ = app.emit("new-logs", new_entries);

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
pub fn get_file_metadata(path: &Path) -> Result<crate::storage::FileMetadata> {
    use std::time::SystemTime;

    let metadata = path.metadata().map_err(AppError::Io)?;

    let modified = metadata.modified().map_err(AppError::Io)?;

    let modified_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::validation_error(format!("Invalid timestamp: {}", e)))?
        .as_secs() as i64;

    Ok(crate::storage::FileMetadata {
        id: 0,                       // Will be auto-generated
        sha256_hash: String::new(),  // Will be filled by caller
        virtual_path: String::new(), // Will be filled by caller
        original_name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        size: metadata.len() as i64,
        modified_time,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_parser_iso8601() {
        let line = "2024-01-15T10:30:45.123 [INFO] Application started";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15T10:30:45.123");
    }

    #[test]
    fn test_timestamp_parser_common() {
        let line = "2024-01-15 10:30:45 [ERROR] Database connection failed";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15 10:30:45");
    }

    #[test]
    fn test_timestamp_parser_us() {
        let line = "01/15/2024 10:30:45.456 [WARN] Low memory warning";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "01/15/2024 10:30:45.456");
    }

    #[test]
    fn test_timestamp_parser_no_match() {
        let line = "This is a log line without timestamp";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_none());
    }
}
