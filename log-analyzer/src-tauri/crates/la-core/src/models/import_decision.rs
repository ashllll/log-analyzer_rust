//! 导入决策模型
//!
//! 定义文件导入决策的数据模型，包括决策结果、原因和置信度。

use serde::{Deserialize, Serialize};
use std::fmt;

/// 导入决策结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportDecision {
    /// 允许导入
    Allow,
    /// 拒绝导入
    Reject,
    /// 延迟决策（需要进一步分析）
    Defer,
}

impl fmt::Display for ImportDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportDecision::Allow => write!(f, "Allow"),
            ImportDecision::Reject => write!(f, "Reject"),
            ImportDecision::Defer => write!(f, "Defer"),
        }
    }
}

/// 拒绝原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// 二进制文件
    BinaryFile,
    /// 文件过大
    FileTooLarge,
    /// 压缩炸弹风险
    ZipBombRisk,
    /// 超过嵌套深度限制
    NestingDepthExceeded,
    /// 文件类型不匹配
    FileTypeMismatch,
    /// 内容可读性不足
    LowReadability,
    /// 其他原因
    Other(String),
}

impl fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectionReason::BinaryFile => write!(f, "Binary file detected"),
            RejectionReason::FileTooLarge => write!(f, "File size exceeds limit"),
            RejectionReason::ZipBombRisk => write!(f, "Potential zip bomb detected"),
            RejectionReason::NestingDepthExceeded => write!(f, "Maximum nesting depth exceeded"),
            RejectionReason::FileTypeMismatch => write!(f, "File type does not match filter"),
            RejectionReason::LowReadability => write!(f, "Content readability score too low"),
            RejectionReason::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// 文件类型分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeInfo {
    /// 是否为文本文件
    pub is_text: bool,
    /// 检测到的文件类型
    pub detected_type: String,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 文件编码（如果是文本）
    pub encoding: Option<String>,
    /// 是否为日志文件
    pub is_log_file: bool,
}

impl Default for FileTypeInfo {
    fn default() -> Self {
        Self {
            is_text: false,
            detected_type: "unknown".to_string(),
            confidence: 0.0,
            encoding: None,
            is_log_file: false,
        }
    }
}

/// 导入决策详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecisionDetails {
    /// 决策结果
    pub decision: ImportDecision,

    /// 决策原因（如果拒绝）
    pub rejection_reason: Option<RejectionReason>,

    /// 置信度 (0.0-1.0)
    pub confidence: f64,

    /// 文件类型信息
    pub file_type_info: FileTypeInfo,

    /// 附加信息
    pub metadata: DecisionMetadata,
}

impl Default for ImportDecisionDetails {
    fn default() -> Self {
        Self {
            decision: ImportDecision::Defer,
            rejection_reason: None,
            confidence: 0.0,
            file_type_info: FileTypeInfo::default(),
            metadata: DecisionMetadata::default(),
        }
    }
}

/// 决策元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionMetadata {
    /// 文件路径
    pub file_path: String,

    /// 文件大小（字节）
    pub file_size: u64,

    /// 分析耗时（毫秒）
    pub analysis_duration_ms: u64,

    /// 采样大小（字节）
    pub sample_size: usize,

    /// 嵌套深度（如果是压缩包）
    pub nesting_depth: Option<usize>,

    /// 是否使用了启发式分析
    pub used_heuristics: bool,
}

impl ImportDecisionDetails {
    /// 创建允许决策
    pub fn allow(confidence: f64, file_type_info: FileTypeInfo) -> Self {
        Self {
            decision: ImportDecision::Allow,
            rejection_reason: None,
            confidence,
            file_type_info,
            metadata: DecisionMetadata::default(),
        }
    }

    /// 创建拒绝决策
    pub fn reject(reason: RejectionReason, confidence: f64) -> Self {
        Self {
            decision: ImportDecision::Reject,
            rejection_reason: Some(reason),
            confidence,
            file_type_info: FileTypeInfo::default(),
            metadata: DecisionMetadata::default(),
        }
    }

    /// 创建延迟决策
    pub fn defer(confidence: f64, file_type_info: FileTypeInfo) -> Self {
        Self {
            decision: ImportDecision::Defer,
            rejection_reason: None,
            confidence,
            file_type_info,
            metadata: DecisionMetadata::default(),
        }
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: DecisionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 检查是否允许导入
    pub fn is_allowed(&self) -> bool {
        self.decision == ImportDecision::Allow
    }

    /// 检查是否拒绝导入
    pub fn is_rejected(&self) -> bool {
        self.decision == ImportDecision::Reject
    }

    /// 检查是否延迟决策
    pub fn is_deferred(&self) -> bool {
        self.decision == ImportDecision::Defer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_decision_display() {
        assert_eq!(ImportDecision::Allow.to_string(), "Allow");
        assert_eq!(ImportDecision::Reject.to_string(), "Reject");
        assert_eq!(ImportDecision::Defer.to_string(), "Defer");
    }

    #[test]
    fn test_rejection_reason_display() {
        assert_eq!(
            RejectionReason::BinaryFile.to_string(),
            "Binary file detected"
        );
        assert_eq!(
            RejectionReason::FileTooLarge.to_string(),
            "File size exceeds limit"
        );
        assert_eq!(
            RejectionReason::Other("custom reason".to_string()).to_string(),
            "custom reason"
        );
    }

    #[test]
    fn test_import_decision_details_allow() {
        let file_type_info = FileTypeInfo {
            is_text: true,
            detected_type: "text/log".to_string(),
            confidence: 0.95,
            encoding: Some("utf-8".to_string()),
            is_log_file: true,
        };

        let details = ImportDecisionDetails::allow(0.95, file_type_info.clone());

        assert!(details.is_allowed());
        assert!(!details.is_rejected());
        assert!(!details.is_deferred());
        assert_eq!(details.decision, ImportDecision::Allow);
        assert_eq!(details.confidence, 0.95);
        assert!(details.rejection_reason.is_none());
        assert_eq!(details.file_type_info.detected_type, "text/log");
    }

    #[test]
    fn test_import_decision_details_reject() {
        let details = ImportDecisionDetails::reject(RejectionReason::BinaryFile, 1.0);

        assert!(!details.is_allowed());
        assert!(details.is_rejected());
        assert!(!details.is_deferred());
        assert_eq!(details.decision, ImportDecision::Reject);
        assert_eq!(details.confidence, 1.0);
        assert!(details.rejection_reason.is_some());
        match details.rejection_reason {
            Some(RejectionReason::BinaryFile) => {}
            _ => panic!("Expected BinaryFile rejection reason"),
        }
    }

    #[test]
    fn test_import_decision_details_defer() {
        let file_type_info = FileTypeInfo {
            is_text: true,
            detected_type: "text/unknown".to_string(),
            confidence: 0.5,
            encoding: None,
            is_log_file: false,
        };

        let details = ImportDecisionDetails::defer(0.5, file_type_info);

        assert!(!details.is_allowed());
        assert!(!details.is_rejected());
        assert!(details.is_deferred());
        assert_eq!(details.decision, ImportDecision::Defer);
        assert_eq!(details.confidence, 0.5);
    }

    #[test]
    fn test_with_metadata() {
        let metadata = DecisionMetadata {
            file_path: "/test/path.log".to_string(),
            file_size: 1024,
            analysis_duration_ms: 10,
            sample_size: 0,
            nesting_depth: None,
            used_heuristics: false,
        };

        let details = ImportDecisionDetails::allow(
            0.9,
            FileTypeInfo {
                is_text: true,
                detected_type: "text/log".to_string(),
                confidence: 0.9,
                encoding: Some("utf-8".to_string()),
                is_log_file: true,
            },
        )
        .with_metadata(metadata);

        assert_eq!(details.metadata.file_path, "/test/path.log");
        assert_eq!(details.metadata.file_size, 1024);
        assert_eq!(details.metadata.analysis_duration_ms, 10);
    }

    #[test]
    fn test_file_type_info_default() {
        let info = FileTypeInfo::default();
        assert!(!info.is_text);
        assert_eq!(info.detected_type, "unknown");
        assert_eq!(info.confidence, 0.0);
        assert!(info.encoding.is_none());
        assert!(!info.is_log_file);
    }
}
