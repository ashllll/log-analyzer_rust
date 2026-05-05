//! 文件统计计算（纯函数，无外部 crate 依赖）
//!
//! 从主 crate 的 `compute_file_stats` 复制，用于 la-archive 内部在 CAS 存储后
//! 立即计算文件的时间戳范围和日志级别掩码，支持增量分析。

use once_cell::sync::Lazy;
use regex::Regex;

/// 解析单行日志元数据（时间戳和级别）
///
/// 从 `file_watcher::parse_metadata` 复制，避免依赖主 crate。
fn parse_metadata(line: &str) -> (String, String) {
    static LOG_LEVEL_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\b(ERROR|WARN|INFO|DEBUG)\b")
            .expect("LOG_LEVEL_REGEX is a valid static regex pattern")
    });

    let level = LOG_LEVEL_REGEX
        .find(line)
        .map(|m| m.as_str())
        .unwrap_or("DEBUG");

    let timestamp = parse_timestamp(line).unwrap_or_default();

    (timestamp, level.to_string())
}

/// 解析日志行中的时间戳
///
/// 支持多种常见格式，从 `la_search::parse_log_timestamp_to_unix` 简化而来。
fn parse_timestamp(line: &str) -> Option<String> {
    // 常见时间戳正则模式（按优先级排序）
    static TS_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
            (\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:\s*[+-]\d{2}:?\d{2})?)
            |(\d{2}/\d{2}/\d{4}\s+\d{2}:\d{2}:\d{2})
            |(\d{2}-\d{2}-\d{4}\s+\d{2}:\d{2}:\d{2})
            |(\d{4}/\d{2}/\d{2}\s+\d{2}:\d{2}:\d{2})
            |(\d{4}\d{2}\d{2}\s+\d{2}:\d{2}:\d{2})
        ",
        )
        .expect("TS_REGEX is valid")
    });

    TS_REGEX.find(line).map(|m| m.as_str().to_string())
}

/// 将时间戳字符串解析为 Unix 时间戳（秒）
///
/// 支持纯数字（毫秒/秒）和多种日期时间格式。
fn parse_log_timestamp_to_unix(timestamp: &str) -> Option<i64> {
    let trimmed = timestamp.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(raw) = trimmed.parse::<i64>() {
        return Some(if trimmed.len() >= 13 { raw / 1000 } else { raw });
    }

    const FORMATS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.3f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.3f",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.3f%:z",
        "%Y/%m/%d %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%d-%m-%Y %H:%M:%S",
        "%Y%m%d %H:%M:%S",
    ];

    for format in FORMATS {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(dt.and_utc().timestamp());
        }
    }

    chrono::DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.timestamp())
}

/// 计算文件内容统计信息
///
/// 返回 `(min_timestamp, max_timestamp, level_mask)`，用于增量分析。
///
/// # Arguments
/// * `content` — 文件原始内容（字节）
///
/// # Returns
/// `(min_timestamp, max_timestamp, level_mask)`
pub fn compute_file_stats(content: &[u8]) -> (Option<i64>, Option<i64>, Option<u8>) {
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return (None, None, None),
    };

    // 大文件优化：超过 10MB 只解析前 1000 行和后 1000 行
    const MAX_FULL_PARSE_BYTES: usize = 10 * 1024 * 1024;
    const MAX_LINES_SAMPLE: usize = 1000;

    let mut min_ts: Option<i64> = None;
    let mut max_ts: Option<i64> = None;
    let mut level_mask: u8 = 0;
    let mut has_any_level = false;

    let mut process_line = |line: &str| {
        if line.is_empty() {
            return;
        }
        let (timestamp_str, level) = parse_metadata(line);
        if !level.is_empty() {
            has_any_level = true;
            let mask_bit = match level.as_str() {
                "DEBUG" => 1u8 << 0,
                "INFO" => 1u8 << 1,
                "WARN" => 1u8 << 2,
                "ERROR" => 1u8 << 3,
                _ => 0,
            };
            level_mask |= mask_bit;
        }
        if !timestamp_str.is_empty() {
            if let Some(ts) = parse_log_timestamp_to_unix(&timestamp_str) {
                min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
                max_ts = Some(max_ts.map_or(ts, |m| m.max(ts)));
            }
        }
    };

    if text.len() > MAX_FULL_PARSE_BYTES {
        let all_lines: Vec<&str> = text.lines().collect();
        let total = all_lines.len();
        if total > MAX_LINES_SAMPLE * 2 {
            for line in &all_lines[..MAX_LINES_SAMPLE] {
                process_line(line);
            }
            for line in &all_lines[total - MAX_LINES_SAMPLE..] {
                process_line(line);
            }
        } else {
            for line in &all_lines {
                process_line(line);
            }
        }
    } else {
        for line in text.lines() {
            process_line(line);
        }
    }

    (
        min_ts,
        max_ts,
        if has_any_level {
            Some(level_mask)
        } else {
            None
        },
    )
}
