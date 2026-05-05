//! 存储层共享数据类型

use serde::{Deserialize, Serialize};

/// 文件分析状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisStatus {
    Pending,
    Analyzing,
    Ready,
    Failed,
}

impl AnalysisStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnalysisStatus::Pending => "PENDING",
            AnalysisStatus::Analyzing => "ANALYZING",
            AnalysisStatus::Ready => "READY",
            AnalysisStatus::Failed => "FAILED",
        }
    }
}

impl std::str::FromStr for AnalysisStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(AnalysisStatus::Pending),
            "ANALYZING" => Ok(AnalysisStatus::Analyzing),
            "READY" => Ok(AnalysisStatus::Ready),
            "FAILED" => Ok(AnalysisStatus::Failed),
            _ => Err(format!("Unknown analysis status: {}", s)),
        }
    }
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub size: i64,
    pub modified_time: i64,
    pub mime_type: Option<String>,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
    pub min_timestamp: Option<i64>,
    pub max_timestamp: Option<i64>,
    pub level_mask: Option<u8>,
    pub analysis_status: AnalysisStatus,
}

/// Archive metadata for nested tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub archive_type: String,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
    pub extraction_status: String,
}
