//! 智能文件类型过滤器
//!
//! 基于内容分析的智能文件类型检测，包括：
//! - 日志格式检测（时间戳、日志级别、JSON）
//! - 文本可读性评分
//! - 内容采样分析
//!
//! 相比基础 FileTypeFilter 的增强：
//! - 基于内容而非扩展名判断
//! - 智能日志格式识别
//! - 可读性评分机制
//! - 编码检测

use crate::models::{
    config::ArchiveProcessingConfig,
    import_decision::{FileTypeInfo, ImportDecisionDetails, RejectionReason},
};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::{debug, info, warn};

/// 智能文件类型分析器
pub struct IntelligentFileFilter {
    config: ArchiveProcessingConfig,
}

impl IntelligentFileFilter {
    /// 创建新的智能过滤器
    pub fn new(config: ArchiveProcessingConfig) -> Self {
        Self { config }
    }

    /// 分析文件并做出导入决策
    pub fn analyze_file(&self, path: &Path) -> ImportDecisionDetails {
        let start_time = std::time::Instant::now();

        // 获取文件元数据
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                warn!(file = %path.display(), error = %e, "Failed to read file metadata");
                return ImportDecisionDetails::reject(
                    RejectionReason::Other(format!("Cannot read metadata: {}", e)),
                    1.0,
                );
            }
        };

        let file_size = metadata.len();
        let sample_size = self.config.content_sample_size as usize;

        // 检查文件大小限制
        if self.config.max_file_size > 0 && file_size > self.config.max_file_size {
            info!(
                file = %path.display(),
                size = file_size,
                limit = self.config.max_file_size,
                "File size exceeds limit"
            );
            return ImportDecisionDetails::reject(RejectionReason::FileTooLarge, 1.0)
                .with_metadata(crate::models::import_decision::DecisionMetadata {
                    file_path: path.display().to_string(),
                    file_size,
                    analysis_duration_ms: start_time.elapsed().as_millis() as u64,
                    sample_size: 0,
                    nesting_depth: None,
                    used_heuristics: false,
                });
        }

        // 读取文件内容采样
        let mut buffer = vec![0u8; file_size.min(sample_size as u64) as usize];
        match File::open(path).and_then(|mut f| f.read_exact(&mut buffer)) {
            Ok(_) => {}
            Err(e) => {
                warn!(
                    file = %path.display(),
                    error = %e,
                    "Failed to read file content"
                );
                return ImportDecisionDetails::reject(
                    RejectionReason::Other(format!("Cannot read content: {}", e)),
                    1.0,
                );
            }
        }

        // 分析文件类型
        let file_type_info = self.analyze_content(&buffer);

        // 计算可读性评分
        let readability_score = self.calculate_readability(&buffer);

        // 做出决策
        let decision = self.make_decision(&file_type_info, readability_score);

        let elapsed = start_time.elapsed();

        debug!(
            file = %path.display(),
            decision = %decision.decision,
            confidence = decision.confidence,
            file_type = %file_type_info.detected_type,
            is_log = file_type_info.is_log_file,
            duration_ms = elapsed.as_millis(),
            "File analysis complete"
        );

        decision.with_metadata(crate::models::import_decision::DecisionMetadata {
            file_path: path.display().to_string(),
            file_size,
            analysis_duration_ms: elapsed.as_millis() as u64,
            sample_size: buffer.len(),
            nesting_depth: None,
            used_heuristics: true,
        })
    }

    /// 分析内容并检测文件类型
    fn analyze_content(&self, buffer: &[u8]) -> FileTypeInfo {
        // 1. 二进制检测
        if self.is_binary_content(buffer) {
            return FileTypeInfo {
                is_text: false,
                detected_type: "application/binary".to_string(),
                confidence: 0.95,
                encoding: None,
                is_log_file: false,
            };
        }

        // 2. 编码检测
        let (encoding, is_utf8) = self.detect_encoding(buffer);

        // 3. 尝试解码文本
        let text = if is_utf8 {
            std::str::from_utf8(buffer).ok().map(|s| s.to_string())
        } else {
            // 尝试使用检测到的编码
            None // 简化处理，实际可以使用 encoding_rs 解码
        };

        // 4. 日志格式检测
        let (detected_type, is_log_file, confidence) = if let Some(ref text) = text {
            self.detect_log_format(text)
        } else {
            ("text/plain".to_string(), false, 0.5)
        };

        FileTypeInfo {
            is_text: true,
            detected_type,
            confidence,
            encoding: Some(encoding),
            is_log_file,
        }
    }

    /// 检测二进制内容
    fn is_binary_content(&self, buffer: &[u8]) -> bool {
        if buffer.is_empty() {
            return false;
        }

        // 检查魔数
        let binary_magic = [
            (&[0xFF, 0xD8, 0xFF][..], "JPEG"),
            (&[0x89, 0x50, 0x4E, 0x47][..], "PNG"),
            (&[0x47, 0x49, 0x46][..], "GIF"),
            (&[0x42, 0x4D][..], "BMP"),
            (&[0x4D, 0x5A][..], "EXE"),
            (&[0x7F, 0x45, 0x4C, 0x46][..], "ELF"),
            (&[0x50, 0x4B, 0x03, 0x04][..], "ZIP"),
            (&[0x1F, 0x8B][..], "GZ"),
        ];

        for (magic, _name) in &binary_magic {
            if buffer.starts_with(magic) {
                return true;
            }
        }

        // 检查空字节比例
        let null_count = buffer.iter().filter(|&&b| b == 0).count();
        let null_ratio = null_count as f64 / buffer.len() as f64;

        null_ratio > 0.05
    }

    /// 检测文本编码
    fn detect_encoding(&self, buffer: &[u8]) -> (String, bool) {
        // 简化版本：检查UTF-8有效性
        if std::str::from_utf8(buffer).is_ok() {
            ("utf-8".to_string(), true)
        } else {
            // 假设为 Windows-1252（常见于Windows日志）
            ("windows-1252".to_string(), false)
        }
    }

    /// 检测日志格式
    fn detect_log_format(&self, text: &str) -> (String, bool, f64) {
        // 日志级别关键词
        let log_levels = [
            "TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL", "CRITICAL",
        ];

        // ISO 8601 时间戳模式 (简化)
        let has_timestamp = text
            .chars()
            .take(200)
            .any(|c| c == ':' || c == '-' || c == 'T')
            && text.chars().take(200).any(|c| c.is_ascii_digit());

        // 检查日志级别
        let text_upper = text.to_uppercase();
        let has_log_level = log_levels.iter().any(|&level| text_upper.contains(level));

        // 检查JSON格式（更严格的检测）
        let trimmed = text.trim_start();
        let looks_like_json = trimmed.starts_with('{')
            && (trimmed.contains(r#""level""#) || trimmed.contains(r#""message""#));

        // 综合判断
        if looks_like_json {
            ("application/json".to_string(), true, 0.90)
        } else if has_log_level && has_timestamp {
            ("text/x-syslog".to_string(), true, 0.95)
        } else if has_log_level {
            ("text/x-log".to_string(), true, 0.85)
        } else if has_timestamp {
            ("text/x-log".to_string(), true, 0.75)
        } else {
            ("text/plain".to_string(), false, 0.60)
        }
    }

    /// 计算文本可读性评分
    fn calculate_readability(&self, buffer: &[u8]) -> f64 {
        if buffer.is_empty() {
            return 0.0;
        }

        // 如果已确认为二进制，返回低分
        if self.is_binary_content(buffer) {
            return 0.1;
        }

        // 尝试解码为UTF-8
        let text = match std::str::from_utf8(buffer) {
            Ok(t) => t,
            Err(_) => return 0.2, // 不是有效UTF-8
        };

        // 可打印字符比例
        let printable_count = text
            .chars()
            .filter(|c| c.is_ascii_graphic() || c.is_whitespace())
            .count();
        let printable_ratio = printable_count as f64 / text.chars().count() as f64;

        // 字母数字字符比例
        let alnum_count = text.chars().filter(|c| c.is_alphanumeric()).count();
        let alnum_ratio = alnum_count as f64 / text.chars().count() as f64;

        // 换行符比例（文本文件通常有换行）
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        let has_newlines = newline_count > 0;

        // 综合评分
        let mut score = 0.0;
        score += printable_ratio * 0.4;
        score += alnum_ratio * 0.3;
        if has_newlines {
            score += 0.2;
        }
        score += 0.1; // 基础分

        score.min(1.0)
    }

    /// 基于分析结果做出决策
    fn make_decision(
        &self,
        file_type_info: &FileTypeInfo,
        readability_score: f64,
    ) -> ImportDecisionDetails {
        // 如果不是文本文件
        if !file_type_info.is_text {
            return ImportDecisionDetails::reject(
                RejectionReason::BinaryFile,
                file_type_info.confidence,
            );
        }

        // 检查可读性评分
        if readability_score < self.config.min_readability_score {
            return ImportDecisionDetails::reject(RejectionReason::LowReadability, 0.8);
        }

        // 如果是日志文件，高置信度允许
        if file_type_info.is_log_file {
            return ImportDecisionDetails::allow(file_type_info.confidence, file_type_info.clone());
        }

        // 普通文本文件，中等置信度
        ImportDecisionDetails::allow(file_type_info.confidence * 0.8, file_type_info.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ArchiveProcessingConfig {
        ArchiveProcessingConfig {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_total_size: 0,
            max_file_count: 0,
            max_nesting_depth: 15,
            nested_archive_policy: Default::default(),
            file_size_policy: Default::default(),
            enable_intelligent_file_filter: true,
            content_sample_size: 10 * 1024, // 10KB
            min_readability_score: 0.3,
            enable_progress_reporting: true,
            progress_report_interval_ms: 500,
        }
    }

    #[test]
    fn test_binary_detection() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // JPEG文件头
        let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert!(filter.is_binary_content(&jpeg_header));

        // ZIP文件头
        let zip_header = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];
        assert!(filter.is_binary_content(&zip_header));
    }

    #[test]
    fn test_text_detection() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // 纯文本
        let text = b"Hello, world!\nThis is a test.\n";
        assert!(!filter.is_binary_content(text));

        // 带空字节的内容
        let with_nulls = b"Hello\0\0\0\0World";
        assert!(filter.is_binary_content(with_nulls));
    }

    #[test]
    fn test_encoding_detection() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // UTF-8文本
        let utf8_text = "Hello, 世界! 🌍".as_bytes();
        let (encoding, is_utf8) = filter.detect_encoding(utf8_text);
        assert_eq!(encoding, "utf-8");
        assert!(is_utf8);
    }

    #[test]
    fn test_log_format_detection() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // syslog格式
        let syslog = "2024-01-01 12:00:00 INFO Application started\n";
        let (file_type, is_log, confidence) = filter.detect_log_format(syslog);
        assert!(is_log);
        assert!(file_type.contains("log"));
        assert!(confidence > 0.8);

        // JSON格式
        let json_log = r#"{"timestamp":"2024-01-01T12:00:00Z","level":"INFO","message":"Started"}"#;
        let (file_type, is_log, confidence) = filter.detect_log_format(json_log);
        assert!(is_log);
        assert_eq!(file_type, "application/json");
        assert!(confidence > 0.8);

        // 普通文本
        let plain_text = "Just some random text\nWithout any structure";
        let (file_type, is_log, confidence) = filter.detect_log_format(plain_text);
        assert_eq!(file_type, "text/plain");
        assert!(!is_log);
        assert!(confidence < 0.8);
    }

    #[test]
    fn test_readability_scoring() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // 高可读性文本
        let readable = "Line 1\nLine 2\nLine 3\n".as_bytes();
        let score = filter.calculate_readability(readable);
        assert!(score > 0.5);

        // 二进制内容
        let binary = &[0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
        let score = filter.calculate_readability(binary);
        assert!(score < 0.5);
    }

    #[test]
    fn test_decision_making() {
        let filter = IntelligentFileFilter::new(create_test_config());

        // 日志文件
        let log_info = FileTypeInfo {
            is_text: true,
            detected_type: "text/x-syslog".to_string(),
            confidence: 0.95,
            encoding: Some("utf-8".to_string()),
            is_log_file: true,
        };
        let decision = filter.make_decision(&log_info, 0.9);
        assert!(decision.is_allowed());

        // 二进制文件
        let binary_info = FileTypeInfo {
            is_text: false,
            detected_type: "application/binary".to_string(),
            confidence: 0.95,
            encoding: None,
            is_log_file: false,
        };
        let decision = filter.make_decision(&binary_info, 0.1);
        assert!(decision.is_rejected());

        // 低可读性文本
        let low_readability_info = FileTypeInfo {
            is_text: true,
            detected_type: "text/plain".to_string(),
            confidence: 0.5,
            encoding: Some("utf-8".to_string()),
            is_log_file: false,
        };
        let decision = filter.make_decision(&low_readability_info, 0.2);
        assert!(decision.is_rejected());
    }
}
