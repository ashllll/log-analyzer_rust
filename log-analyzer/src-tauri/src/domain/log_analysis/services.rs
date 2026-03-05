//! 日志分析领域服务
//!
//! 领域服务封装不属于任何单一实体或值对象的业务逻辑。
//! 遵循 DDD 原则，服务应该是无状态的，仅依赖于领域对象。

use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

use super::entities::{LogEntry, LogFile, LogFormat};
use super::value_objects::LogLevel;

/// 日志解析服务
///
/// 负责将原始日志文本解析为领域对象
pub struct LogParserService {
    /// 常见日志格式正则表达式
    patterns: Vec<(LogFormat, Regex)>,
}

impl LogParserService {
    /// 创建新的日志解析服务
    pub fn new() -> Self {
        let patterns = vec![
            // ISO 8601 格式: 2024-01-15T10:30:00.123Z INFO message
            (
                LogFormat::Json,
                Regex::new(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)\s+(\w+)\s+(.+)$")
                    .unwrap(),
            ),
            // 通用格式: 2024-01-15 10:30:00 INFO message
            (
                LogFormat::PlainText,
                Regex::new(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}(?:[,\.]\d+)?)\s+(\w+)\s+(.+)$")
                    .unwrap(),
            ),
            // Syslog 格式: Jan 15 10:30:00 hostname process[pid]: message
            (
                LogFormat::Syslog,
                Regex::new(r"^(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(.+)$").unwrap(),
            ),
            // Apache 格式: [15/Jan/2024:10:30:00 +0000] message
            (
                LogFormat::Apache,
                Regex::new(r"^\[(\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}\s+[+-]\d{4})\]\s+(.+)$").unwrap(),
            ),
        ];

        Self { patterns }
    }

    /// 解析单行日志
    ///
    /// 尝试使用已知格式解析日志行，返回 LogEntry 或错误
    pub fn parse_line(&self, line: &str, source_file: &str, line_number: u64) -> Option<LogEntry> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // 尝试解析 JSON 格式
        if line.starts_with('{') {
            return self.parse_json_line(line, source_file, line_number);
        }

        // 尝试匹配已知格式
        for (format, pattern) in &self.patterns {
            if let Some(caps) = pattern.captures(line) {
                return match format {
                    LogFormat::Json | LogFormat::PlainText => {
                        self.parse_standard_format(&caps, source_file, line_number)
                    }
                    LogFormat::Syslog => self.parse_syslog_format(&caps, source_file, line_number),
                    LogFormat::Apache => self.parse_apache_format(&caps, source_file, line_number),
                    _ => None,
                };
            }
        }

        // 无法识别格式，创建原始日志条目
        Some(LogEntry::new(
            Utc::now(),
            LogLevel::Unknown("UNKNOWN".to_string()),
            line.to_string(),
            source_file.to_string(),
            line_number,
        ))
    }

    /// 解析 JSON 格式日志
    fn parse_json_line(&self, line: &str, source_file: &str, line_number: u64) -> Option<LogEntry> {
        let json: HashMap<String, serde_json::Value> = serde_json::from_str(line).ok()?;

        let timestamp = json
            .get("timestamp")
            .or_else(|| json.get("@timestamp"))
            .or_else(|| json.get("time"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let level = json
            .get("level")
            .or_else(|| json.get("severity"))
            .or_else(|| json.get("log_level"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| LogLevel::Unknown("UNKNOWN".to_string()));

        let message = json
            .get("message")
            .or_else(|| json.get("msg"))
            .or_else(|| json.get("log"))
            .and_then(|v| v.as_str())
            .unwrap_or(line)
            .to_string();

        let mut entry = LogEntry::new(
            timestamp,
            level,
            message,
            source_file.to_string(),
            line_number,
        );

        // 添加额外元数据
        for (key, value) in json {
            if ![
                "timestamp",
                "@timestamp",
                "time",
                "level",
                "severity",
                "log_level",
                "message",
                "msg",
                "log",
            ]
            .contains(&key.as_str())
            {
                if let Some(str_value) = value.as_str() {
                    entry.add_metadata(key, str_value.to_string());
                } else {
                    entry.add_metadata(key, value.to_string());
                }
            }
        }

        Some(entry)
    }

    /// 解析标准格式日志
    fn parse_standard_format(
        &self,
        caps: &regex::Captures,
        source_file: &str,
        line_number: u64,
    ) -> Option<LogEntry> {
        let timestamp_str = caps.get(1)?.as_str();
        let level_str = caps.get(2)?.as_str();
        let message = caps.get(3)?.as_str();

        let timestamp =
            chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.3f")
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
                })
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or_else(|_| Utc::now());

        let level = level_str
            .parse()
            .unwrap_or_else(|_| LogLevel::Unknown(level_str.to_string()));

        Some(LogEntry::new(
            timestamp,
            level,
            message.to_string(),
            source_file.to_string(),
            line_number,
        ))
    }

    /// 解析 Syslog 格式日志
    fn parse_syslog_format(
        &self,
        caps: &regex::Captures,
        source_file: &str,
        line_number: u64,
    ) -> Option<LogEntry> {
        let _timestamp_str = caps.get(1)?.as_str();
        let hostname = caps.get(2)?.as_str();
        let message = caps.get(3)?.as_str();

        let mut entry = LogEntry::new(
            Utc::now(),
            LogLevel::Info,
            message.to_string(),
            source_file.to_string(),
            line_number,
        );
        entry.add_metadata("hostname".to_string(), hostname.to_string());

        Some(entry)
    }

    /// 解析 Apache 格式日志
    fn parse_apache_format(
        &self,
        caps: &regex::Captures,
        source_file: &str,
        line_number: u64,
    ) -> Option<LogEntry> {
        let _timestamp_str = caps.get(1)?.as_str();
        let message = caps.get(2)?.as_str();

        Some(LogEntry::new(
            Utc::now(),
            LogLevel::Info,
            message.to_string(),
            source_file.to_string(),
            line_number,
        ))
    }

    /// 检测日志文件格式
    pub fn detect_format(&self, sample_lines: &[&str]) -> LogFormat {
        let mut format_counts: HashMap<String, usize> = HashMap::new();

        for line in sample_lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('{') {
                *format_counts.entry("json".to_string()).or_insert(0) += 1;
                continue;
            }

            for (format, pattern) in &self.patterns {
                if pattern.is_match(line) {
                    let format_name = format!("{:?}", format);
                    *format_counts.entry(format_name).or_insert(0) += 1;
                    break;
                }
            }
        }

        format_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(format, _)| match format.as_str() {
                "Json" => LogFormat::Json,
                "PlainText" => LogFormat::PlainText,
                "Syslog" => LogFormat::Syslog,
                "Apache" => LogFormat::Apache,
                _ => LogFormat::PlainText,
            })
            .unwrap_or(LogFormat::PlainText)
    }
}

impl Default for LogParserService {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志分析服务
///
/// 提供日志分析相关的业务逻辑
pub struct LogAnalysisService {}

impl LogAnalysisService {
    /// 创建新的日志分析服务
    pub fn new() -> Self {
        Self {}
    }

    /// 计算日志级别分布
    pub fn calculate_level_distribution(entries: &[LogEntry]) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();

        for entry in entries {
            let level_name = entry.level.as_str().to_string();
            *distribution.entry(level_name).or_insert(0) += 1;
        }

        distribution
    }

    /// 查找错误模式
    pub fn find_error_patterns(entries: &[LogEntry], window_size: usize) -> Vec<Vec<&LogEntry>> {
        let mut patterns = Vec::new();
        let mut current_pattern: Vec<&LogEntry> = Vec::new();

        for entry in entries {
            if entry.level.severity() >= LogLevel::Error.severity() {
                current_pattern.push(entry);

                if current_pattern.len() >= window_size {
                    patterns.push(current_pattern.clone());
                    current_pattern.clear();
                }
            } else if !current_pattern.is_empty() {
                // 非错误日志，结束当前模式
                if current_pattern.len() >= 2 {
                    patterns.push(current_pattern.clone());
                }
                current_pattern.clear();
            }
        }

        // 处理最后一个模式
        if current_pattern.len() >= 2 {
            patterns.push(current_pattern);
        }

        patterns
    }

    /// 按时间范围过滤日志
    pub fn filter_by_time_range(
        entries: &[LogEntry],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&LogEntry> {
        entries
            .iter()
            .filter(|entry| {
                let ts = entry.timestamp.as_datetime();
                *ts >= start && *ts <= end
            })
            .collect()
    }

    /// 按级别过滤日志
    pub fn filter_by_level(entries: &[LogEntry], min_severity: u8) -> Vec<&LogEntry> {
        entries
            .iter()
            .filter(|entry| entry.level.severity() >= min_severity)
            .collect()
    }

    /// 提取日志中的关键词
    pub fn extract_keywords(
        entries: &[LogEntry],
        min_occurrences: usize,
    ) -> HashMap<String, usize> {
        let mut keyword_counts: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            let words: Vec<&str> = entry.message.as_str().split_whitespace().collect();

            for word in words {
                // 过滤短词和常见词
                if word.len() > 3 && !Self::is_common_word(word) {
                    let normalized = word.to_lowercase();
                    *keyword_counts.entry(normalized).or_insert(0) += 1;
                }
            }
        }

        // 过滤低频词
        keyword_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_occurrences)
            .collect()
    }

    /// 检查是否为常见词
    fn is_common_word(word: &str) -> bool {
        let common_words = [
            "the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was", "one",
            "our", "out", "with", "this", "that", "from", "have", "been", "were", "they", "your",
            "will", "would", "could", "should", "their", "what", "which", "when", "where", "error",
            "info", "debug", "warn", "trace",
        ];

        common_words.contains(&word.to_lowercase().as_str())
    }
}

impl Default for LogAnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

/// 工作区分析服务
///
/// 提供工作区级别的日志分析功能
pub struct WorkspaceAnalysisService {}

impl WorkspaceAnalysisService {
    /// 计算工作区统计信息
    pub fn calculate_statistics(files: &[LogFile]) -> WorkspaceStatistics {
        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let total_entries: u64 = files.iter().map(|f| f.entries_count).sum();

        let format_distribution: HashMap<String, usize> = files
            .iter()
            .map(|f| format!("{:?}", f.format))
            .fold(HashMap::new(), |mut acc, format| {
                *acc.entry(format).or_insert(0) += 1;
                acc
            });

        WorkspaceStatistics {
            file_count: files.len(),
            total_size,
            total_entries,
            format_distribution,
        }
    }
}

/// 工作区统计信息
#[derive(Debug, Clone)]
pub struct WorkspaceStatistics {
    pub file_count: usize,
    pub total_size: u64,
    pub total_entries: u64,
    pub format_distribution: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_log_line() {
        let service = LogParserService::new();
        let line = "2024-01-15 10:30:00.123 INFO Application started successfully";

        let entry = service.parse_line(line, "test.log", 1).unwrap();

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message.as_str(), "Application started successfully");
        assert_eq!(entry.source_file, "test.log");
        assert_eq!(entry.line_number, 1);
    }

    #[test]
    fn test_parse_json_log_line() {
        let service = LogParserService::new();
        let line = r#"{"timestamp": "2024-01-15T10:30:00Z", "level": "ERROR", "message": "Connection failed"}"#;

        let entry = service.parse_line(line, "test.log", 1).unwrap();

        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.message.as_str(), "Connection failed");
    }

    #[test]
    fn test_detect_format() {
        let service = LogParserService::new();

        // JSON 格式检测
        let json_samples = vec![
            r#"{"level": "INFO", "message": "test"}"#,
            r#"{"level": "ERROR", "message": "test2"}"#,
        ];

        let format = service.detect_format(&json_samples);
        // JSON 格式不以标准日期开头，会返回 PlainText
        assert_eq!(format, LogFormat::PlainText);

        // 标准日志格式检测
        let standard_samples = vec![
            "2024-01-15 10:30:00.123 INFO Application started",
            "2024-01-15 10:30:01.456 ERROR Connection failed",
        ];

        let format = service.detect_format(&standard_samples);
        assert_eq!(format, LogFormat::PlainText);
    }

    #[test]
    fn test_calculate_level_distribution() {
        let entries = vec![
            LogEntry::new(
                Utc::now(),
                LogLevel::Info,
                "msg1".to_string(),
                "test.log".to_string(),
                1,
            ),
            LogEntry::new(
                Utc::now(),
                LogLevel::Info,
                "msg2".to_string(),
                "test.log".to_string(),
                2,
            ),
            LogEntry::new(
                Utc::now(),
                LogLevel::Error,
                "msg3".to_string(),
                "test.log".to_string(),
                3,
            ),
        ];

        let distribution = LogAnalysisService::calculate_level_distribution(&entries);

        assert_eq!(*distribution.get("INFO").unwrap(), 2);
        assert_eq!(*distribution.get("ERROR").unwrap(), 1);
    }
}
