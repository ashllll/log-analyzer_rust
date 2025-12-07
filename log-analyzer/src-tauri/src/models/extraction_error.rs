//! 解压错误追踪模块
//!
//! 提供解压过程中的错误收集、分类和报告功能。
//! 支持详细的错误信息记录,为前端提供完整的错误报告。

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 错误类型枚举
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "details")]
pub enum ExtractionErrorType {
    /// 路径过长
    PathTooLong {
        actual_length: usize,
        max_length: usize,
    },
    /// 权限不足
    PermissionDenied,
    /// IO错误
    IoError { error_message: String },
    /// 不安全的路径
    UnsafePath { reason: String },
    /// 编码错误
    EncodingError,
    /// 磁盘空间不足
    DiskFull,
    /// 压缩包损坏
    CorruptedArchive,
    /// 不支持的格式
    UnsupportedFormat,
}

impl ExtractionErrorType {
    /// 获取错误类型的友好描述
    pub fn description(&self) -> String {
        match self {
            ExtractionErrorType::PathTooLong {
                actual_length,
                max_length,
            } => {
                format!("路径过长({}字符,最大{}字符)", actual_length, max_length)
            }
            ExtractionErrorType::PermissionDenied => "权限不足".to_string(),
            ExtractionErrorType::IoError { error_message } => {
                format!("IO错误: {}", error_message)
            }
            ExtractionErrorType::UnsafePath { reason } => {
                format!("不安全路径: {}", reason)
            }
            ExtractionErrorType::EncodingError => "编码错误".to_string(),
            ExtractionErrorType::DiskFull => "磁盘空间不足".to_string(),
            ExtractionErrorType::CorruptedArchive => "压缩包损坏".to_string(),
            ExtractionErrorType::UnsupportedFormat => "不支持的格式".to_string(),
        }
    }

    /// 获取建议信息
    pub fn suggestion(&self) -> Option<String> {
        match self {
            ExtractionErrorType::PathTooLong { .. } => {
                Some("文件名过长已跳过,不影响其他文件".to_string())
            }
            ExtractionErrorType::PermissionDenied => Some("检查文件权限设置".to_string()),
            ExtractionErrorType::IoError { .. } => Some("文件可能损坏或正在被使用".to_string()),
            ExtractionErrorType::UnsafePath { .. } => {
                Some("文件路径不安全已跳过,这可能是恶意压缩包".to_string())
            }
            ExtractionErrorType::EncodingError => Some("文件名编码无法识别".to_string()),
            ExtractionErrorType::DiskFull => Some("请释放磁盘空间后重试".to_string()),
            ExtractionErrorType::CorruptedArchive => Some("压缩包文件可能损坏".to_string()),
            ExtractionErrorType::UnsupportedFormat => Some("该压缩格式不支持".to_string()),
        }
    }
}

/// 错误详情结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtractionError {
    /// 失败的文件路径(相对于压缩包)
    pub file_path: String,
    /// 错误类型
    pub error_type: ExtractionErrorType,
    /// 详细错误信息
    pub error_message: String,
    /// 给用户的建议
    pub suggestion: Option<String>,
    /// 错误发生时间(ISO 8601格式)
    pub timestamp: String,
}

impl ExtractionError {
    /// 创建新的错误记录
    pub fn new(file_path: String, error_type: ExtractionErrorType, error_message: String) -> Self {
        let suggestion = error_type.suggestion();
        let timestamp = chrono::Utc::now().to_rfc3339();

        Self {
            file_path,
            error_type,
            error_message,
            suggestion,
            timestamp,
        }
    }

    /// 从 IO错误创建
    #[allow(dead_code)]
    pub fn from_io_error(file_path: String, error: std::io::Error) -> Self {
        let error_type = match error.kind() {
            std::io::ErrorKind::PermissionDenied => ExtractionErrorType::PermissionDenied,
            std::io::ErrorKind::NotFound => ExtractionErrorType::IoError {
                error_message: "文件未找到".to_string(),
            },
            _ => ExtractionErrorType::IoError {
                error_message: error.to_string(),
            },
        };

        Self::new(file_path, error_type, error.to_string())
    }
}

/// 解压结果摘要
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtractionSummary {
    /// 工作区ID
    pub workspace_id: String,
    /// 总条目数
    pub total_entries: usize,
    /// 成功数量
    pub success_count: usize,
    /// 跳过数量(目录等)
    pub skipped_count: usize,
    /// 失败数量
    pub failed_count: usize,
    /// 详细错误列表
    pub errors: Vec<ExtractionError>,
    /// 解压耗时(毫秒)
    pub duration_ms: u64,
    /// 压缩包类型
    pub archive_type: String,
}

impl ExtractionSummary {
    /// 创建空的摘要
    #[allow(dead_code)]
    pub fn new(workspace_id: String, archive_type: String) -> Self {
        Self {
            workspace_id,
            total_entries: 0,
            success_count: 0,
            skipped_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            duration_ms: 0,
            archive_type,
        }
    }

    /// 判断是否有错误
    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 获取成功率
    #[allow(dead_code)]
    pub fn success_rate(&self) -> f64 {
        if self.total_entries == 0 {
            100.0
        } else {
            (self.success_count as f64 / self.total_entries as f64) * 100.0
        }
    }
}

/// 错误统计摘要
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorTypeSummary {
    /// 错误类型描述
    pub error_type: String,
    /// 出现次数
    pub count: usize,
}

/// 解压元数据
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtractionMetadata {
    /// 工作区ID
    pub workspace_id: String,
    /// 原始文件名
    pub source_archive: String,
    /// 原始文件路径
    pub source_path: String,
    /// 解压时间(ISO 8601格式)
    pub extraction_time: String,
    /// 解压耗时(毫秒)
    pub extraction_duration_ms: u64,
    /// 总条目数
    pub total_entries: usize,
    /// 成功文件数
    pub successful_files: usize,
    /// 失败文件数
    pub failed_files: usize,
    /// 总大小(字节)
    pub total_size_bytes: u64,
    /// 提取器版本
    pub extractor_version: String,
    /// 错误摘要
    pub error_summary: Vec<ErrorTypeSummary>,
}

/// 错误收集器
#[derive(Debug)]
pub struct ErrorCollector {
    /// 错误列表
    errors: Vec<ExtractionError>,
    /// 开始时间
    start_time: Instant,
}

impl ErrorCollector {
    /// 创建新的错误收集器
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ExtractionError) {
        self.errors.push(error);
    }

    /// 添加IO错误
    #[allow(dead_code)]
    pub fn add_io_error(&mut self, file_path: String, error: std::io::Error) {
        let extraction_error = ExtractionError::from_io_error(file_path, error);
        self.add_error(extraction_error);
    }

    /// 添加路径安全错误
    pub fn add_unsafe_path_error(&mut self, file_path: String, reason: String) {
        let error = ExtractionError::new(
            file_path,
            ExtractionErrorType::UnsafePath {
                reason: reason.clone(),
            },
            reason,
        );
        self.add_error(error);
    }

    /// 获取错误数量
    #[allow(dead_code)]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 获取所有错误
    #[allow(dead_code)]
    pub fn errors(&self) -> &[ExtractionError] {
        &self.errors
    }

    /// 构建解压摘要
    pub fn into_summary(
        self,
        workspace_id: String,
        total_entries: usize,
        success_count: usize,
        skipped_count: usize,
        archive_type: String,
    ) -> ExtractionSummary {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        let failed_count = self.errors.len();

        ExtractionSummary {
            workspace_id,
            total_entries,
            success_count,
            skipped_count,
            failed_count,
            errors: self.errors,
            duration_ms,
            archive_type,
        }
    }
}

impl Default for ErrorCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_error_new() {
        let error = ExtractionError::new(
            "test/file.log".to_string(),
            ExtractionErrorType::PermissionDenied,
            "Permission denied".to_string(),
        );

        assert_eq!(error.file_path, "test/file.log");
        assert_eq!(error.error_type, ExtractionErrorType::PermissionDenied);
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_extraction_error_from_io_error() {
        let io_error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let error = ExtractionError::from_io_error("test/file.log".to_string(), io_error);

        assert_eq!(error.file_path, "test/file.log");
        assert_eq!(error.error_type, ExtractionErrorType::PermissionDenied);
    }

    #[test]
    fn test_error_type_description() {
        let error_type = ExtractionErrorType::PathTooLong {
            actual_length: 300,
            max_length: 255,
        };
        let desc = error_type.description();
        assert!(desc.contains("300"));
        assert!(desc.contains("255"));
    }

    #[test]
    fn test_error_type_suggestion() {
        let error_type = ExtractionErrorType::PermissionDenied;
        let suggestion = error_type.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("权限"));
    }

    #[test]
    fn test_error_collector_new() {
        let collector = ErrorCollector::new();
        assert_eq!(collector.error_count(), 0);
    }

    #[test]
    fn test_error_collector_add_error() {
        let mut collector = ErrorCollector::new();

        let error = ExtractionError::new(
            "test/file.log".to_string(),
            ExtractionErrorType::PermissionDenied,
            "Permission denied".to_string(),
        );

        collector.add_error(error);
        assert_eq!(collector.error_count(), 1);
    }

    #[test]
    fn test_error_collector_add_io_error() {
        let mut collector = ErrorCollector::new();
        let io_error = std::io::Error::from(std::io::ErrorKind::NotFound);

        collector.add_io_error("test/file.log".to_string(), io_error);
        assert_eq!(collector.error_count(), 1);
    }

    #[test]
    fn test_error_collector_add_unsafe_path_error() {
        let mut collector = ErrorCollector::new();

        collector
            .add_unsafe_path_error("test/../etc/passwd".to_string(), "包含路径穿越".to_string());

        assert_eq!(collector.error_count(), 1);
        let errors = collector.errors();
        assert!(matches!(
            errors[0].error_type,
            ExtractionErrorType::UnsafePath { .. }
        ));
    }

    #[test]
    fn test_error_collector_into_summary() {
        let mut collector = ErrorCollector::new();

        collector.add_io_error(
            "test/file.log".to_string(),
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );

        let summary =
            collector.into_summary("workspace-123".to_string(), 100, 95, 4, "ZIP".to_string());

        assert_eq!(summary.workspace_id, "workspace-123");
        assert_eq!(summary.total_entries, 100);
        assert_eq!(summary.success_count, 95);
        assert_eq!(summary.skipped_count, 4);
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.archive_type, "ZIP");
        assert!(summary.has_errors());
    }

    #[test]
    fn test_extraction_summary_success_rate() {
        let mut summary = ExtractionSummary::new("workspace-123".to_string(), "ZIP".to_string());
        summary.total_entries = 100;
        summary.success_count = 95;

        assert_eq!(summary.success_rate(), 95.0);
    }

    #[test]
    fn test_extraction_summary_success_rate_zero_entries() {
        let summary = ExtractionSummary::new("workspace-123".to_string(), "ZIP".to_string());
        assert_eq!(summary.success_rate(), 100.0);
    }

    #[test]
    fn test_extraction_summary_has_errors() {
        let mut summary = ExtractionSummary::new("workspace-123".to_string(), "ZIP".to_string());
        assert!(!summary.has_errors());

        summary.errors.push(ExtractionError::new(
            "test/file.log".to_string(),
            ExtractionErrorType::PermissionDenied,
            "Permission denied".to_string(),
        ));

        assert!(summary.has_errors());
    }

    #[test]
    fn test_serialization() {
        let summary = ExtractionSummary {
            workspace_id: "workspace-123".to_string(),
            total_entries: 100,
            success_count: 95,
            skipped_count: 4,
            failed_count: 1,
            errors: vec![ExtractionError::new(
                "test/file.log".to_string(),
                ExtractionErrorType::PermissionDenied,
                "Permission denied".to_string(),
            )],
            duration_ms: 1000,
            archive_type: "ZIP".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ExtractionSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.workspace_id, summary.workspace_id);
        assert_eq!(deserialized.failed_count, summary.failed_count);
    }
}
