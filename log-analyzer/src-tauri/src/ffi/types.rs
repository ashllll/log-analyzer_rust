//! FFI 专用类型定义
//!
//! 这些类型专门用于 Flutter Rust Bridge 的数据传输。
//! 大多数情况下复用现有的 models 类型，但对于 FFI 特定的场景，
//! 这里定义了额外的数据结构。

use serde::{Deserialize, Serialize};

/// 工作区数据（FFI 格式）
///
/// 对应 Dart 端的 Workspace 模型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceData {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub size: String,
    pub files: i32,
    pub watching: Option<bool>,
}

/// 工作区状态数据（FFI 格式）
///
/// 用于 ffi_get_workspace_status 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceStatusData {
    pub id: String,
    pub name: String,
    pub status: String,
    pub size: String,
    pub files: i32,
}

/// 工作区加载响应数据（FFI 格式）
///
/// 用于 ffi_load_workspace 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceLoadResponseData {
    pub workspace_id: String,
    pub status: String,
    pub file_count: i32,
    pub total_size: String,
}

/// 关键词组数据（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeywordGroupData {
    pub id: String,
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
}

/// 关键词组输入（用于创建/更新）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeywordGroupInput {
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
}

/// 任务信息数据（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskInfoData {
    pub task_id: String,
    pub target: String,
    pub message: String,
    pub status: String,
    pub progress: i32,
}

/// 任务指标数据（FFI 格式）
///
/// 用于 ffi_get_task_metrics 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskMetricsData {
    pub total_tasks: i32,
    pub running_tasks: i32,
    pub completed_tasks: i32,
    pub failed_tasks: i32,
    pub stopped_tasks: i32,
}

impl From<crate::task_manager::TaskInfo> for TaskInfoData {
    fn from(task: crate::task_manager::TaskInfo) -> Self {
        use crate::task_manager::TaskStatus;
        // 转换状态为字符串
        let status_str = match task.status {
            TaskStatus::Running => "Running".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::Failed => "Failed".to_string(),
            TaskStatus::Stopped => "Stopped".to_string(),
        };

        Self {
            task_id: task.task_id,
            target: task.target,
            message: task.message,
            status: status_str,
            progress: task.progress as i32,
        }
    }
}

/// 文件过滤器配置（FFI 格式）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileFilterConfigData {
    pub enabled: bool,
    pub binary_detection_enabled: bool,
    pub mode: String,
    pub filename_patterns: Vec<String>,
    pub allowed_extensions: Vec<String>,
    pub forbidden_extensions: Vec<String>,
}

/// 高级功能配置（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvancedFeaturesConfigData {
    pub enable_filter_engine: bool,
    pub enable_regex_engine: bool,
    pub enable_time_partition: bool,
    pub enable_autocomplete: bool,
    pub regex_cache_size: i32,
    pub autocomplete_limit: i32,
    pub time_partition_size_secs: i32,
}

impl Default for AdvancedFeaturesConfigData {
    fn default() -> Self {
        Self {
            enable_filter_engine: false,
            enable_regex_engine: true,
            enable_time_partition: false,
            enable_autocomplete: true,
            regex_cache_size: 1000,
            autocomplete_limit: 100,
            time_partition_size_secs: 3600,
        }
    }
}

/// 配置数据（FFI 格式）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigData {
    pub file_filter: FileFilterConfigData,
    pub advanced_features: AdvancedFeaturesConfigData,
}

/// 性能指标数据（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetricsData {
    pub search_latency: f64,
    pub search_throughput: f64,
    pub cache_hit_rate: f64,
    pub cache_size: i32,
    pub total_queries: i32,
    pub cache_hits: i32,
    pub latency_history: Vec<f64>,
    pub avg_latency: f64,
}

impl Default for PerformanceMetricsData {
    fn default() -> Self {
        Self {
            search_latency: 0.0,
            search_throughput: 0.0,
            cache_hit_rate: 0.0,
            cache_size: 0,
            total_queries: 0,
            cache_hits: 0,
            latency_history: vec![],
            avg_latency: 0.0,
        }
    }
}

/// 搜索过滤器（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchFiltersData {
    pub time_range: Option<TimeRangeData>,
    pub levels: Vec<String>,
    pub file_pattern: Option<String>,
}

/// 时间范围（FFI 格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRangeData {
    pub start: Option<String>,
    pub end: Option<String>,
}

// ==================== Typestate Session 类型 ====================

/// 会话状态枚举（FFI 格式）
///
/// 用于表示 Session 的当前状态
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SessionState {
    /// 未映射状态
    #[default]
    Unmapped,
    /// 已映射状态
    Mapped,
    /// 已索引状态
    Indexed,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Unmapped => write!(f, "Unmapped"),
            SessionState::Mapped => write!(f, "Mapped"),
            SessionState::Indexed => write!(f, "Indexed"),
        }
    }
}

/// 会话信息（FFI 格式）
///
/// 用于 ffi_open_session 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    /// 会话 ID
    pub session_id: String,
    /// 文件路径
    pub file_path: String,
    /// 当前状态
    pub state: SessionState,
    /// 文件大小（字节）
    pub file_size: u64,
}

/// 视口数据（FFI 格式）
///
/// 用于 ffi_get_viewport 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewportData {
    /// 起始偏移
    pub start_offset: u64,
    /// 数据内容（Base64 编码）
    pub data: String,
    /// 数据长度
    pub data_len: usize,
    /// 是否有更多数据
    pub has_more: bool,
}

/// 索引条目（FFI 格式）
///
/// 用于 ffi_get_index_entries 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexEntryData {
    /// 行号
    pub line_number: u64,
    /// 字节偏移
    pub byte_offset: u64,
    /// 行长度
    pub length: u32,
}

/// 行数据（FFI 格式）
///
/// 用于 ffi_get_line 返回
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineData {
    /// 行号
    pub line_number: u64,
    /// 行内容
    pub content: String,
    /// 字节偏移
    pub byte_offset: u64,
    /// 下一行偏移
    pub next_offset: u64,
}

// ==================== 搜索历史类型 ====================

/// 搜索历史条目数据（FFI 格式）
///
/// 用于 Flutter 端搜索历史展示
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHistoryData {
    /// 查询内容
    pub query: String,
    /// 工作区ID
    pub workspace_id: String,
    /// 结果数量
    pub result_count: i32,
    /// 搜索时间（ISO 8601 格式）
    pub searched_at: String,
}

impl From<crate::models::SearchHistoryEntry> for SearchHistoryData {
    fn from(entry: crate::models::SearchHistoryEntry) -> Self {
        Self {
            query: entry.query,
            workspace_id: entry.workspace_id,
            result_count: entry.result_count as i32,
            searched_at: entry.searched_at.to_rfc3339(),
        }
    }
}

// ==================== 虚拟文件树类型 ====================

/// 虚拟文件树节点数据（FFI 格式）
///
/// 用于 Flutter 端虚拟文件树展示
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VirtualTreeNodeData {
    /// 文件节点
    #[serde(rename = "file")]
    File {
        /// 文件名
        name: String,
        /// 虚拟路径
        path: String,
        /// SHA-256 哈希
        hash: String,
        /// 文件大小（字节）
        size: i64,
        /// MIME 类型
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
    /// 归档节点（压缩包）
    #[serde(rename = "archive")]
    Archive {
        /// 归档名
        name: String,
        /// 虚拟路径
        path: String,
        /// SHA-256 哈希
        hash: String,
        /// 归档类型（zip, tar, gz 等）
        #[serde(rename = "archiveType")]
        archive_type: String,
        /// 子节点（懒加载时为空）
        children: Vec<VirtualTreeNodeData>,
    },
}

impl From<crate::commands::virtual_tree::VirtualTreeNode> for VirtualTreeNodeData {
    fn from(node: crate::commands::virtual_tree::VirtualTreeNode) -> Self {
        match node {
            crate::commands::virtual_tree::VirtualTreeNode::File {
                name,
                path,
                hash,
                size,
                mime_type,
            } => VirtualTreeNodeData::File {
                name,
                path,
                hash,
                size,
                mime_type,
            },
            crate::commands::virtual_tree::VirtualTreeNode::Archive {
                name,
                path,
                hash,
                archive_type,
                children,
            } => VirtualTreeNodeData::Archive {
                name,
                path,
                hash,
                archive_type,
                children: children.into_iter().map(VirtualTreeNodeData::from).collect(),
            },
        }
    }
}

/// 文件内容响应数据（FFI 格式）
///
/// 用于通过哈希读取文件内容
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileContentResponseData {
    /// 文件内容（UTF-8 字符串）
    pub content: String,
    /// SHA-256 哈希
    pub hash: String,
    /// 文件大小（字节）
    pub size: i64,
}

impl From<crate::commands::virtual_tree::FileContentResponse> for FileContentResponseData {
    fn from(response: crate::commands::virtual_tree::FileContentResponse) -> Self {
        Self {
            content: response.content,
            hash: response.hash,
            size: response.size as i64,
        }
    }
}
