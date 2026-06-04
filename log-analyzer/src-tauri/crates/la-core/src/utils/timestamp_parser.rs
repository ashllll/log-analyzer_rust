//! 时间戳解析器 — 从日志行中提取和验证时间戳
//!
//! 由 `services/file_watcher.rs` 提取到 `la_core`，以便被 commands 和 crates 共享。
//! 纯数据转换，无 I/O 或 Tauri 依赖。

use once_cell::sync::Lazy;
use regex::Regex;

/// 时间戳解析器 — 从日志行文本中提取时间戳。
///
/// 支持 ISO 8601、US、EU、Asian、Syslog、Apache 等多种格式。
/// 同时作为搜索过滤器的日期时间解析后端（`parse_naive_datetime`）。
pub struct TimestampParser;

impl TimestampParser {
    /// 支持的主时间戳格式
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

    /// 搜索过滤器额外交互的时间格式。
    ///
    /// 前端 `datetime-local` 默认提交 `YYYY-MM-DDTHH:MM`（无秒/时区），
    /// 搜索过滤器需要额外处理这类不带完整时间的输入。
    const FILTER_FORMATS: &'static [&'static str] = &[
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
    ];

    /// 将时间字符串解析为 `NaiveDateTime`。
    ///
    /// 遍历所有支持的主格式和过滤器格式，返回第一个成功匹配。
    /// 同时尝试 RFC 3339 解析作为后备。
    pub fn parse_naive_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        for format in Self::FORMATS
            .iter()
            .chain(Self::FILTER_FORMATS.iter())
            .copied()
        {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, format) {
                return Some(dt);
            }
        }

        chrono::DateTime::parse_from_rfc3339(trimmed)
            .ok()
            .map(|dt| dt.naive_utc())
    }

    /// 从日志行中解析时间戳字符串。
    ///
    /// 按预编译正则模式匹配，再用 `parse_naive_datetime` 验证。
    /// 返回第一个验证通过的时间戳字符串，若无法识别则返回 `None`。
    pub fn parse_timestamp(line: &str) -> Option<String> {
        static TIMESTAMP_PATTERNS: Lazy<Vec<(Regex, usize)>> = Lazy::new(|| {
            let patterns = [
                (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}", 23), // ISO 8601 with ms
                (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", 19),        // ISO 8601
                (r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}", 23), // Common with ms
                (r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}", 19),        // Common
                (r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}\.\d{3}", 23), // US with ms
                (r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}", 19),        // US
                (r"\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}\.\d{3}", 23), // Asian with ms
                (r"\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}", 19),        // Asian
            ];

            patterns
                .iter()
                .filter_map(|(pat, len)| Regex::new(pat).ok().map(|r| (r, *len)))
                .collect()
        });

        for (pattern, _length) in TIMESTAMP_PATTERNS.iter() {
            if let Some(mat) = pattern.find(line) {
                let timestamp_str = mat.as_str();
                if Self::parse_naive_datetime(timestamp_str).is_some() {
                    return Some(timestamp_str.to_string());
                }
            }
        }

        None
    }
}
