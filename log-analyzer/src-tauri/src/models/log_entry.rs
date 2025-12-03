use serde::{Deserialize, Serialize};

use crate::services::MatchDetail;

/// 日志条目
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub id: usize,
    pub timestamp: String,
    pub level: String,
    pub file: String,
    pub real_path: String,
    pub line: usize,
    pub content: String,
    pub tags: Vec<String>,
    /// 匹配详情（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_details: Option<Vec<MatchDetail>>,
}

/// 任务进度
#[derive(Serialize, Clone)]
pub struct TaskProgress {
    pub task_id: String,
    pub task_type: String, // 任务类型: "Import", "Export", etc.
    pub target: String,    // 目标路径或文件名
    pub status: String,
    pub message: String,
    pub progress: u8,
    pub workspace_id: Option<String>, // 工作区 ID，用于前端匹配
}

/// 文件变化事件
#[derive(Serialize, Clone, Debug)]
pub struct FileChangeEvent {
    pub event_type: String,   // "modified", "created", "deleted"
    pub file_path: String,    // 变化的文件路径
    pub workspace_id: String, // 所属工作区
    pub timestamp: i64,       // 事件发生时间戳
}
