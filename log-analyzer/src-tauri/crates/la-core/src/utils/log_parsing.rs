//! 日志行解析工具 — 从日志行中提取元数据和创建 LogEntry
//!
//! 由 `services/file_watcher.rs` 提取到 `la_core`，以便被 commands 和 crates 共享。
//! 纯数据转换函数，无 I/O 或 Tauri 依赖。

use once_cell::sync::Lazy;
use regex::Regex;
use tracing::debug;

use crate::utils::timestamp_parser::TimestampParser;

/// 从日志行中提取时间戳和日志级别。
///
/// # 返回
///
/// `(timestamp, log_level)` 元组：
/// - `timestamp`：时间戳字符串，无法识别时为空字符串。
/// - `log_level`：静态字符串，映射为小写（"error" / "warn" / "info" / "debug"），默认为 "debug"。
///
/// # 提取规则
///
/// - **时间戳**：委托给 `TimestampParser::parse_timestamp` 处理。
/// - **日志级别**：使用正则 `\b(ERROR|WARN|INFO|DEBUG)\b` 按优先级匹配：
///   ERROR > WARN > INFO > DEBUG，返回值是零分配的 `&'static str`。
pub fn parse_metadata(line: &str) -> (String, &'static str) {
    static LOG_LEVEL_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\b(ERROR|WARN|INFO|DEBUG)\b")
            .expect("LOG_LEVEL_REGEX is a valid static regex pattern")
    });

    let level: &'static str = LOG_LEVEL_REGEX
        .find(line)
        .map(|m| match m.as_str() {
            "ERROR" => "error",
            "WARN" => "warn",
            "INFO" => "info",
            _ => "debug",
        })
        .unwrap_or("debug");

    let timestamp = TimestampParser::parse_timestamp(line).unwrap_or_else(|| {
        debug!(line = %line, "No timestamp found in line");
        String::new()
    });

    (timestamp, level)
}

/// 将日志行批量解析为 `LogEntry` 列表。
///
/// # 参数
///
/// * `lines` — 日志行内容列表。
/// * `file_path` — 文件虚拟路径（用于显示）。
/// * `real_path` — 实际文件路径。
/// * `start_id` — 起始 ID。
/// * `start_line_number` — 起始行号。
///
/// # 注意
///
/// - 无搜索上下文时，`match_details` 和 `matched_keywords` 均为 `None`。
/// - ID 和行号按顺序自动递增。
pub fn parse_log_lines(
    lines: &[String],
    file_path: &str,
    real_path: &str,
    start_id: usize,
    start_line_number: usize,
) -> Vec<crate::models::LogEntry> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let (timestamp, level) = parse_metadata(line);
            crate::models::LogEntry {
                id: start_id + i,
                timestamp: timestamp.into(),
                level: level.into(),
                file: file_path.to_string().into(),
                real_path: real_path.to_string().into(),
                line: start_line_number + i,
                content: line.clone().into(),
                tags: vec![],
                match_details: None,
                matched_keywords: None,
            }
        })
        .collect()
}
